use crate::constants::snapshots;
use crate::error::{GoblinError, Result};
use crate::hashing::{hash_canonical_json, sha256_bytes, sha256_file};
use crate::ledger::{self, LedgerEvent};
use crate::parser::parse_source;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const FREEZE_SUFFIX: &str = ".freeze.json";
pub const LINEAGE_SUFFIX: &str = ".lineage.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyCheck {
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
pub struct FreezeReport {
    pub verified: bool,
    pub classification: String,
    pub receipt_path: String,
    pub receipt_file_sha256: Option<String>,
    pub receipt: Option<Value>,
    pub current_source_sha256: Option<String>,
    pub current_canonical_sha256: Option<String>,
    pub checks: Vec<CustodyCheck>,
}

pub fn freeze_path(source: impl AsRef<Path>) -> PathBuf {
    let source = source.as_ref();
    source.with_file_name(format!(
        "{}{}",
        source.file_name().unwrap_or_default().to_string_lossy(),
        FREEZE_SUFFIX
    ))
}

pub fn lineage_path(source: impl AsRef<Path>) -> PathBuf {
    let source = source.as_ref();
    source.with_file_name(format!(
        "{}{}",
        source.file_name().unwrap_or_default().to_string_lossy(),
        LINEAGE_SUFFIX
    ))
}

pub fn constant_registry_sha256() -> Result<String> {
    hash_canonical_json(&snapshots())
}

pub fn create_freeze(source: impl AsRef<Path>) -> Result<(PathBuf, LedgerEvent)> {
    let source = source.as_ref();
    require_regular_source(source, "freeze")?;
    let receipt_path = freeze_path(source);
    if receipt_path.exists() {
        return Err(GoblinError::freeze(format!(
            "Refusing to overwrite existing freeze receipt: {}",
            receipt_path.display()
        )));
    }
    let bytes = fs::read(source)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| GoblinError::freeze("Source is not valid UTF-8."))?;
    let parsed = parse_source(text)?;
    parsed.require_executable_program()?;
    let lineage_file = lineage_path(source);
    let lineage = if lineage_file.exists() {
        let report = verify_lineage(&lineage_file)?;
        if !report.0 {
            return Err(GoblinError::freeze(format!(
                "Refusing to freeze child with damaged lineage: {}",
                report.1
            )));
        }
        Some(json!({
            "path": lineage_file.file_name().unwrap().to_string_lossy(),
            "receipt_file_sha256": sha256_file(&lineage_file)?,
            "receipt_core_sha256": report.2.get("lineage_receipt_sha256").and_then(Value::as_str),
            "parent": report.2.get("parent"),
        }))
    } else {
        None
    };
    let root = ledger::project_root_for(source)?;
    let preflight = ledger::audit(&root);
    if preflight.exists && !preflight.verified {
        return Err(GoblinError::freeze(
            "Refusing to freeze while the custody ledger is damaged.",
        ));
    }
    let mut receipt = json!({
        "schema": "goblin.freeze-receipt.v1",
        "goblin_version": crate::VERSION,
        "created_at": timestamp(),
        "policy": "EXACT_SOURCE_BYTES_AND_CANONICAL_PROGRAM",
        "source": { "path": source.file_name().unwrap().to_string_lossy(), "sha256": sha256_bytes(&bytes) },
        "canonical_source": { "sha256": parsed.canonical_sha256()? },
        "constant_registry": { "sha256": constant_registry_sha256()?, "entries": snapshots() },
        "inline_rust": parsed.inline_rust.iter().map(|block| json!({"index": block.index, "sha256": block.sha256})).collect::<Vec<_>>(),
        "lineage": lineage,
    });
    let seal = sealed_hash(&receipt, "freeze_receipt_sha256")?;
    receipt
        .as_object_mut()
        .unwrap()
        .insert("freeze_receipt_sha256".into(), Value::String(seal.clone()));
    write_json_exclusive(&receipt_path, &receipt)?;
    let subject = ledger::subject_for(source, &root)?;
    let event = ledger::append(
        &root,
        "FREEZE",
        subject,
        json!({
            "source_sha256": sha256_bytes(&bytes), "canonical_source_sha256": parsed.canonical_sha256()?,
            "freeze_receipt_path": ledger::subject_for(&receipt_path, &root)?,
            "freeze_receipt_file_sha256": sha256_file(&receipt_path)?, "freeze_receipt_core_sha256": seal,
        }),
    )?;
    Ok((receipt_path, event))
}

