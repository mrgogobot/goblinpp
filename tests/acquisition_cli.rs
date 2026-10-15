#![cfg(unix)]
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_goblinpp")
}

#[test]
fn downloaded_fits_example_accepts_an_explicit_local_payload_path() {
    let dir = tempdir();
    let source = dir.path().join("downloaded_fits.gbl");
    std::fs::write(&source, include_str!("../examples/downloaded_fits.gbl")).unwrap();
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sample.fits");
    let output = Command::new(binary())
        .current_dir(dir.path())
        .arg("run")
        .arg(&source)
        .arg("--")
        .arg(data)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("HDU count = 2"));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let run = stdout
        .lines()
        .find_map(|s| s.strip_prefix("RUN_DIR="))
        .unwrap();
    let verified = Command::new(binary())
        .current_dir(dir.path())
        .args(["verify", run])
        .output()
        .unwrap();
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
}
fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

#[test]
fn fetch_is_an_explicit_command_with_no_insecure_switch() {
    let help = Command::new(binary())
        .args(["fetch", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    for flag in [
        "--allow-host",
        "--sha256",
        "--record-only",
        "--max-bytes",
        "--timeout-seconds",
    ] {
        assert!(text.contains(flag));
    }
    assert!(!text.contains("--insecure"));
    let error = Command::new(binary())
        .args(["fetch", "--insecure"])
        .output()
        .unwrap();
    assert!(!error.status.success());
}

#[test]
fn no_hash_and_no_explicit_record_only_is_a_preflight_error() {
    let dir = tempdir();
    let out = dir.path().join("must-not-exist");
    let result = Command::new(binary())
        .args([
            "fetch",
            "https://download.example.org/data",
            "--allow-host",
            "download.example.org",
            "--output",
        ])
        .arg(&out)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!out.exists());
}

#[test]
fn forbidden_url_credentials_are_not_echoed_and_create_no_evidence() {
    let dir = tempdir();
    let out = dir.path().join("must-not-exist");
    let result = Command::new(binary())
        .args([
            "fetch",
            "https://user:DO_NOT_ECHO_THIS@download.example.org/data?key=DO_NOT_ECHO_THIS",
            "--allow-host",
            "download.example.org",
            "--record-only",
            "--output",
        ])
        .arg(&out)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!out.exists());
    assert!(!String::from_utf8_lossy(&result.stdout).contains("DO_NOT_ECHO_THIS"));
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("G901"));
    assert!(!stderr.contains("DO_NOT_ECHO_THIS"));
}

#[test]
fn existing_output_refuses_before_any_http_request() {
    let dir = tempdir();
    let out = dir.path().join("existing");
    std::fs::create_dir(&out).unwrap();
    std::fs::write(out.join("keep"), b"unchanged").unwrap();
    let result = Command::new(binary())
        .args([
            "fetch",
            "https://download.example.org/data",
            "--allow-host",
            "download.example.org",
            "--record-only",
            "--output",
        ])
        .arg(&out)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(std::fs::read(out.join("keep")).unwrap(), b"unchanged");
    assert_eq!(std::fs::read_dir(out).unwrap().count(), 1);
}

#[test]
fn check_and_capabilities_do_not_acquire_remote_content() {
    let dir = tempdir();
    let source = dir.path().join("offline.gbl");
    std::fs::write(
        &source,
        "url = \"https://download.example.org/data\"\nprint(url)\n",
    )
    .unwrap();
    let result = Command::new(binary())
        .arg("check")
        .arg(&source)
        .arg("--json")
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    let result = Command::new(binary())
        .args(["capabilities", "--json"])
        .output()
        .unwrap();
    assert!(result.status.success());
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["network_acquisition"]["automatic_gbl_network"], false);
    assert_eq!(value["network_acquisition"]["redirects"], 0);
}

#[test]
fn offline_verification_of_failed_transfer_never_claims_source_hash_match() {
    use goblinpp::acquisition::{CLIENT, PARTIAL, POLICY, RECEIPT, SCHEMA};
    use goblinpp::hashing::{hash_canonical_json, sha256_bytes};
    let dir = tempdir();
    std::fs::write(dir.path().join(PARTIAL), b"partial").unwrap();
    // Synthetic self-consistent failure record; an external anchor is separately required for trust.
    let mut r = serde_json::json!({"schema":SCHEMA,"policy":POLICY,"goblin_version":goblinpp::VERSION,
        "client":CLIENT,"client_lock_sha256":sha256_bytes(include_bytes!("../Cargo.lock")),
        "created_utc":"2026-10-15T12:00:00Z","finished_utc":"2026-10-15T12:00:01Z",
        "os":std::env::consts::OS,"arch":std::env::consts::ARCH,"status":"FAILED",
        "source_origin":"https://download.example.org:443","source_url_sha256":"1".repeat(64),
        "allowed_host":"download.example.org","validated_dns_addresses":["1.1.1.1:443"],
        "security":{"https_only":true,"port":443,"public_addresses_only":true,"redirects":0,
            "application_retries":0,"proxies":false,"cookies":false,"credential_headers":false,
            "client_certificates":false,"server_certificate_verification":true,"http_content_decoding":false,
            "max_response_header_bytes":16384,"max_payload_bytes":1024,"transfer_timeout_seconds":30},
        "expected_sha256":"0".repeat(64),"payload_sha256":sha256_bytes(b"partial"),"payload_bytes":7,
        "payload_path":PARTIAL,"http_status":200,"declared_content_length":7,"etag_sha256":null,
        "last_modified_sha256":null,"failure":"EXPECTED_SHA256_MISMATCH","elapsed_milliseconds":1000,
        "authentication":"CHECKSUM_ONLY_NOT_SIGNATURE_OR_SCIENTIFIC_VALIDATION"});
    r["receipt_core_sha256"] = hash_canonical_json(&r).unwrap().into();
    std::fs::write(dir.path().join(RECEIPT), serde_json::to_vec(&r).unwrap()).unwrap();
    let output = Command::new(binary())
        .arg("verify-fetch")
        .arg(dir.path())
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["verification_status"], "PASS");
    assert_eq!(v["acquisition_status"], "FAILED");
    assert_eq!(v["expected_source_hash_declared"], true);
    assert_eq!(v["source_content_hash_verified"], false);
    assert_eq!(v["offline"], true);
}

#[test]
fn command_line_syntax_errors_do_not_echo_unexpected_secret_url_arguments() {
    let dir = tempdir();
    let out = dir.path().join("absent");
    let output = Command::new(binary())
        .args([
            "fetch",
            "https://download.example.org/data",
            "https://other.example.org/?token=DO_NOT_ECHO_THIS",
            "--allow-host",
            "download.example.org",
            "--record-only",
            "--output",
        ])
        .arg(&out)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!out.exists());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("DO_NOT_ECHO_THIS"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("DO_NOT_ECHO_THIS"));
}
