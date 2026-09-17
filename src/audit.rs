use crate::error::{GoblinError, Result};
use crate::hashing::{hash_canonical_json, sha256_file};
use crate::parser::parse_source;
use crate::runtime::read_receipt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditCheck {
    pub check: String,
    pub pass: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub verified: bool,
    pub status: String,
    pub checks: Vec<AuditCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    pub source_bytes_same: bool,
    pub canonical_program_same: bool,
    pub sealed_artifacts_same: bool,
    pub generated_artifacts_same: bool,
    pub constants_same: bool,
    pub data_imports_same: bool,
    pub interaction_same: bool,
    pub stdout_same: bool,
    pub stderr_same: bool,
    pub status_same: bool,
    pub environment_same: bool,
    pub paranoid_mode_same: bool,
    pub execution_engine_same: bool,
    pub classification: String,
}

pub fn verify_run(run_dir: impl AsRef<Path>) -> Result<Verification> {
    let run_dir = run_dir.as_ref();
    if !run_dir.is_dir() {
        return Err(GoblinError::new(
            "G701",
            format!("Run directory not found: {}", run_dir.display()),
        ));
    }
    let receipt = read_receipt(run_dir)?;
    let mut checks = Vec::new();
    let schema = string_at(&receipt, &["schema"]);
    let schema_v2 = schema.as_deref() == Some("goblin.run-receipt.v2");
    checks.push(check(
        "SCHEMA_SUPPORTED",
        matches!(
            schema.as_deref(),
            Some("goblin.run-receipt.v1" | "goblin.run-receipt.v2")
        ),
        Some("goblin.run-receipt.v1 or goblin.run-receipt.v2".into()),
        schema,
        None,
    ));
    for field in [
        "goblin_version",
        "status",
        "source",
        "sealed_artifacts",
        "receipt_core_sha256",
    ] {
        checks.push(check(
            &format!("REQUIRED_FIELD:{field}"),
            receipt.get(field).is_some(),
            None,
            None,
            None,
        ));
    }
    let expected_core = string_at(&receipt, &["receipt_core_sha256"]);
    let mut core = receipt.clone();
    core.as_object_mut().unwrap().remove("receipt_core_sha256");
    let actual_core = hash_canonical_json(&core).ok();
    checks.push(check(
        "RECEIPT_CORE_SHA256",
        expected_core == actual_core,
        expected_core,
        actual_core,
        None,
    ));
    let source_path = string_at(&receipt, &["source", "path"]);
    let expected_source = string_at(&receipt, &["source", "sha256"]);
    if let Some(path) = source_path {
        compare_file(
            &mut checks,
            "SOURCE_SHA256",
            &run_dir.join(path),
            expected_source,
        );
    } else {
        checks.push(check(
            "SOURCE_SHA256",
            false,
            expected_source,
            None,
            Some("source path missing".into()),
        ));
    }
    if let (Some(path), Some(expected)) = (
        string_at(&receipt, &["source", "path"]),
        string_at(&receipt, &["canonical_source", "sha256"]),
    ) {
        let actual = fs::read_to_string(run_dir.join(path))
            .ok()
            .and_then(|text| parse_source(&text).ok())
            .and_then(|parsed| parsed.canonical_sha256().ok());
        checks.push(check(
            "CANONICAL_SOURCE_SHA256",
            actual.as_deref() == Some(&expected),
            Some(expected),
            actual,
            None,
        ));
    }
    if schema_v2 {
        let declared_paranoid = receipt.get("paranoid_mode").and_then(Value::as_bool);
        let source_paranoid = string_at(&receipt, &["source", "path"])
            .and_then(|path| fs::read_to_string(run_dir.join(path)).ok())
            .and_then(|text| parse_source(&text).ok())
            .map(|parsed| parsed.paranoid());
        checks.push(check(
            "PARANOID_MODE_MATCHES_SOURCE",
            declared_paranoid.is_some()
                && (declared_paranoid == source_paranoid
                    || (source_paranoid.is_none()
                        && receipt["status"] != "PASS"
                        && declared_paranoid == Some(false))),
            source_paranoid.map(|value| value.to_string()),
            declared_paranoid.map(|value| value.to_string()),
            if source_paranoid.is_none() {
                Some("Unparseable source is permitted only for a failed, non-paranoid run.".into())
            } else {
                None
            },
        ));
        if declared_paranoid == Some(true) {
            let postflight = &receipt["paranoid_postflight"];
            let status = postflight.get("status").and_then(Value::as_str);
            let start = string_at(postflight, &["source_start_sha256"]);
            let source_hash = string_at(&receipt, &["source", "sha256"]);
            checks.push(check(
                "PARANOID_POSTFLIGHT_START_SHA256",
                start.is_some() && start == source_hash,
                source_hash,
                start.clone(),
                None,
            ));
            if matches!(status, Some("SOURCE_UNAVAILABLE" | "EVIDENCE_UNAVAILABLE")) {
                checks.push(check(
                    "PARANOID_POSTFLIGHT_UNAVAILABLE_RECORDED",
                    receipt["status"] != "PASS"
                        && postflight
                            .get("detail")
                            .and_then(Value::as_str)
                            .is_some_and(|value| !value.is_empty())
                        && postflight.get("source_end_evidence_path").is_none()
                        && postflight
                            .get("self_verification_gate")
                            .and_then(Value::as_str)
                            == Some("REQUIRED_BEFORE_LEDGER_REGISTRATION")
                        && (status != Some("EVIDENCE_UNAVAILABLE")
                            || string_at(postflight, &["source_end_sha256"]).is_some()),
                    Some("honest failed run with no source-end evidence".into()),
                    status.map(str::to_string),
                    None,
                ));
            } else {
                let path = string_at(postflight, &["source_end_evidence_path"]);
                checks.push(check(
                    "PARANOID_POSTFLIGHT_EVIDENCE_PATH",
                    path.as_deref() == Some("postflight/source.observed"),
                    Some("postflight/source.observed".into()),
                    path,
                    None,
                ));
                let end = string_at(postflight, &["source_end_sha256"]);
                let evidence_hash = string_at(postflight, &["source_end_evidence_sha256"]);
                compare_file(
                    &mut checks,
                    "PARANOID_POSTFLIGHT_EVIDENCE_SHA256",
                    &run_dir.join("postflight/source.observed"),
                    evidence_hash.clone(),
                );
                let equal = end.is_some() && end == start;
                checks.push(check(
                    "PARANOID_POSTFLIGHT_CLASSIFICATION",
                    end.is_some()
                        && end == evidence_hash
                        && postflight
                            .get("source_bytes_equal_at_postflight")
                            .and_then(Value::as_bool)
                            == Some(equal)
                        && match status {
                            Some("PASS") => equal,
                            Some("SOURCE_CHANGED") => !equal && receipt["status"] != "PASS",
                            _ => false,
                        }
                        && postflight
                            .get("self_verification_gate")
                            .and_then(Value::as_str)
                            == Some("REQUIRED_BEFORE_LEDGER_REGISTRATION"),
                    None,
                    status.map(str::to_string),
                    None,
                ));
            }
        } else {
            checks.push(check(
                "PARANOID_POSTFLIGHT_NOT_REQUESTED",
                receipt["paranoid_postflight"].is_null(),
                Some("null".into()),
                Some(receipt["paranoid_postflight"].to_string()),
                None,
            ));
        }
    }
    if !receipt["interaction"].is_null() {
        let path = string_at(&receipt, &["interaction", "evidence_path"]);
        checks.push(check(
            "INTERACTION_EVIDENCE_PATH",
            path.as_deref() == Some("interaction.json"),
            Some("interaction.json".into()),
            path,
            None,
        ));
        let evidence_path = run_dir.join("interaction.json");
        compare_file(
            &mut checks,
            "INTERACTION_EVIDENCE_SHA256",
            &evidence_path,
            string_at(&receipt, &["interaction", "evidence_sha256"]),
        );
        let evidence = fs::read(&evidence_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
        let well_formed = evidence.as_ref().is_some_and(|value| {
            value["schema"] == "goblin.interaction.v1"
                && value["argv"].as_array().is_some_and(|values| {
                    !values.is_empty()
                        && values.iter().all(Value::is_string)
                        && receipt["interaction"]["argc"].as_u64() == Some(values.len() as u64)
                })
                && value["input_events"].as_array().is_some_and(|events| {
                    receipt["interaction"]["input_count"].as_u64() == Some(events.len() as u64)
                        && events.iter().all(|event| {
                            event["prompt"].is_string()
                                && (event["response"].is_string() || event["response"].is_null())
                        })
                })
        });
        checks.push(check(
            "INTERACTION_EVIDENCE_SCHEMA",
            well_formed,
            Some("goblin.interaction.v1 with consistent counts".into()),
            evidence.map(|value| value["schema"].to_string()),
            None,
        ));
    }
    compare_file(
        &mut checks,
        "STDOUT_SHA256",
        &run_dir.join("stdout.log"),
        string_at(&receipt, &["stdout_sha256"]),
    );
    compare_file(
        &mut checks,
        "STDERR_SHA256",
        &run_dir.join("stderr.log"),
        string_at(&receipt, &["stderr_sha256"]),
    );
    if let Some(artifacts) = receipt.get("sealed_artifacts").and_then(Value::as_array) {
        for artifact in artifacts {
            let name = artifact.get("name").and_then(Value::as_str).unwrap_or("?");
            let path = artifact.get("path").and_then(Value::as_str);
            let expected = artifact
                .get("sha256")
                .and_then(Value::as_str)
                .map(str::to_string);
            if let Some(path) = path {
                compare_file(
                    &mut checks,
                    &format!("ARTIFACT:{name}"),
                    &run_dir.join(path),
                    expected,
                );
            } else {
                checks.push(check(
                    &format!("ARTIFACT:{name}"),
                    false,
                    expected,
                    None,
                    Some("artifact path missing".into()),
                ));
            }
        }
    }
    if let Some(artifacts) = receipt.get("generated_artifacts").and_then(Value::as_array) {
        for artifact in artifacts {
            let name = artifact.get("name").and_then(Value::as_str).unwrap_or("?");
            let path = artifact.get("path").and_then(Value::as_str);
            let expected = artifact
                .get("sha256")
                .and_then(Value::as_str)
                .map(str::to_string);
            if let Some(path) = path {
                let safe_path = generated_artifact_path(run_dir, path);
                checks.push(check(
                    &format!("GENERATED_ARTIFACT_PATH:{name}"),
                    safe_path.is_some(),
                    Some(format!("outputs/{name}")),
                    Some(path.into()),
                    safe_path
                        .is_none()
                        .then(|| "generated artifact path escapes or bypasses outputs/".into()),
                ));
                if let Some(safe_path) = safe_path {
                    compare_file(
                        &mut checks,
                        &format!("GENERATED_ARTIFACT:{name}"),
                        &safe_path,
                        expected,
                    );
                    let expected_bytes = artifact.get("byte_count").and_then(Value::as_u64);
                    let actual_bytes = fs::metadata(&safe_path).ok().map(|value| value.len());
                    checks.push(check(
                        &format!("GENERATED_ARTIFACT_BYTE_COUNT:{name}"),
                        expected_bytes.is_some() && expected_bytes == actual_bytes,
                        expected_bytes.map(|value| value.to_string()),
                        actual_bytes.map(|value| value.to_string()),
                        None,
                    ));
                }
            } else {
                checks.push(check(
                    &format!("GENERATED_ARTIFACT:{name}"),
                    false,
                    expected,
                    None,
                    Some("generated artifact path missing".into()),
                ));
            }
        }
    }
    if let Some(imports) = receipt.get("data_imports").and_then(Value::as_array) {
        for (index, import) in imports.iter().enumerate() {
            let source_hash = import
                .get("sha256")
                .and_then(Value::as_str)
                .map(str::to_string);
            let expected = import
                .get("evidence_sha256")
                .and_then(Value::as_str)
                .map(str::to_string);
            checks.push(check(
                &format!("DATA_IMPORT_{}_SOURCE_EVIDENCE_MATCH", index + 1),
                source_hash.is_some() && source_hash == expected,
                source_hash,
                expected.clone(),
                None,
            ));
            if let Some(path) = import.get("evidence_path").and_then(Value::as_str) {
                compare_file(
                    &mut checks,
                    &format!("DATA_IMPORT_{}_EVIDENCE_SHA256", index + 1),
                    &run_dir.join(path),
                    expected,
                );
                let expected_bytes = import.get("byte_count").and_then(Value::as_u64);
                let actual_bytes = fs::metadata(run_dir.join(path))
                    .ok()
                    .map(|value| value.len());
                checks.push(check(
                    &format!("DATA_IMPORT_{}_BYTE_COUNT", index + 1),
                    expected_bytes.is_some() && expected_bytes == actual_bytes,
                    expected_bytes.map(|value| value.to_string()),
                    actual_bytes.map(|value| value.to_string()),
                    None,
                ));
            } else {
                checks.push(check(
                    &format!("DATA_IMPORT_{}_EVIDENCE_PRESENT", index + 1),
                    false,
                    None,
                    None,
                    Some("data input was not copied into run evidence".into()),
                ));
            }
        }
    }
    for (label, path_field, hash_field) in [
        (
            "LEDGER_EVIDENCE_SHA256",
            &["ledger", "evidence_path"][..],
            &["ledger", "evidence_sha256"][..],
        ),
        (
            "LEDGER_HEAD_EVIDENCE_SHA256",
            &["ledger", "head_evidence_path"][..],
            &["ledger", "head_evidence_sha256"][..],
        ),
    ] {
        if let Some(path) = string_at(&receipt, path_field) {
            compare_file(
                &mut checks,
                label,
                &run_dir.join(path),
                string_at(&receipt, hash_field),
            );
        }
    }
    if let Some(path) = string_at(&receipt, &["freeze", "evidence_path"]) {
        compare_file(
            &mut checks,
            "FREEZE_RECEIPT_EVIDENCE_SHA256",
            &run_dir.join(path),
            string_at(&receipt, &["freeze", "evidence_sha256"]),
        );
    }
    if let Some(path) = string_at(&receipt, &["execution", "compiler", "binary_path"]) {
        compare_file(
            &mut checks,
            "COMPILED_BINARY_SHA256",
            &run_dir.join(path),
            string_at(&receipt, &["execution", "compiler", "binary_sha256"]),
        );
    }
    if let Some(path) = string_at(
        &receipt,
        &["execution", "compiler", "generated_source_path"],
    ) {
        compare_file(
            &mut checks,
            "GENERATED_RUST_SHA256",
            &run_dir.join(path),
            string_at(
                &receipt,
                &["execution", "compiler", "generated_source_sha256"],
            ),
        );
    }
    if let Some(path) = string_at(
        &receipt,
        &["execution", "compiler", "native_result_manifest_path"],
    ) {
        compare_file(
            &mut checks,
            "NATIVE_RESULT_MANIFEST_SHA256",
            &run_dir.join(path),
            string_at(
                &receipt,
                &["execution", "compiler", "native_result_manifest_sha256"],
            ),
        );
    }
    let verified = checks.iter().all(|item| item.pass);
    Ok(Verification {
        verified,
        status: if verified { "PASS" } else { "FAIL" }.into(),
        checks,
    })
}

pub fn diff_runs(left_dir: impl AsRef<Path>, right_dir: impl AsRef<Path>) -> Result<DiffReport> {
    if !verify_run(&left_dir)?.verified || !verify_run(&right_dir)?.verified {
        return Err(GoblinError::new(
            "G701",
            "Both runs must verify before they can be diffed.",
        ));
    }
    let left = read_receipt(left_dir)?;
    let right = read_receipt(right_dir)?;
    let source_bytes_same = at(&left, &["source", "sha256"]) == at(&right, &["source", "sha256"]);
    let canonical_program_same =
        at(&left, &["canonical_source", "sha256"]) == at(&right, &["canonical_source", "sha256"]);
    let sealed_artifacts_same = normalized_artifacts(&left) == normalized_artifacts(&right);
    let generated_artifacts_same =
        normalized_generated_artifacts(&left) == normalized_generated_artifacts(&right);
    let constants_same = at(&left, &["constants_used"]) == at(&right, &["constants_used"]);
    let data_imports_same = normalized_imports(&left) == normalized_imports(&right);
    let interaction_same = at(&left, &["interaction", "evidence_sha256"])
        == at(&right, &["interaction", "evidence_sha256"]);
    let stdout_same = at(&left, &["stdout_sha256"]) == at(&right, &["stdout_sha256"]);
    let stderr_same = at(&left, &["stderr_sha256"]) == at(&right, &["stderr_sha256"]);
    let status_same = at(&left, &["status"]) == at(&right, &["status"]);
    let environment_same = at(&left, &["environment"]) == at(&right, &["environment"]);
    let paranoid_mode_same = at(&left, &["paranoid_mode"]) == at(&right, &["paranoid_mode"]);
    let execution_engine_same =
        at(&left, &["execution", "engine"]) == at(&right, &["execution", "engine"]);
    let equivalent_result = canonical_program_same
        && sealed_artifacts_same
        && generated_artifacts_same
        && constants_same
        && data_imports_same
        && interaction_same
        && stdout_same
        && status_same;
    let protocol_notation = [
        string_at(&left, &["protocol_violation", "classification"]),
        string_at(&right, &["protocol_violation", "classification"]),
    ]
    .into_iter()
    .flatten()
    .any(|value| value == "NOTATION_ONLY_CHANGE_AFTER_FREEZE");
    let different_named_sources = at(&left, &["source", "path"]) != at(&right, &["source", "path"]);
    let classification = if source_bytes_same && equivalent_result {
        "IDENTICAL_RESULT"
    } else if !source_bytes_same && equivalent_result && protocol_notation {
        "NOTATION_ONLY_CHANGE_AFTER_FREEZE"
    } else if !source_bytes_same && equivalent_result && different_named_sources {
        "NOTATION_ONLY_REVISION"
    } else if !source_bytes_same && equivalent_result {
        "NOTATION_ONLY_CHANGE"
    } else if canonical_program_same && !sealed_artifacts_same {
        "RUNTIME_DIVERGENCE"
    } else {
        "SEMANTIC_OR_RESULT_CHANGE"
    };
    Ok(DiffReport {
        source_bytes_same,
        canonical_program_same,
        sealed_artifacts_same,
        generated_artifacts_same,
        constants_same,
        data_imports_same,
        interaction_same,
        stdout_same,
        stderr_same,
        status_same,
        environment_same,
        paranoid_mode_same,
        execution_engine_same,
        classification: classification.into(),
    })
}

fn compare_file(checks: &mut Vec<AuditCheck>, label: &str, path: &Path, expected: Option<String>) {
    let actual = sha256_file(path).ok();
    let pass = expected.is_some() && expected == actual;
    checks.push(check(
        label,
        pass,
        expected,
        actual,
        (!path.exists()).then(|| format!("missing {}", path.display())),
    ));
}
fn generated_artifact_path(run_dir: &Path, path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
    let components = path.components().collect::<Vec<_>>();
    if components.len() != 2
        || components[0].as_os_str() != "outputs"
        || !matches!(components[1], Component::Normal(_))
    {
        return None;
    }
    Some(run_dir.join(path))
}
fn check(
    name: &str,
    pass: bool,
    expected: Option<String>,
    actual: Option<String>,
    detail: Option<String>,
) -> AuditCheck {
    AuditCheck {
        check: name.into(),
        pass,
        expected,
        actual,
        detail,
    }
}
fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    at(value, path).and_then(Value::as_str).map(str::to_string)
}
fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}
fn normalized_artifacts(receipt: &Value) -> Vec<(String, String)> {
    receipt
        .get("sealed_artifacts")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    (
                        item.get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                        item.get("sha256")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}
fn normalized_generated_artifacts(receipt: &Value) -> Vec<(String, String, Value)> {
    receipt
        .get("generated_artifacts")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    (
                        item.get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                        item.get("sha256")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                        item.get("metadata").cloned().unwrap_or(Value::Null),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}
fn normalized_imports(receipt: &Value) -> Vec<(String, String, Value)> {
    receipt
        .get("data_imports")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    (
                        item.get("sha256")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                        item.get("format")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                        item.get("access").cloned().unwrap_or(Value::Null),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}
