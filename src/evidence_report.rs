//! Offline, checksum-only reporting. No acquisition, scanning, or execution.
use crate::error::{GoblinError, Result};
use crate::hashing::{hash_canonical_json, sha256_bytes};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub const POLICY: &str = "goblin.offline-evidence-report.v1";
pub const CASE_SCHEMA: &str = "goblin.evidence-case.v1";
pub const CASE_BYTES: usize = 256 * 1024;
pub const FILE_BYTES: usize = 8 * 1024 * 1024;
pub const TOTAL_BYTES: usize = 32 * 1024 * 1024;
const MAX_ITEMS: usize = 128;
const DISCLAIMER: &str = "CHECKSUM_ONLY: byte integrity, not authenticated authorship, verified permission, certified time, forensic acquisition, legal admissibility, or proof that findings are true. This bundle contains plaintext evidence; review confidentiality before sharing. No scanning, exploitation, execution of attachments, or network requests are performed.";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseDocument {
    pub schema: String,
    pub case_id: String,
    pub purpose: Purpose,
    pub title: String,
    pub author: String,
    pub authorization: String,
    pub scope: Vec<String>,
    pub summary: String,
    pub evidence: Vec<Evidence>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    SecurityAssessment,
    ForensicAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub id: String,
    pub path: String,
    pub sha256: String,
    pub description: String,
    pub collected_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub observed_utc: String,
    pub observation: String,
    pub impact: String,
    pub recommendation: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Informational,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Confirmed,
    Suspected,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub path: String,
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportReceipt {
    pub schema: String,
    pub policy: String,
    pub goblin_version: String,
    pub created_utc: String,
    pub os: String,
    pub arch: String,
    pub authentication: String,
    pub case_sha256: String,
    pub evidence: Vec<Artifact>,
    pub outputs: Vec<Artifact>,
    pub receipt_core_sha256: String,
}

fn invalid(message: impl Into<String>) -> GoblinError {
    GoblinError::new("G801", message)
}

fn text(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty()
        || value.len() > 8192
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
        || value
            .chars()
            .any(|c| matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
    {
        return Err(invalid(format!(
            "{label}: require nonempty text <=8192 UTF-8 bytes, without control or bidirectional override characters"
        )));
    }
    Ok(())
}

fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(invalid(
            "IDs must contain 1..64 ASCII letters, digits, underscores or hyphens",
        ));
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid(
            "Expected SHA-256 must be exactly 64 lowercase hexadecimal characters",
        ));
    }
    Ok(())
}

fn utc(value: &str) -> Result<()> {
    if value.len() > 64
        || !DateTime::parse_from_rfc3339(value).is_ok_and(|d| d.offset().local_minus_utc() == 0)
    {
        return Err(invalid(
            "Times must be explicit RFC3339 UTC, e.g. 2026-10-13T14:00:00Z; they are declarations, not certified time",
        ));
    }
    Ok(())
}

fn relative(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 1024
        || value.contains(['\\', ':'])
        || value.chars().any(char::is_control)
        || value
            .chars()
            .any(|c| matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || Path::new(value)
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(invalid(
            "Evidence paths must be relative ordinary paths below the case directory; no traversal, absolute paths, backslashes or colons",
        ));
    }
    Ok(())
}

pub fn validate_case(case: &CaseDocument) -> Result<()> {
    if case.schema != CASE_SCHEMA {
        return Err(invalid("Unsupported case schema"));
    }
    identifier(&case.case_id)?;
    for (value, name) in [
        (&case.title, "title"),
        (&case.author, "author"),
        (&case.authorization, "authorization"),
        (&case.summary, "summary"),
    ] {
        text(value, name)?;
    }
    if case.scope.is_empty() || case.scope.len() > 64 {
        return Err(invalid("Declare 1..64 scope entries"));
    }
    for entry in &case.scope {
        text(entry, "scope")?;
    }
    if case.evidence.is_empty()
        || case.evidence.len() > MAX_ITEMS
        || case.findings.len() > MAX_ITEMS
    {
        return Err(invalid("Require 1..128 evidence files and 0..128 findings"));
    }
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for item in &case.evidence {
        identifier(&item.id)?;
        relative(&item.path)?;
        digest(&item.sha256)?;
        text(&item.description, "evidence description")?;
        utc(&item.collected_utc)?;
        if !ids.insert(&item.id) || !paths.insert(&item.path) {
            return Err(invalid("Duplicate evidence ID or path"));
        }
    }
    let mut findings = BTreeSet::new();
    for finding in &case.findings {
        identifier(&finding.id)?;
        if !findings.insert(&finding.id) {
            return Err(invalid("Duplicate finding ID"));
        }
        utc(&finding.observed_utc)?;
        for (value, label) in [
            (&finding.title, "finding title"),
            (&finding.observation, "observation"),
            (&finding.impact, "impact"),
            (&finding.recommendation, "recommendation"),
        ] {
            text(value, label)?;
        }
        if finding.evidence.is_empty() || finding.evidence.len() > MAX_ITEMS {
            return Err(invalid("Every finding must reference 1..128 evidence IDs"));
        }
        let mut references = BTreeSet::new();
        for reference in &finding.evidence {
            if !ids.contains(reference) || !references.insert(reference) {
                return Err(invalid(format!(
                    "Finding {} has an unknown or duplicate evidence reference: {reference}",
                    finding.id
                )));
            }
        }
    }
    Ok(())
}

