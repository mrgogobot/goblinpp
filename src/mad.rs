//! Bounded final-state evidence. Never substitutes for execution or scientific replay.
use crate::ast::{Program, Stmt};
use crate::error::{GoblinError, Result};
use crate::evaluator::{Evaluation, Value as EvalValue};
use crate::hashing::{canonical_json_bytes, sha256_bytes};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub const POLICY: &str = "goblin.completely-mad.final-state.v1";
pub const MAX_VARIABLES: usize = 256;
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
pub const FILE: &str = "final-state.json";
pub const PREFIX: &str = "final:";

fn refusal(message: impl Into<String>) -> GoblinError {
    GoblinError::new("G703", message)
}

pub fn initial() -> Value {
    json!({"policy": POLICY, "requested": false, "capture_status": "NOT_REQUESTED"})
}

pub fn prepare(program: &Program) -> Value {
    if crate::parser::completely_mad(program) {
        json!({"policy": POLICY, "requested": true, "capture_status": "NOT_STARTED"})
    } else {
        initial()
    }
}

fn supported(value: &EvalValue) -> bool {
    match value {
        EvalValue::Quantity(q) => q.value_si.is_finite(),
        EvalValue::Array(items) => items
            .iter()
            .all(|v| !matches!(v, EvalValue::Array(_)) && supported(v)),
        _ => true,
    }
}

pub fn values(evaluation: &Evaluation) -> Result<BTreeMap<String, EvalValue>> {
    if evaluation.final_state_names.len() > MAX_VARIABLES {
        return Err(refusal(
            "COMPLETELY_MAD final state exceeds 256 variables; nothing was silently omitted.",
        ));
    }
    let mut values = BTreeMap::new();
    for name in &evaluation.final_state_names {
        let value = evaluation
            .env
            .get(name)
            .ok_or_else(|| refusal(format!("Missing final-state binding: {name}")))?;
        if !supported(value) {
            return Err(refusal(format!("Unsupported final-state value: {name}")));
        }
        values.insert(name.clone(), value.clone());
    }
    // Bound the serialized representation as well as the binding count.
    encode(&values)?;
    Ok(values)
}

struct BoundedBytes(Vec<u8>);
impl Write for BoundedBytes {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() > MAX_BYTES.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("Final-state evidence exceeds 8 MiB"));
        }
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn encode(values: &BTreeMap<String, EvalValue>) -> Result<Vec<u8>> {
    let document = json!({"schema":"goblin.final-state.v1", "values": values.iter().map(|(n,v)| (n.clone(), v.to_json())).collect::<serde_json::Map<_,_>>()});
    let mut writer = BoundedBytes(Vec::new());
    serde_json::to_writer(&mut writer, &document)
        .map_err(|e| refusal(format!("COMPLETELY_MAD capture refused: {e}")))?;
    Ok(writer.0)
}

pub fn native_expected(evaluation: &Evaluation) -> Result<BTreeMap<String, EvalValue>> {
    let mut result = evaluation.sealed.clone();
    if evaluation.completely_mad {
        result.extend(
            values(evaluation)?
                .into_iter()
                .map(|(n, v)| (format!("{PREFIX}{n}"), v)),
        );
    }
    Ok(result)
}

