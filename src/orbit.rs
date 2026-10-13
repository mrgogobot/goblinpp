//! Orbit v1: bounded, read-only observations, not a security verdict.
use crate::error::{GoblinError, Result};
use crate::evidence_report::{self, CaseDocument, Evidence, Purpose};
use crate::hashing::sha256_bytes;
use chrono::{DateTime, SecondsFormat, Utc};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "orbit_scan.rs"]
mod scanner;

pub const POLICY: &str = "goblin.orbit.bytes-only.v1";
pub const NOTICE: &str = "Authorized local scope only. The user is responsible for permission, confidentiality, lawful handling and interpretation. Baseline capture does not establish known-good state. A mismatch may be tampering or a legitimate change. MATCH is byte/entry agreement, not a clean-system verdict. No repair, execution, network scanning, media-origin detection or escalation. Reads may update OS access times. This is not an atomic snapshot, rootkit detection, authenticated authorship, certified time or legal admissibility. A privileged attacker or hostile filesystem can undermine observations. Original file contents are not retained.";
const JSON_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Args, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// All entries, including directories and unsupported entries (hard cap 4096).
    #[arg(long, default_value_t = 1024)]
    pub max_entries: usize,
    #[arg(long, default_value_t = 16)]
    pub max_depth: usize,
    #[arg(long, default_value_t = 268_435_456)]
    pub max_file_bytes: u64,
    #[arg(long, default_value_t = 1_073_741_824)]
    pub max_total_bytes: u64,
    /// Cooperative wall-clock deadline, not a hard timeout on blocked OS I/O.
    #[arg(long, default_value_t = 60)]
    pub max_seconds: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: 1024,
            max_depth: 16,
            max_file_bytes: 268_435_456,
            max_total_bytes: 1_073_741_824,
            max_seconds: 60,
        }
    }
}