// Check every observed path component. This is not a hostile-filesystem sandbox.
fn no_symlinks(path: &Path) -> Result<()> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut current = PathBuf::new();
    for component in absolute.components() {
        if matches!(component, Component::ParentDir) {
            return Err(invalid("Path traversal is refused"));
        }
        current.push(component);
        if fs::symlink_metadata(&current)?.file_type().is_symlink() {
            return Err(invalid(format!(
                "Symlink path component refused: {}",
                current.display()
            )));
        }
    }
    Ok(())
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    no_symlinks(path)?;
    if !fs::metadata(path)?.is_file() {
        return Err(invalid("Only ordinary regular files may be read"));
    }
    let file = File::open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > limit as u64 {
        return Err(invalid(format!(
            "Require regular file <= {limit} bytes: {}",
            path.display()
        )));
    }
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(invalid("File grew beyond the reporting limit"));
    }
    Ok(bytes)
}

fn parse_case(bytes: &[u8]) -> Result<CaseDocument> {
    let case: CaseDocument =
        serde_json::from_slice(bytes).map_err(|e| invalid(format!("Invalid case JSON: {e}")))?;
    validate_case(&case)?;
    Ok(case)
}

fn snapshot_path(index: usize) -> String {
    format!("evidence/{:04}.bin", index + 1)
}

fn artifact(path: &str, bytes: &[u8]) -> Artifact {
    Artifact {
        path: path.into(),
        sha256: sha256_bytes(bytes),
        bytes: bytes.len(),
    }
}

fn core_hash(receipt: &ReportReceipt) -> Result<String> {
    let mut core = serde_json::to_value(receipt)?;
    core.as_object_mut().unwrap().remove("receipt_core_sha256");
    hash_canonical_json(&core)
}

fn md(value: &str) -> String {
    let mut escaped = String::new();
    for c in value.chars() {
        match c {
            '\n' => escaped.push_str(" / "),
            '\t' => escaped.push(' '),
            c if c.is_ascii_punctuation() => {
                escaped.push('\\');
                escaped.push(c);
            }
            _ => escaped.push(c),
        }
    }
    escaped
}

