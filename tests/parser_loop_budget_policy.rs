use goblinpp::ast::{Expr, Stmt};
use goblinpp::audit::verify_run;
use goblinpp::custody::{create_freeze, verify_freeze};
use goblinpp::hashing::{hash_canonical_json, sha256_bytes};
use goblinpp::parser::{
    LEGACY_COMPOUND_PARSER_POLICY, PARSER_POLICY, SyntaxMode, evidence_mode, parse_source,
    parse_source_with_mode,
};
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::{Value, json};
use std::fs;
use tempfile::tempdir;

#[test]
fn genuine_alpha26_identifier_receipt_and_canonical_hash_remain_verifiable() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/alpha26-loop-budget-identifiers");
    let receipt = read_receipt(&fixture).unwrap();
    assert_eq!(receipt["goblin_version"], "0.1.0-alpha.26");
    assert_eq!(receipt["parser_policy"], LEGACY_COMPOUND_PARSER_POLICY);
    let text = fs::read_to_string(fixture.join("historical-identifiers.gbl")).unwrap();
    let parsed = parse_source_with_mode(&text, SyntaxMode::CompoundUnitsNoLoopBudget).unwrap();
    assert_eq!(
        parsed.canonical_sha256().unwrap(),
        "af30e65831bb194d3f226503299c9819411883640f540a35813b63abc6c5d20a"
    );
    assert!(parse_source(&text).is_err());
    assert!(
        verify_run(&fixture).unwrap().verified,
        "{:?}",
        verify_run(&fixture).unwrap()
    );
}

#[test]
fn old_identifier_positions_and_implicit_multiplication_preserve_old_ast() {
    for mode in [
        SyntaxMode::CompoundUnitsNoLoopBudget,
        SyntaxMode::LegacyQuantity,
    ] {
        for text in [
            "GO_LOOP_BUDGET = 2",
            "g_func GO_LOOP_BUDGET() { return 1 }",
            "g_func helper(GO_LOOP_BUDGET) { return GO_LOOP_BUDGET }",
            "for GO_LOOP_BUDGET in range(2) { value = GO_LOOP_BUDGET }",
        ] {
            assert!(
                parse_source_with_mode(text, mode).is_ok(),
                "{mode:?}: {text}"
            );
            assert!(parse_source(text).is_err(), "{text}");
        }
        let old = parse_source_with_mode("GO_LOOP_BUDGET 50", mode).unwrap();
        assert!(matches!(
            &old.program.statements[0],
            Stmt::Expression(Expr::Binary { op: '*', .. })
        ));
        assert_ne!(
            old.canonical_sha256().unwrap(),
            parse_source("GO_LOOP_BUDGET 50")
                .unwrap()
                .canonical_sha256()
                .unwrap()
        );
    }
    for text in [
        "x = 1.13e-10 m/s^2",
        "x = 3 m^2",
        "x = 4 kg/(m*s^2)",
        "x = 1 m\ny = 2\nseal x",
    ] {
        assert_eq!(
            parse_source_with_mode(text, SyntaxMode::CompoundUnitsNoLoopBudget)
                .unwrap()
                .canonical_sha256()
                .unwrap(),
            parse_source(text).unwrap().canonical_sha256().unwrap()
        );
    }
}

#[test]
fn parser_evidence_cannot_downgrade_a_new_release() {
    for alpha in [25, 26] {
        assert_eq!(evidence_mode(&json!({"goblin_version":format!("0.1.0-alpha.{alpha}"),"parser_policy":LEGACY_COMPOUND_PARSER_POLICY})).unwrap(),SyntaxMode::CompoundUnitsNoLoopBudget);
    }
    for alpha in [27, 28] {
        assert!(evidence_mode(&json!({"goblin_version":format!("0.1.0-alpha.{alpha}"),"parser_policy":LEGACY_COMPOUND_PARSER_POLICY})).is_err());
        assert!(evidence_mode(&json!({"goblin_version":format!("0.1.0-alpha.{alpha}")})).is_err());
    }
    assert_eq!(
        evidence_mode(&json!({"goblin_version":"0.1.0-alpha.24"})).unwrap(),
        SyntaxMode::LegacyQuantity
    );
    assert_eq!(
        evidence_mode(&json!({"goblin_version":"0.1.0-alpha.27","parser_policy":PARSER_POLICY}))
            .unwrap(),
        SyntaxMode::Current
    );
    let dir = tempdir().unwrap();
    let source = dir.path().join("downgrade.gbl");
    fs::write(&source, "x = 1\nseal x").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    receipt["parser_policy"] = json!(LEGACY_COMPOUND_PARSER_POLICY);
    receipt
        .as_object_mut()
        .unwrap()
        .remove("receipt_core_sha256");
    receipt["receipt_core_sha256"] = json!(hash_canonical_json(&receipt).unwrap());
    fs::write(
        run.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert!(!verify_run(&run).unwrap().verified);
}

#[test]
fn v1_freeze_verifies_but_execution_and_compilation_require_revision() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("frozen.gbl");
    fs::write(&source, "GO_PARANOID\nx = 3 m/s\nseal x").unwrap();
    let (path, event) = create_freeze(&source).unwrap();
    let mut frozen: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    frozen["goblin_version"] = json!("0.1.0-alpha.26");
    frozen["parser_policy"] = json!(LEGACY_COMPOUND_PARSER_POLICY);
    frozen
        .as_object_mut()
        .unwrap()
        .remove("freeze_receipt_sha256");
    frozen["freeze_receipt_sha256"] = json!(hash_canonical_json(&frozen).unwrap());
    let bytes = serde_json::to_vec_pretty(&frozen).unwrap();
    fs::write(&path, &bytes).unwrap();
    let mut payload = event.payload;
    payload["freeze_receipt_core_sha256"] = frozen["freeze_receipt_sha256"].clone();
    payload["freeze_receipt_file_sha256"] = json!(sha256_bytes(&bytes));
    goblinpp::ledger::append(dir.path(), "FREEZE", event.subject, payload).unwrap();
    assert!(verify_freeze(&source).verified);
    for compile in [false, true] {
        let run = run_file(
            &source,
            &RunOptions {
                compile,
                ..RunOptions::default()
            },
        )
        .unwrap();
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "PROTOCOL_VIOLATION");
        assert_eq!(
            receipt["protocol_violation"]["classification"],
            "PARSER_POLICY_CHANGED_AFTER_FREEZE"
        );
        assert!(verify_run(run).unwrap().verified);
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .arg("compile")
        .arg(&source)
        .arg("--output")
        .arg(dir.path().join("native"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("PARSER_POLICY_CHANGED_AFTER_FREEZE"));
}