/// Called after run outcome is known. Only new private evidence is written.
pub fn preserve(run: &Path, evaluation: &Evaluation, receipt: &mut Value) -> Result<()> {
    if receipt["final_state"]["requested"] != true || !evaluation.completely_mad {
        return Ok(());
    }
    let capture = (|| {
        let values = values(evaluation)?;
        let bytes = encode(&values)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(run.join(FILE))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok::<_, GoblinError>((values, bytes))
    })();
    match capture {
        Ok((values, bytes)) => {
            receipt["final_state"] = json!({
                "policy": POLICY, "requested": true, "capture_status": "CAPTURED",
                "completeness": if receipt["status"] == "PASS" {"COMPLETE"} else {"PARTIAL"},
                "origin": if receipt["execution"]["engine"] == "rust-native-compiled" {"INTERPRETER_REFERENCE"} else {"INTERPRETER"},
                "native_parity": if receipt["execution"]["engine"] == "rust-native-compiled" {if receipt["status"] == "PASS" {"PASS"} else {"NOT_COMPLETED"}} else {"NOT_REQUESTED"},
                "path": FILE, "sha256": sha256_bytes(&bytes), "byte_count": bytes.len(),
                "variable_names": values.keys().collect::<Vec<_>>(), "variable_count": values.len(),
                "max_variables": MAX_VARIABLES, "max_bytes": MAX_BYTES
            });
            Ok(())
        }
        Err(error) => {
            receipt["final_state"] = json!({"policy": POLICY, "requested": true, "capture_status": "REFUSED", "reason": error.to_string()});
            Err(error)
        }
    }
}

pub fn mark_partial(receipt: &mut Value) {
    if receipt["final_state"]["capture_status"] == "CAPTURED" {
        receipt["final_state"]["completeness"] = json!("PARTIAL");
        if receipt["final_state"]["native_parity"] == "PASS" {
            receipt["final_state"]["native_parity"] = json!("NOT_COMPLETED");
        }
    }
}

fn assignments(statements: &[Stmt], names: &mut BTreeSet<String>) {
    for statement in statements {
        match statement {
            Stmt::Assign { name, .. } => {
                names.insert(name.clone());
            }
            Stmt::For { body, .. } | Stmt::ForEach { body, .. } | Stmt::While { body, .. } => {
                assignments(body, names)
            }
            Stmt::If {
                branches,
                else_body,
            } => {
                for (_, body) in branches {
                    assignments(body, names);
                }
                if let Some(body) = else_body {
                    assignments(body, names);
                }
            }
            Stmt::Switch { cases, default, .. } => {
                for (_, body) in cases {
                    assignments(body, names);
                }
                if let Some(body) = default {
                    assignments(body, names);
                }
            }
            _ => {}
        }
    }
}

fn valid_json_value(value: &Value, in_array: bool) -> bool {
    match value {
        Value::String(_) | Value::Bool(_) => true,
        Value::Object(fields) if fields.get("kind").and_then(Value::as_str) == Some("array") => {
            !in_array
                && fields.len() == 2
                && fields
                    .get("items")
                    .and_then(Value::as_array)
                    .is_some_and(|items| {
                        items.len() <= crate::evaluator::MAX_ARRAY_ITEMS
                            && items.iter().all(|v| valid_json_value(v, true))
                    })
        }
        Value::Object(fields) => {
            let Some(dimension) = fields.get("dimension").and_then(Value::as_array) else {
                return false;
            };
            let dims = dimension
                .iter()
                .map(|n| n.as_i64().and_then(|n| i32::try_from(n).ok()))
                .collect::<Option<Vec<_>>>();
            let Some(dims) = dims.and_then(|v| <[i32; 6]>::try_from(v).ok()) else {
                return false;
            };
            dims.iter().all(|d| d.checked_abs().is_some())
                && fields.len() == 3
                && fields
                    .get("value_si")
                    .and_then(Value::as_f64)
                    .is_some_and(f64::is_finite)
                && fields.get("unit_si").and_then(Value::as_str)
                    == Some(crate::quantity::format_dimension(dims).as_str())
        }
        _ => false,
    }
}

