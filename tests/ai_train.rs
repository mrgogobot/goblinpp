use goblinpp::hashing::sha256_bytes;
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

const DATA: &str = include_str!("../examples/ai_linear.csv");

fn train(dir: &Path, model: &str, flags: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .current_dir(dir)
        .args(["ai", "train", "data.csv", "--output", model])
        .args(flags)
        .output()
        .unwrap()
}

fn assert_refused(result: Output, reason: &str) {
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains(reason), "{stderr}");
    assert!(!String::from_utf8_lossy(&result.stdout).contains("TRAINING_STATUS=PASS"));
}

#[test]
fn learns_known_line_and_preserves_repeatable_model_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    fs::write(path.join("data.csv"), DATA).unwrap();
    let peak_bytes = DATA.len() + 5 * 16;
    let budget = peak_bytes.to_string();
    let flags = ["--steps", "300", "--max-memory-bytes", &budget];
    let first = train(path, "first.json", &flags);
    let second = train(path, "second.json", &flags);
    assert!(first.status.success(), "{first:?}");
    assert!(second.status.success(), "{second:?}");

    let bytes = fs::read(path.join("first.json")).unwrap();
    assert_eq!(bytes, fs::read(path.join("second.json")).unwrap());
    let model: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(model["input_sha256"], sha256_bytes(DATA.as_bytes()));
    assert_eq!(model["data_buffer_peak_bytes"], peak_bytes);
    assert_eq!(model["rows"], 5);
    assert_eq!(model["completed_steps"], 300);
    assert_eq!(model["cpu_threads"], 1);
    assert_eq!(model["initial_mse"], 11.0);
    assert!(model["final_mse"].as_f64().unwrap() < 1e-12);
    assert!((model["model"]["slope"].as_f64().unwrap() - 2.0).abs() < 1e-6);
    assert!((model["model"]["intercept"].as_f64().unwrap() - 3.0).abs() < 1e-6);
    let stdout = String::from_utf8(first.stdout).unwrap();
    assert!(stdout.contains(&format!("MODEL_SHA256={}", sha256_bytes(&bytes))));

    assert_refused(train(path, "first.json", &flags), "must be a new file");
    assert_eq!(fs::read(path.join("first.json")).unwrap(), bytes);
}

#[test]
fn refuses_over_budget_or_invalid_training_without_publishing_a_model() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    assert_refused(
        train(path, "model.json", &["--steps", "11", "--max-steps", "10"]),
        "steps <= max-steps",
    );
    fs::write(path.join("data.csv"), DATA).unwrap();
    assert_refused(
        train(
            path,
            "model.json",
            &["--max-memory-bytes", &DATA.len().to_string()],
        ),
        "allocation budget exceeded",
    );
    assert_refused(
        train(path, "model.json", &["--max-seconds", "0"]),
        "max-seconds",
    );
    fs::write(
        path.join("data.csv"),
        format!("x,y\n{}", "0,1\n".repeat(100)),
    )
    .unwrap();
    assert_refused(
        train(
            path,
            "model.json",
            &["--steps", "100000", "--max-steps", "100000"],
        ),
        "row-evaluation ceiling",
    );
    fs::write(path.join("data.csv"), "x,y\n1,2\n2,NaN\n").unwrap();
    assert_refused(train(path, "model.json", &[]), "CSV line 3");
    fs::write(path.join("data.csv"), "x,y\n1,1e300\n2,1e300\n").unwrap();
    assert_refused(
        train(path, "model.json", &[]),
        "Non-finite loss or gradient",
    );
    assert!(!path.join("model.json").exists());
}
