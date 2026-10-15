//! Explicit, bounded public HTTPS acquisition. Never execute or decode downloaded content.
use crate::error::{GoblinError, Result};
use crate::hashing::{hash_canonical_json, sha256_bytes};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use ureq::http::Uri;
use ureq::tls::{RootCerts, TlsConfig};
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};

mod storage;
use storage::Bundle;

pub const POLICY: &str = "goblin.public-https-acquisition.v1";
pub const SCHEMA: &str = "goblin.acquisition-receipt.v1";
pub const CLIENT: &str = "ureq-3.4.2/rustls/platform-verifier";
pub const RECEIPT: &str = "acquisition-receipt.json";
pub const PARTIAL: &str = "payload.part";
pub const DEFAULT_MAX_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_BYTES: u64 = 1024 * 1024 * 1024;
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 300;
pub const MAX_TIMEOUT_SECONDS: u64 = 3600;
const HEADER_BYTES: usize = 16 * 1024;
const RECEIPT_BYTES: u64 = 64 * 1024;

// Deliberately no Debug/Serialize for request URLs. They may contain private path/query text.
pub struct FetchRequest {
    pub url: String,
    pub allow_host: String,
    pub expected_sha256: Option<String>,
    pub record_only: bool,
    pub filename: String,
    pub max_bytes: u64,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AcquisitionStatus {
    CompleteHashVerified,
    CompleteObservedOnly,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPolicy {
    pub https_only: bool,
    pub port: u16,
    pub public_addresses_only: bool,
    pub redirects: u8,
    pub application_retries: u8,
    pub proxies: bool,
    pub cookies: bool,
    pub credential_headers: bool,
    pub client_certificates: bool,
    pub server_certificate_verification: bool,
    pub http_content_decoding: bool,
    pub max_response_header_bytes: usize,
    pub max_payload_bytes: u64,
    pub transfer_timeout_seconds: u64,
}

impl SecurityPolicy {
    fn new(max_bytes: u64, timeout: u64) -> Self {
        Self {
            https_only: true,
            port: 443,
            public_addresses_only: true,
            redirects: 0,
            application_retries: 0,
            proxies: false,
            cookies: false,
            credential_headers: false,
            client_certificates: false,
            server_certificate_verification: true,
            http_content_decoding: false,
            max_response_header_bytes: HEADER_BYTES,
            max_payload_bytes: max_bytes,
            transfer_timeout_seconds: timeout,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionReceipt {
    pub schema: String,
    pub policy: String,
    pub goblin_version: String,
    pub client: String,
    pub client_lock_sha256: String,
    pub created_utc: String,
    pub finished_utc: String,
    pub os: String,
    pub arch: String,
    pub status: AcquisitionStatus,
    pub source_origin: String,
    pub source_url_sha256: String,
    pub allowed_host: String,
    // Candidates validated by our resolver, not a claim of an observed socket peer.
    pub validated_dns_addresses: Vec<SocketAddr>,
    pub security: SecurityPolicy,
    pub expected_sha256: Option<String>,
    pub payload_sha256: String,
    pub payload_bytes: u64,
    pub payload_path: String,
    pub http_status: Option<u16>,
    pub declared_content_length: Option<u64>,
    pub etag_sha256: Option<String>,
    pub last_modified_sha256: Option<String>,
    pub failure: Option<String>,
    pub elapsed_milliseconds: u64,
    pub authentication: String,
    pub receipt_core_sha256: String,
}

pub(crate) fn invalid(message: impl Into<String>) -> GoblinError {
    GoblinError::new("G901", message)
}

fn digest_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn domain_valid(host: &str) -> bool {
    host.len() <= 253
        && host.contains('.')
        && host.parse::<IpAddr>().is_err()
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        && !["localhost", "local", "internal", "invalid", "test", "onion"]
            .iter()
            .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
}

fn filename_valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && !name.ends_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        && ![RECEIPT, PARTIAL, "POSTFLIGHT_FAILED.json"]
            .iter()
            .any(|reserved| name.eq_ignore_ascii_case(reserved))
        && ![
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ]
        .contains(
            &name
                .split('.')
                .next()
                .unwrap_or("")
                .to_ascii_uppercase()
                .as_str(),
        )
}

impl FetchRequest {
    fn validate(&self) -> Result<Uri> {
        if self.url.len() > 4096
            || !self.url.is_ascii()
            || self
                .url
                .bytes()
                .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
            || self.url.contains(['#', '\\'])
        {
            return Err(invalid(
                "Require an ASCII HTTPS URL <=4096 bytes without whitespace, fragments or backslashes; URL omitted",
            ));
        }
        let uri: Uri = self
            .url
            .parse()
            .map_err(|_| invalid("Invalid download URL (omitted)"))?;
        let host = uri.host().unwrap_or("").to_ascii_lowercase();
        if uri.scheme_str() != Some("https")
            || uri.port_u16().is_some_and(|p| p != 443)
            || uri
                .authority()
                .is_none_or(|a| a.as_str().contains(['@', '%']))
            || !domain_valid(&host)
        {
            return Err(invalid(
                "Only HTTPS to an ordinary public DNS hostname on port 443 is permitted; credentials, IP literals and special-use names are refused",
            ));
        }
        if self.allow_host != host || !domain_valid(&self.allow_host) {
            return Err(invalid(
                "--allow-host must exactly name the lowercase URL hostname; no wildcards or implicit host permission",
            ));
        }
        if self.record_only == self.expected_sha256.is_some()
            || self
                .expected_sha256
                .as_deref()
                .is_some_and(|h| !digest_valid(h))
        {
            return Err(invalid(
                "Choose exactly one: a lowercase expected --sha256, or explicit --record-only (no externally anchored content verification)",
            ));
        }
        if !filename_valid(&self.filename)
            || !(1..=MAX_BYTES).contains(&self.max_bytes)
            || !(1..=MAX_TIMEOUT_SECONDS).contains(&self.timeout_seconds)
        {
            return Err(invalid(
                "Invalid local filename or limits: payload 1..1073741824 bytes; transfer timeout 1..3600 seconds",
            ));
        }
        Ok(uri)
    }
}

// Conservative public routing policy: intentionally excludes special-use/transition ranges.
pub(crate) fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let n = u32::from(ip);
            ![
                (0x00000000, 8),
                (0x0a000000, 8),
                (0x64400000, 10),
                (0x7f000000, 8),
                (0xa9fe0000, 16),
                (0xac100000, 12),
                (0xc0000000, 24),
                (0xc0000200, 24),
                (0xc0586300, 24),
                (0xc0a80000, 16),
                (0xc6120000, 15),
                (0xc6336400, 24),
                (0xcb007100, 24),
                (0xe0000000, 3),
            ]
            .iter()
            .any(|(base, prefix)| n >> (32 - prefix) == base >> (32 - prefix))
        }
        IpAddr::V6(ip) => {
            let s = ip.segments();
            s[0] & 0xe000 == 0x2000
                && !(s[0] == 0x2001 && (s[1] < 0x200 || s[1] == 0xdb8))
                && s[0] != 0x2002
                && !(s[0] == 0x3fff && s[1] < 0x1000)
        }
    }
}

#[derive(Debug, Default)]
struct Trace {
    addresses: Vec<SocketAddr>,
    denied: bool,
}

#[derive(Debug)]
struct PublicResolver {
    trace: Arc<Mutex<Trace>>,
    #[cfg(test)]
    test_address: Option<SocketAddr>,
}

fn vetted_addresses(addresses: &[SocketAddr]) -> bool {
    !addresses.is_empty()
        && addresses.len() <= 16
        && addresses
            .iter()
            .all(|a| public_ip(a.ip()) && a.port() == 443)
}

impl Resolver for PublicResolver {
    fn resolve(
        &self,
        uri: &Uri,
        config: &ureq::config::Config,
        timeout: NextTimeout,
    ) -> std::result::Result<ResolvedSocketAddrs, ureq::Error> {
        #[cfg(test)]
        if let Some(address) = self.test_address {
            let mut r = self.empty();
            r.push(address);
            // Unit fixtures simulate vetted public DNS while routing TLS to a local server.
            // This field and alternate routing do not exist in production builds.
            self.trace
                .lock()
                .map_err(|_| ureq::Error::ConnectionFailed)?
                .addresses = vec![
                "93.184.216.34:443"
                    .parse()
                    .map_err(|_| ureq::Error::ConnectionFailed)?,
            ];
            return Ok(r);
        }
        let addresses = DefaultResolver::default().resolve(uri, config, timeout)?;
        let mut trace = self
            .trace
            .lock()
            .map_err(|_| ureq::Error::ConnectionFailed)?;
        if !vetted_addresses(&addresses.iter().copied().collect::<Vec<_>>()) {
            trace.denied = true;
            return Err(ureq::Error::ConnectionFailed);
        }
        trace.addresses = addresses.iter().copied().collect();
        // These exact vetted SocketAddrs go to the connector; no second hostname lookup.
        Ok(addresses)
    }
}

fn agent(request: &FetchRequest, resolver: PublicResolver, roots: RootCerts) -> ureq::Agent {
    let deadline = Duration::from_secs(request.timeout_seconds);
    let config = ureq::Agent::config_builder()
        .https_only(true)
        .proxy(None)
        .http_status_as_error(false)
        .max_redirects(0)
        .max_idle_connections(0)
        .max_idle_connections_per_host(0)
        .max_response_header_size(HEADER_BYTES)
        .timeout_global(Some(deadline))
        .timeout_resolve(Some(deadline.min(Duration::from_secs(5))))
        .timeout_connect(Some(deadline.min(Duration::from_secs(10))))
        .timeout_recv_response(Some(deadline.min(Duration::from_secs(30))))
        .timeout_recv_body(Some(deadline))
        .user_agent(format!("Goblin++/{}", crate::VERSION))
        .accept_encoding("identity")
        .tls_config(
            TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::Rustls)
                .root_certs(roots)
                .disable_verification(false)
                .use_sni(true)
                .client_cert(None)
                .build(),
        )
        .build();
    ureq::Agent::with_parts(config, DefaultConnector::default(), resolver)
}

