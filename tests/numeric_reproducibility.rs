use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::comparison::compare_sealed_quantities;
use goblinpp::custody::{create_freeze, verify_freeze};
use goblinpp::evaluator::{Evaluation, Value as EvalValue};
use goblinpp::hashing::{hash_canonical_json, sha256_bytes};
use goblinpp::numeric_comparison::is_close;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::{Value, json};
use std::{fs, io::Write, path::Path, process::Command};
use tempfile::tempdir;

#[test]
fn tolerance_validation_boundaries_extremes_and_signed_zero() {
    for (a, b, absolute, relative, expected) in [
        (1.0, 1.5, 0.5, 0.0, true),
        (1.0, 1.5, 0.49, 0.0, false),
        (1.0, 2.0, 0.0, 0.5, true),
        (1.0, 2.0, 0.0, 0.49, false),
        (0.0, -0.0, 0.0, 0.0, true),
        (0.0, f64::from_bits(1), 0.0, 0.0, false),
        (0.0, f64::from_bits(1), f64::from_bits(1), 0.0, true),
        (f64::MAX, -f64::MAX, f64::MAX, 0.0, false),
        (f64::MAX, -f64::MAX, 0.0, 1.99, false),
        (f64::MAX, -f64::MAX, 0.0, 2.0, true),
        (
            f64::MAX,
            f64::from_bits(f64::MAX.to_bits() - 1),
            0.0,
            0.0,
            false,
        ),
    ] {
        assert_eq!(is_close(a, b, absolute, relative).unwrap(), expected);
        assert_eq!(is_close(b, a, absolute, relative).unwrap(), expected);
    }
    for args in [
        (1.0, 1.0, -1.0, 0.0),
        (1.0, 1.0, 0.0, -1.0),
        (f64::NAN, 0.0, 0.0, 0.0),
        (0.0, f64::INFINITY, 0.0, 0.0),
        (0.0, 0.0, f64::INFINITY, 0.0),
        (0.0, 0.0, 0.0, f64::NAN),
    ] {
        assert!(is_close(args.0, args.1, args.2, args.3).is_err());
    }
}

#[test]
fn comparison_builtins_are_dimension_aware_and_have_no_defaults() {
    let source = "a = is_close(1 m, 1.0001 m, 0.001 m, 0)\nb = is_close(1 m, 1.1 m, 0 m, 0)\nbits_match = same_bits(0, -0)\nd = is_close(0, -0, 0, 0)\ne = same_bits(1000 m, 1 km)\n";
    let parsed = parse_source(source).unwrap();
    let result = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    for (name, expected) in [
        ("a", true),
        ("b", false),
        ("bits_match", false),
        ("d", true),
        ("e", true),
    ] {
        assert_eq!(result.env[name], EvalValue::Bool(expected));
    }
    for (source, code) in [
        ("x = is_close(1 m, 1 s, 0 m, 0)\n", "G201"),
        ("x = is_close(1 m, 1 m, 0, 0)\n", "G201"),
        ("x = is_close(1 m, 1 m, 0 m, 0 s)\n", "G201"),
        ("x = is_close(1, 1, -1, 0)\n", "G202"),
        ("x = is_close(1, 1, 0, -1)\n", "G202"),
        ("x = is_close(1, 1)\n", "G002"),
        ("x = same_bits(1 m, 1 s)\n", "G201"),
        ("is_close(1, 1, 0, 0)\n", "G002"),
        ("same_bits(1, 1)\n", "G002"),
        ("g_func is_close(x) { return x }\ny = is_close(1)\n", "G002"),
    ] {
        let parsed = parse_source(source).unwrap();
        let error = Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_err();
        assert_eq!(error.code, code, "{source}: {error}");
    }
}

#[test]
fn both_engines_record_policy_and_identity_without_weakened_verification() {
    let root = tempdir().unwrap();
    let source = root.path().join("comparison.gbl");
    fs::write(&source, include_str!("../examples/numeric_comparison.gbl")).unwrap();
    create_freeze(&source).unwrap();
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
        assert_eq!(receipt["status"], "PASS", "{:?}", receipt["failure"]);
        assert_eq!(receipt["math_policy"], goblinpp::math_policy::policy());
        assert!(
            receipt["math_environment"]["launcher_build"]["rustc"]
                .as_str()
                .unwrap()
                .starts_with("rustc ")
        );
        assert_eq!(
            receipt["math_environment"]["launcher_binary_sha256"]
                .as_str()
                .unwrap()
                .len(),
            64
        );
        assert_eq!(
            receipt["math_environment"]["native_compiler"].is_string(),
            compile
        );
        assert!(verify_run(&run).unwrap().verified);
        runs.push(run);
    }
    let report = diff_runs(&runs[0], &runs[1]).unwrap();
    assert!(report.math_policy_same);
    assert!(!report.math_environment_same);
    assert!(report.sealed_artifacts_same);
    let comparison = compare_sealed_quantities(&runs[0], &runs[1], "observed", 0.0, 0.0).unwrap();
    assert_eq!(comparison["classification"], "BITWISE_IDENTICAL");
    fs::write(runs[1].join("observed.json"), b"{}").unwrap();
    assert!(!verify_run(&runs[1]).unwrap().verified);
    assert!(compare_sealed_quantities(&runs[0], &runs[1], "observed", 1e100, 1e100).is_err());
}

