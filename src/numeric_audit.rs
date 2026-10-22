//! Explicit same-machine interpreter/native scalar consistency; not an accuracy certificate.
use crate::ast::{Program, Stmt};
use crate::error::{GoblinError, Result};
use crate::evaluator::Value as EvalValue;
use crate::hashing::sha256_bytes;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub const POLICY: &str = "goblin.named-scalar-native-consistency.v1";
pub const CONFIG_FILE: &str = "numeric-audit-config.json";
const CONFIG_LIMIT: u64 = 16 * 1024;
const MANIFEST_LIMIT: u64 = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub comparison: String,
    pub quantities: Vec<String>,
}

fn error(message: impl Into<String>) -> GoblinError {
    GoblinError::new("G702", message)
}

pub fn requested(program: &Program) -> Option<&str> {
    program.statements.iter().find_map(|s| {
        if let Stmt::NumericAudit(p) = s {
            Some(p.as_str())
        } else {
            None
        }
    })
}

pub fn validate_path(path: &str) -> Result<()> {
    crate::parser::validate_numeric_audit_path(path)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(error("Audit evidence/configuration symlinks are refused."));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > limit {
        return Err(error(
            "Audit evidence/configuration must be a bounded regular file.",
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(error("Audit evidence grew beyond its limit."));
    }
    Ok(bytes)
}

fn parse_config(bytes: &[u8]) -> Result<Config> {
    let config: Config = serde_json::from_slice(bytes)
        .map_err(|_| error("Malformed audit configuration or unknown/duplicate fields."))?;
    let mut names = HashSet::new();
    if config.schema != "goblin.numeric-audit-config.v1"
        || config.comparison != "interpreter-native-same-bits"
        || !(1..=16).contains(&config.quantities.len())
        || config.quantities.iter().any(|n| {
            n.is_empty()
                || n.len() > 64
                || n.as_bytes()[0].is_ascii_digit()
                || !n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || !names.insert(n)
        })
    {
        return Err(error(
            "Require explicit interpreter-native-same-bits comparison and 1..16 unique ASCII scalar seal names; no tolerances/default modes.",
        ));
    }
    Ok(config)
}

pub fn snapshot(source: &Path, program: &Program) -> Result<Value> {
    let Some(path) = requested(program) else {
        return Ok(Value::Null);
    };
    validate_path(path)?;
    let bytes = read_bounded(
        &source.parent().unwrap_or(Path::new(".")).join(path),
        CONFIG_LIMIT,
    )?;
    let config = parse_config(&bytes)?;
    Ok(json!({"policy":POLICY, "path":path, "sha256":sha256_bytes(&bytes), "config":config}))
}

pub fn initial(program: Option<&Program>) -> Value {
    let path = program.and_then(requested);
    json!({"policy":POLICY, "requested":path.is_some(), "config_path":path, "config_sha256":null,
        "comparison_completed":false, "status":if path.is_some() {"ERROR"} else {"NOT_REQUESTED"},
        "reason":if path.is_some() {json!("AUDIT_NOT_COMPLETED")} else {Value::Null}, "comparisons":[]})
}

pub fn prepare(
    source: &Path,
    program: &Program,
    run: &Path,
    report: &mut Value,
) -> Result<Option<Config>> {
    let Some(path) = requested(program) else {
        return Ok(None);
    };
    validate_path(path)?;
    let bytes = read_bounded(
        &source.parent().unwrap_or(Path::new(".")).join(path),
        CONFIG_LIMIT,
    )?;
    let config = parse_config(&bytes)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(run.join(CONFIG_FILE))?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    report["config_sha256"] = json!(sha256_bytes(&bytes));
    Ok(Some(config))
}

fn scalar(value: Option<&EvalValue>) -> Option<Value> {
    let Some(EvalValue::Quantity(q)) = value else {
        return None;
    };
    Some(
        json!({"value_si":q.value_si, "bits_hex":format!("{:016x}",q.value_si.to_bits()), "dimension":q.dimension}),
    )
}

pub fn assess(
    config: &Config,
    reference: &BTreeMap<String, EvalValue>,
    native: &BTreeMap<String, EvalValue>,
    base: &Value,
) -> Value {
    let mut result = base.clone();
    let mut rows = Vec::new();
    let mut incomplete = false;
    let mut mismatch = false;
    for name in &config.quantities {
        let a = scalar(reference.get(name));
        let b = scalar(native.get(name));
        let status = match (&a, &b) {
            (Some(a), Some(b)) if a == b => "PASS",
            (Some(_), Some(_)) => {
                mismatch = true;
                "FAIL"
            }
            _ => {
                incomplete = true;
                "INCONCLUSIVE"
            }
        };
        rows.push(json!({"name":name, "status":status, "interpreter":a, "native":b}));
    }
    result["comparison_completed"] = json!(true);
    result["status"] = json!(if mismatch {
        "FAIL"
    } else if incomplete {
        "INCONCLUSIVE"
    } else {
        "PASS"
    });
    result["reason"] = Value::Null;
    result["comparisons"] = json!(rows);
    result
}

pub fn native_values(run: &Path) -> Result<BTreeMap<String, EvalValue>> {
    let bytes = read_bounded(&run.join("native-results.tsv"), MANIFEST_LIMIT)?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| error("Invalid native manifest encoding."))?;
    crate::runtime::decode_native_values(text)
}

/// Verified historical non-audit runs and explicit NOT_REQUESTED carry the same policy meaning.
pub fn comparison_evidence(receipt: &Value) -> &Value {
    if receipt["numeric_audit"]["requested"] == true {
        &receipt["numeric_audit"]
    } else {
        &Value::Null
    }
}

pub fn run_failed(receipt: &mut Value) {
    if receipt["numeric_audit"]["status"] == "PASS" {
        receipt["numeric_audit"]["status"] = json!("ERROR");
        receipt["numeric_audit"]["reason"] = json!("RUN_FAILED");
    }
}

/// Reconstruct comparison rows from preserved scalar artifacts and native manifest. No execution.
pub fn verify(run: &Path, receipt: &Value) -> Result<()> {
    let source = receipt["source"]["path"]
        .as_str()
        .ok_or_else(|| error("Missing audit source path."))?;
    if Path::new(source).components().count() != 1 {
        return Err(error("Unsafe audit source path."));
    }
    let mode = crate::parser::evidence_mode(receipt)?;
    let parsed = fs::read_to_string(run.join(source))
        .ok()
        .and_then(|text| crate::parser::parse_source_with_mode(&text, mode).ok());
    let old = receipt["goblin_version"].as_str().is_some_and(|v| {
        v.starts_with("0.0.")
            || v.strip_prefix("0.1.0-alpha.")
                .and_then(|v| v.parse::<u32>().ok())
                .is_some_and(|v| v <= 36)
    });
    let Some(parsed) = parsed else {
        if receipt["status"] != "PASS"
            && (receipt["numeric_audit"] == initial(None)
                || (old && receipt.get("numeric_audit").is_none()))
        {
            return Ok(());
        }
        return Err(error("Audit source cannot be parsed."));
    };
    let mut expected = initial(Some(&parsed.program));
    let observed = &receipt["numeric_audit"];
    if observed.is_null() {
        if old && requested(&parsed.program).is_none() {
            return Ok(());
        }
        return Err(error(
            "Numeric audit evidence is required; it cannot be removed from new receipts.",
        ));
    }
    if requested(&parsed.program).is_none() {
        if observed == &expected {
            return Ok(());
        }
        return Err(error("Audit declaration/evidence mismatch."));
    }
    let has_config = !observed["config_sha256"].is_null();
    let config = if has_config {
        let bytes = read_bounded(&run.join(CONFIG_FILE), CONFIG_LIMIT)?;
        if observed["config_sha256"] != sha256_bytes(&bytes) {
            return Err(error("Audit configuration hash mismatch."));
        }
        expected["config_sha256"] = observed["config_sha256"].clone();
        Some(parse_config(&bytes)?)
    } else {
        None
    };
    if observed["comparison_completed"] == true {
        let config = config.ok_or_else(|| error("Completed audit lacks configuration."))?;
        if receipt["execution"]["engine"] != "rust-native-compiled"
            || receipt["execution"]["compiler"]["native_result_manifest_path"]
                != "native-results.tsv"
            || receipt["inline_rust"]
                .as_array()
                .is_none_or(|v| !v.is_empty())
        {
            return Err(error("Audit engine/evidence contract mismatch."));
        }
        let mut reference = BTreeMap::new();
        for name in &config.quantities {
            let entries = receipt["sealed_artifacts"]
                .as_array()
                .ok_or_else(|| error("Missing seals."))?;
            let found = entries
                .iter()
                .filter(|e| e["name"].as_str() == Some(name))
                .collect::<Vec<_>>();
            if found.len() > 1 {
                return Err(error("Duplicate audit seal."));
            }
            if let Some(entry) = found.first() {
                let path = entry["path"]
                    .as_str()
                    .ok_or_else(|| error("Missing seal path."))?;
                if path != format!("{name}.json") {
                    return Err(error("Unexpected audit seal path."));
                }
                let bytes = read_bounded(&run.join(path), MANIFEST_LIMIT)?;
                if entry["sha256"] != sha256_bytes(&bytes) {
                    return Err(error("Audit seal hash mismatch."));
                }
                let artifact: Value = serde_json::from_slice(&bytes)?;
                if artifact["schema"] != "goblin.sealed-artifact.v1"
                    || artifact["name"] != name.as_str()
                {
                    return Err(error("Audit seal identity mismatch."));
                }
                if let (Some(n), Some(axes)) = (
                    artifact["value"]["value_si"].as_f64(),
                    artifact["value"]["dimension"].as_array(),
                ) {
                    if axes.len() != 6 {
                        return Err(error("Audit requires six dimension axes."));
                    }
                    let mut dim = [0; 6];
                    for (slot, axis) in dim.iter_mut().zip(axes) {
                        *slot = axis
                            .as_i64()
                            .and_then(|v| i32::try_from(v).ok())
                            .ok_or_else(|| error("Invalid audit dimensions."))?;
                    }
                    reference.insert(
                        name.clone(),
                        EvalValue::Quantity(crate::quantity::Quantity::new(n, dim)?),
                    );
                }
            }
        }
        expected = assess(&config, &reference, &native_values(run)?, &expected);
        if expected["status"] == "PASS" && receipt["status"] != "PASS" {
            expected["status"] = json!("ERROR");
            expected["reason"] = json!("RUN_FAILED");
        }
    }
    if receipt["status"] == "PASS" && expected["status"] != "PASS" {
        return Err(error(
            "Requested audit did not pass; run cannot claim PASS.",
        ));
    }
    if observed != &expected {
        return Err(error(
            "Audit rows/status/policy do not reconstruct from evidence.",
        ));
    }
    Ok(())
}
