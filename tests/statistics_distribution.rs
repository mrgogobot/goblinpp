use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::compiler;
use goblinpp::custody::{create_freeze, verify_freeze, verify_registration};
use goblinpp::evaluator::{Evaluation, Value};
use goblinpp::hashing::{hash_canonical_json, sha256_bytes};
use goblinpp::parser::parse_source;
use goblinpp::quantity::{DIMENSIONLESS, LENGTH};
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::json;
use std::{fs, process::Command};
use tempfile::tempdir;

const PROGRAM: &str = r#"
measurements = [4 m, 1 m, 3 m, 2 m]
ordered = sort(measurements)
ordered[0] = 99 m
middle = median(measurements)
quarter = quantile(measurements, 0.25)
population = std_population(measurements)
sample = std_sample(measurements)
fraction = ecdf([2 m, 1 m, 2 m, 4 m], 2 m)
empty = sort([])
zeros = sort([0, -0, 0, -0])
negative_zero = quantile([-0, 0], 0)
odd = median([1 kg, 9 kg, 3 kg])
scaled = median([1 km, 500 m])
single = std_population([7 m])
large = std_population([1e16, 10000000000000002])
extreme = median([-1e308, 1e308])
g_func middle_of_copy(values) { return median(values) }
function_middle = middle_of_copy(measurements)
print("median = {middle}; Q1 = {quarter}; ECDF = {fraction}")
seal measurements
seal ordered
seal middle
seal quarter
seal population
seal sample
seal fraction
seal empty
seal zeros
seal negative_zero
seal odd
seal scaled
seal single
seal large
seal extreme
seal function_middle
"#;

#[test]
fn distribution_results_are_dimension_aware_copies() {
    let parsed = parse_source(PROGRAM).unwrap();
    let result = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(result.env["measurements"].render(), "[4 m, 1 m, 3 m, 2 m]");
    assert_eq!(result.env["ordered"].render(), "[99 m, 2 m, 3 m, 4 m]");
    for (name, expected, dimension) in [
        ("middle", 2.5, LENGTH),
        ("quarter", 1.75, LENGTH),
        ("fraction", 0.75, DIMENSIONLESS),
        ("scaled", 750.0, LENGTH),
        ("single", 0.0, LENGTH),
        ("large", 1.0, DIMENSIONLESS),
        ("extreme", 0.0, DIMENSIONLESS),
        ("function_middle", 2.5, LENGTH),
    ] {
        let Value::Quantity(value) = result.env[name] else {
            panic!("{name}")
        };
        assert_eq!(value.value_si, expected, "{name}");
        assert_eq!(value.dimension, dimension, "{name}");
    }
    assert_eq!(result.env["zeros"].render(), "[0, -0, 0, -0]");
    assert_eq!(result.env["negative_zero"].render(), "-0");
    assert_eq!(result.env["empty"].render(), "[]");
}

#[test]
fn distributions_match_native_execution_and_preserve_exact_evidence() {
    let root = tempdir().unwrap();
    let source = root.path().join("statistics.gbl");
    for paranoid in [false, true] {
        fs::write(
            &source,
            format!("{}{PROGRAM}", if paranoid { "GO_PARANOID\n" } else { "" }),
        )
        .unwrap();
        let mut receipts = Vec::new();
        let mut runs = Vec::new();
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
            assert_eq!(receipt["status"], "PASS", "{receipt}");
            assert!(verify_run(&run).unwrap().verified);
            assert_eq!(
                receipt["statistics_policy"],
                goblinpp::science::statistics_policy()
            );
            receipts.push(receipt);
            runs.push(run);
        }
        assert_eq!(
            receipts[0]["sealed_artifacts"],
            receipts[1]["sealed_artifacts"]
        );
        assert_eq!(
            fs::read(runs[0].join("stdout.log")).unwrap(),
            fs::read(runs[1].join("stdout.log")).unwrap()
        );
        assert!(
            diff_runs(&runs[0], &runs[1])
                .unwrap()
                .statistics_policy_same
        );
        let parsed = parse_source(&fs::read_to_string(&source).unwrap()).unwrap();
        let native =
            compiler::compile(&parsed, root.path().join(format!("native-{paranoid}")), &[])
                .unwrap();
        let output = Command::new(&native.binary).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains("median = 2.5 m; Q1 = 1.75 m; ECDF = 0.75")
        );
        let mut receipt = receipts[0].clone();
        receipt["statistics_policy"]["quantile"] = json!("ALTERED");
        fs::write(
            runs[0].join("receipt.json"),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
        assert!(!verify_run(&runs[0]).unwrap().verified);
    }
}

#[test]
fn invalid_distribution_calls_refuse_without_dropping_values() {
    for program in [
        "x = sort(1)",
        "x = sort([true])",
        "x = sort([\"1\"])",
        "x = median([])",
        "x = quantile([], 0.5)",
        "x = std_population([])",
        "x = std_sample([1])",
        "x = ecdf([], 1)",
        "x = median([1 m, 1 s])",
        "x = quantile([1], -0.1)",
        "x = quantile([1], 1.1)",
        "x = quantile([1], 0.5 m)",
        "x = quantile([1], true)",
        "x = ecdf([1 m], 1 s)",
        "x = ecdf([1], \"1\")",
        "x = median([1, 1/0])",
        "x = std_sample([-1.7976931348623157e308, 1.7976931348623157e308])",
        "x = median()",
        "x = quantile([1])",
        "x = std_population([1], [2])",
        "sort([1])",
        "median([1])",
        "quantile([1], 0.5)",
        "std_population([1])",
        "std_sample([1, 2])",
        "ecdf([1], 1)",
    ] {
        let root = tempdir().unwrap();
        let source = root.path().join("invalid.gbl");
        fs::write(&source, format!("GO_PARANOID\n{program}\n")).unwrap();
        for compile in [false, true] {
            let run = run_file(
                &source,
                &RunOptions {
                    compile,
                    ..RunOptions::default()
                },
            )
            .unwrap();
            assert_eq!(
                read_receipt(&run).unwrap()["status"],
                "MACHINERY_FAIL",
                "{program}"
            );
            assert!(verify_run(&run).unwrap().verified, "{program}");
        }
    }
}

