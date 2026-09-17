use crate::error::{GoblinError, Result};
use crate::hashing::{hash_canonical_json, sha256_file};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEvent {
    pub schema: String,
    pub sequence: u64,
    pub timestamp: String,
    pub event_type: String,
    pub subject: String,
    pub payload: Value,
    pub previous_event_sha256: Option<String>,
    pub event_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerCheck {
    pub check: String,
    pub pass: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerReport {
    pub ledger_path: String,
    pub exists: bool,
    pub verified: bool,
    pub status: String,
    pub authentication: String,
    pub head_event_sha256: Option<String>,
    pub events: Vec<LedgerEvent>,
    pub checks: Vec<LedgerCheck>,
}

pub fn project_root_for(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    let start = if path.is_dir() {
        path
    } else {
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
    };
    let absolute = start.canonicalize().map_err(|error| {
        GoblinError::ledger(format!(
            "Unable to resolve project root from {}: {error}",
            path.display()
        ))
    })?;
    let mut cargo_fallback = None;
    for ancestor in absolute.ancestors() {
        if ancestor.join(".goblin").is_dir()
            || ancestor.join("goblin.toml").is_file()
            || ancestor.join(".git").exists()
        {
            return Ok(ancestor.to_path_buf());
        }
        if cargo_fallback.is_none() && ancestor.join("Cargo.toml").is_file() {
            cargo_fallback = Some(ancestor.to_path_buf());
        }
    }
    Ok(cargo_fallback.unwrap_or(absolute))
}

pub fn ledger_path(project_root: impl AsRef<Path>) -> PathBuf {
    project_root.as_ref().join(".goblin/custody-ledger.jsonl")
}
pub fn head_path(project_root: impl AsRef<Path>) -> PathBuf {
    project_root
        .as_ref()
        .join(".goblin/custody-ledger-head.json")
}

pub fn subject_for(path: impl AsRef<Path>, root: impl AsRef<Path>) -> Result<String> {
    let root = root.as_ref().canonicalize()?;
    let absolute = if path.as_ref().exists() {
        path.as_ref().canonicalize()?
    } else {
        let parent = path
            .as_ref()
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .canonicalize()?;
        parent.join(path.as_ref().file_name().unwrap_or_default())
    };
    absolute
        .strip_prefix(&root)
        .map(|value| value.to_string_lossy().replace('\\', "/"))
        .map_err(|_| {
            GoblinError::ledger(format!(
                "Custody subject {} is outside project {}.",
                absolute.display(),
                root.display()
            ))
        })
}

pub fn audit(project: impl AsRef<Path>) -> LedgerReport {
    let root =
        project_root_for(project.as_ref()).unwrap_or_else(|_| project.as_ref().to_path_buf());
    let path = ledger_path(&root);
    if !path.exists() {
        return LedgerReport {
            ledger_path: path.display().to_string(),
            exists: false,
            verified: true,
            status: "PASS".into(),
            authentication: "CHECKSUM_ONLY".into(),
            head_event_sha256: None,
            events: vec![],
            checks: vec![],
        };
    }
    let mut checks = Vec::new();
    let mut events = Vec::new();
    let file = match fs::File::open(&path) {
        Ok(file) => file,
        Err(error) => return failed_report(&path, format!("Unable to read ledger: {error}")),
    };
    let mut previous: Option<String> = None;
    for (line_index, line) in BufReader::new(file).lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                return failed_report(
                    &path,
                    format!("Unable to read ledger line {}: {error}", line_index + 1),
                );
            }
        };
        if line.trim().is_empty() {
            checks.push(check(
                "NO_BLANK_LINES",
                false,
                Some(format!("blank line {}", line_index + 1)),
            ));
            continue;
        }
        let event: LedgerEvent = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                checks.push(check(
                    "EVENT_JSON",
                    false,
                    Some(format!("line {}: {error}", line_index + 1)),
                ));
                break;
            }
        };
        let sequence_ok = event.sequence == events.len() as u64 + 1;
        checks.push(check(
            "EVENT_SEQUENCE",
            sequence_ok,
            (!sequence_ok)
                .then(|| format!("expected {}, got {}", events.len() + 1, event.sequence)),
        ));
        let previous_ok = event.previous_event_sha256 == previous;
        checks.push(check(
            "PREVIOUS_EVENT_SHA256",
            previous_ok,
            (!previous_ok).then(|| format!("event {} chain link mismatch", event.sequence)),
        ));
        let expected_hash = event_hash(&event).ok();
        let hash_ok = expected_hash.as_ref() == Some(&event.event_sha256);
        checks.push(check(
            "EVENT_SHA256",
            hash_ok,
            (!hash_ok).then(|| format!("event {} seal mismatch", event.sequence)),
        ));
        previous = Some(event.event_sha256.clone());
        events.push(event);
    }
    let head_file = head_path(&root);
    let mut head_ok = false;
    if let Ok(value) = fs::read_to_string(&head_file)
        && let Ok(json) = serde_json::from_str::<Value>(&value)
    {
        head_ok = json.get("head_event_sha256").and_then(Value::as_str) == previous.as_deref()
            && json.get("sequence").and_then(Value::as_u64) == Some(events.len() as u64);
    }
    checks.push(check(
        "HEAD_CHECKPOINT",
        head_ok,
        (!head_ok).then(|| "ledger head checkpoint is missing or disagrees".into()),
    ));
    let verified = checks.iter().all(|item| item.pass);
    LedgerReport {
        ledger_path: path.display().to_string(),
        exists: true,
        verified,
        status: if verified { "PASS" } else { "FAIL" }.into(),
        authentication: "CHECKSUM_ONLY".into(),
        head_event_sha256: previous,
        events,
        checks,
    }
}