pub fn verify_freeze(source: impl AsRef<Path>) -> FreezeReport {
    let source = source.as_ref();
    let path = freeze_path(source);
    let mut checks = Vec::new();
    let bytes = match fs::read(source) {
        Ok(value) => value,
        Err(error) => {
            return freeze_failure(
                &path,
                "SEMANTIC_CHANGE_AFTER_FREEZE",
                format!("Unable to read source: {error}"),
            );
        }
    };
    let source_sha = sha256_bytes(&bytes);
    let canonical_sha = std::str::from_utf8(&bytes)
        .ok()
        .and_then(|text| parse_source(text).ok())
        .and_then(|parsed| parsed.canonical_sha256().ok());
    let receipt_bytes = match fs::read(&path) {
        Ok(value) => value,
        Err(error) => {
            return FreezeReport {
                verified: false,
                classification: "FREEZE_RECEIPT_MISSING".into(),
                receipt_path: path.display().to_string(),
                receipt_file_sha256: None,
                receipt: None,
                current_source_sha256: Some(source_sha),
                current_canonical_sha256: canonical_sha,
                checks: vec![custody_check(
                    "FREEZE_RECEIPT_PRESENT",
                    false,
                    None,
                    None,
                    Some(error.to_string()),
                )],
            };
        }
    };
    let receipt: Value = match serde_json::from_slice(&receipt_bytes) {
        Ok(Value::Object(map)) => Value::Object(map),
        Ok(_) => {
            return freeze_failure(
                &path,
                "FREEZE_RECEIPT_TAMPERED",
                "Freeze receipt is not a JSON object.".into(),
            );
        }
        Err(error) => {
            return freeze_failure(
                &path,
                "FREEZE_RECEIPT_TAMPERED",
                format!("Invalid freeze receipt JSON: {error}"),
            );
        }
    };
    let field = |path: &[&str]| -> Option<String> {
        let mut current = &receipt;
        for key in path {
            current = current.get(*key)?;
        }
        current.as_str().map(str::to_string)
    };
    let schema = field(&["schema"]);
    let schema_ok = schema.as_deref() == Some("goblin.freeze-receipt.v1");
    checks.push(custody_check(
        "SCHEMA_SUPPORTED",
        schema_ok,
        Some("goblin.freeze-receipt.v1".into()),
        schema,
        None,
    ));
    let expected_seal = field(&["freeze_receipt_sha256"]);
    let actual_seal = sealed_hash(&receipt, "freeze_receipt_sha256").ok();
    let seal_ok = expected_seal == actual_seal;
    checks.push(custody_check(
        "FREEZE_RECEIPT_SHA256",
        seal_ok,
        expected_seal.clone(),
        actual_seal,
        None,
    ));
    let expected_path = field(&["source", "path"]);
    let actual_path = source
        .file_name()
        .map(|value| value.to_string_lossy().to_string());
    let path_ok = expected_path == actual_path;
    checks.push(custody_check(
        "SOURCE_PATH",
        path_ok,
        expected_path,
        actual_path,
        None,
    ));
    let expected_source = field(&["source", "sha256"]);
    let source_ok = expected_source.as_deref() == Some(&source_sha);
    checks.push(custody_check(
        "SOURCE_SHA256",
        source_ok,
        expected_source,
        Some(source_sha.clone()),
        None,
    ));
    let expected_canonical = field(&["canonical_source", "sha256"]);
    let canonical_ok = expected_canonical == canonical_sha;
    checks.push(custody_check(
        "CANONICAL_SOURCE_SHA256",
        canonical_ok,
        expected_canonical,
        canonical_sha.clone(),
        None,
    ));
    let expected_registry = field(&["constant_registry", "sha256"]);
    let actual_registry = constant_registry_sha256().ok();
    let registry_ok = expected_registry == actual_registry;
    checks.push(custody_check(
        "CONSTANT_REGISTRY_SHA256",
        registry_ok,
        expected_registry,
        actual_registry,
        None,
    ));
    let (lineage_ok, lineage_detail) =
        if let Some(lineage) = receipt.get("lineage").filter(|value| !value.is_null()) {
            let expected_file = lineage.get("receipt_file_sha256").and_then(Value::as_str);
            let lineage_file = lineage_path(source);
            match sha256_file(&lineage_file) {
                Ok(actual) if Some(actual.as_str()) == expected_file => {
                    match verify_lineage(&lineage_file) {
                        Ok((true, _, _)) => (true, None),
                        Ok((false, detail, _)) => (false, Some(detail)),
                        Err(error) => (false, Some(error.to_string())),
                    }
                }
                Ok(actual) => (false, Some(format!("lineage file hash mismatch: {actual}"))),
                Err(error) => (false, Some(error.to_string())),
            }
        } else {
            (true, None)
        };
    checks.push(custody_check(
        "LINEAGE_RECEIPT",
        lineage_ok,
        None,
        None,
        lineage_detail,
    ));
    let classification = if !schema_ok || !seal_ok {
        "FREEZE_RECEIPT_TAMPERED"
    } else if !path_ok {
        "FROZEN_SOURCE_IDENTITY_CHANGED"
    } else if !registry_ok {
        "CONSTANT_REGISTRY_CHANGED_AFTER_FREEZE"
    } else if !lineage_ok {
        "LINEAGE_RECEIPT_TAMPERED"
    } else if !source_ok && canonical_ok {
        "NOTATION_ONLY_CHANGE_AFTER_FREEZE"
    } else if !source_ok || !canonical_ok {
        "SEMANTIC_CHANGE_AFTER_FREEZE"
    } else {
        "FROZEN_SOURCE_VERIFIED"
    };
    FreezeReport {
        verified: checks.iter().all(|item| item.pass),
        classification: classification.into(),
        receipt_path: path.display().to_string(),
        receipt_file_sha256: Some(sha256_bytes(&receipt_bytes)),
        receipt: Some(receipt),
        current_source_sha256: Some(source_sha),
        current_canonical_sha256: canonical_sha,
        checks,
    }
}