#[test]
fn native_runtime_independently_refuses_invalid_statistics() {
    for program in [
        "x = std_sample([1])",
        "x = quantile([1], 1.01)",
        "x = ecdf([1 m], 2 s)",
        "x = sort([true])",
    ] {
        let root = tempdir().unwrap();
        let parsed = parse_source(program).unwrap();
        let native = compiler::compile(&parsed, root.path().join("native"), &[]).unwrap();
        let output = Command::new(native.binary).output().unwrap();
        assert!(!output.status.success(), "{program}");
        assert!(!output.stderr.is_empty(), "{program}");
    }
}

#[test]
fn old_function_syntax_still_hashes_but_new_builtin_collisions_refuse_execution() {
    for name in [
        "sort",
        "median",
        "quantile",
        "std_population",
        "std_sample",
        "ecdf",
    ] {
        for source in [
            format!("g_func {name}(x) {{ return x }}\ny = {name}(1)\n"),
            format!("g_func f({name}) {{ return {name} }}\ny = f(1)\n"),
        ] {
            let parsed = parse_source(&source).unwrap();
            assert!(parsed.canonical_sha256().is_ok());
            assert!(goblinpp::parser::validate_execution(&parsed.program).is_err());
            assert!(Evaluation::new(".").eval_program(&parsed.program).is_err());
        }
        let parsed = parse_source(&format!("{name} = 7\nprint({name})\n")).unwrap();
        assert!(Evaluation::new(".").eval_program(&parsed.program).is_ok());
    }
}

#[test]
fn statistics_freeze_pins_policy_and_legacy_missing_policy_is_not_invented() {
    for legacy in [false, true] {
        let root = tempdir().unwrap();
        let source = root.path().join("frozen.gbl");
        fs::write(&source, "GO_PARANOID\nx = median([1 m, 3 m])\nseal x\n").unwrap();
        let (path, event) = create_freeze(&source).unwrap();
        let mut frozen: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            frozen["statistics_policy"],
            goblinpp::science::statistics_policy()
        );
        if legacy {
            frozen.as_object_mut().unwrap().remove("statistics_policy");
        } else {
            frozen["statistics_policy"]["quantile"] = json!("DIFFERENT_POLICY");
        }
        frozen
            .as_object_mut()
            .unwrap()
            .remove("freeze_receipt_sha256");
        frozen["freeze_receipt_sha256"] = json!(hash_canonical_json(&frozen).unwrap());
        let bytes = serde_json::to_vec_pretty(&frozen).unwrap();
        fs::write(&path, &bytes).unwrap();
        // Register a checksum-valid historical/policy fixture before enforcing it.
        let mut payload = event.payload;
        payload["freeze_receipt_core_sha256"] = frozen["freeze_receipt_sha256"].clone();
        payload["freeze_receipt_file_sha256"] = json!(sha256_bytes(&bytes));
        goblinpp::ledger::append(
            goblinpp::ledger::project_root_for(&source).unwrap(),
            "FREEZE",
            event.subject,
            payload,
        )
        .unwrap();
        let freeze = verify_freeze(&source);
        assert!(freeze.verified && verify_registration(&source, &freeze).unwrap());
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
            assert_eq!(
                receipt["status"],
                if legacy { "PASS" } else { "PROTOCOL_VIOLATION" }
            );
            if !legacy {
                assert_eq!(
                    receipt["freeze"]["classification"],
                    "STATISTICS_POLICY_CHANGED_AFTER_FREEZE"
                );
            }
            assert!(verify_run(&run).unwrap().verified);
        }
        let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
            .arg("compile")
            .arg(&source)
            .arg("-o")
            .arg(root.path().join("native"))
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            legacy,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn policy_changes_are_reported_separately_from_equal_scientific_results() {
    let root = tempdir().unwrap();
    let source = root.path().join("same.gbl");
    fs::write(&source, "x = median([1, 3])\nseal x\n").unwrap();
    let a = run_file(&source, &RunOptions::default()).unwrap();
    let b = run_file(&source, &RunOptions::default()).unwrap();
    let mut receipt = read_receipt(&b).unwrap();
    receipt["statistics_policy"]["quantile"] = json!("HISTORICAL_DIFFERENT_POLICY");
    receipt
        .as_object_mut()
        .unwrap()
        .remove("receipt_core_sha256");
    receipt["receipt_core_sha256"] = json!(hash_canonical_json(&receipt).unwrap());
    fs::write(
        b.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert!(verify_run(&b).unwrap().verified);
    let diff = diff_runs(a, b).unwrap();
    assert!(diff.sealed_artifacts_same && !diff.statistics_policy_same);
    assert_eq!(diff.classification, "STATISTICS_POLICY_CHANGE");
}