pub fn append(
    project_root: impl AsRef<Path>,
    event_type: &str,
    subject: String,
    payload: Value,
) -> Result<LedgerEvent> {
    let root = project_root.as_ref();
    fs::create_dir_all(root.join(".goblin"))?;
    let _lock = LedgerLock::acquire(root)?;
    let report = audit(root);
    if report.exists && !report.verified {
        return Err(GoblinError::ledger(
            "Refusing to append to a damaged custody ledger.",
        ));
    }
    let mut event = LedgerEvent {
        schema: "goblin.custody-event.v1".into(),
        sequence: report.events.len() as u64 + 1,
        timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
        event_type: event_type.into(),
        subject,
        payload,
        previous_event_sha256: report.head_event_sha256,
        event_sha256: String::new(),
    };
    event.event_sha256 = event_hash(&event)?;
    let path = ledger_path(root);
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    serde_json::to_writer(&mut file, &event)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    let checkpoint = serde_json::json!({
        "schema": "goblin.custody-head.v1", "sequence": event.sequence,
        "head_event_sha256": event.event_sha256, "ledger_file_sha256": sha256_file(&path)?,
    });
    atomic_write_json(&head_path(root), &checkpoint)?;
    Ok(event)
}

fn event_hash(event: &LedgerEvent) -> Result<String> {
    let mut core = serde_json::to_value(event)?;
    core.as_object_mut().unwrap().remove("event_sha256");
    hash_canonical_json(&core)
}

fn atomic_write_json(path: &Path, value: &Value) -> Result<()> {
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    Ok(())
}

fn check(name: &str, pass: bool, detail: Option<String>) -> LedgerCheck {
    LedgerCheck {
        check: name.into(),
        pass,
        detail,
    }
}
fn failed_report(path: &Path, detail: String) -> LedgerReport {
    LedgerReport {
        ledger_path: path.display().to_string(),
        exists: true,
        verified: false,
        status: "FAIL".into(),
        authentication: "CHECKSUM_ONLY".into(),
        head_event_sha256: None,
        events: vec![],
        checks: vec![check("LEDGER_READ", false, Some(detail))],
    }
}

struct LedgerLock {
    path: PathBuf,
}
impl LedgerLock {
    fn acquire(root: &Path) -> Result<Self> {
        let path = root.join(".goblin/custody-ledger.lock");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                GoblinError::ledger(format!(
                    "Custody ledger is locked by another operation ({}): {error}",
                    path.display()
                ))
            })?;
        Ok(Self { path })
    }
}
impl Drop for LedgerLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn ledger_chain_detects_tampering() {
        let root = tempdir().unwrap();
        append(
            root.path(),
            "TEST",
            "x.gbl".into(),
            serde_json::json!({"x": 1}),
        )
        .unwrap();
        append(
            root.path(),
            "TEST",
            "x.gbl".into(),
            serde_json::json!({"x": 2}),
        )
        .unwrap();
        assert!(audit(root.path()).verified);
        let path = ledger_path(root.path());
        let changed = fs::read_to_string(&path)
            .unwrap()
            .replace("\"x\":1", "\"x\":9");
        fs::write(path, changed).unwrap();
        assert!(!audit(root.path()).verified);
    }
}