pub fn verify_registration(source: impl AsRef<Path>, report: &FreezeReport) -> Result<bool> {
    if !report.verified {
        return Ok(false);
    }
    let source = source.as_ref();
    let root = ledger::project_root_for(source)?;
    let ledger_report = ledger::audit(&root);
    if !ledger_report.verified {
        return Ok(false);
    }
    let subject = ledger::subject_for(source, &root)?;
    let receipt = report.receipt.as_ref().unwrap();
    let core = receipt.get("freeze_receipt_sha256").and_then(Value::as_str);
    Ok(ledger_report.events.iter().rev().any(|event| {
        event.event_type == "FREEZE"
            && event.subject == subject
            && event
                .payload
                .get("freeze_receipt_core_sha256")
                .and_then(Value::as_str)
                == core
            && event
                .payload
                .get("freeze_receipt_file_sha256")
                .and_then(Value::as_str)
                == report.receipt_file_sha256.as_deref()
    }))
}

pub fn create_revision(
    parent: impl AsRef<Path>,
    child: impl AsRef<Path>,
    reason: &str,
) -> Result<(PathBuf, PathBuf, LedgerEvent)> {
    let parent = parent.as_ref();
    let child = child.as_ref();
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(GoblinError::revision(
            "A non-empty revision reason is required.",
        ));
    }
    require_regular_source(parent, "revise")?;
    if child.exists() || freeze_path(child).exists() || lineage_path(child).exists() {
        return Err(GoblinError::revision(format!(
            "Refusing to overwrite existing child or custody data: {}",
            child.display()
        )));
    }
    let report = verify_freeze(parent);
    if !report.verified {
        return Err(GoblinError::revision(format!(
            "Parent freeze verification failed: {}",
            report.classification
        )));
    }
    if !verify_registration(parent, &report)? {
        return Err(GoblinError::revision(
            "Parent freeze is not registered in a clean custody ledger.",
        ));
    }
    let root = ledger::project_root_for(parent)?;
    let child_parent = child
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(child_parent)?;
    let child_parent_root = child_parent.canonicalize()?;
    if !child_parent_root.starts_with(&root) {
        return Err(GoblinError::revision(
            "Child revision must remain inside the parent project.",
        ));
    }
    let parent_bytes = fs::read(parent)?;
    let parent_sha = sha256_bytes(&parent_bytes);
    let parent_canonical = report.current_canonical_sha256.clone();
    let parent_freeze = freeze_path(parent);
    let parent_display = relative_path(parent, child_parent);
    let mut lineage = json!({
        "schema": "goblin.revision-lineage.v1", "goblin_version": crate::VERSION,
        "created_at": timestamp(), "reason": reason,
        "parent": { "path": parent_display, "source_sha256": parent_sha, "canonical_source_sha256": parent_canonical,
            "freeze_receipt": { "path": parent_freeze.file_name().unwrap().to_string_lossy(), "file_sha256": sha256_file(&parent_freeze)?, "core_sha256": report.receipt.as_ref().and_then(|value| value.get("freeze_receipt_sha256")) } },
        "child": { "path": child.file_name().unwrap().to_string_lossy(), "initial_source_sha256": parent_sha, "initial_canonical_source_sha256": parent_canonical },
    });
    let seal = sealed_hash(&lineage, "lineage_receipt_sha256")?;
    lineage
        .as_object_mut()
        .unwrap()
        .insert("lineage_receipt_sha256".into(), Value::String(seal.clone()));
    let mut child_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(child)
        .map_err(|error| {
            GoblinError::revision(format!(
                "Unable to create child {}: {error}",
                child.display()
            ))
        })?;
    child_file.write_all(&parent_bytes)?;
    child_file.sync_all()?;
    let lineage_file = lineage_path(child);
    if let Err(error) = write_json_exclusive(&lineage_file, &lineage) {
        return Err(GoblinError::revision(format!(
            "Child source was created, but lineage receipt failed: {error}"
        )));
    }
    let event = ledger::append(
        &root,
        "REVISION",
        ledger::subject_for(child, &root)?,
        json!({
            "reason": reason, "parent_subject": ledger::subject_for(parent, &root)?, "parent_source_sha256": parent_sha,
            "parent_freeze_receipt_file_sha256": sha256_file(parent_freeze)?, "child_initial_source_sha256": parent_sha,
            "lineage_receipt_path": ledger::subject_for(&lineage_file, &root)?, "lineage_receipt_file_sha256": sha256_file(&lineage_file)?, "lineage_receipt_core_sha256": seal,
        }),
    )?;
    Ok((child.to_path_buf(), lineage_file, event))
}

