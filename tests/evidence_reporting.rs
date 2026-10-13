use goblinpp::evidence_report::{
    CASE_BYTES, CaseDocument, FILE_BYTES, create_report, verify_report,
};
use goblinpp::hashing::{hash_canonical_json, sha256_bytes};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{TempDir, tempdir};

fn fixture() -> (TempDir, PathBuf, Value) {
    let temporary = tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    fs::write(root.join("log.txt"), b"synthetic authorised lab evidence\n").unwrap();
    let case = json!({
        "schema":"goblin.evidence-case.v1", "case_id":"LAB-001", "purpose":"security_assessment",
        "title":"Synthetic lab report", "author":"Lab analyst", "authorization":"Own disposable lab only; no external targets",
        "scope":["supplied offline log"], "summary":"One synthetic finding; not a production security assessment",
        "evidence":[{"id":"E1","path":"log.txt","sha256":sha256_bytes(b"synthetic authorised lab evidence\n"),"description":"Synthetic data", "collected_utc":"2026-10-13T14:00:00Z"}],
        "findings":[{"id":"F1","title":"Example observation","severity":"low","confidence":"suspected","observed_utc":"2026-10-13T14:01:00Z","observation":"A line in the supplied log", "impact":"Not a proven compromise","recommendation":"Check configuration in the authorised lab","evidence":["E1"]}]
    });
    (temporary, root, case)
}

fn write_case(root: &Path, case: &Value) -> PathBuf {
    let path = root.join("input.json");
    fs::write(&path, serde_json::to_vec_pretty(case).unwrap()).unwrap();
    path
}