pub fn fetch(request: &FetchRequest, output: &Path) -> Result<AcquisitionReceipt> {
    let uri = request.validate()?;
    let trace = Arc::new(Mutex::new(Trace::default()));
    let resolver = PublicResolver {
        trace: trace.clone(),
        #[cfg(test)]
        test_address: None,
    };
    let client = agent(request, resolver, RootCerts::PlatformVerifier);
    acquire(request, &uri, output, &client, &trace)
}

fn utc_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn core_hash(receipt: &AcquisitionReceipt) -> Result<String> {
    let mut value = serde_json::to_value(receipt)?;
    value
        .as_object_mut()
        .ok_or_else(|| invalid("Invalid receipt object"))?
        .remove("receipt_core_sha256");
    hash_canonical_json(&value)
}

fn acquire(
    request: &FetchRequest,
    uri: &Uri,
    output: &Path,
    client: &ureq::Agent,
    trace: &Arc<Mutex<Trace>>,
) -> Result<AcquisitionReceipt> {
    request.validate()?;
    let directory = Bundle::create(output)?;
    let started = Instant::now();
    let mut payload = directory.create_file(PARTIAL)?;
    let mut receipt = AcquisitionReceipt {
        schema: SCHEMA.into(),
        policy: POLICY.into(),
        goblin_version: crate::VERSION.into(),
        client: CLIENT.into(),
        client_lock_sha256: sha256_bytes(include_bytes!("../Cargo.lock")),
        created_utc: utc_now(),
        finished_utc: String::new(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        status: AcquisitionStatus::Failed,
        source_origin: format!("https://{}:443", request.allow_host),
        source_url_sha256: sha256_bytes(request.url.as_bytes()),
        allowed_host: request.allow_host.clone(),
        validated_dns_addresses: vec![],
        security: SecurityPolicy::new(request.max_bytes, request.timeout_seconds),
        expected_sha256: request.expected_sha256.clone(),
        payload_sha256: String::new(),
        payload_bytes: 0,
        payload_path: PARTIAL.into(),
        http_status: None,
        declared_content_length: None,
        etag_sha256: None,
        last_modified_sha256: None,
        failure: None,
        elapsed_milliseconds: 0,
        authentication: "CHECKSUM_ONLY_NOT_SIGNATURE_OR_SCIENTIFIC_VALIDATION".into(),
        receipt_core_sha256: String::new(),
    };
    let outcome = transfer(request, uri, client, &mut payload, &mut receipt, started);
    payload.sync_all()?;
    let (bytes, sha) = hash_file(&mut payload, request.max_bytes)?;
    receipt.payload_bytes = bytes;
    receipt.payload_sha256 = sha;
    if let Ok(t) = trace.lock() {
        receipt.validated_dns_addresses = t.addresses.clone();
        if t.denied {
            receipt.failure = Some("DESTINATION_ADDRESS_REFUSED".into());
        }
    }
    if let Err(reason) = outcome {
        receipt.failure.get_or_insert(reason.into());
    }
    if receipt.failure.is_none()
        && request
            .expected_sha256
            .as_ref()
            .is_some_and(|h| *h != receipt.payload_sha256)
    {
        receipt.failure = Some("EXPECTED_SHA256_MISMATCH".into());
    }
    if receipt.failure.is_none() {
        directory.publish(PARTIAL, &request.filename)?;
        receipt.payload_path.clone_from(&request.filename);
        receipt.status = if request.expected_sha256.is_some() {
            AcquisitionStatus::CompleteHashVerified
        } else {
            AcquisitionStatus::CompleteObservedOnly
        };
    }
    receipt.finished_utc = utc_now();
    receipt.elapsed_milliseconds = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    receipt.receipt_core_sha256 = core_hash(&receipt)?;
    let mut record = directory.create_file(RECEIPT)?;
    record.write_all(&serde_json::to_vec_pretty(&receipt)?)?;
    record.sync_all()?;
    directory.finish()?;
    // Verify independently from the persisted bytes before returning any successful acquisition.
    if verify(output, Some(&receipt.receipt_core_sha256)).is_err() {
        // Preserve the attempted receipt, but ensure a failed postflight can never verify.
        if let Ok(mut marker) = directory.create_file("POSTFLIGHT_FAILED.json") {
            let _ = marker.write_all(b"{\"status\":\"FAILED\",\"reason\":\"ACQUISITION_POSTFLIGHT_FAILED\",\"receipt_valid\":false}");
            let _ = marker.sync_all();
        }
        return Err(invalid(
            "Acquisition postflight failed; no successful acquisition is established",
        ));
    }
    if receipt.status == AcquisitionStatus::Failed {
        return Err(invalid(format!(
            "Acquisition refused: {}. Partial bytes and failure receipt preserved; URL omitted.",
            receipt.failure.as_deref().unwrap_or("TRANSFER_FAILED")
        )));
    }
    Ok(receipt)
}

fn transfer(
    request: &FetchRequest,
    uri: &Uri,
    client: &ureq::Agent,
    payload: &mut File,
    receipt: &mut AcquisitionReceipt,
    start: Instant,
) -> std::result::Result<(), &'static str> {
    let mut response = client
        .get(uri.to_string())
        .header("Accept-Encoding", "identity")
        .header("Connection", "close")
        .call()
        .map_err(|_| "HTTPS_REQUEST_FAILED")?;
    let status = response.status().as_u16();
    receipt.http_status = Some(status);
    if (300..400).contains(&status) {
        return Err("REDIRECT_REFUSED");
    }
    if status != 200 {
        return Err("HTTP_STATUS_REFUSED");
    }
    let headers = response.headers();
    if headers.contains_key("content-range") {
        return Err("PARTIAL_RESPONSE_REFUSED");
    }
    let encoding: Vec<_> = headers.get_all("content-encoding").iter().collect();
    if encoding.len() > 1
        || encoding
            .first()
            .is_some_and(|v| !v.as_bytes().eq_ignore_ascii_case(b"identity"))
    {
        return Err("HTTP_CONTENT_ENCODING_REFUSED");
    }
    let lengths: Vec<_> = headers.get_all("content-length").iter().collect();
    if lengths.len() > 1 || (!lengths.is_empty() && headers.contains_key("transfer-encoding")) {
        return Err("AMBIGUOUS_BODY_FRAMING");
    }
    if let Some(length) = lengths.first() {
        let length = length
            .to_str()
            .ok()
            .and_then(|s| {
                if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
                    s.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .ok_or("INVALID_CONTENT_LENGTH")?;
        receipt.declared_content_length = Some(length);
        if length > request.max_bytes {
            return Err("PAYLOAD_LIMIT_EXCEEDED");
        }
    }
    receipt.etag_sha256 = headers.get("etag").map(|v| sha256_bytes(v.as_bytes()));
    receipt.last_modified_sha256 = headers
        .get("last-modified")
        .map(|v| sha256_bytes(v.as_bytes()));
    let mut reader = response.body_mut().as_reader();
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        if start.elapsed() >= Duration::from_secs(request.timeout_seconds) {
            return Err("TRANSFER_DEADLINE_EXCEEDED");
        }
        let size = usize::try_from((request.max_bytes - total + 1).min(buffer.len() as u64))
            .map_err(|_| "PAYLOAD_LIMIT_EXCEEDED")?;
        let n = reader
            .read(&mut buffer[..size])
            .map_err(|_| "BODY_READ_FAILED_OR_DEADLINE")?;
        if n == 0 {
            break;
        }
        let accepted = (n as u64).min(request.max_bytes - total) as usize;
        payload
            .write_all(&buffer[..accepted])
            .map_err(|_| "PAYLOAD_WRITE_FAILED")?;
        total += accepted as u64;
        if accepted < n {
            return Err("PAYLOAD_LIMIT_EXCEEDED");
        }
    }
    if receipt.declared_content_length.is_some_and(|n| n != total) {
        return Err("CONTENT_LENGTH_MISMATCH");
    }
    Ok(())
}

fn hash_file(file: &mut File, limit: u64) -> Result<(u64, String)> {
    use std::os::unix::fs::MetadataExt;
    let before = file.metadata()?;
    if !before.is_file() || before.len() > limit {
        return Err(invalid("Payload is not a bounded regular file"));
    }
    file.seek(SeekFrom::Start(0))?;
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        if bytes > limit {
            return Err(invalid("File grew beyond acquisition bounds"));
        }
        hash.update(&buffer[..n]);
    }
    let after = file.metadata()?;
    if after.len() != bytes
        || (
            before.len(),
            before.mtime(),
            before.mtime_nsec(),
            before.ctime(),
            before.ctime_nsec(),
        ) != (
            after.len(),
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec(),
        )
    {
        return Err(invalid("File changed while hashing"));
    }
    Ok((bytes, format!("{:x}", hash.finalize())))
}