impl Limits {
    pub fn validate(&self) -> Result<()> {
        if !(1..=4096).contains(&self.max_entries)
            || !(1..=32).contains(&self.max_depth)
            || !(1..=1_073_741_824).contains(&self.max_file_bytes)
            || !(1..=8_589_934_592).contains(&self.max_total_bytes)
            || !(1..=3600).contains(&self.max_seconds)
        {
            return Err(invalid(
                "Limits outside v1 hard caps: entries 1..4096, depth 1..32, file 1..1GiB, total 1..8GiB, seconds 1..3600",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Kind {
    File,
    Directory,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Observation {
    Hashed,
    Directory,
    Unreadable,
    Unsupported,
    UnstableDuringRead,
    BudgetExceeded,
}

/// Metadata detects some changes during reading; not an ACL/security assessment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Fingerprint {
    pub bytes: u64,
    pub device: u64,
    pub inode: u64,
    pub mode: u32,
    pub modified: String,
    pub changed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub path: String,
    pub kind: Kind,
    pub observation: Observation,
    pub bytes_read: u64,
    pub sha256: Option<String>,
    pub before: Option<Fingerprint>,
    pub after: Option<Fingerprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema: String,
    pub policy: String,
    pub tool_version: String,
    pub root: String,
    pub started_utc: String,
    pub finished_utc: String,
    pub os: String,
    pub arch: String,
    pub limits: Limits,
    pub bytes_read: u64,
    pub complete: bool,
    /// Coverage issues are retained even when no safe pathname can be emitted.
    pub issues: Vec<String>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub schema: String,
    pub notice: String,
    pub trust_declaration: String,
    pub authorization: String,
    pub reason: String,
    /// External digest of the preserved immediate parent, not a signature.
    pub parent_baseline_sha256: Option<String>,
    pub snapshot: Snapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Classification {
    Match,
    ContentChanged,
    Missing,
    Added,
    Unreadable,
    UnstableDuringRead,
    Unsupported,
    NotObserved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
    pub path: String,
    pub classification: Classification,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub schema: String,
    pub policy: String,
    pub baseline_sha256: String,
    pub observation_sha256: String,
    pub status: String,
    pub complete: bool,
    pub results: Vec<Comparison>,
    pub notice: String,
}

#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub check: Check,
    pub receipt_core_sha256: String,
    pub directory: PathBuf,
}

pub(crate) fn invalid(message: impl Into<String>) -> GoblinError {
    GoblinError::new("G802", message)
}
pub(crate) fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn safe_text(value: &str) -> Result<()> {
    if value.trim().is_empty()
        || value.len() > 4096
        || value.chars().any(|c| {
            c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
    {
        return Err(invalid(
            "Require nonempty text <=4096 UTF-8 bytes, without controls or bidi overrides",
        ));
    }
    Ok(())
}

pub(crate) fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && !value.contains(['\\', ':'])
        && safe_text(value).is_ok()
        && value
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != "..")
        && Path::new(value)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(invalid(
            "Require an independently retained, lowercase SHA-256 digest",
        ));
    }
    Ok(())
}

fn utc(value: &str) -> Result<()> {
    if value.len() > 64
        || !DateTime::parse_from_rfc3339(value).is_ok_and(|t| t.offset().local_minus_utc() == 0)
    {
        return Err(invalid("Invalid declared UTC time"));
    }
    Ok(())
}

/// Resolve a path without silently accepting symlink components or traversal.
pub(crate) fn absolute(path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut checked = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir | Component::Normal(_) => checked.push(component),
            Component::CurDir => continue,
            _ => return Err(invalid("Path traversal is not supported")),
        }
        match fs::symlink_metadata(&checked) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(invalid(
                    "Symlink path components are not supported; use the actual directory path",
                ));
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && checked == path => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(checked)
}

fn outside(root: &Path, path: &Path) -> Result<PathBuf> {
    let path = absolute(path)?;
    if path.starts_with(root) {
        return Err(invalid(
            "Baseline/report destinations and baseline inputs must be outside the inspected root",
        ));
    }
    Ok(path)
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    if bytes.len() > JSON_LIMIT {
        return Err(invalid("Orbit JSON exceeds 8MiB; reduce scope"));
    }
    Ok(bytes)
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let path = absolute(path)?;
    if !fs::symlink_metadata(&path)?.is_file() {
        return Err(invalid("Require a regular Orbit JSON file"));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_NOCTTY);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("Orbit JSON changed to a non-regular file"));
    }
    let mut bytes = Vec::new();
    file.take((JSON_LIMIT + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > JSON_LIMIT {
        return Err(invalid("Orbit JSON exceeds 8MiB"));
    }
    Ok(bytes)
}

fn new_directory(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn scan(root: &Path, limits: &Limits) -> Result<Snapshot> {
    limits.validate()?;
    let root = absolute(root)?;
    if root.parent().is_none() || !fs::symlink_metadata(&root)?.is_dir() {
        return Err(invalid(
            "Select an ordinary directory, not the filesystem root",
        ));
    }
    safe_text(root.to_str().ok_or_else(|| invalid("Root must be UTF-8"))?)?;
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        scanner::scan(&root, limits)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Err(invalid(
            "Orbit v1 scanning supports Linux/macOS only; this platform is unsupported",
        ))
    }
}

fn validate_snapshot(snapshot: &Snapshot) -> Result<()> {
    if snapshot.schema != "goblin.orbit-snapshot.v1" || snapshot.policy != POLICY {
        return Err(invalid("Unsupported Orbit snapshot policy/schema"));
    }
    snapshot.limits.validate()?;
    for t in [&snapshot.started_utc, &snapshot.finished_utc] {
        utc(t)?;
    }
    if DateTime::parse_from_rfc3339(&snapshot.started_utc).unwrap()
        > DateTime::parse_from_rfc3339(&snapshot.finished_utc).unwrap()
    {
        return Err(invalid("Snapshot time range reversed"));
    }
    for t in [
        &snapshot.root,
        &snapshot.tool_version,
        &snapshot.os,
        &snapshot.arch,
    ] {
        safe_text(t)?;
    }
    if !Path::new(&snapshot.root).is_absolute()
        || Path::new(&snapshot.root)
            .components()
            .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
        || snapshot.entries.len() > snapshot.limits.max_entries
        || snapshot.issues.len() > 64
    {
        return Err(invalid("Invalid snapshot scope/entry/issue limits"));
    }
    for issue in &snapshot.issues {
        safe_text(issue)?;
    }
    let mut previous = "";
    let mut total = 0_u64;
    for entry in &snapshot.entries {
        if !safe_relative(&entry.path) || entry.path.as_str() <= previous {
            return Err(invalid(
                "Snapshot paths must be safe, unique and strictly sorted",
            ));
        }
        previous = &entry.path;
        for metadata in [&entry.before, &entry.after].into_iter().flatten() {
            for time in [&metadata.modified, &metadata.changed] {
                let valid = time.split_once(':').is_some_and(|(seconds, nanos)| {
                    seconds.parse::<i64>().is_ok()
                        && nanos.parse::<u32>().is_ok_and(|n| n < 1_000_000_000)
                });
                if !valid || time.len() > 64 {
                    return Err(invalid("Invalid metadata timestamp evidence"));
                }
            }
        }
        if entry.path.split('/').count() > snapshot.limits.max_depth + 1 {
            return Err(invalid("Entry exceeds declared traversal depth"));
        }
        total = total
            .checked_add(entry.bytes_read)
            .ok_or_else(|| invalid("Byte count overflow"))?;
        if entry.bytes_read > snapshot.limits.max_file_bytes {
            return Err(invalid("File budget exceeded in evidence"));
        }
        match entry.observation {
            Observation::Hashed => {
                digest(
                    entry
                        .sha256
                        .as_deref()
                        .ok_or_else(|| invalid("Missing file digest"))?,
                )?;
                if entry.kind != Kind::File
                    || entry.before.is_none()
                    || entry.before != entry.after
                    || entry.before.as_ref().unwrap().bytes != entry.bytes_read
                {
                    return Err(invalid("Inconsistent hashed-file evidence"));
                }
            }
            Observation::Directory => {
                if entry.kind != Kind::Directory
                    || entry.sha256.is_some()
                    || entry.bytes_read != 0
                    || entry.before.is_none()
                    || entry.before != entry.after
                {
                    return Err(invalid("Inconsistent directory evidence"));
                }
            }
            _ => {
                if entry.sha256.is_some() {
                    return Err(invalid(
                        "Incomplete observations cannot contain authoritative file digests",
                    ));
                }
            }
        }
    }
    if total != snapshot.bytes_read || total > snapshot.limits.max_total_bytes {
        return Err(invalid("Inconsistent total-byte evidence"));
    }
    let complete = snapshot.issues.is_empty()
        && snapshot
            .entries
            .iter()
            .all(|e| matches!(e.observation, Observation::Hashed | Observation::Directory));
    if snapshot.complete != complete {
        return Err(invalid("Inconsistent coverage claim"));
    }
    Ok(())
}

fn validate_baseline(baseline: &Baseline) -> Result<()> {
    if baseline.schema != "goblin.orbit-baseline.v1" || baseline.notice != NOTICE {
        return Err(invalid("Unsupported baseline schema/notice"));
    }
    for t in [
        &baseline.trust_declaration,
        &baseline.authorization,
        &baseline.reason,
    ] {
        safe_text(t)?;
    }
    if let Some(parent) = &baseline.parent_baseline_sha256 {
        digest(parent)?;
    }
    validate_snapshot(&baseline.snapshot)?;
    if !baseline.snapshot.complete {
        return Err(invalid("Incomplete baseline refused"));
    }
    Ok(())
}

fn pinned_baseline(path: &Path, expected: &str) -> Result<(Baseline, Vec<u8>)> {
    digest(expected)?;
    let bytes = read(path)?;
    if sha256_bytes(&bytes) != expected {
        return Err(invalid(
            "Trusted baseline SHA-256 mismatch; do not replace the trusted hash just to silence this error",
        ));
    }
    let baseline: Baseline = serde_json::from_slice(&bytes)?;
    validate_baseline(&baseline)?;
    Ok((baseline, bytes))
}

/// Verify a captured baseline and its preserved immediate parent, if declared.
/// A hash anchor is not an assertion that the original directory was safe.
pub fn verify_baseline(directory: &Path, expected: &str) -> Result<Baseline> {
    let directory = absolute(directory)?;
    let (baseline, _) = pinned_baseline(&directory.join("baseline.json"), expected)?;
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(&directory)?.take(4) {
        names.insert(entry?.file_name());
    }
    let mut required = BTreeSet::from([std::ffi::OsString::from("baseline.json")]);
    if let Some(hash) = &baseline.parent_baseline_sha256 {
        required.insert(std::ffi::OsString::from("parent-baseline.json"));
        let (parent, _) = pinned_baseline(&directory.join("parent-baseline.json"), hash)?;
        if parent.snapshot.root != baseline.snapshot.root {
            return Err(invalid("Parent scope mismatch"));
        }
    }
    if names != required {
        return Err(invalid("Unexpected or missing baseline bundle files"));
    }
    Ok(baseline)
}

pub struct BaselineOptions<'a> {
    pub reason: &'a str,
    pub trust_declaration: &'a str,
    pub authorization: &'a str,
    pub parent: Option<(&'a Path, &'a str)>,
}

/// Capture only complete baselines. A new directory and explicit reason are mandatory.
pub fn capture(
    root: &Path,
    output: &Path,
    limits: &Limits,
    options: BaselineOptions<'_>,
) -> Result<String> {
    for t in [
        options.reason,
        options.trust_declaration,
        options.authorization,
    ] {
        safe_text(t)?;
    }
    let root = absolute(root)?;
    let output = outside(&root, output)?;
    if output.exists() {
        return Err(invalid(
            "Baseline destination already exists; nothing overwritten",
        ));
    }
    let parent_bytes = if let Some((path, hash)) = options.parent {
        let path = outside(&root, path)?;
        let (parent, bytes) = pinned_baseline(&path, hash)?;
        if parent.snapshot.root != root.to_string_lossy() {
            return Err(invalid("Re-baselining requires the same declared root"));
        }
        Some(bytes)
    } else {
        None
    };
    let snapshot = scan(&root, limits)?;
    if !snapshot.complete {
        let failures: Vec<_> = snapshot
            .entries
            .iter()
            .filter(|e| !matches!(e.observation, Observation::Hashed | Observation::Directory))
            .take(8)
            .map(|e| (&e.path, &e.observation))
            .collect();
        return Err(invalid(format!(
            "Incomplete scan; no trusted baseline created. Issues: {:?}; first incomplete entries: {:?}. Reduce scope or resolve coverage gaps; do not treat skipped files as safe.",
            snapshot.issues, failures
        )));
    }
    let baseline = Baseline {
        schema: "goblin.orbit-baseline.v1".into(),
        notice: NOTICE.into(),
        reason: options.reason.into(),
        trust_declaration: options.trust_declaration.into(),
        authorization: options.authorization.into(),
        parent_baseline_sha256: options.parent.map(|(_, h)| h.to_owned()),
        snapshot,
    };
    validate_baseline(&baseline)?;
    let bytes = encode(&baseline)?;
    new_directory(&output)?;
    if let Some(parent) = parent_bytes {
        write(&output.join("parent-baseline.json"), &parent)?;
    }
    write(&output.join("baseline.json"), &bytes)?;
    Ok(sha256_bytes(&bytes))
}

fn comparison(
    baseline: &Baseline,
    snapshot: &Snapshot,
    baseline_hash: &str,
    observation_hash: &str,
) -> Check {
    let old: BTreeMap<_, _> = baseline
        .snapshot
        .entries
        .iter()
        .map(|e| (&e.path, e))
        .collect();
    let new: BTreeMap<_, _> = snapshot.entries.iter().map(|e| (&e.path, e)).collect();
    let paths: BTreeSet<_> = old.keys().chain(new.keys()).copied().collect();
    let results: Vec<_> = paths
        .into_iter()
        .map(|path| {
            let classification = match new.get(path) {
                None if snapshot.complete => Classification::Missing,
                None => Classification::NotObserved,
                Some(e) => match e.observation {
                    Observation::Unreadable => Classification::Unreadable,
                    Observation::Unsupported => Classification::Unsupported,
                    Observation::UnstableDuringRead => Classification::UnstableDuringRead,
                    Observation::BudgetExceeded => Classification::NotObserved,
                    _ => match old.get(path) {
                        None => Classification::Added,
                        Some(before) if before.kind == e.kind && before.sha256 == e.sha256 => {
                            Classification::Match
                        }
                        Some(_) => Classification::ContentChanged,
                    },
                },
            };
            Comparison {
                path: path.clone(),
                classification,
            }
        })
        .collect();
    let status = if !snapshot.complete {
        "INCOMPLETE"
    } else if results
        .iter()
        .all(|r| r.classification == Classification::Match)
    {
        "MATCH"
    } else {
        "REVIEW_REQUIRED"
    };
    Check {
        schema: "goblin.orbit-check.v1".into(),
        policy: POLICY.into(),
        baseline_sha256: baseline_hash.into(),
        observation_sha256: observation_hash.into(),
        status: status.into(),
        complete: snapshot.complete,
        results,
        notice: NOTICE.into(),
    }
}

fn case(
    check: &Check,
    snapshot: &Snapshot,
    author: &str,
    authorization: &str,
    baseline_bytes: &[u8],
    observation_bytes: &[u8],
    check_bytes: &[u8],
) -> CaseDocument {
    let mut summary = format!(
        "{}; {} compared entries. {}\n\n",
        check.status,
        check.results.len(),
        NOTICE
    );
    summary.push_str("Orbit directory observation precedes report generation. The offline-report disclaimer refers to rendering these JSON copies, not to the earlier directory observation.\n\n");
    let mismatches: Vec<_> = check
        .results
        .iter()
        .filter(|r| r.classification != Classification::Match)
        .collect();
    for row in mismatches.iter().take(8) {
        let line = format!("{:?}: {}\n", row.classification, row.path);
        if summary.len() + line.len() > 4096 {
            break;
        }
        summary.push_str(&line);
    }
    if !mismatches.is_empty() {
        summary.push_str("Full entry classifications are preserved in comparison evidence; this preview is bounded.\n");
    }
    if !snapshot.complete {
        summary.push_str("Coverage is incomplete. Missing entries cannot be asserted; inspect observations evidence for issues/budgets.\n");
    }
    CaseDocument {
        schema: evidence_report::CASE_SCHEMA.into(), case_id: "orbit-check".into(), purpose: Purpose::ForensicAnalysis,
        title: "Orbit scoped integrity observations (not a compromise verdict)".into(), author: author.into(), authorization: authorization.into(),
        scope: vec![snapshot.root.clone()], summary,
        evidence: [("baseline", "baseline.json", baseline_bytes), ("observations", "observations.json", observation_bytes), ("comparison", "orbit-check.json", check_bytes)].into_iter().map(|(id, path, bytes)| Evidence {
            id: id.into(), path: path.into(), sha256: sha256_bytes(bytes), description: "Machine observations/declarations only; source contents are not retained or independently rehashed by report verification".into(), collected_utc: snapshot.finished_utc.clone(),
        }).collect(), findings: vec![],
    }
}

pub fn check(
    root: &Path,
    baseline_file: &Path,
    expected: &str,
    output: &Path,
    limits: &Limits,
    author: &str,
    authorization: &str,
) -> Result<CheckResult> {
    safe_text(author)?;
    safe_text(authorization)?;
    let root = absolute(root)?;
    let output = outside(&root, output)?;
    let baseline_file = outside(&root, baseline_file)?;
    if output.exists() {
        return Err(invalid(
            "Check destination already exists; nothing overwritten",
        ));
    }
    let (baseline, baseline_bytes) = pinned_baseline(&baseline_file, expected)?;
    if baseline.snapshot.root != root.to_string_lossy() {
        return Err(invalid(
            "Baseline root differs; scope relocation is not implicit",
        ));
    }
    let snapshot = scan(&root, limits)?;
    validate_snapshot(&snapshot)?;
    let observation_bytes = encode(&snapshot)?;
    let check = comparison(
        &baseline,
        &snapshot,
        expected,
        &sha256_bytes(&observation_bytes),
    );
    let check_bytes = encode(&check)?;
    let case = case(
        &check,
        &snapshot,
        author,
        authorization,
        &baseline_bytes,
        &observation_bytes,
        &check_bytes,
    );
    evidence_report::validate_case(&case)?;
    let case_bytes = encode(&case)?;
    new_directory(&output)?;
    for (name, bytes) in [
        ("baseline.json", &baseline_bytes),
        ("observations.json", &observation_bytes),
        ("orbit-check.json", &check_bytes),
        ("case.json", &case_bytes),
    ] {
        write(&output.join(name), bytes)?;
    }
    let receipt =
        evidence_report::create_report(&output.join("case.json"), &output.join("report"))?;
    verify(&output, expected, Some(&receipt.receipt_core_sha256))?;
    Ok(CheckResult {
        check,
        receipt_core_sha256: receipt.receipt_core_sha256,
        directory: output,
    })
}

/// Offline consistency/copy verification. Does not rehash the inspected originals.
pub fn verify(
    directory: &Path,
    expected_baseline: &str,
    expected_receipt: Option<&str>,
) -> Result<Check> {
    let directory = absolute(directory)?;
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(&directory)?.take(7) {
        names.insert(entry?.file_name());
    }
    let expected_names: BTreeSet<_> = [
        "baseline.json",
        "observations.json",
        "orbit-check.json",
        "case.json",
        "report",
    ]
    .into_iter()
    .map(std::ffi::OsString::from)
    .collect();
    if names != expected_names {
        return Err(invalid("Unexpected or missing Orbit bundle files"));
    }
    let (baseline, baseline_bytes) =
        pinned_baseline(&directory.join("baseline.json"), expected_baseline)?;
    let observation_bytes = read(&directory.join("observations.json"))?;
    let snapshot: Snapshot = serde_json::from_slice(&observation_bytes)?;
    validate_snapshot(&snapshot)?;
    if snapshot.root != baseline.snapshot.root {
        return Err(invalid("Observation scope differs from baseline"));
    }
    let actual: Check = serde_json::from_slice(&read(&directory.join("orbit-check.json"))?)?;
    let derived = comparison(
        &baseline,
        &snapshot,
        expected_baseline,
        &sha256_bytes(&observation_bytes),
    );
    if actual != derived {
        return Err(invalid(
            "Orbit classification/coverage does not match recorded observations",
        ));
    }
    let case_bytes = read(&directory.join("case.json"))?;
    let declared: CaseDocument = serde_json::from_slice(&case_bytes)?;
    let check_bytes = read(&directory.join("orbit-check.json"))?;
    let reconstructed = case(
        &actual,
        &snapshot,
        &declared.author,
        &declared.authorization,
        &baseline_bytes,
        &observation_bytes,
        &check_bytes,
    );
    if serde_json::to_value(&declared)? != serde_json::to_value(&reconstructed)? {
        return Err(invalid(
            "Orbit report case differs from observations adapter",
        ));
    }
    let receipt = evidence_report::verify_report(&directory.join("report"), expected_receipt)?;
    if receipt.case_sha256 != sha256_bytes(&case_bytes) {
        return Err(invalid("Orbit case copy mismatch"));
    }
    for (i, bytes) in [&baseline_bytes, &observation_bytes, &check_bytes]
        .into_iter()
        .enumerate()
    {
        if read(&directory.join(format!("report/evidence/{:04}.bin", i + 1)))? != *bytes {
            return Err(invalid("Orbit raw observation/report copy mismatch"));
        }
    }
    Ok(actual)
}
