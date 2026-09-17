use goblinpp::audit::verify_run;
use goblinpp::hashing::{hash_canonical_json, sha256_file};
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn everyday_program_needs_neither_directive_nor_seal() {
    let root = tempdir().unwrap();
    let source = root.path().join("everyday.gbl");
    fs::write(
        &source,
        "x = 2\nprint(\"x = {x}\")\nwrite_text(\"answer.txt\", \"x = {x}\")\n",
    )
    .unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PASS");
    assert_eq!(receipt["paranoid_mode"], false);
    assert!(receipt["sealed_artifacts"].as_array().unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(run.join("outputs/answer.txt")).unwrap(),
        "x = 2\n"
    );
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn paranoid_postflight_is_preserved_and_rehashed() {
    let root = tempdir().unwrap();
    let source = root.path().join("paranoid.gbl");
    fs::write(&source, "GO_PARANOID\nx = 2\nprint(\"x = {x}\")\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["schema"], "goblin.run-receipt.v2");
    assert_eq!(receipt["status"], "PASS");
    assert_eq!(receipt["paranoid_postflight"]["status"], "PASS");
    assert_eq!(
        receipt["paranoid_postflight"]["source_start_sha256"],
        receipt["source"]["sha256"]
    );
    assert_eq!(
        receipt["paranoid_postflight"]["source_end_sha256"],
        receipt["source"]["sha256"]
    );
    assert_eq!(
        receipt["paranoid_postflight"]["source_end_evidence_sha256"],
        sha256_file(run.join("postflight/source.observed")).unwrap()
    );
    assert_eq!(
        receipt["paranoid_postflight"]["self_verification_gate"],
        "REQUIRED_BEFORE_LEDGER_REGISTRATION"
    );
    let report = verify_run(&run).unwrap();
    assert!(report.verified);
    assert!(
        report
            .checks
            .iter()
            .any(|item| item.check == "PARANOID_POSTFLIGHT_EVIDENCE_SHA256" && item.pass)
    );
    fs::write(run.join("postflight/source.observed"), b"tampered").unwrap();
    let tampered = verify_run(run).unwrap();
    assert!(!tampered.verified);
    assert!(
        tampered
            .checks
            .iter()
            .any(|item| item.check == "PARANOID_POSTFLIGHT_EVIDENCE_SHA256" && !item.pass)
    );
}

#[test]
fn changed_source_at_postflight_refuses_pass_but_retains_verifiable_run() {
    let root = tempdir().unwrap();
    let source = root.path().join("change.gbl");
    let program = "GO_PARANOID\nx = 1\nRUST_INLINE_BEGIN\nstd::fs::write(\"change.gbl\", b\"x = 9\\n\").unwrap();\nRUST_INLINE_END\n";
    fs::write(&source, program).unwrap();
    let block_hash = parse_source(program).unwrap().inline_rust[0].sha256.clone();
    let run = run_file(
        &source,
        &RunOptions {
            compile: true,
            allowed_inline_rust: vec![block_hash],
            ..RunOptions::default()
        },
    )
    .unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PROTOCOL_VIOLATION");
    assert_eq!(
        receipt["protocol_violation"]["classification"],
        "SOURCE_CHANGED_AT_POSTFLIGHT"
    );
    assert_eq!(receipt["paranoid_postflight"]["status"], "SOURCE_CHANGED");
    assert_eq!(
        receipt["paranoid_postflight"]["source_bytes_equal_at_postflight"],
        false
    );
    assert_eq!(
        fs::read_to_string(run.join("postflight/source.observed")).unwrap(),
        "x = 9\n"
    );
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn missing_source_at_postflight_is_a_verifiable_refusal() {
    let root = tempdir().unwrap();
    let source = root.path().join("vanish.gbl");
    let program = "GO_PARANOID\nx = 1\nRUST_INLINE_BEGIN\nstd::fs::remove_file(\"vanish.gbl\").unwrap();\nRUST_INLINE_END\n";
    fs::write(&source, program).unwrap();
    let block_hash = parse_source(program).unwrap().inline_rust[0].sha256.clone();
    let run = run_file(
        &source,
        &RunOptions {
            compile: true,
            allowed_inline_rust: vec![block_hash],
            ..RunOptions::default()
        },
    )
    .unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PROTOCOL_VIOLATION");
    assert_eq!(
        receipt["protocol_violation"]["classification"],
        "SOURCE_UNAVAILABLE_AT_POSTFLIGHT"
    );
    assert_eq!(
        receipt["paranoid_postflight"]["status"],
        "SOURCE_UNAVAILABLE"
    );
    assert!(!source.exists());
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn unavailable_postflight_evidence_is_a_verifiable_refusal() {
    let root = tempdir().unwrap();
    let source = root.path().join("occupied.gbl");
    let program = "GO_PARANOID\nx = 1\nRUST_INLINE_BEGIN\nlet result = std::env::var(\"GOBLIN_NATIVE_RESULT_PATH\").unwrap();\nlet run = std::path::Path::new(&result).parent().unwrap();\nstd::fs::write(run.join(\"postflight\"), b\"occupied\").unwrap();\nRUST_INLINE_END\n";
    fs::write(&source, program).unwrap();
    let block_hash = parse_source(program).unwrap().inline_rust[0].sha256.clone();
    let run = run_file(
        &source,
        &RunOptions {
            compile: true,
            allowed_inline_rust: vec![block_hash],
            ..RunOptions::default()
        },
    )
    .unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PROTOCOL_VIOLATION");
    assert_eq!(
        receipt["protocol_violation"]["classification"],
        "POSTFLIGHT_EVIDENCE_UNAVAILABLE"
    );
    assert_eq!(
        receipt["paranoid_postflight"]["status"],
        "EVIDENCE_UNAVAILABLE"
    );
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn corrupt_paranoid_evidence_fails_self_check_and_is_not_ledger_registered() {
    let root = tempdir().unwrap();
    let source = root.path().join("gate.gbl");
    let program = "GO_PARANOID\nx = 1\nRUST_INLINE_BEGIN\nlet result = std::env::var(\"GOBLIN_NATIVE_RESULT_PATH\").unwrap();\nlet run = std::path::Path::new(&result).parent().unwrap();\nstd::fs::write(run.join(\"gate.gbl\"), b\"x = 999\\n\").unwrap();\nRUST_INLINE_END\n";
    fs::write(&source, program).unwrap();
    let block_hash = parse_source(program).unwrap().inline_rust[0].sha256.clone();
    let run = run_file(
        &source,
        &RunOptions {
            compile: true,
            allowed_inline_rust: vec![block_hash],
            ..RunOptions::default()
        },
    )
    .unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["failure"]["code"], "G405");
    assert!(!verify_run(run).unwrap().verified);
    assert!(!root.path().join(".goblin/custody-ledger.jsonl").exists());
}

#[test]
fn v1_run_receipts_remain_verifiable() {
    let root = tempdir().unwrap();
    let source = root.path().join("legacy.gbl");
    fs::write(&source, "x = 2\nprint(\"x = {x}\")\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt_path = run.join("receipt.json");
    let mut receipt: Value = read_receipt(&run).unwrap();
    receipt["schema"] = Value::String("goblin.run-receipt.v1".into());
    receipt
        .as_object_mut()
        .unwrap()
        .remove("paranoid_postflight");
    receipt
        .as_object_mut()
        .unwrap()
        .remove("receipt_core_sha256");
    receipt["receipt_core_sha256"] = Value::String(hash_canonical_json(&receipt).unwrap());
    let mut bytes = serde_json::to_vec_pretty(&receipt).unwrap();
    bytes.push(b'\n');
    fs::write(receipt_path, bytes).unwrap();
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn malformed_source_still_has_a_verifiable_failure_receipt() {
    let root = tempdir().unwrap();
    let source = root.path().join("bad.gbl");
    fs::write(&source, "for i in range(3) {\n x = i\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&run).unwrap()["status"], "MACHINERY_FAIL");
    assert!(verify_run(run).unwrap().verified);
}