fn html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render(case: &CaseDocument, receipt: &ReportReceipt) -> (Vec<u8>, Vec<u8>) {
    let purpose = match case.purpose {
        Purpose::SecurityAssessment => "Declared authorised security assessment",
        Purpose::ForensicAnalysis => "Forensic analysis of supplied copies",
    };
    let mut report = format!(
        "# {}\n\nCase: {}\n\nPurpose: {purpose}\n\nDeclared author: {}\n\nReport created (system UTC): {}\n\nProducer: Goblin++ {} on {}/{}\n\nPolicy: {POLICY}\n\n{DISCLAIMER}\n\n## Declared authorisation\n\n{}\n\n## Declared scope\n\n",
        md(&case.title),
        md(&case.case_id),
        md(&case.author),
        md(&receipt.created_utc),
        md(&receipt.goblin_version),
        md(&receipt.os),
        md(&receipt.arch),
        md(&case.authorization)
    );
    for scope in &case.scope {
        report.push_str(&format!("- {}\n", md(scope)));
    }
    report.push_str(&format!("\n## Analyst summary (not independently validated)\n\n{}\n\n## Evidence inventory\n\nCase source SHA-256: {}\n\n", md(&case.summary), receipt.case_sha256));
    for (index, evidence) in case.evidence.iter().enumerate() {
        let item = &receipt.evidence[index];
        report.push_str(&format!("### {}\n\n{}\n\nDeclared source: {}\n\nDeclared collection UTC: {}\n\nPreserved copy: {}\n\nBytes: {}\n\nExpected/observed SHA-256: {}\n\n", md(&evidence.id), md(&evidence.description), md(&evidence.path), md(&evidence.collected_utc), item.path, item.bytes, item.sha256));
    }
    report.push_str("## Findings (analyst declarations)\n\nSeverity and confidence are supplied by the analyst, not inferred or certified by Goblin++. Zero findings do not prove absence of problems.\n\n");
    for finding in &case.findings {
        let severity = serde_json::to_value(&finding.severity).unwrap();
        let confidence = serde_json::to_value(&finding.confidence).unwrap();
        report.push_str(&format!("### {}: {}\n\nDeclared severity: {}\n\nDeclared confidence: {}\n\nDeclared observation UTC: {}\n\nObservation: {}\n\nImpact: {}\n\nRecommendation: {}\n\nEvidence IDs: {}\n\n", md(&finding.id), md(&finding.title), severity.as_str().unwrap(), confidence.as_str().unwrap(), md(&finding.observed_utc), md(&finding.observation), md(&finding.impact), md(&finding.recommendation), finding.evidence.join(", ")));
    }
    report.push_str("## Verification\n\nRun `goblin++ verify-report REPORT_DIRECTORY` on this bundle. Compare its receipt core SHA-256 with a trusted independently stored copy. An attacker able to replace all evidence and checksums can create another internally consistent bundle. Original inputs are not reopened by verification.\n");
    // Plain text in pre: attachments and user content cannot become active HTML.
    let mut plain = String::new();
    let mut chars = report.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek().is_some_and(|c| c.is_ascii_punctuation()) {
            plain.push(chars.next().unwrap());
        } else {
            plain.push(c);
        }
    }
    let page = format!(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'\"><meta name=\"referrer\" content=\"no-referrer\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{}</title><style>body{{max-width:960px;margin:2em auto;padding:0 1em;background:#fff;color:#18212b}}pre{{white-space:pre-wrap;overflow-wrap:anywhere;font:16px/1.6 ui-monospace,monospace}}@media print{{body{{margin:0;max-width:none}}pre{{font-size:11px}}}}</style></head><body><pre>{}</pre></body></html>\n",
        html(&case.title),
        html(&plain)
    );
    (report.into_bytes(), page.into_bytes())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
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

fn private_directory(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    Ok(())
}

/// Preflight all inputs before creating a new output directory. No replacements.
pub fn create_report(case_file: &Path, output: &Path) -> Result<ReportReceipt> {
    let bytes = read_bounded(case_file, CASE_BYTES)?;
    let case = parse_case(&bytes)?;
    let base = case_file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut snapshots = Vec::new();
    let mut total = 0;
    for item in &case.evidence {
        let content = read_bounded(&base.join(&item.path), FILE_BYTES)?;
        if sha256_bytes(&content) != item.sha256 {
            return Err(invalid(format!("Evidence SHA-256 mismatch: {}", item.id)));
        }
        total += content.len();
        if total > TOTAL_BYTES {
            return Err(invalid("Evidence total exceeds 32 MiB reporting limit"));
        }
        snapshots.push(content);
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    no_symlinks(parent)?;
    if output.file_name().is_none()
        || output
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(invalid("Output must name a new ordinary directory"));
    }
    if fs::symlink_metadata(output).is_ok() {
        return Err(invalid(
            "Report destination already exists; refusing overwrite",
        ));
    }
    let mut receipt = ReportReceipt {
        schema: "goblin.evidence-report.v1".into(),
        policy: POLICY.into(),
        goblin_version: crate::VERSION.into(),
        created_utc: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        authentication: "CHECKSUM_ONLY".into(),
        case_sha256: sha256_bytes(&bytes),
        evidence: snapshots
            .iter()
            .enumerate()
            .map(|(i, b)| artifact(&snapshot_path(i), b))
            .collect(),
        outputs: Vec::new(),
        receipt_core_sha256: String::new(),
    };
    let (markdown, page) = render(&case, &receipt);
    receipt.outputs = vec![
        artifact("report.md", &markdown),
        artifact("report.html", &page),
    ];
    receipt.receipt_core_sha256 = core_hash(&receipt)?;
    private_directory(output)?;
    // Keep any partially written bundle on I/O failure. No success receipt until last.
    let written = (|| -> Result<()> {
        private_directory(&output.join("evidence"))?;
        write_new(&output.join("case.json"), &bytes)?;
        for (index, content) in snapshots.iter().enumerate() {
            write_new(&output.join(snapshot_path(index)), content)?;
        }
        write_new(&output.join("report.md"), &markdown)?;
        write_new(&output.join("report.html"), &page)?;
        write_new(
            &output.join("report-receipt.json"),
            &serde_json::to_vec_pretty(&receipt)?,
        )?;
        verify_report(output, Some(&receipt.receipt_core_sha256))?;
        Ok(())
    })();
    written.map_err(|e| invalid(format!("Report creation incomplete: {e}. Preserve and inspect {} before retrying with a new destination", output.display())))?;
    Ok(receipt)
}