/// Integrity and declared-scope checks, NOT replay of arbitrary user calculations.
pub fn verify(run: &Path, receipt: &Value) -> Result<()> {
    let program = receipt["source"]["path"]
        .as_str()
        .and_then(|p| {
            let path = Path::new(p);
            (path.file_name().and_then(|v| v.to_str()) == Some(p)).then(|| run.join(p))
        })
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|text| {
            crate::parser::evidence_mode(receipt)
                .ok()
                .and_then(|mode| crate::parser::parse_source_with_mode(&text, mode).ok())
        });
    let requested = program
        .as_ref()
        .is_some_and(|p| crate::parser::completely_mad(&p.program));
    let state = &receipt["final_state"];
    if !requested && state.is_null() {
        return Ok(());
    }
    if state["policy"] != POLICY || state["requested"].as_bool() != Some(requested) {
        return Err(refusal("Final-state policy/declaration mismatch."));
    }
    let status = state["capture_status"].as_str().unwrap_or("");
    if !requested {
        return if status == "NOT_REQUESTED" {
            Ok(())
        } else {
            Err(refusal("Unexpected final state."))
        };
    }
    if receipt["paranoid_mode"] != true {
        return Err(refusal("COMPLETELY_MAD requires paranoid custody checks."));
    }
    if status != "CAPTURED" {
        return if receipt["status"] != "PASS" && matches!(status, "REFUSED" | "NOT_STARTED") {
            Ok(())
        } else {
            Err(refusal("Missing mandatory final-state capture."))
        };
    }
    let complete = receipt["status"] == "PASS";
    if state["completeness"] != if complete { "COMPLETE" } else { "PARTIAL" }
        || state["max_variables"] != MAX_VARIABLES
        || state["max_bytes"] != MAX_BYTES
        || state["path"] != FILE
    {
        return Err(refusal("Invalid final-state completeness, limits or path."));
    }
    let path = run.join(FILE);
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES as u64 {
        return Err(refusal("Invalid final-state file or size."));
    }
    let bytes = fs::read(path)?;
    if state["sha256"] != sha256_bytes(&bytes) || state["byte_count"] != bytes.len() {
        return Err(refusal("Final-state hash/size mismatch."));
    }
    let document: Value = serde_json::from_slice(&bytes)?;
    let values = document["values"]
        .as_object()
        .ok_or_else(|| refusal("Final-state values missing."))?;
    let names = values.keys().cloned().collect::<BTreeSet<_>>();
    if document["schema"] != "goblin.final-state.v1"
        || !values.values().all(|v| valid_json_value(v, false))
        || names.len() > MAX_VARIABLES
        || state["variable_names"] != json!(names.iter().collect::<Vec<_>>())
        || state["variable_count"] != names.len()
    {
        return Err(refusal("Final-state inventory mismatch."));
    }
    let program = &program.unwrap().program;
    let mut candidates = BTreeSet::new();
    assignments(&program.statements, &mut candidates);
    if !names.is_subset(&candidates)
        || (complete
            && program
                .statements
                .iter()
                .any(|s| matches!(s, Stmt::Assign {name,..} if !names.contains(name))))
    {
        return Err(refusal("Final-state bindings do not match source scope."));
    }
    let compiled = receipt["execution"]["engine"] == "rust-native-compiled";
    if state["origin"]
        != if compiled {
            "INTERPRETER_REFERENCE"
        } else {
            "INTERPRETER"
        }
        || state["native_parity"]
            != if compiled {
                if complete { "PASS" } else { "NOT_COMPLETED" }
            } else {
                "NOT_REQUESTED"
            }
    {
        return Err(refusal("Invalid final-state execution origin/parity."));
    }
    if complete && compiled {
        let native = crate::runtime::read_native_values(&run.join("native-results.tsv"))?;
        let native_names = native
            .keys()
            .filter_map(|n| n.strip_prefix(PREFIX).map(str::to_owned))
            .collect::<BTreeSet<_>>();
        if native_names != names {
            return Err(refusal("Native final-state inventory mismatch."));
        }
        for (name, value) in values {
            if canonical_json_bytes(&native[&format!("{PREFIX}{name}")].to_json())?
                != canonical_json_bytes(value)?
            {
                return Err(refusal(format!(
                    "Native final-state value mismatch: {name}"
                )));
            }
        }
    }
    Ok(())
}
