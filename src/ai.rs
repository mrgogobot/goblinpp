//! Offline AI resource planning. No model/backend is loaded and no training runs.
use crate::{
    ai_hardware::{self, Hardware, MAX_BYTES},
    error::{GoblinError, Result},
    hashing::sha256_bytes,
};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub const REQUEST_SCHEMA: &str = "goblin.ai-plan-request.v1";
pub const PLAN_SCHEMA: &str = "goblin.ai-resource-plan.v1";
pub const POLICY: &str = "goblin.ai-resource-estimator.v1";
pub const MAX_JSON: u64 = 65_536;
const MIB: u64 = 1 << 20;
const GIB: u64 = 1 << 30;

fn error(message: &str) -> GoblinError {
    GoblinError::new("G910", message)
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    Full,
    Lora,
    Qlora,
}
fn default_percent() -> u32 {
    50
}
fn default_headroom() -> u32 {
    25
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub model: String,
    /// Exact observed device ID, or cpu. No implicit fallback or substitution.
    pub device: String,
    pub method: Method,
    pub model_parameters: u64,
    pub trainable_parameters: u64,
    /// Resident frozen/base weight representation, not a promise of kernel support.
    pub weight_bits: u32,
    pub micro_batch: u64,
    pub sequence_length: u64,
    pub gradient_accumulation: u64,
    /// User-supplied estimates for this model/configuration, not measured by Goblin++.
    pub activation_bytes_per_token: u64,
    pub attention_workspace_bytes_per_sequence: u64,
    pub runtime_workspace_bytes: u64,
    pub host_buffer_bytes: u64,
    #[serde(default)]
    pub quantization_overhead_bytes: u64,
    #[serde(default)]
    pub additional_disk_bytes: u64,
    #[serde(default)]
    pub gradient_checkpointing: bool,
    pub max_steps: u64,
    pub max_seconds: u64,
    #[serde(default)]
    pub max_threads: Option<u32>,
    #[serde(default)]
    pub max_memory_bytes: Option<u64>,
    #[serde(default)]
    pub max_device_bytes: Option<u64>,
    #[serde(default)]
    pub max_disk_bytes: Option<u64>,
    #[serde(default)]
    pub reserve_ram_bytes: u64,
    #[serde(default)]
    pub reserve_device_bytes: u64,
    #[serde(default)]
    pub reserve_disk_bytes: u64,
    #[serde(default = "default_percent")]
    pub memory_percent: u32,
    #[serde(default = "default_headroom")]
    pub headroom_percent: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Estimate {
    pub base_weights_bytes: u64,
    pub trainable_weights_and_master_bytes: u64,
    pub gradients_bytes: u64,
    pub adam_states_bytes: u64,
    pub activations_bytes: u64,
    pub attention_workspace_bytes: u64,
    pub runtime_workspace_bytes: u64,
    pub quantization_overhead_bytes: u64,
    pub training_bytes_with_headroom: u64,
    pub host_bytes_with_headroom: u64,
    pub shared_pool_peak_bytes: Option<u64>,
    pub checkpoint_two_slots_bytes: u64,
    pub disk_required_bytes: u64,
    pub effective_batch_size: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub ram_bytes: Option<u64>,
    pub device_bytes: Option<u64>,
    pub disk_bytes: Option<u64>,
    pub threads: u32,
    pub steps: u64,
    pub seconds: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanCore {
    pub schema: String,
    pub policy: String,
    pub hardware: Hardware,
    pub request: Request,
    pub request_canonical_sha256: String,
    pub decision: String,
    pub reasons: Vec<String>,
    pub estimate: Estimate,
    pub limits: Limits,
    pub backend_hint: String,
    pub training_execution: String,
    pub enforcement: String,
    pub assumptions: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub core: PlanCore,
    pub core_sha256: String,
}

fn sum(values: &[u64]) -> Result<u64> {
    values.iter().try_fold(0u64, |s, v| {
        s.checked_add(*v)
            .filter(|n| *n <= MAX_BYTES)
            .ok_or_else(|| error("AI estimate exceeds the exact supported byte range."))
    })
}
fn product(values: &[u64]) -> Result<u64> {
    values.iter().try_fold(1u64, |s, v| {
        s.checked_mul(*v)
            .filter(|n| *n <= MAX_BYTES)
            .ok_or_else(|| error("AI estimate multiplication exceeds the supported range."))
    })
}
fn ceil_ratio(value: u64, numerator: u64, denominator: u64) -> Result<u64> {
    let result = (u128::from(value) * u128::from(numerator)).div_ceil(u128::from(denominator));
    u64::try_from(result)
        .ok()
        .filter(|n| *n <= MAX_BYTES)
        .ok_or_else(|| error("AI estimate exceeds the supported byte range."))
}
fn headroom(value: u64, r: &Request) -> Result<u64> {
    ceil_ratio(value, 100 + u64::from(r.headroom_percent), 100)
}
fn cap(value: Option<u64>, maximum: Option<u64>) -> Option<u64> {
    value.map(|v| maximum.map_or(v, |m| v.min(m)))
}

pub fn validate_request(r: &Request) -> Result<()> {
    if r.schema != REQUEST_SCHEMA
        || r.model.is_empty()
        || r.model.len() > 128
        || r.model.chars().any(char::is_control)
        || r.device.is_empty()
        || r.device.len() > 64
        || r.device.chars().any(char::is_control)
        || !(1..=1_000_000_000_000).contains(&r.model_parameters)
        || r.trainable_parameters == 0
        || r.trainable_parameters > r.model_parameters
        || ![4, 8, 16, 32].contains(&r.weight_bits)
        || !(1..=65_536).contains(&r.micro_batch)
        || !(1..=1_048_576).contains(&r.sequence_length)
        || !(1..=1_000_000).contains(&r.gradient_accumulation)
        || r.activation_bytes_per_token == 0
        || r.runtime_workspace_bytes == 0
        || !(1..=1_000_000_000).contains(&r.max_steps)
        || !(1..=604_800).contains(&r.max_seconds)
        || r.max_threads.is_some_and(|v| v == 0 || v > 65_536)
        || !(1..=80).contains(&r.memory_percent)
        || !(20..=100).contains(&r.headroom_percent)
        || [
            r.activation_bytes_per_token,
            r.attention_workspace_bytes_per_sequence,
            r.runtime_workspace_bytes,
            r.host_buffer_bytes,
            r.quantization_overhead_bytes,
            r.additional_disk_bytes,
            r.reserve_ram_bytes,
            r.reserve_device_bytes,
            r.reserve_disk_bytes,
        ]
        .iter()
        .any(|v| *v > MAX_BYTES)
        || [r.max_memory_bytes, r.max_device_bytes, r.max_disk_bytes]
            .iter()
            .any(|v| v.is_some_and(|v| v > MAX_BYTES))
        || matches!(r.method, Method::Full)
            && (r.trainable_parameters != r.model_parameters || r.weight_bits < 16)
        || matches!(r.method, Method::Lora) && r.weight_bits < 16
        || matches!(r.method, Method::Qlora)
            && (r.weight_bits > 8 || r.quantization_overhead_bytes == 0)
        || !matches!(r.method, Method::Qlora) && r.quantization_overhead_bytes != 0
        || r.device == "cpu" && (r.max_device_bytes.is_some() || r.reserve_device_bytes != 0)
    {
        return Err(error(
            "Invalid AI plan request: check schema, method, integer sizes, estimates and resource limits.",
        ));
    }
    Ok(())
}
pub fn read_request(path: &Path) -> Result<(Request, Vec<u8>)> {
    use std::io::Read;
    let mut raw = Vec::new();
    crate::crypto_cli::open_regular(path, MAX_JSON)
        .map_err(|_| {
            error("AI request must be a bounded regular JSON file; final symlinks are refused.")
        })?
        .take(MAX_JSON + 1)
        .read_to_end(&mut raw)
        .map_err(|_| error("Unable to read AI request."))?;
    if raw.len() as u64 > MAX_JSON {
        return Err(error("AI request exceeds 64 KiB."));
    }
    let request: Request = serde_json::from_slice(&raw).map_err(|_| {
        error("AI request JSON has missing, duplicate, unknown or incorrectly typed fields.")
    })?;
    validate_request(&request)?;
    Ok((request, raw))
}
fn ram_budget(h: &Hardware, r: &Request) -> Result<Option<u64>> {
    let Some(mut total) = h.total_ram_bytes else {
        return Ok(None);
    };
    let Some(mut available) = h.available_ram_bytes else {
        return Ok(None);
    };
    for limit in &h.memory_limits {
        total = total.min(limit.limit_bytes);
        let Some(used) = limit.current_bytes else {
            return Ok(None);
        };
        available = available.min(limit.limit_bytes.saturating_sub(used));
    }
    let reserve = (total / 4)
        .clamp(256 * MIB, 2 * GIB)
        .max(r.reserve_ram_bytes);
    let percent = ((u128::from(total) * u128::from(r.memory_percent)) / 100) as u64;
    Ok(cap(
        Some(
            available
                .saturating_sub(reserve)
                .min(total.saturating_sub(reserve))
                .min(percent),
        ),
        r.max_memory_bytes,
    ))
}
pub fn plan(h: &Hardware, r: &Request) -> Result<Report> {
    ai_hardware::validate(h)?;
    validate_request(r)?;
    let cpu = r.device == "cpu";
    let device = if cpu {
        None
    } else {
        h.devices.iter().find(|d| d.id == r.device)
    };
    let unified = device.is_some_and(|d| d.unified_memory);
    let base = ceil_ratio(r.model_parameters, u64::from(r.weight_bits), 8)?;
    let trainable = if matches!(r.method, Method::Full) {
        if r.weight_bits == 16 {
            product(&[r.trainable_parameters, 4])?
        } else {
            0
        }
    } else {
        product(&[r.trainable_parameters, 4])?
    };
    let gradients = product(&[r.trainable_parameters, 4])?;
    let adam = product(&[r.trainable_parameters, 8])?;
    let activations = product(&[
        r.activation_bytes_per_token,
        r.micro_batch,
        r.sequence_length,
    ])?;
    let attention = product(&[r.attention_workspace_bytes_per_sequence, r.micro_batch])?;
    let training = headroom(
        sum(&[
            base,
            trainable,
            gradients,
            adam,
            activations,
            attention,
            r.runtime_workspace_bytes,
            r.quantization_overhead_bytes,
        ])?,
        r,
    )?;
    let host = if cpu {
        headroom(
            sum(&[
                base,
                trainable,
                gradients,
                adam,
                activations,
                attention,
                r.runtime_workspace_bytes,
                r.quantization_overhead_bytes,
                r.host_buffer_bytes,
            ])?,
            r,
        )?
    } else {
        headroom(r.host_buffer_bytes, r)?
    };
    let shared = if unified {
        Some(sum(&[training, host])?)
    } else {
        None
    };
    let checkpoint_weights = if matches!(r.method, Method::Full) {
        base
    } else {
        product(&[r.trainable_parameters, 4])?
    };
    // Two complete checkpoint generations, FP32 master/Adam state and 10% + 4KiB serialization allowance.
    let checkpoint = product(&[
        sum(&[
            ceil_ratio(
                sum(&[
                    checkpoint_weights,
                    if matches!(r.method, Method::Full) {
                        trainable
                    } else {
                        0
                    },
                    adam,
                ])?,
                110,
                100,
            )?,
            4096,
        ])?,
        2,
    ])?;
    let disk_required = sum(&[checkpoint, r.additional_disk_bytes])?;
    let ram = ram_budget(h, r)?;
    let gpu = if cpu {
        None
    } else if let Some(d) = device {
        let budget = if unified {
            ram.map(|v| {
                d.recommended_working_set_bytes
                    .map_or(v, |m| v.min(m))
                    .saturating_sub(r.reserve_device_bytes)
            })
        } else {
            d.total_bytes
                .zip(d.available_bytes)
                .map(|(total, available)| {
                    let reserve = (total / 10).max(128 * MIB).max(r.reserve_device_bytes);
                    available
                        .saturating_sub(reserve)
                        .min(total.saturating_sub(reserve))
                })
        };
        cap(budget, r.max_device_bytes)
    } else {
        None
    };
    let disk = cap(
        h.available_disk_bytes
            .map(|a| a.saturating_sub((a / 4).clamp(64 * MIB, GIB).max(r.reserve_disk_bytes))),
        r.max_disk_bytes,
    );
    let threads = if h.cpu.usable_logical_threads == 0 {
        0
    } else {
        h.cpu
            .usable_logical_threads
            .saturating_sub(1)
            .max(1)
            .min(r.max_threads.unwrap_or(8))
    };
    let mut reasons = Vec::new();
    let mut refused = false;
    let mut unknown = false;
    let mut check =
        |capacity: Option<u64>, needed: u64, missing: &str, exceeded: &str| match capacity {
            None => {
                unknown = true;
                reasons.push(missing.into());
            }
            Some(v) if needed > v => {
                refused = true;
                reasons.push(exceeded.into());
            }
            _ => {}
        };
    check(
        ram,
        shared.unwrap_or(host),
        "RAM_CAPACITY_UNKNOWN",
        "RAM_BUDGET_EXCEEDED",
    );
    if !cpu {
        check(
            gpu,
            shared.unwrap_or(training),
            "DEVICE_CAPACITY_UNKNOWN",
            "DEVICE_BUDGET_EXCEEDED",
        );
    }
    check(
        disk,
        disk_required,
        "WORKSPACE_DISK_CAPACITY_UNKNOWN",
        "CHECKPOINT_DISK_BUDGET_EXCEEDED",
    );
    // A known explicit ceiling can refuse work even when live capacity is unknown.
    // It must never turn an unknown capacity into an affirmative admission.
    for (maximum, needed, reason) in [
        (
            r.max_memory_bytes,
            shared.unwrap_or(host),
            "RAM_BUDGET_EXCEEDED",
        ),
        (
            r.max_device_bytes,
            shared.unwrap_or(training),
            "DEVICE_BUDGET_EXCEEDED",
        ),
        (
            r.max_disk_bytes,
            disk_required,
            "CHECKPOINT_DISK_BUDGET_EXCEEDED",
        ),
    ] {
        if maximum.is_some_and(|m| needed > m) {
            refused = true;
            if !reasons.iter().any(|r| r == reason) {
                reasons.push(reason.into());
            }
        }
    }
    if threads == 0 {
        unknown = true;
        reasons.push("CPU_THREAD_CAPACITY_UNKNOWN".into());
    }
    if !cpu && device.is_none() {
        refused = true;
        reasons.push("REQUESTED_DEVICE_NOT_OBSERVED".into());
    }
    let backend = if cpu {
        "cpu-portable-candidate"
    } else if device.is_some_and(|d| d.api == "metal")
        && h.os == "macos"
        && h.architecture == "aarch64"
    {
        "mlx-candidate"
    } else {
        "gpu-backend-unvalidated"
    };
    let core=PlanCore {schema:PLAN_SCHEMA.into(),policy:POLICY.into(),hardware:h.clone(),request:r.clone(),request_canonical_sha256:sha256_bytes(&serde_json::to_vec(r)?),decision:if refused {"REFUSED"}else if unknown {"INSUFFICIENT_INFORMATION"}else{"WITHIN_ESTIMATED_BUDGET"}.into(),reasons,estimate:Estimate {base_weights_bytes:base,trainable_weights_and_master_bytes:trainable,gradients_bytes:gradients,adam_states_bytes:adam,activations_bytes:activations,attention_workspace_bytes:attention,runtime_workspace_bytes:r.runtime_workspace_bytes,quantization_overhead_bytes:r.quantization_overhead_bytes,training_bytes_with_headroom:training,host_bytes_with_headroom:host,shared_pool_peak_bytes:shared,checkpoint_two_slots_bytes:checkpoint,disk_required_bytes:disk_required,effective_batch_size:product(&[r.micro_batch,r.gradient_accumulation])?},limits:Limits {ram_bytes:ram,device_bytes:gpu,disk_bytes:disk,threads,steps:r.max_steps,seconds:r.max_seconds},backend_hint:backend.into(),training_execution:"NOT_IMPLEMENTED_NOT_STARTED".into(),enforcement:"PREFLIGHT_ESTIMATE_ONLY_NO_OS_OR_BACKEND_RESERVATION".into(),assumptions:vec!["Activation/attention/runtime/host/quantization memory inputs are user-declared, not measured.".into(),"Gradient accumulation increases effective batch size, not simultaneous micro-batch activations.".into(),"Checkpointing does not silently reduce the declared activation estimate; calibrate that estimate for this configuration.".into(),"No model quality ranking, automatic model substitution, hidden Python fallback or training-speed guarantee.".into(),"Recheck live resources before a future backend allocates. This report does not authorize training or prevent system OOM.".into()]};
    let digest = sha256_bytes(&serde_json::to_vec(&core)?);
    Ok(Report {
        core,
        core_sha256: digest,
    })
}
pub fn checked_report(text: &str) -> Result<Report> {
    if text.len() > MAX_JSON as usize {
        return Err(error("AI report exceeds 64 KiB."));
    }
    let report: Report =
        serde_json::from_str(text).map_err(|_| error("Malformed AI resource report."))?;
    let expected = plan(&report.core.hardware, &report.core.request)?;
    if report != expected {
        return Err(error(
            "AI report checksum or reconstructed policy/arithmetic differs.",
        ));
    }
    Ok(report)
}
pub fn code_call(
    base: &Path,
    name: &str,
    args: &[crate::evaluator::Value],
) -> Result<crate::evaluator::Value> {
    arity(name, args.len())?;
    use crate::evaluator::Value;
    match name {
        "ai_hardware" => Ok(Value::Text(serde_json::to_string_pretty(
            &ai_hardware::probe(base)?,
        )?)),
        "ai_plan" => {
            let path = Path::new(args[0].text(name)?);
            let path = if path.is_absolute() {
                path.to_owned()
            } else {
                base.join(path)
            };
            let (request, _) = read_request(&path)?;
            Ok(Value::Text(serde_json::to_string_pretty(&plan(
                &ai_hardware::probe(base)?,
                &request,
            )?)?))
        }
        "ai_plan_status" => Ok(Value::Text(
            checked_report(args[0].text(name)?)?.core.decision,
        )),
        _ => Err(error("Unknown AI planning function.")),
    }
}
pub fn is_function(name: &str) -> bool {
    matches!(name, "ai_hardware" | "ai_plan" | "ai_plan_status")
}
pub fn arity(name: &str, n: usize) -> Result<()> {
    if (name == "ai_hardware" && n == 0) || (matches!(name, "ai_plan" | "ai_plan_status") && n == 1)
    {
        Ok(())
    } else {
        Err(error("Invalid AI planning function argument count."))
    }
}

/// Cooperative accounting for future native backends. The caller must integrate
/// every allocation/step; this cannot preempt a blocked kernel or govern other code.
#[derive(Debug)]
pub struct ResourceGuard {
    limit: u64,
    live: u64,
    max_steps: u64,
    steps: u64,
    duration: Duration,
    started: Instant,
    cancelled: bool,
}
impl ResourceGuard {
    pub fn new(limit: u64, max_steps: u64, max_seconds: u64) -> Result<Self> {
        if limit == 0
            || limit > MAX_BYTES
            || max_steps == 0
            || max_steps > 1_000_000_000
            || !(1..=604_800).contains(&max_seconds)
        {
            return Err(error("Invalid AI cooperative resource guard."));
        }
        Ok(Self {
            limit,
            live: 0,
            max_steps,
            steps: 0,
            duration: Duration::from_secs(max_seconds),
            started: Instant::now(),
            cancelled: false,
        })
    }
    fn check(&self) -> Result<()> {
        if self.cancelled || self.started.elapsed() >= self.duration {
            Err(error("AI cooperative budget cancelled or expired."))
        } else {
            Ok(())
        }
    }
    pub fn reserve(&mut self, bytes: u64) -> Result<()> {
        self.check()?;
        let next = self
            .live
            .checked_add(bytes)
            .filter(|v| *v <= self.limit)
            .ok_or_else(|| error("AI allocation budget exceeded before reservation."))?;
        self.live = next;
        Ok(())
    }
    pub fn release(&mut self, bytes: u64) -> Result<()> {
        self.live = self
            .live
            .checked_sub(bytes)
            .ok_or_else(|| error("AI allocation accounting underflow."))?;
        Ok(())
    }
    pub fn step(&mut self) -> Result<()> {
        self.check()?;
        if self.steps >= self.max_steps {
            return Err(error("AI step budget exhausted."));
        }
        self.steps += 1;
        Ok(())
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    pub fn live_bytes(&self) -> u64 {
        self.live
    }
    pub fn completed_steps(&self) -> u64 {
        self.steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deadline_prevents_further_reservations_and_steps() {
        let mut guard = ResourceGuard::new(100, 10, 1).unwrap();
        guard.started = Instant::now() - Duration::from_secs(2);
        assert!(guard.reserve(1).is_err());
        assert!(guard.step().is_err());
        assert_eq!(guard.live_bytes(), 0);
        assert_eq!(guard.completed_steps(), 0);
    }
}