/// Verify copies and independently reconstruct both renderings from the case.
pub fn verify_report(directory: &Path, trusted_core: Option<&str>) -> Result<ReportReceipt> {
    no_symlinks(directory)?;
    let receipt: ReportReceipt = serde_json::from_slice(&read_bounded(
        &directory.join("report-receipt.json"),
        CASE_BYTES,
    )?)?;
    if receipt.schema != "goblin.evidence-report.v1"
        || receipt.policy != POLICY
        || receipt.authentication != "CHECKSUM_ONLY"
    {
        return Err(invalid(
            "Unsupported report schema, policy or authentication",
        ));
    }
    for (value, label) in [
        (&receipt.goblin_version, "producer version"),
        (&receipt.os, "producer OS"),
        (&receipt.arch, "producer architecture"),
    ] {
        text(value, label)?;
    }
    utc(&receipt.created_utc)?;
    digest(&receipt.receipt_core_sha256)?;
    if core_hash(&receipt)? != receipt.receipt_core_sha256 {
        return Err(invalid("Report receipt core SHA-256 mismatch"));
    }
    if let Some(expected) = trusted_core {
        digest(expected)?;
        if expected != receipt.receipt_core_sha256 {
            return Err(invalid("Report differs from trusted receipt core SHA-256"));
        }
    }
    let bytes = read_bounded(&directory.join("case.json"), CASE_BYTES)?;
    if sha256_bytes(&bytes) != receipt.case_sha256 {
        return Err(invalid("Case source SHA-256 mismatch"));
    }
    let case = parse_case(&bytes)?;
    if receipt.evidence.len() != case.evidence.len() {
        return Err(invalid("Evidence count mismatch"));
    }
    let mut expected_files = BTreeSet::from([
        "case.json".to_owned(),
        "report.md".to_owned(),
        "report.html".to_owned(),
        "report-receipt.json".to_owned(),
    ]);
    let mut total = 0;
    for (index, (item, declaration)) in receipt.evidence.iter().zip(&case.evidence).enumerate() {
        let expected_path = snapshot_path(index);
        if item.path != expected_path || item.sha256 != declaration.sha256 {
            return Err(invalid(
                "Evidence path or expected digest differs from case",
            ));
        }
        let content = read_bounded(&directory.join(&expected_path), FILE_BYTES)?;
        if artifact(&expected_path, &content) != *item {
            return Err(invalid(format!(
                "Evidence bytes/hash mismatch: {}",
                declaration.id
            )));
        }
        total += content.len();
        if total > TOTAL_BYTES {
            return Err(invalid("Evidence total exceeds reporting limit"));
        }
        expected_files.insert(expected_path);
    }
    let (markdown, page) = render(&case, &receipt);
    let expected_outputs = vec![
        artifact("report.md", &markdown),
        artifact("report.html", &page),
    ];
    if receipt.outputs != expected_outputs {
        return Err(invalid(
            "Report output inventory does not match independent rendering",
        ));
    }
    for (name, expected) in [("report.md", markdown), ("report.html", page)] {
        if read_bounded(&directory.join(name), 4 * 1024 * 1024)? != expected {
            return Err(invalid(format!(
                "Report bytes differ from independent rendering: {name}"
            )));
        }
    }
    let mut observed = BTreeSet::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("Non-UTF8 report filename"))?;
        if name == "evidence" && entry.file_type()?.is_dir() {
            for child in fs::read_dir(entry.path())? {
                let child = child?;
                if !child.file_type()?.is_file() {
                    return Err(invalid("Non-regular evidence entry refused"));
                }
                let name = format!(
                    "evidence/{}",
                    child
                        .file_name()
                        .into_string()
                        .map_err(|_| invalid("Non-UTF8 evidence filename"))?
                );
                if !expected_files.contains(&name) {
                    return Err(invalid("Unlisted evidence file"));
                }
                observed.insert(name);
            }
        } else if entry.file_type()?.is_file() {
            if !expected_files.contains(&name) {
                return Err(invalid("Unlisted report file"));
            }
            observed.insert(name);
        } else {
            return Err(invalid("Unexpected report directory or symlink"));
        }
    }
    if observed != expected_files {
        return Err(invalid("Unlisted or missing report files"));
    }
    Ok(receipt)
}
