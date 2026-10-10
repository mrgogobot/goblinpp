use crate::error::{GoblinError, Result};
use crate::hashing::{hash_canonical_json, sha256_file};
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
    pub batch_policy_same: bool,
    pub batch_evidence_same: bool,
    pub resource_policy_same: bool,
    pub resource_evidence_same: bool,
    pub inference_policy_same: bool,
    pub inference_evidence_same: bool,
    pub rng_policy_same: bool,
    pub rng_evidence_same: bool,
    pub parser_policy_same: bool,
    pub statistics_policy_same: bool,
    pub math_policy_same: bool,
    pub math_environment_same: bool,
    pub language_semantics_same: bool,
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
    let parser_mode = crate::parser::evidence_mode(&receipt);
    checks.push(check(
        "PARSER_POLICY_SUPPORTED",
        parser_mode.is_ok(),
        None,
        string_at(&receipt, &["parser_policy"]),
        parser_mode.as_ref().err().map(|e| e.to_string()),
    ));
    let schema = string_at(&receipt, &["schema"]);
    let batch_evidence_needed = receipt.get("batch_policy").is_some()
        || receipt.get("batches").is_some()
        || crate::batches::requires_policy(&receipt);
    if batch_evidence_needed {
        checks.push(check(
            "BATCH_POLICY_SUPPORTED",
            receipt["batch_policy"] == crate::batches::policy(),
            None,
            None,
            None,
        ));
        checks.push(check("BATCH_LIFECYCLE_EVIDENCE",crate::batches::validate_evidence(&receipt),None,None,Some("Structural integrity and input/output binding; not numerical replay or a process RAM quota.".into())));
    }
    if receipt.get("inference_policy").is_some()
        || receipt.get("inference").is_some()
        || crate::inference_policy::requires_policy(&receipt)
    {
        checks.push(check(
            "INFERENCE_POLICY_SUPPORTED",
            receipt["inference_policy"] == crate::inference_policy::policy(),
            None,
            None,
            None,
        ));
        checks.push(check(
            "INFERENCE_TRACE_STRUCTURE",
            crate::inference_policy::validate_evidence(&receipt["inference"]),
            None,
            None,
            Some("Execution summaries are integrity evidence, not mathematical replay.".into()),
        ));
    }
    let resource_evidence_needed = receipt.get("resource_policy").is_some()
        || receipt.get("resources").is_some()
        || crate::resources::requires_policy(&receipt);
    if resource_evidence_needed {
        checks.push(check(
            "RESOURCE_POLICY_SUPPORTED",
            receipt["resource_policy"] == crate::resources::policy(),
            None,
            None,
            None,
        ));
        let request = string_at(&receipt, &["source", "path"])
            .and_then(|p| fs::read_to_string(run_dir.join(p)).ok())
            .and_then(|text| crate::parser::parse_source(&text).ok())
            .map(|parsed| crate::resources::requested(&parsed.program));
        let valid = crate::resources::validate_evidence(&receipt["resources"], request);
        checks.push(check(
            "RESOURCE_BUDGET_EVIDENCE",
            valid.is_ok(),
            None,
            None,
            valid.err().map(|e| e.to_string()),
        ));
    }
    if receipt.get("rng_policy").is_some()
        || receipt.get("rng").is_some()
        || crate::random::requires_policy(&receipt)
    {
        checks.push(check(
            "RNG_POLICY_SUPPORTED",
            receipt["rng_policy"] == crate::random::policy(),
            None,
            None,
            None,
        ));
        let replay = crate::random::verify_evidence(&receipt["rng"]);
        checks.push(check(
            "RNG_EVIDENCE_REPLAY",
            replay.is_ok(),
            None,
            None,
            replay.err().map(|e| e.to_string()),
        ));
    }
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
            .and_then(|text| {
                if let Some(entries) = receipt.get("module_imports") {
                    parser_mode.as_ref().ok().and_then(|mode| {
                        crate::modules::verify_preserved_with_mode(run_dir, &text, entries, *mode)
                            .ok()
                    })
                } else {
                    parser_mode
                        .as_ref()
                        .ok()
                        .and_then(|mode| crate::parser::parse_source_with_mode(&text, *mode).ok())
                }
            })
            .and_then(|parsed| parsed.canonical_sha256().ok());
        checks.push(check(
            "CANONICAL_SOURCE_SHA256",
            actual.as_deref() == Some(&expected),
            Some(expected),
            actual,
            None,
        ));
    }
    if let Some(entries) = receipt.get("module_imports") {
        let verified = string_at(&receipt, &["source", "path"])
            .and_then(|path| fs::read_to_string(run_dir.join(path)).ok())
            .is_some_and(|text| {
                parser_mode.as_ref().ok().is_some_and(|mode| {
                    crate::modules::verify_preserved_with_mode(run_dir, &text, entries, *mode)
                        .is_ok()
                })
            });
        checks.push(check("MODULE_SOURCE_EVIDENCE", verified, None, None, None));
    }
    if schema_v2 {
        let declared_paranoid = receipt.get("paranoid_mode").and_then(Value::as_bool);
        let source_paranoid = string_at(&receipt, &["source", "path"])
            .and_then(|path| fs::read_to_string(run_dir.join(path)).ok())
            .and_then(|text| {
                parser_mode
                    .as_ref()
                    .ok()
                    .and_then(|mode| crate::parser::parse_source_with_mode(&text, *mode).ok())
            })
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
            &run_dir.join(&path),
            string_at(
                &receipt,
                &["execution", "compiler", "native_result_manifest_sha256"],
            ),
        );
        if resource_evidence_needed {
            let matches = path == "native-results.tsv"
                && fs::read_to_string(run_dir.join(&path))
                    .ok()
                    .and_then(|text| crate::resources::native_evidence(&text).ok())
                    .is_some_and(|value| value == receipt["resources"]);
            checks.push(check(
                "NATIVE_RESOURCE_BUDGET_PARITY",
                matches,
                None,
                None,
                None,
            ));
        }
    }
    if let Some(path) = string_at(
        &receipt,
        &["execution", "compiler", "support_manifest_path"],
    ) {
        let safe_path = path == "program-native.native-build/support-manifest.json";
        checks.push(check(
            "NATIVE_SUPPORT_MANIFEST_PATH",
            safe_path,
            None,
            None,
            None,
        ));
        if safe_path {
            compare_file(
                &mut checks,
                "NATIVE_SUPPORT_MANIFEST_SHA256",
                &run_dir.join(&path),
                string_at(
                    &receipt,
                    &["execution", "compiler", "support_manifest_sha256"],
                ),
            );
            let manifest = fs::read(run_dir.join(&path))
                .ok()
                .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
            let verified = manifest.as_ref().is_some_and(|m| {
                m["schema"] == "goblin.native-support.v1"
                    && m["files"].as_array().is_some_and(|files| {
                        if files.is_empty() {
                            return false;
                        }
                        let base = run_dir.join(&path).parent().unwrap().to_path_buf();
                        files.iter().all(|file| {
                            let Some(name) = file["path"].as_str() else {
                                return false;
                            };
                            if Path::new(name)
                                .components()
                                .any(|c| !matches!(c, Component::Normal(_)))
                            {
                                return false;
                            }
                            let target = base.join(name);
                            fs::symlink_metadata(&target)
                                .ok()
                                .is_some_and(|m| m.is_file() && !m.file_type().is_symlink())
                                && sha256_file(&target).ok().as_deref() == file["sha256"].as_str()
                                && fs::metadata(target).ok().map(|m| m.len())
                                    == file["bytes"].as_u64()
                        })
                    })
            });
            checks.push(check(
                "NATIVE_SUPPORT_SOURCE_INVENTORY",
                verified,
                None,
                None,
                None,
            ));
        }
    }
    if let Some(path) = string_at(
        &receipt,
        &["execution", "compiler", "native_data_manifest_path"],
    ) {
        let safe_path = path == "native-data.json";
        checks.push(check(
            "NATIVE_DATA_MANIFEST_PATH",
            safe_path,
            None,
            None,
            None,
        ));
        if safe_path {
            compare_file(
                &mut checks,
                "NATIVE_DATA_MANIFEST_SHA256",
                &run_dir.join(&path),
                string_at(
                    &receipt,
                    &["execution", "compiler", "native_data_manifest_sha256"],
                ),
            );
            let manifest = fs::read(run_dir.join(&path))
                .ok()
                .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
            let verified = manifest.as_ref().is_some_and(|m| {
                if m["schema"] != "goblin.native-data.v1" {
                    return false;
                }
                if receipt.get("rng").is_some() && m["rng"] != receipt["rng"] {
                    return false;
                }
                if crate::inference_policy::requires_policy(&receipt)
                    && (m["inference_policy"] != receipt["inference_policy"]
                        || m["inference"] != receipt["inference"]
                        || m["rng_policy"] != receipt["rng_policy"]
                        || m["rng"] != receipt["rng"]
                        || m["goblin_version"] != receipt["goblin_version"])
                {
                    return false;
                }
                if resource_evidence_needed
                    && (m["resource_policy"] != receipt["resource_policy"]
                        || m["resources"] != receipt["resources"])
                {
                    return false;
                }
                if batch_evidence_needed
                    && (m["batch_policy"] != receipt["batch_policy"]
                        || m["batches"] != receipt["batches"])
                {
                    return false;
                }
                let mut expected = receipt["data_imports"].clone();
                if let Some(entries) = expected.as_array_mut() {
                    for entry in entries {
                        entry["evidence_path"] = Value::Null;
                        entry["evidence_sha256"] = Value::Null;
                    }
                }
                if m["data_imports"] != expected {
                    return false;
                }
                let Some(artifacts) = m["generated_artifacts"].as_array() else {
                    return false;
                };
                let Some(outputs) = receipt["generated_artifacts"].as_array() else {
                    return false;
                };
                artifacts.len() == outputs.len()
                    && artifacts.iter().zip(outputs).all(|(native, output)| {
                        let Some(name) = native["name"].as_str() else {
                            return false;
                        };
                        if crate::output::validate_name(name).is_err() {
                            return false;
                        }
                        let mut expected = output.clone();
                        let Some(expected_object) = expected.as_object_mut() else {
                            return false;
                        };
                        expected_object.remove("path");
                        if expected["producer"] == "stream_write"
                            && expected["metadata"]["complete"] == false
                        {
                            expected["name"] = expected["metadata"]["requested_name"].clone();
                            expected["metadata"]["complete"] = Value::Bool(true);
                        }
                        let path = run_dir.join("native-outputs").join(name);
                        *native == expected
                            && sha256_file(&path).ok().as_deref() == native["sha256"].as_str()
                            && fs::metadata(path).ok().map(|m| m.len())
                                == native["byte_count"].as_u64()
                    })
            });
            checks.push(check(
                "NATIVE_DATA_EVIDENCE_PARITY",
                verified,
                None,
                None,
                None,
            ));
        }
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
    let module_sources = |receipt: &Value| {
        receipt["module_imports"]
            .as_array()
            .map(|imports| {
                imports
                    .iter()
                    .map(|item| (item["path"].clone(), item["sha256"].clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let source_bytes_same = at(&left, &["source", "sha256"]) == at(&right, &["source", "sha256"])
        && module_sources(&left) == module_sources(&right);
    let language_semantics_same =
        at(&left, &["language_semantics"]) == at(&right, &["language_semantics"]);
    let parser_policy_same = at(&left, &["parser_policy"]) == at(&right, &["parser_policy"]);
    let math_policy_same = at(&left, &["math_policy"]) == at(&right, &["math_policy"]);
    let rng_policy_same = at(&left, &["rng_policy"]) == at(&right, &["rng_policy"]);
    let inference_policy_same =
        at(&left, &["inference_policy"]) == at(&right, &["inference_policy"]);
    let inference_evidence_same = at(&left, &["inference"]) == at(&right, &["inference"]);
    let rng_evidence_same = at(&left, &["rng"]) == at(&right, &["rng"]);
    let resource_policy_same = at(&left, &["resource_policy"]) == at(&right, &["resource_policy"]);
    let resource_evidence_same = at(&left, &["resources"]) == at(&right, &["resources"]);
    let batch_policy_same = at(&left, &["batch_policy"]) == at(&right, &["batch_policy"]);
    let batch_evidence_same = at(&left, &["batches"]) == at(&right, &["batches"]);
    let statistics_policy_same =
        at(&left, &["statistics_policy"]) == at(&right, &["statistics_policy"]);
    let math_environment_same =
        at(&left, &["math_environment"]) == at(&right, &["math_environment"]);
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
        && batch_evidence_same
        && inference_evidence_same
        && resource_evidence_same
        && rng_evidence_same
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
    let classification = if !language_semantics_same {
        "LANGUAGE_SEMANTICS_CHANGE"
    } else if !parser_policy_same {
        "PARSER_POLICY_CHANGE"
    } else if !math_policy_same {
        "MATH_POLICY_CHANGE"
    } else if !statistics_policy_same {
        "STATISTICS_POLICY_CHANGE"
    } else if !rng_policy_same {
        "RNG_POLICY_CHANGE"
    } else if !inference_policy_same {
        "INFERENCE_POLICY_CHANGE"
    } else if !resource_policy_same {
        "RESOURCE_POLICY_CHANGE"
    } else if !batch_policy_same {
        "BATCH_POLICY_CHANGE"
    } else if source_bytes_same && equivalent_result {
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
        batch_policy_same,
        batch_evidence_same,
        inference_policy_same,
        inference_evidence_same,
        resource_policy_same,
        resource_evidence_same,
        rng_policy_same,
        rng_evidence_same,
        parser_policy_same,
        statistics_policy_same,
        math_policy_same,
        math_environment_same,
        language_semantics_same,
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
