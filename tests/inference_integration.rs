use goblinpp::{
    audit::{diff_runs, verify_run},
    custody::create_freeze,
    evaluator::Evaluation,
    hashing::hash_canonical_json,
    parser::parse_source,
    runtime::{RunOptions, read_receipt, run_file},
};
use serde_json::json;
use std::{fs, process::Command};
use tempfile::tempdir;

const PROGRAM: &str = include_str!("../examples/inference.gbl");

#[test]
fn inference_interpreter_native_and_standalone_evidence_agree() {
    let root = tempdir().unwrap();
    let source = root.path().join("inference.gbl");
    fs::write(&source, PROGRAM).unwrap();
    let reference = run_file(&source, &RunOptions::default()).unwrap();
    let reference_receipt = read_receipt(&reference).unwrap();
    assert_eq!(reference_receipt["status"], "PASS");
    assert!(verify_run(&reference).unwrap().verified);
    let native = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    let native_receipt = read_receipt(&native).unwrap();
    assert_eq!(
        native_receipt["status"],
        "PASS",
        "{}",
        fs::read_to_string(native.join("stderr.log")).unwrap()
    );
    assert!(verify_run(&native).unwrap().verified);
    assert_eq!(reference_receipt["rng"], native_receipt["rng"]);
    assert_eq!(reference_receipt["inference"], native_receipt["inference"]);
    assert_eq!(
        reference_receipt["sealed_artifacts"],
        native_receipt["sealed_artifacts"]
    );
    let report = diff_runs(&reference, &native).unwrap();
    assert!(
        report.inference_policy_same
            && report.inference_evidence_same
            && report.resource_evidence_same
    );
    let binary = root.path().join("standalone");
    goblinpp::compiler::compile(&parse_source(PROGRAM).unwrap(), &binary, &[]).unwrap();
    let result = Command::new(&binary)
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("[58, 64, 139, 154]"));
    let evidence_path = String::from_utf8_lossy(&result.stderr)
        .lines()
        .find_map(|s| s.strip_prefix("NATIVE_DATA_DIR="))
        .unwrap()
        .to_string();
    let evidence: serde_json::Value = serde_json::from_slice(
        &fs::read(root.path().join(evidence_path).join("native-data.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(reference_receipt["inference"], evidence["inference"]);
}

#[test]
fn new_value_only_calls_collisions_and_arity_refuse_before_draws() {
    for name in [
        "resample",
        "bootstrap_mean",
        "bootstrap_median",
        "rng_normal",
        "rng_normal_array",
        "matrix_transpose",
        "matrix_multiply",
        "vector_norm",
        "vector_unit",
        "covariance",
        "cholesky",
        "mvnormal",
    ] {
        let historical =
            parse_source(&format!("g_func {name}(value) {{ return value }}\n")).unwrap();
        assert!(
            Evaluation::new(".")
                .register_functions(&historical.program)
                .is_err()
        );
        assert!(
            parse_source(&format!("{name}()\n"))
                .and_then(|p| goblinpp::parser::validate_execution(&p.program))
                .is_err()
        );
        let program = parse_source(&format!("rng_seed(42)\nvalue = {name}(rng_word(), rng_word(), rng_word(), rng_word(), rng_word(), rng_word(), rng_word())\n")).unwrap();
        let mut eval = Evaluation::new(".");
        eval.register_functions(&program.program).unwrap();
        eval.eval_stmt(&program.program.statements[0]).unwrap();
        let before = eval.randomness.evidence();
        assert!(eval.eval_stmt(&program.program.statements[1]).is_err());
        assert_eq!(eval.randomness.evidence(), before);
    }
}

#[test]
fn inference_policy_missing_or_tampered_is_not_hidden_by_rehash() {
    let root = tempdir().unwrap();
    let source = root.path().join("test.gbl");
    fs::write(&source, "result = vector_unit([3,4])\nseal result\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    receipt["inference"]["functions"][0]["function"] = json!("not_a_helper");
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
    receipt.as_object_mut().unwrap().remove("inference");
    receipt.as_object_mut().unwrap().remove("inference_policy");
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
fn inference_freeze_pins_policy_and_preserves_refusal() {
    let root = tempdir().unwrap();
    let source = root.path().join("test.gbl");
    fs::write(
        &source,
        "result = matrix_transpose([1,2,3,4],2,2)\nseal result\n",
    )
    .unwrap();
    create_freeze(&source).unwrap();
    let path = source.with_file_name("test.gbl.freeze.json");
    let mut freeze: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    freeze["inference_policy"]["id"] = json!("unknown");
    freeze
        .as_object_mut()
        .unwrap()
        .remove("freeze_receipt_sha256");
    freeze["freeze_receipt_sha256"] = json!(hash_canonical_json(&freeze).unwrap());
    fs::write(path, serde_json::to_vec_pretty(&freeze).unwrap()).unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PROTOCOL_VIOLATION");
    // The registration ledger independently catches rewritten freeze evidence too.
    assert!(receipt["protocol_violation"].is_object());
    assert!(verify_run(&run).unwrap().verified);
}

#[test]
fn rehashed_native_summary_cannot_disagree_with_receipt() {
    let root = tempdir().unwrap();
    let source = root.path().join("native.gbl");
    fs::write(
        &source,
        "GO_PARANOID\nresult = vector_unit([3,4])\nseal result\n",
    )
    .unwrap();
    let run = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    let path = run.join("native-data.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest["inference"]["functions"][0]["calls"] = json!(2);
    fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    // Rehash both containers; independent summary parity must still fail.
    receipt["execution"]["compiler"]["native_data_manifest_sha256"] =
        json!(goblinpp::hashing::sha256_file(&path).unwrap());
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
    let report = verify_run(&run).unwrap();
    assert!(!report.verified);
    assert!(
        report
            .checks
            .iter()
            .any(|c| c.check == "NATIVE_DATA_EVIDENCE_PARITY" && !c.pass),
        "{report:?}"
    );
}

#[test]
fn current_freeze_cannot_drop_required_inference_policy() {
    let root = tempdir().unwrap();
    let source = root.path().join("frozen.gbl");
    fs::write(&source, "value = 1\nseal value\n").unwrap();
    create_freeze(&source).unwrap();
    let path = source.with_file_name("frozen.gbl.freeze.json");
    let mut receipt: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    receipt.as_object_mut().unwrap().remove("inference_policy");
    receipt
        .as_object_mut()
        .unwrap()
        .remove("freeze_receipt_sha256");
    receipt["freeze_receipt_sha256"] = json!(hash_canonical_json(&receipt).unwrap());
    fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert!(!goblinpp::custody::verify_freeze(&source).verified);
}
