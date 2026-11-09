//! Offline AI planning and CPU training commands.
use crate::{
    ai::{self, Report},
    ai_hardware::{self, Hardware},
    error::{GoblinError, Result},
    hashing::sha256_bytes,
};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Args)]
pub struct AiArgs {
    #[command(subcommand)]
    pub action: AiAction,
}
#[derive(Debug, Subcommand)]
pub enum AiAction {
    /// Fit a small linear model to a local two-column CSV with enforced trainer budgets.
    Train(crate::ai_train::TrainArgs),
    /// Observe local CPU/RAM/disk and available GPU metadata; do not allocate a model.
    Hardware {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Estimate one explicit training configuration. A fitting estimate never starts training.
    Plan {
        request: PathBuf,
        /// New directory for a reconstructable local planning bundle; never overwrite.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Filesystem used for checkpoint-capacity observation; defaults to request folder.
        #[arg(long)]
        workspace: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Verify copied inputs, arithmetic and optional independently retained receipt hash offline.
    VerifyPlan {
        directory: PathBuf,
        #[arg(long)]
        expected_receipt_sha256: Option<String>,
        #[arg(long)]
        json: bool,
    },
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    bytes: u64,
    sha256: String,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct BundleCore {
    schema: String,
    policy: String,
    decision: String,
    files: Vec<Entry>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Receipt {
    core: BundleCore,
    core_sha256: String,
}
fn error(message: &str) -> GoblinError {
    GoblinError::new("G910", message)
}
fn bytes(path: &Path) -> Result<Vec<u8>> {
    let mut value = Vec::new();
    crate::crypto_cli::open_regular(path, ai::MAX_JSON)
        .map_err(|_| error("AI bundle entry is missing, unsafe or too large."))?
        .take(ai::MAX_JSON + 1)
        .read_to_end(&mut value)
        .map_err(|_| error("Unable to read AI bundle entry."))?;
    if value.len() as u64 > ai::MAX_JSON {
        return Err(error("AI bundle entry exceeds 64 KiB."));
    }
    Ok(value)
}
pub fn save_bundle(directory: &Path, raw_request: &[u8], report: &Report) -> Result<String> {
    let decoded: ai::Request =
        serde_json::from_slice(raw_request).map_err(|_| error("Invalid original AI request."))?;
    if decoded != report.core.request
        || ai::checked_report(&serde_json::to_string(report)?)? != *report
    {
        return Err(error(
            "AI bundle inputs do not match the reconstructed plan.",
        ));
    }
    let payload = [
        (
            "hardware.json",
            serde_json::to_vec_pretty(&report.core.hardware)?,
        ),
        ("plan.json", serde_json::to_vec_pretty(report)?),
        ("request.json", raw_request.to_vec()),
    ];
    if payload.iter().any(|(_, b)| b.len() as u64 > ai::MAX_JSON) {
        return Err(error("AI planning bundle exceeds entry limits."));
    }
    let core = BundleCore {
        schema: "goblin.ai-plan-bundle.v1".into(),
        policy: ai::POLICY.into(),
        decision: report.core.decision.clone(),
        files: payload
            .iter()
            .map(|(name, b)| Entry {
                name: (*name).into(),
                bytes: b.len() as u64,
                sha256: sha256_bytes(b),
            })
            .collect(),
    };
    let receipt = Receipt {
        core_sha256: sha256_bytes(&serde_json::to_vec(&core)?),
        core,
    };
    let parent = directory
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !fs::symlink_metadata(parent)
        .map_err(|_| error("AI output parent must already exist."))?
        .is_dir()
    {
        return Err(error(
            "AI output parent must be a trusted directory, not a final symlink.",
        ));
    }
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(directory).map_err(|_| {
        error("AI output directory must be new; existing files and directories are preserved.")
    })?;
    for (name, data) in payload {
        crate::recipient_crypto::save_new(&directory.join(name), &data)
            .map_err(|_| error("AI bundle write failed; incomplete output is preserved."))?;
    }
    crate::recipient_crypto::save_new(
        &directory.join("receipt.json"),
        &serde_json::to_vec_pretty(&receipt)?,
    )
    .map_err(|_| error("AI receipt write failed; incomplete output is preserved."))?;
    Ok(receipt.core_sha256)
}
pub fn verify_bundle(directory: &Path, expected: Option<&str>) -> Result<Report> {
    if !fs::symlink_metadata(directory)
        .map_err(|_| error("AI bundle directory is unavailable."))?
        .is_dir()
    {
        return Err(error(
            "AI bundle must be a regular directory, not a final symlink.",
        ));
    }
    let mut names = fs::read_dir(directory)
        .map_err(|_| error("Unable to inspect AI bundle."))?
        .take(5)
        .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| error("Unable to inspect AI bundle inventory."))?;
    names.sort();
    if names != ["hardware.json", "plan.json", "receipt.json", "request.json"] {
        return Err(error(
            "AI bundle inventory differs from the exact four expected files.",
        ));
    }
    let receipt: Receipt = serde_json::from_slice(&bytes(&directory.join("receipt.json"))?)
        .map_err(|_| error("Invalid AI receipt."))?;
    let digest = sha256_bytes(&serde_json::to_vec(&receipt.core)?);
    if receipt.core.schema != "goblin.ai-plan-bundle.v1"
        || receipt.core.policy != ai::POLICY
        || receipt.core_sha256 != digest
        || expected.is_some_and(|e| e != digest)
        || receipt.core.files.len() != 3
    {
        return Err(error(
            "AI receipt checksum, expected anchor or policy differs.",
        ));
    }
    for (entry, name) in
        receipt
            .core
            .files
            .iter()
            .zip(["hardware.json", "plan.json", "request.json"])
    {
        if entry.name != name {
            return Err(error("AI receipt inventory/path differs."));
        }
        let data = bytes(&directory.join(name))?;
        if data.len() as u64 != entry.bytes || sha256_bytes(&data) != entry.sha256 {
            return Err(error("AI bundle byte count or content hash differs."));
        }
    }
    let hardware: Hardware = serde_json::from_slice(&bytes(&directory.join("hardware.json"))?)
        .map_err(|_| error("Invalid saved AI hardware snapshot."))?;
    let (request, _) = ai::read_request(&directory.join("request.json"))?;
    let saved = String::from_utf8(bytes(&directory.join("plan.json"))?)
        .map_err(|_| error("AI plan must be UTF-8 JSON."))?;
    let report = ai::checked_report(&saved)?;
    if report.core.hardware != hardware
        || report.core.request != request
        || ai::plan(&hardware, &request)? != report
        || receipt.core.decision != report.core.decision
    {
        return Err(error("AI plan does not reconstruct from saved inputs."));
    }
    Ok(report)
}
pub fn run(args: AiArgs) -> Result<i32> {
    match args.action {
        AiAction::Train(args) => crate::ai_train::run(args),
        AiAction::Hardware { workspace, json } => {
            let h = ai_hardware::probe(&workspace)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&h)?);
            } else {
                println!(
                    "GOBLIN AI HARDWARE\n\nCPU_THREADS={}\nARCHITECTURE={}\nRAM_TOTAL_BYTES={}\nRAM_AVAILABLE_BYTES={}\nWORKSPACE_DISK_AVAILABLE_BYTES={}\nDEVICE_COUNT={}\nTRAINING_EXECUTION=NOT_IMPLEMENTED_NOT_STARTED\nCAPACITY=OBSERVATION_ONLY_NOT_RESERVED",
                    h.cpu.usable_logical_threads,
                    h.architecture,
                    h.total_ram_bytes
                        .map_or("UNKNOWN".into(), |n| n.to_string()),
                    h.available_ram_bytes
                        .map_or("UNKNOWN".into(), |n| n.to_string()),
                    h.available_disk_bytes
                        .map_or("UNKNOWN".into(), |n| n.to_string()),
                    h.devices.len()
                );
                for device in h.devices {
                    println!(
                        "DEVICE={} API={} UNIFIED_MEMORY={}",
                        device.id, device.api, device.unified_memory
                    );
                }
            }
            Ok(0)
        }
        AiAction::Plan {
            request,
            output,
            workspace,
            json,
        } => {
            let (r, raw) = ai::read_request(&request)?;
            let base = workspace.as_deref().unwrap_or_else(|| {
                request
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or(Path::new("."))
            });
            let report = ai::plan(&ai_hardware::probe(base)?, &r)?;
            let receipt = if let Some(output) = output {
                Some(save_bundle(&output, &raw, &report)?)
            } else {
                None
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "GOBLIN AI RESOURCE PLAN\n\nDECISION={}\nDEVICE={}\nTHREAD_BUDGET={}\nESTIMATED_TRAINING_BYTES={}\nESTIMATED_HOST_BYTES={}\nCHECKPOINT_AND_STAGING_BYTES={}\nEFFECTIVE_BATCH_SIZE={}\nPLAN_CORE_SHA256={}\nTRAINING_EXECUTION=NOT_IMPLEMENTED_NOT_STARTED\nENFORCEMENT=PREFLIGHT_ESTIMATE_ONLY",
                    report.core.decision,
                    report.core.request.device,
                    report.core.limits.threads,
                    report.core.estimate.training_bytes_with_headroom,
                    report.core.estimate.host_bytes_with_headroom,
                    report.core.estimate.disk_required_bytes,
                    report.core.estimate.effective_batch_size,
                    report.core_sha256
                );
                for reason in &report.core.reasons {
                    println!("REASON={reason}");
                }
                if let Some(hash) = receipt {
                    println!("PLANNING_RECEIPT_CORE_SHA256={hash}");
                }
            }
            Ok(match report.core.decision.as_str() {
                "WITHIN_ESTIMATED_BUDGET" => 0,
                "REFUSED" => 1,
                _ => 2,
            })
        }
        AiAction::VerifyPlan {
            directory,
            expected_receipt_sha256,
            json,
        } => {
            let report = verify_bundle(&directory, expected_receipt_sha256.as_deref())?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({"verified":true,"decision":report.core.decision,"training_execution":report.core.training_execution,"scope":"saved bytes and reconstructed arithmetic only; no live resource reservation or model validation"})
                );
            } else {
                println!(
                    "GOBLIN AI PLAN VERIFY\n\nVERIFICATION_STATUS=PASS\nSAVED_DECISION={}\nTRAINING_EXECUTION=NOT_IMPLEMENTED_NOT_STARTED\nSaved evidence/arithmetic only; capacity and model correctness are not certified.",
                    report.core.decision
                );
            }
            Ok(0)
        }
    }
}