/// Offline byte verification. A verified failure receipt is not a successful download.
pub fn verify(directory: &Path, expected_receipt: Option<&str>) -> Result<AcquisitionReceipt> {
    if expected_receipt.is_some_and(|h| !digest_valid(h)) {
        return Err(invalid("Invalid expected receipt SHA-256"));
    }
    let bundle = Bundle::open(directory)?;
    let record = bundle.read_file(RECEIPT)?;
    if record.metadata()?.len() > RECEIPT_BYTES {
        return Err(invalid("Acquisition receipt exceeds 64 KiB"));
    }
    let mut bytes = Vec::new();
    record.take(RECEIPT_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > RECEIPT_BYTES {
        return Err(invalid("Receipt grew beyond bound"));
    }
    let r: AcquisitionReceipt = serde_json::from_slice(&bytes)
        .map_err(|_| invalid("Malformed/unsupported acquisition receipt"))?;
    if r.schema != SCHEMA
        || r.policy != POLICY
        || r.client != CLIENT
        || !(1..=MAX_BYTES).contains(&r.security.max_payload_bytes)
        || !(1..=MAX_TIMEOUT_SECONDS).contains(&r.security.transfer_timeout_seconds)
        || r.security
            != SecurityPolicy::new(
                r.security.max_payload_bytes,
                r.security.transfer_timeout_seconds,
            )
        || !domain_valid(&r.allowed_host)
        || r.source_origin != format!("https://{}:443", r.allowed_host)
        || !digest_valid(&r.source_url_sha256)
        || !digest_valid(&r.payload_sha256)
        || !digest_valid(&r.client_lock_sha256)
        || r.goblin_version.is_empty()
        || r.goblin_version.len() > 64
        || !r
            .goblin_version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
        || ![
            "macos",
            "linux",
            "freebsd",
            "openbsd",
            "netbsd",
            "dragonfly",
            "android",
            "ios",
            "illumos",
            "solaris",
        ]
        .contains(&r.os.as_str())
        || r.arch.is_empty()
        || r.arch.len() > 32
        || !r
            .arch
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        || r.http_status.is_some_and(|s| !(100..=599).contains(&s))
        || !digest_valid(&r.receipt_core_sha256)
        || r.expected_sha256
            .as_deref()
            .is_some_and(|h| !digest_valid(h))
        || r.validated_dns_addresses.len() > 16
        || r.validated_dns_addresses
            .iter()
            .any(|a| !public_ip(a.ip()) || a.port() != 443)
        || [&r.etag_sha256, &r.last_modified_sha256]
            .iter()
            .any(|h| h.as_deref().is_some_and(|h| !digest_valid(h)))
        || [&r.created_utc, &r.finished_utc].iter().any(|s| {
            s.len() > 40
                || !chrono::DateTime::parse_from_rfc3339(s)
                    .is_ok_and(|t| t.offset().local_minus_utc() == 0)
        })
        || r.authentication != "CHECKSUM_ONLY_NOT_SIGNATURE_OR_SCIENTIFIC_VALIDATION"
        || core_hash(&r)? != r.receipt_core_sha256
        || expected_receipt.is_some_and(|h| h != r.receipt_core_sha256)
    {
        return Err(invalid(
            "Acquisition schema/policy/hash/anchor validation failed",
        ));
    }
    match r.status {
        AcquisitionStatus::Failed
            if r.payload_path == PARTIAL && r.failure.as_deref().is_some_and(failure_valid) => {}
        AcquisitionStatus::CompleteHashVerified
            if filename_valid(&r.payload_path)
                && r.failure.is_none()
                && r.expected_sha256.as_deref() == Some(r.payload_sha256.as_str())
                && r.http_status == Some(200) => {}
        AcquisitionStatus::CompleteObservedOnly
            if filename_valid(&r.payload_path)
                && r.failure.is_none()
                && r.expected_sha256.is_none()
                && r.http_status == Some(200) => {}
        _ => {
            return Err(invalid(
                "Inconsistent acquisition status, source anchor or failure evidence",
            ));
        }
    }
    if r.payload_bytes > r.security.max_payload_bytes
        || (r.status != AcquisitionStatus::Failed && r.validated_dns_addresses.is_empty())
        || (r.status != AcquisitionStatus::Failed
            && r.declared_content_length
                .is_some_and(|n| n != r.payload_bytes))
    {
        return Err(invalid("Acquisition byte-count evidence failed"));
    }
    let mut payload = bundle.read_file(&r.payload_path)?;
    let (length, hash) = hash_file(&mut payload, r.security.max_payload_bytes)?;
    if length != r.payload_bytes || hash != r.payload_sha256 {
        return Err(invalid("Acquisition payload bytes/hash mismatch"));
    }
    bundle.exact_files(&[RECEIPT, &r.payload_path])?;
    bundle.finish()?;
    Ok(r)
}

fn failure_valid(reason: &str) -> bool {
    [
        "DESTINATION_ADDRESS_REFUSED",
        "HTTPS_REQUEST_FAILED",
        "REDIRECT_REFUSED",
        "HTTP_STATUS_REFUSED",
        "PARTIAL_RESPONSE_REFUSED",
        "HTTP_CONTENT_ENCODING_REFUSED",
        "AMBIGUOUS_BODY_FRAMING",
        "INVALID_CONTENT_LENGTH",
        "PAYLOAD_LIMIT_EXCEEDED",
        "TRANSFER_DEADLINE_EXCEEDED",
        "BODY_READ_FAILED_OR_DEADLINE",
        "PAYLOAD_WRITE_FAILED",
        "CONTENT_LENGTH_MISMATCH",
        "EXPECTED_SHA256_MISMATCH",
    ]
    .contains(&reason)
}

#[cfg(test)]
mod tests;
