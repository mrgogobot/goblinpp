//! Small, offline, single-threaded CPU trainer with bounded data and work.
use crate::{
    ai::ResourceGuard,
    error::{GoblinError, Result},
    hashing::sha256_bytes,
};
use clap::Args;
use serde::Serialize;
use std::{io::Read, path::PathBuf};

const MAX_INPUT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_ROWS: usize = 100_000;
const MAX_STEPS: u64 = 100_000;
const MAX_SECONDS: u64 = 300;
const MAX_ROW_EVALUATIONS: u64 = 10_000_000;
const MAX_ROW_BYTES: usize = 256;
const ROW_CHECK_INTERVAL: usize = 256;

#[derive(Debug, Args)]
pub struct TrainArgs {
    /// Local UTF-8 CSV with an x,y header and two finite numbers per row.
    dataset: PathBuf,
    /// New JSON model file. Existing paths are preserved.
    #[arg(long)]
    output: PathBuf,
    #[command(flatten)]
    config: TrainingConfig,
}

#[derive(Debug, Args, Serialize)]
struct TrainingConfig {
    /// Number of full-batch gradient updates to perform.
    #[arg(long, default_value_t = 100)]
    steps: u64,
    /// Update budget. Requests above the budget are refused before reading input.
    #[arg(long, default_value_t = 1_000)]
    max_steps: u64,
    /// Positive learning rate, at most 1. Scale features before fitting.
    #[arg(long, default_value_t = 0.1)]
    learning_rate: f64,
    /// Combined payload limit for CSV bytes and parsed numeric rows.
    #[arg(long, default_value_t = 1_048_576)]
    max_memory_bytes: u64,
    /// Elapsed-time budget checked during input, fitting and evaluation.
    #[arg(long, default_value_t = 10)]
    max_seconds: u64,
}

impl TrainingConfig {
    fn guard(&self) -> Result<ResourceGuard> {
        if !(1..=MAX_STEPS).contains(&self.max_steps) || !(1..=self.max_steps).contains(&self.steps)
        {
            return Err(error("Require 1 <= steps <= max-steps <= 100000."));
        }
        if !(1..=MAX_SECONDS).contains(&self.max_seconds) {
            return Err(error("Require 1 <= max-seconds <= 300."));
        }
        if !(1..=MAX_INPUT_BYTES).contains(&self.max_memory_bytes) {
            return Err(error("Require 1 <= max-memory-bytes <= 8388608."));
        }
        if !self.learning_rate.is_finite() || self.learning_rate <= 0.0 || self.learning_rate > 1.0
        {
            return Err(error("Learning rate must be finite and in (0, 1]."));
        }
        ResourceGuard::new(self.max_memory_bytes, self.max_steps, self.max_seconds)
    }
}

#[derive(Debug, Default, Serialize)]
struct Line {
    intercept: f64,
    slope: f64,
}

#[derive(Serialize)]
struct TrainingResult<'a> {
    schema: &'static str,
    goblin_version: &'static str,
    target_os: &'static str,
    target_arch: &'static str,
    algorithm: &'static str,
    cpu_threads: u8,
    config: &'a TrainingConfig,
    input_sha256: String,
    input_bytes: usize,
    rows: usize,
    data_buffer_peak_bytes: u64,
    completed_steps: u64,
    initial_mse: f64,
    final_mse: f64,
    model: Line,
}

fn error(message: impl Into<String>) -> GoblinError {
    GoblinError::new("G910", message)
}

fn finite_pair(row: &str, line_number: usize) -> Result<[f64; 2]> {
    let parse = |cell: &str| cell.trim().parse::<f64>().ok().filter(|n| n.is_finite());
    if row.len() <= MAX_ROW_BYTES
        && let Some((x, y)) = row.split_once(',')
        && let (Some(x), Some(y)) = (parse(x), parse(y))
    {
        return Ok([x, y]);
    }
    Err(error(format!(
        "CSV line {line_number} must be at most {MAX_ROW_BYTES} bytes and contain two finite numbers."
    )))
}