pub fn verify_lineage(path: impl AsRef<Path>) -> Result<(bool, String, Value)> {
    let path = path.as_ref();
    let value: Value = serde_json::from_slice(&fs::read(path)?)
        .map_err(|error| GoblinError::revision(format!("Invalid lineage JSON: {error}")))?;
    if !value.is_object() {
        return Err(GoblinError::revision(
            "Lineage receipt must be a JSON object.",
        ));
    }
    let schema_ok =
        value.get("schema").and_then(Value::as_str) == Some("goblin.revision-lineage.v1");
    let expected = value
        .get("lineage_receipt_sha256")
        .and_then(Value::as_str)
        .map(str::to_string);
    let actual = sealed_hash(&value, "lineage_receipt_sha256")?;
    let ok = schema_ok && expected.as_deref() == Some(&actual);
    let detail = if !schema_ok {
        "Unsupported lineage schema".into()
    } else if !ok {
        "Lineage receipt seal mismatch".into()
    } else {
        "PASS".into()
    };
    Ok((ok, detail, value))
}

pub fn lineage_chain(source: impl AsRef<Path>) -> Result<Vec<Value>> {
    let mut current = source.as_ref().to_path_buf();
    let mut reverse = Vec::new();
    for _ in 0..256 {
        let freeze = verify_freeze(&current);
        let lineage_file = lineage_path(&current);
        let mut node = json!({ "path": current.display().to_string(), "state": if freeze.verified { "FROZEN_VERIFIED" } else if lineage_file.exists() { "REVISION_UNFROZEN" } else { "UNTRACKED" } });
        if !lineage_file.exists() {
            reverse.push(node);
            break;
        }
        let (verified, detail, lineage) = verify_lineage(&lineage_file)?;
        node.as_object_mut()
            .unwrap()
            .insert("lineage_verified".into(), Value::Bool(verified));
        node.as_object_mut().unwrap().insert(
            "reason".into(),
            lineage.get("reason").cloned().unwrap_or(Value::Null),
        );
        reverse.push(node);
        if !verified {
            return Err(GoblinError::revision(detail));
        }
        let parent = lineage
            .get("parent")
            .and_then(|value| value.get("path"))
            .and_then(Value::as_str)
            .ok_or_else(|| GoblinError::revision("Lineage receipt has no parent path."))?;
        current = current
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(parent);
    }
    if reverse.len() == 256 {
        return Err(GoblinError::revision(
            "Lineage exceeds 256 generations or contains a cycle.",
        ));
    }
    reverse.reverse();
    Ok(reverse)
}