fn reseal(root: &Path, edit: impl FnOnce(&mut Value)) {
    let path = root.join("report-receipt.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    edit(&mut receipt);
    receipt
        .as_object_mut()
        .unwrap()
        .remove("receipt_core_sha256");
    receipt["receipt_core_sha256"] = json!(hash_canonical_json(&receipt).unwrap());
    fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}

#[test]
fn report_preserves_inputs_and_stands_alone_after_originals_disappear() {
    let (_temp, root, case) = fixture();
    let source = write_case(&root, &case);
    let before = fs::read(&source).unwrap();
    let output = root.join("report");
    let receipt = create_report(&source, &output).unwrap();
    assert_eq!(fs::read(&source).unwrap(), before);
    assert_eq!(fs::read(output.join("case.json")).unwrap(), before);
    assert_eq!(
        fs::read(root.join("log.txt")).unwrap(),
        fs::read(output.join("evidence/0001.bin")).unwrap()
    );
    assert_eq!(receipt.authentication, "CHECKSUM_ONLY");
    fs::remove_file(source).unwrap();
    fs::remove_file(root.join("log.txt")).unwrap();
    verify_report(&output, Some(&receipt.receipt_core_sha256)).unwrap();
}

#[test]
fn wrong_expected_digest_refuses_before_output_creation() {
    let (_temp, root, mut case) = fixture();
    case["evidence"][0]["sha256"] = json!("0".repeat(64));
    let output = root.join("report");
    assert!(
        create_report(&write_case(&root, &case), &output)
            .unwrap_err()
            .message
            .contains("SHA-256 mismatch")
    );
    assert!(!output.exists());
}

#[test]
fn missing_inputs_and_non_regular_files_refuse_without_bundle() {
    let (_temp, root, mut case) = fixture();
    for name in ["missing.txt", "directory"] {
        fs::create_dir_all(root.join("directory")).unwrap();
        case["evidence"][0]["path"] = json!(name);
        assert!(create_report(&write_case(&root, &case), &root.join("report")).is_err());
        assert!(!root.join("report").exists());
    }
}

#[test]
fn strict_manifest_rejects_unknown_missing_duplicate_and_bad_enum_fields() {
    let (_temp, root, case) = fixture();
    for pointer in ["/ignored", "/evidence/0/ignored", "/findings/0/ignored"] {
        let mut edited = case.clone();
        if pointer == "/ignored" {
            edited["ignored"] = json!(true);
        } else if pointer.contains("evidence") {
            edited["evidence"][0]["ignored"] = json!(true);
        } else {
            edited["findings"][0]["ignored"] = json!(true);
        }
        assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    }
    let mut edited = case.clone();
    edited.as_object_mut().unwrap().remove("authorization");
    assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    edited = case.clone();
    edited["findings"][0]["severity"] = json!("catastrophic");
    assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    let encoded = serde_json::to_string(&case).unwrap();
    let duplicate = encoded.replacen("{", "{\"schema\":\"goblin.evidence-case.v1\",", 1);
    fs::write(root.join("input.json"), duplicate).unwrap();
    assert!(create_report(&root.join("input.json"), &root.join("report")).is_err());
}

#[test]
fn declarations_require_scope_authorization_and_explicit_utc() {
    let (_temp, root, case) = fixture();
    for field in ["authorization", "author", "summary", "title"] {
        let mut edited = case.clone();
        edited[field] = json!(" ");
        assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    }
    let mut edited = case.clone();
    edited["scope"] = json!([]);
    assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    for stamp in ["2026-10-13 14:00", "2026-10-13T14:00:00+01:00", "nonsense"] {
        edited = case.clone();
        edited["evidence"][0]["collected_utc"] = json!(stamp);
        assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    }
}

#[test]
fn ids_references_and_hashes_are_unambiguous() {
    let (_temp, root, case) = fixture();
    for refs in [json!([]), json!(["unknown"]), json!(["E1", "E1"])] {
        let mut edited = case.clone();
        edited["findings"][0]["evidence"] = refs;
        assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    }
    let mut edited = case.clone();
    edited["evidence"]
        .as_array_mut()
        .unwrap()
        .push(case["evidence"][0].clone());
    assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    edited = case.clone();
    edited["findings"]
        .as_array_mut()
        .unwrap()
        .push(case["findings"][0].clone());
    assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    for value in ["abc".into(), "F".repeat(64), "g".repeat(64)] {
        edited = case.clone();
        edited["evidence"][0]["sha256"] = json!(value);
        assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    }
}

#[test]
fn path_traversal_and_cross_platform_unsafe_paths_refuse() {
    let (_temp, root, case) = fixture();
    for path in [
        "../log.txt",
        "/etc/passwd",
        "a/../log.txt",
        "a//b",
        "./log.txt",
        "C:\\secret",
        "https://example.test",
        "log.txt\n",
    ] {
        let mut edited = case.clone();
        edited["evidence"][0]["path"] = json!(path);
        assert!(
            create_report(&write_case(&root, &edited), &root.join("report")).is_err(),
            "{path}"
        );
    }
}

#[test]
fn never_overwrites_an_existing_report_or_file() {
    let (_temp, root, case) = fixture();
    let source = write_case(&root, &case);
    let output = root.join("report");
    create_report(&source, &output).unwrap();
    let before = fs::read(output.join("report-receipt.json")).unwrap();
    assert!(create_report(&source, &output).is_err());
    assert_eq!(
        fs::read(output.join("report-receipt.json")).unwrap(),
        before
    );
    fs::write(root.join("existing"), b"keep me").unwrap();
    assert!(create_report(&source, &root.join("existing")).is_err());
    assert_eq!(fs::read(root.join("existing")).unwrap(), b"keep me");
}

#[test]
fn snapshot_case_and_rendered_file_tampering_all_refuse() {
    for path in [
        "case.json",
        "evidence/0001.bin",
        "report.md",
        "report.html",
        "report-receipt.json",
    ] {
        let (_temp, root, case) = fixture();
        let output = root.join("report");
        create_report(&write_case(&root, &case), &output).unwrap();
        fs::write(output.join(path), b"changed").unwrap();
        assert!(verify_report(&output, None).is_err(), "{path}");
    }
}

#[test]
fn resealing_fudged_report_hash_does_not_bypass_reconstruction() {
    let (_temp, root, case) = fixture();
    let output = root.join("report");
    create_report(&write_case(&root, &case), &output).unwrap();
    fs::write(output.join("report.html"), b"<script>fake</script>").unwrap();
    reseal(&output, |receipt| {
        receipt["outputs"][1]["sha256"] = json!(sha256_bytes(b"<script>fake</script>"));
        receipt["outputs"][1]["bytes"] = json!(21);
    });
    assert!(
        verify_report(&output, None)
            .unwrap_err()
            .message
            .contains("independent rendering")
    );
}

#[test]
fn resealed_inventory_cannot_redirect_reads_or_change_policy() {
    for (field, value) in [
        ("path", "../../secret"),
        ("policy", "unknown"),
        ("authentication", "SIGNED"),
    ] {
        let (_temp, root, case) = fixture();
        let output = root.join("report");
        create_report(&write_case(&root, &case), &output).unwrap();
        reseal(&output, |receipt| {
            if field == "path" {
                receipt["evidence"][0][field] = json!(value);
            } else {
                receipt[field] = json!(value);
            }
        });
        assert!(verify_report(&output, None).is_err());
    }
}

#[test]
fn trusted_receipt_and_unlisted_files_are_checked() {
    let (_temp, root, case) = fixture();
    let output = root.join("report");
    let receipt = create_report(&write_case(&root, &case), &output).unwrap();
    verify_report(&output, Some(&receipt.receipt_core_sha256)).unwrap();
    assert!(verify_report(&output, Some(&"0".repeat(64))).is_err());
    assert!(verify_report(&output, Some("bad")).is_err());
    fs::write(output.join("unlisted.txt"), "not inventoried").unwrap();
    assert!(verify_report(&output, None).is_err());
    fs::remove_file(output.join("unlisted.txt")).unwrap();
    fs::write(output.join("evidence/unlisted.bin"), "not inventoried").unwrap();
    assert!(verify_report(&output, None).is_err());
}

#[test]
fn injected_markup_remains_inert_and_unicode_is_preserved() {
    let (_temp, root, mut case) = fixture();
    let output = root.join("report");
    case["title"] = json!("研究 <script>alert('x')</script>");
    case["findings"][0]["observation"] =
        json!("![remote](https://example.test/track) <img src=x onerror=alert(1)> & \\u{202e}");
    create_report(&write_case(&root, &case), &output).unwrap();
    let page = fs::read_to_string(output.join("report.html")).unwrap();
    assert!(!page.contains("<script>"));
    assert!(!page.contains("<img"));
    assert!(page.contains("&lt;script&gt;"));
    assert!(page.contains("研究"));
    assert!(page.contains("default-src 'none'"));
    let md = fs::read_to_string(output.join("report.md")).unwrap();
    assert!(!md.contains("![remote]("));
    assert!(md.contains("\\<img"));
    assert!(!md.contains(" <img"));
    verify_report(&output, None).unwrap();
}

#[test]
fn control_characters_and_bidi_overrides_refuse_in_declarations() {
    let (_temp, root, case) = fixture();
    for text in [
        "hello\0",
        "bad\u{202e}title",
        "bad\u{2066}title",
        "bad\x1bterminal",
    ] {
        let mut edited = case.clone();
        edited["title"] = json!(text);
        assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    }
}

#[test]
fn forensic_purpose_and_zero_findings_do_not_claim_clean_system() {
    let (_temp, root, mut case) = fixture();
    let output = root.join("report");
    case["purpose"] = json!("forensic_analysis");
    case["findings"] = json!([]);
    create_report(&write_case(&root, &case), &output).unwrap();
    let report = fs::read_to_string(output.join("report.md")).unwrap();
    assert!(report.contains("Forensic analysis of supplied copies"));
    assert!(report.contains("Zero findings do not prove absence"));
    assert!(report.contains("not authenticated authorship"));
}

#[test]
fn explicit_file_case_item_and_aggregate_limits_refuse() {
    let (_temp, root, case) = fixture();
    fs::write(root.join("huge.json"), vec![b' '; CASE_BYTES + 1]).unwrap();
    assert!(create_report(&root.join("huge.json"), &root.join("report")).is_err());
    let oversized = vec![b'x'; FILE_BYTES + 1];
    fs::write(root.join("log.txt"), &oversized).unwrap();
    let mut edited = case.clone();
    edited["evidence"][0]["sha256"] = json!(sha256_bytes(&oversized));
    assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    let payload = vec![b'x'; FILE_BYTES];
    let hash = sha256_bytes(&payload);
    let mut evidence = Vec::new();
    for i in 0..5 {
        let name = format!("{i}.bin");
        fs::write(root.join(&name), &payload).unwrap();
        let mut e = case["evidence"][0].clone();
        e["id"] = json!(format!("E{i}"));
        e["path"] = json!(name);
        e["sha256"] = json!(hash);
        evidence.push(e);
    }
    edited = case.clone();
    edited["evidence"] = json!(evidence);
    edited["findings"] = json!([]);
    assert!(
        create_report(&write_case(&root, &edited), &root.join("report"))
            .unwrap_err()
            .message
            .contains("32 MiB")
    );
    let mut many: CaseDocument = serde_json::from_value(case).unwrap();
    many.evidence.resize(129, many.evidence[0].clone());
    assert!(goblinpp::evidence_report::validate_case(&many).is_err());
    assert!(!root.join("report").exists());
}

#[cfg(unix)]
#[test]
fn symlinks_are_refused_and_created_files_are_private() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let (_temp, root, case) = fixture();
    symlink(root.join("log.txt"), root.join("linked.txt")).unwrap();
    let mut edited = case.clone();
    edited["evidence"][0]["path"] = json!("linked.txt");
    assert!(create_report(&write_case(&root, &edited), &root.join("report")).is_err());
    symlink(&root, root.join("linked-dir")).unwrap();
    assert!(create_report(&write_case(&root, &case), &root.join("linked-dir/report")).is_err());
    let output = root.join("report");
    create_report(&write_case(&root, &case), &output).unwrap();
    assert_eq!(
        fs::metadata(&output).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(output.join("case.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    fs::remove_file(output.join("report.md")).unwrap();
    symlink(root.join("log.txt"), output.join("report.md")).unwrap();
    assert!(verify_report(&output, None).is_err());
}

#[test]
fn cli_reports_verifies_and_exits_nonzero_on_tamper() {
    let (_temp, root, case) = fixture();
    write_case(&root, &case);
    let executable = env!("CARGO_BIN_EXE_goblinpp");
    let result = Command::new(executable)
        .current_dir(&root)
        .args([
            "evidence-report",
            "input.json",
            "--output",
            "report",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: Value = serde_json::from_slice(&result.stdout).unwrap();
    let expected = receipt["receipt_core_sha256"].as_str().unwrap();
    let result = Command::new(executable)
        .current_dir(&root)
        .args([
            "verify-report",
            "report",
            "--expected-receipt-sha256",
            expected,
            "--json",
        ])
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stdout).unwrap()["verified"],
        true
    );
    fs::write(root.join("report/evidence/0001.bin"), "changed").unwrap();
    let failed = Command::new(executable)
        .current_dir(&root)
        .args(["verify-report", "report"])
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("G801"));
}