fn read_rows(data: &[u8], steps: u64, guard: &mut ResourceGuard) -> Result<Vec<[f64; 2]>> {
    let csv = std::str::from_utf8(data).map_err(|_| error("Training CSV must be UTF-8."))?;
    let mut lines = csv.lines();
    if lines.next() != Some("x,y") {
        return Err(error("Training CSV must start with the header x,y."));
    }
    let count = lines.clone().take(MAX_ROWS + 1).count();
    if !(2..=MAX_ROWS).contains(&count) {
        return Err(error(
            "Training CSV must contain between 2 and 100000 data rows.",
        ));
    }
    if (steps + 1) * count as u64 > MAX_ROW_EVALUATIONS {
        return Err(error(
            "Training exceeds the 10000000 row-evaluation ceiling. Reduce rows or steps.",
        ));
    }
    guard.reserve((count * size_of::<[f64; 2]>()) as u64)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(count)
        .map_err(|_| error("Unable to reserve the training row buffer."))?;
    for (index, line) in lines.enumerate() {
        if index % ROW_CHECK_INTERVAL == 0 {
            guard.check()?;
        }
        rows.push(finite_pair(line, index + 2)?);
    }
    Ok(rows)
}

#[derive(Default)]
struct Batch {
    squared_error: f64,
    intercept_gradient: f64,
    slope_gradient: f64,
}

fn evaluate(rows: &[[f64; 2]], line: &Line, guard: &ResourceGuard) -> Result<Batch> {
    let mut batch = Batch::default();
    for chunk in rows.chunks(ROW_CHECK_INTERVAL) {
        guard.check()?;
        for &[x, y] in chunk {
            let residual = (line.slope * x + line.intercept) - y;
            batch.squared_error += residual * residual;
            batch.intercept_gradient += residual;
            batch.slope_gradient += residual * x;
        }
    }
    let count = rows.len() as f64;
    batch.squared_error /= count;
    batch.intercept_gradient /= count;
    batch.slope_gradient /= count;
    if !batch.squared_error.is_finite()
        || !batch.intercept_gradient.is_finite()
        || !batch.slope_gradient.is_finite()
    {
        return Err(error(
            "Non-finite loss or gradient. Rescale the training data or reduce the learning rate.",
        ));
    }
    Ok(batch)
}

pub fn run(args: TrainArgs) -> Result<i32> {
    let config = args.config;
    let mut guard = config.guard()?;
    crate::crypto_cli::destination(&args.output)
        .map_err(|_| error("Model output must be a new file in an existing directory."))?;
    let mut file = crate::crypto_cli::open_regular(&args.dataset, config.max_memory_bytes)
        .map_err(|_| {
            error("Training input must be a readable regular file within the data memory budget.")
        })?;
    let input_len = file.metadata()?.len();
    guard.reserve(input_len)?;
    let input_bytes = input_len as usize;
    let mut data = Vec::new();
    data.try_reserve_exact(input_bytes)
        .map_err(|_| error("Unable to reserve the training input buffer."))?;
    data.resize(input_bytes, 0);
    for chunk in data.chunks_mut(64 * 1024) {
        guard.check()?;
        file.read_exact(chunk)
            .map_err(|_| error("Unable to read the full training input."))?;
    }
    if file.read(&mut [0])? != 0 {
        return Err(error("Training input grew while reading."));
    }

    let rows = read_rows(&data, config.steps, &mut guard)?;
    let data_buffer_peak_bytes = guard.live_bytes();
    let input_sha256 = sha256_bytes(&data);
    drop(data);
    guard.release(input_len)?;

    let mut model = Line::default();
    let mut batch = evaluate(&rows, &model, &guard)?;
    let initial_mse = batch.squared_error;
    for _ in 0..config.steps {
        guard.step()?;
        model.intercept -= config.learning_rate * batch.intercept_gradient;
        model.slope -= config.learning_rate * batch.slope_gradient;
        batch = evaluate(&rows, &model, &guard)?;
    }
    let result = TrainingResult {
        schema: "goblin.ai-cpu-linear.v1",
        goblin_version: crate::VERSION,
        target_os: std::env::consts::OS,
        target_arch: std::env::consts::ARCH,
        algorithm: "f64-linear-full-batch-gradient-descent-v1",
        cpu_threads: 1,
        config: &config,
        input_sha256,
        input_bytes,
        rows: rows.len(),
        data_buffer_peak_bytes,
        completed_steps: guard.completed_steps(),
        initial_mse,
        final_mse: batch.squared_error,
        model,
    };
    let model_bytes = serde_json::to_vec_pretty(&result)?;
    guard.check()?;
    crate::recipient_crypto::save_new(&args.output, &model_bytes).map_err(|_| {
        error("Unable to save the model to a new file. Existing output is preserved.")
    })?;
    println!(
        "TRAINING_STATUS=PASS\nTRAINING_ROWS={}\nCOMPLETED_STEPS={}\nINITIAL_MSE={}\nFINAL_MSE={}\nMODEL_SHA256={}",
        result.rows,
        result.completed_steps,
        result.initial_mse,
        result.final_mse,
        sha256_bytes(&model_bytes)
    );
    Ok(0)
}