fn sealed_hash(value: &Value, field: &str) -> Result<String> {
    let mut core = value.clone();
    if let Some(map) = core.as_object_mut() {
        map.remove(field);
    }
    hash_canonical_json(&core)
}
fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}
fn write_json_exclusive(path: &Path, value: &Value) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            GoblinError::freeze(format!("Refusing to overwrite {}: {error}", path.display()))
        })?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
fn require_regular_source(path: &Path, action: &str) -> Result<()> {
    if path.is_symlink() {
        return Err(GoblinError::freeze(format!(
            "Refusing symbolic-link source during {action}."
        )));
    }
    if !path.is_file() {
        return Err(GoblinError::freeze(format!(
            "Source file not found: {}",
            path.display()
        )));
    }
    Ok(())
}
fn custody_check(
    name: &str,
    pass: bool,
    expected: Option<String>,
    actual: Option<String>,
    detail: Option<String>,
) -> CustodyCheck {
    CustodyCheck {
        check: name.into(),
        pass,
        expected,
        actual,
        detail,
    }
}
fn freeze_failure(path: &Path, classification: &str, detail: String) -> FreezeReport {
    FreezeReport {
        verified: false,
        classification: classification.into(),
        receipt_path: path.display().to_string(),
        receipt_file_sha256: None,
        receipt: None,
        current_source_sha256: None,
        current_canonical_sha256: None,
        checks: vec![custody_check(
            "FREEZE_RECEIPT",
            false,
            None,
            None,
            Some(detail),
        )],
    }
}
fn relative_path(path: &Path, base: &Path) -> String {
    path.canonicalize()
        .ok()
        .and_then(|path| {
            base.canonicalize().ok().and_then(|base| {
                path.strip_prefix(&base)
                    .ok()
                    .map(|value| value.to_string_lossy().to_string())
            })
        })
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    fn program() -> &'static str {
        "GO_PARANOID\nmass = 1 kg\nenergy = mass * c^2\nseal energy\n"
    }
    #[test]
    fn freeze_detects_notation_and_semantic_changes() {
        let root = tempdir().unwrap();
        let source = root.path().join("energy.gbl");
        fs::write(&source, program()).unwrap();
        create_freeze(&source).unwrap();
        assert!(verify_freeze(&source).verified);
        fs::write(&source, program().replace("mass * c^2", "mass c²")).unwrap();
        assert_eq!(
            verify_freeze(&source).classification,
            "NOTATION_ONLY_CHANGE_AFTER_FREEZE"
        );
        fs::write(&source, program().replace("c^2", "c^3")).unwrap();
        assert_eq!(
            verify_freeze(&source).classification,
            "SEMANTIC_CHANGE_AFTER_FREEZE"
        );
    }
    #[test]
    fn revision_requires_reason_and_records_lineage() {
        let root = tempdir().unwrap();
        let parent = root.path().join("a.gbl");
        fs::write(&parent, program()).unwrap();
        create_freeze(&parent).unwrap();
        assert_eq!(
            create_revision(&parent, root.path().join("bad.gbl"), " ")
                .unwrap_err()
                .code,
            "G403"
        );
        let (child, lineage, _) =
            create_revision(&parent, root.path().join("b.gbl"), "notation experiment").unwrap();
        assert_eq!(fs::read(child).unwrap(), fs::read(parent).unwrap());
        assert!(verify_lineage(lineage).unwrap().0);
    }
}