fn scalar_run(root: &Path, file: &str, value: &str) -> std::path::PathBuf {
    let source = root.join(file);
    fs::write(&source, format!("x = {value}\nseal x\n")).unwrap();
    run_file(source, &RunOptions::default()).unwrap()
}

#[test]
fn sealed_comparison_has_distinct_labels_dimension_checks_and_cli_exit_codes() {
    let root = tempdir().unwrap();
    let a = scalar_run(root.path(), "a.gbl", "1 m");
    let b = scalar_run(root.path(), "b.gbl", "1.0001 m");
    let time = scalar_run(root.path(), "time.gbl", "1 s");
    let text = scalar_run(root.path(), "text.gbl", "\"one\"");
    assert_eq!(
        compare_sealed_quantities(&a, &b, "x", 0.001, 0.0).unwrap()["classification"],
        "WITHIN_DECLARED_TOLERANCE"
    );
    assert_eq!(
        compare_sealed_quantities(&a, &b, "x", 0.0, 0.0).unwrap()["classification"],
        "OUTSIDE_DECLARED_TOLERANCE"
    );
    assert_eq!(
        diff_runs(&a, &b).unwrap().classification,
        "SEMANTIC_OR_RESULT_CHANGE"
    );
    for target in [&time, &text] {
        assert!(compare_sealed_quantities(&a, target, "x", 100.0, 100.0).is_err());
    }
    assert!(compare_sealed_quantities(&a, &b, "missing", 100.0, 100.0).is_err());
    for (absolute, expected_exit) in [("0.001", 0), ("0", 1), ("-1", 2), ("NaN", 2)] {
        let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
            .args([
                "compare-value",
                a.to_str().unwrap(),
                b.to_str().unwrap(),
                "x",
                "--absolute-tolerance-si",
                absolute,
                "--relative-tolerance",
                "0",
                "--json",
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected_exit),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args([
            "compare-value",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "x",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let positive = scalar_run(root.path(), "positive.gbl", "0");
    let negative = scalar_run(root.path(), "negative.gbl", "-0");
    let report = compare_sealed_quantities(positive, negative, "x", 0.0, 0.0).unwrap();
    assert_eq!(report["numeric_equal"], true);
    assert_eq!(report["bits_equal"], false);
    assert_eq!(report["classification"], "WITHIN_DECLARED_TOLERANCE");
}

#[test]
fn failed_comparisons_are_preserved_and_verifiable_in_both_modes() {
    for expression in [
        "is_close(1 m, 1 m, 0, 0)",
        "is_close(1, 1, -1, 0)",
        "is_close(1, 1)",
        "same_bits(1 m, 1 s)",
    ] {
        let root = tempdir().unwrap();
        let source = root.path().join("bad.gbl");
        fs::write(&source, format!("GO_PARANOID\nx = {expression}\nseal x\n")).unwrap();
        for compile in [false, true] {
            let run = run_file(
                &source,
                &RunOptions {
                    compile,
                    ..RunOptions::default()
                },
            )
            .unwrap();
            assert_eq!(read_receipt(&run).unwrap()["status"], "MACHINERY_FAIL");
            assert!(verify_run(run).unwrap().verified);
        }
    }
}

#[test]
fn unknown_or_changed_math_metadata_does_not_relabel_history_as_identical() {
    let root = tempdir().unwrap();
    let a = scalar_run(root.path(), "same.gbl", "1");
    let b = scalar_run(root.path(), "same.gbl", "1");
    let path = b.join("receipt.json");
    let mut receipt = read_receipt(&b).unwrap();
    receipt.as_object_mut().unwrap().remove("math_policy");
    receipt.as_object_mut().unwrap().remove("math_environment");
    // Construct a checksum-valid historical-shaped fixture; not an authorship proof.
    receipt
        .as_object_mut()
        .unwrap()
        .remove("receipt_core_sha256");
    receipt["receipt_core_sha256"] = json!(hash_canonical_json(&receipt).unwrap());
    fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert!(verify_run(&b).unwrap().verified);
    assert_eq!(
        diff_runs(&a, &b).unwrap().classification,
        "MATH_POLICY_CHANGE"
    );
    let report = compare_sealed_quantities(a, b, "x", 0.0, 0.0).unwrap();
    assert!(report["right"]["math_policy"].is_null());
    assert_eq!(report["classification"], "BITWISE_IDENTICAL");
}

#[test]
fn math_freeze_pins_policy_not_a_false_cross_platform_bitwise_promise() {
    let root = tempdir().unwrap();
    let source = root.path().join("frozen.gbl");
    fs::write(&source, "x = cosr(1)\nseal x\n").unwrap();
    let (path, _) = create_freeze(&source).unwrap();
    let receipt: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(receipt["math_policy"], goblinpp::math_policy::policy());
    assert!(goblinpp::math_policy::frozen_policy_matches(&receipt));
    let mut changed = receipt.clone();
    changed["math_policy"]["id"] = json!("future-policy");
    assert!(!goblinpp::math_policy::frozen_policy_matches(&changed));
    let mut legacy = receipt.clone();
    legacy.as_object_mut().unwrap().remove("math_policy");
    assert!(goblinpp::math_policy::frozen_policy_matches(&legacy));
    fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(!verify_freeze(&source).verified);
}

#[test]
fn declared_freeze_policy_is_enforced_but_legacy_policy_is_not_invented() {
    for legacy in [false, true] {
        let root = tempdir().unwrap();
        let source = root.path().join("frozen.gbl");
        fs::write(&source, "x = cosr(1)\nseal x\n").unwrap();
        let (path, original_event) = create_freeze(&source).unwrap();
        let mut frozen: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if legacy {
            frozen.as_object_mut().unwrap().remove("math_policy");
        } else {
            frozen["math_policy"]["id"] = json!("historical-other-policy");
        }
        frozen
            .as_object_mut()
            .unwrap()
            .remove("freeze_receipt_sha256");
        frozen["freeze_receipt_sha256"] = json!(hash_canonical_json(&frozen).unwrap());
        let bytes = serde_json::to_vec_pretty(&frozen).unwrap();
        fs::write(&path, &bytes).unwrap();
        // Simulate an independently registered historical freeze, not a tamper
        // bypass: integrity/registration must pass before policy enforcement.
        let mut payload = original_event.payload;
        payload["freeze_receipt_core_sha256"] = frozen["freeze_receipt_sha256"].clone();
        payload["freeze_receipt_file_sha256"] = json!(sha256_bytes(&bytes));
        let ledger_root = goblinpp::ledger::project_root_for(&source).unwrap();
        goblinpp::ledger::append(ledger_root, "FREEZE", original_event.subject, payload).unwrap();
        let report = verify_freeze(&source);
        assert!(report.verified);
        assert!(goblinpp::custody::verify_registration(&source, &report).unwrap());
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
                    "MATH_POLICY_CHANGED_AFTER_FREEZE"
                );
            }
            assert!(verify_run(run).unwrap().verified);
        }
        let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
            .args([
                "compile",
                source.to_str().unwrap(),
                "--output",
                root.path().join("standalone").to_str().unwrap(),
            ])
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
fn native_comparison_errors_are_tested_without_interpreter_preflight() {
    for expression in [
        "is_close(1 m, 1 m, 0, 0)",
        "is_close(1, 1, -1, 0)",
        "same_bits(1 m, 1 s)",
    ] {
        let root = tempdir().unwrap();
        let parsed = parse_source(&format!("x = {expression}\nprint(x)\n")).unwrap();
        let binary = root.path().join("native");
        goblinpp::compiler::compile(&parsed, &binary, &[]).unwrap();
        let result = Command::new(binary).output().unwrap();
        assert!(!result.status.success());
        assert!(!result.stderr.is_empty());
    }
}

#[test]
fn new_builtin_names_do_not_break_historical_canonical_evidence() {
    let root = tempdir().unwrap();
    let run = scalar_run(root.path(), "old.gbl", "1");
    let source = "g_func is_close(x) { return x }\nx = is_close(1)\nseal x\n";
    let parsed = parse_source(source).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    fs::write(
        run.join(receipt["source"]["path"].as_str().unwrap()),
        source,
    )
    .unwrap();
    receipt["source"]["sha256"] = json!(sha256_bytes(source.as_bytes()));
    receipt["canonical_source"]["sha256"] = json!(parsed.canonical_sha256().unwrap());
    for field in ["math_policy", "math_environment", "receipt_core_sha256"] {
        receipt.as_object_mut().unwrap().remove(field);
    }
    receipt["receipt_core_sha256"] = json!(hash_canonical_json(&receipt).unwrap());
    fs::write(
        run.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert!(verify_run(run).unwrap().verified);
    assert!(Evaluation::new(".").eval_program(&parsed.program).is_err());
}

#[test]
fn math_metadata_is_hashed_evidence() {
    for field in ["math_policy", "math_environment"] {
        let root = tempdir().unwrap();
        let run = scalar_run(root.path(), "same.gbl", "1");
        let mut receipt = read_receipt(&run).unwrap();
        receipt[field] = json!({"changed":true});
        fs::write(
            run.join("receipt.json"),
            serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();
        assert!(!verify_run(run).unwrap().verified);
    }
}

fn copy_tree(source: &Path, dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dest.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), dest.join(entry.file_name())).unwrap();
        }
    }
}

#[test]
fn shared_platform_math_fixtures() {
    let fixture_bytes = include_bytes!("fixtures/platform_math.json");
    let fixture: Value = serde_json::from_slice(fixture_bytes).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let mut source = "GO_PARANOID\n".to_string();
    for case in cases {
        source.push_str(&format!(
            "{} = {}\nseal {}\n",
            case["name"].as_str().unwrap(),
            case["expression"].as_str().unwrap(),
            case["name"].as_str().unwrap()
        ));
    }
    let root = tempdir().unwrap();
    let path = root.path().join("platform_math.gbl");
    fs::write(&path, &source).unwrap();
    let report_path = std::env::var_os("GOBLIN_MATH_REPORT_PATH").map(std::path::PathBuf::from);
    if let Some(path) = &report_path {
        assert!(!path.exists(), "Refusing to replace a math report");
        assert!(
            !path.parent().unwrap().join("evidence").exists(),
            "Refusing to replace math evidence"
        );
    }
    let mut engines = Vec::new();
    for compile in [false, true] {
        let run = run_file(
            &path,
            &RunOptions {
                compile,
                ..RunOptions::default()
            },
        )
        .unwrap();
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "PASS", "{:?}", receipt["failure"]);
        assert!(verify_run(&run).unwrap().verified);
        let mut results = Vec::new();
        for case in cases {
            let name = case["name"].as_str().unwrap();
            let artifact: Value =
                serde_json::from_slice(&fs::read(run.join(format!("{name}.json"))).unwrap())
                    .unwrap();
            let observed = artifact["value"]["value_si"].as_f64().unwrap();
            let expected = case["expected"].as_f64().unwrap();
            assert_eq!(artifact["value"]["dimension"], case["dimension"]);
            let identical = observed.to_bits() == expected.to_bits();
            let close = is_close(
                observed,
                expected,
                case["absolute"].as_f64().unwrap(),
                case["relative"].as_f64().unwrap(),
            )
            .unwrap();
            assert!(
                close && (case["exact"] != true || identical),
                "{} {}: {observed} expected {expected}",
                receipt["execution"]["engine"],
                name
            );
            results.push(json!({"name":name,"observed":observed,"bits_hex":format!("{:016x}",observed.to_bits()),"dimension":case["dimension"],"reference_bits_equal":identical,"within_fixture_tolerance":close}));
        }
        let label = if compile { "compiled" } else { "interpreted" };
        if let Some(report_path) = &report_path {
            copy_tree(
                &run,
                &report_path.parent().unwrap().join("evidence").join(label),
            );
        }
        engines.push(json!({"engine":receipt["execution"]["engine"],"run_dir":format!("evidence/{label}"),"receipt_core_sha256":receipt["receipt_core_sha256"],"math_environment":receipt["math_environment"],"results":results}));
    }
    if let Some(report_path) = report_path {
        fs::create_dir_all(report_path.parent().unwrap()).unwrap();
        let report = json!({"schema":"goblin.platform-math-report.v1","goblin_version":goblinpp::VERSION,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"math_policy":goblinpp::math_policy::policy(),"fixture_sha256":sha256_bytes(fixture_bytes),"source_sha256":sha256_bytes(source.as_bytes()),"engines":engines});
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(report_path)
            .unwrap();
        file.write_all(serde_json::to_string_pretty(&report).unwrap().as_bytes())
            .unwrap();
    }
}
