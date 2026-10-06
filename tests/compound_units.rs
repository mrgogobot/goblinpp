use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::custody::{create_freeze, create_revision, verify_freeze};
use goblinpp::evaluator::{Evaluation, Value};
use goblinpp::hashing::{hash_canonical_json, sha256_bytes};
use goblinpp::parser::{PARSER_POLICY, SyntaxMode, parse_source, parse_source_with_mode};
use goblinpp::quantity::{Quantity, format_dimension};
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::json;
use std::{fs, process::Command};
use tempfile::tempdir;

const PROGRAM: &str = r#"GO_PARANOID
acceleration = 1.13e-10 m/s^2
unicode = 1.13e-10 m/s²
area = 3 m^2
whole_square = (3 m)^2
negative = -3 m^2
scaled = 3 km^2
inverse = 4 s^-2
pressure = 7 kg/(m*s^2)
single_denominator = 5 m/(s)
energy = 3 kg*m^2/s^2
speed = 3 km/s
explicit = 1.13e-10 m / (1 s)^2
arithmetic = 2 m * 3 / 4
unchanged_group = 2(1 + 2)
unit_spelled_variable = 4
m = 4
ordinary_variable_group = 2(m^2)
g_func acceleration_value() { return 2 m/s^2 }
function_value = acceleration_value()
measurements = [1 m/s^2, 3 m/s^2]
middle = median(measurements)
signed_zero = -0 m/s^2
print("acceleration = {acceleration}; area = {area}; whole square = {whole_square}")
print("pressure = {pressure}; speed = {speed}")
seal acceleration
seal unicode
seal area
seal whole_square
seal negative
seal scaled
seal inverse
seal pressure
seal single_denominator
seal energy
seal speed
seal explicit
seal arithmetic
seal unchanged_group
seal function_value
seal middle
seal signed_zero
"#;

#[test]
fn compound_literals_keep_coefficient_separate_from_unit_powers() {
    let parsed = parse_source(PROGRAM).unwrap();
    let result = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    for (name, number, dimension) in [
        ("acceleration", 1.13e-10, [0, 1, -2, 0, 0, 0]),
        ("area", 3.0, [0, 2, 0, 0, 0, 0]),
        ("whole_square", 9.0, [0, 2, 0, 0, 0, 0]),
        ("negative", -3.0, [0, 2, 0, 0, 0, 0]),
        ("scaled", 3e6, [0, 2, 0, 0, 0, 0]),
        ("inverse", 4.0, [0, 0, -2, 0, 0, 0]),
        ("pressure", 7.0, [1, -1, -2, 0, 0, 0]),
        ("single_denominator", 5.0, [0, 1, -1, 0, 0, 0]),
        ("energy", 3.0, [1, 2, -2, 0, 0, 0]),
        ("speed", 3000.0, [0, 1, -1, 0, 0, 0]),
        ("arithmetic", 1.5, [0, 1, 0, 0, 0, 0]),
        ("unchanged_group", 6.0, [0; 6]),
        ("ordinary_variable_group", 32.0, [0; 6]),
        ("function_value", 2.0, [0, 1, -2, 0, 0, 0]),
        ("middle", 2.0, [0, 1, -2, 0, 0, 0]),
    ] {
        assert_eq!(
            result.env[name],
            Value::Quantity(Quantity::new(number, dimension).unwrap()),
            "{name}"
        );
    }
    assert_eq!(result.env["unicode"], result.env["explicit"]);
    assert_eq!(result.env["signed_zero"].render(), "-0 m/s^2");
    assert_eq!(result.env["pressure"].render(), "7 kg/(m*s^2)");
}

#[test]
fn printed_si_units_round_trip_in_all_six_axes() {
    // Every combination of small SI exponents, including inverse-only,
    // dimensionless, named derived units and several denominator factors.
    for encoded in 0..5_usize.pow(6) {
        let mut remaining = encoded;
        let dimension = std::array::from_fn(|_| {
            let v = (remaining % 5) as i32 - 2;
            remaining /= 5;
            v
        });
        let expected = Quantity::new(2.5, dimension).unwrap();
        let source = format!("q = {}\n", expected.render());
        let parsed = parse_source(&source).unwrap_or_else(|e| panic!("{source}: {e}"));
        let result = Evaluation::new(".").eval_program(&parsed.program).unwrap();
        assert_eq!(
            result.env["q"],
            Value::Quantity(expected),
            "{}",
            format_dimension(dimension)
        );
    }
}

#[test]
fn powers_and_whitespace_have_specified_canonical_meaning() {
    for (a, b) in [
        ("x = 3 m²\n", "x = 3 m^2\n"),
        ("x = 3m / s^2\n", "x = 3 m/s²\n"),
        ("x = 3 kg/(m*s^2)\n", "x = 3 kg / ( m * s ^ +2 )\n"),
    ] {
        assert_eq!(
            parse_source(a).unwrap().canonical_sha256().unwrap(),
            parse_source(b).unwrap().canonical_sha256().unwrap()
        );
    }
    let now = parse_source("x = 3 m^2\n").unwrap();
    let legacy = parse_source_with_mode("x = 3 m^2\n", SyntaxMode::LegacyQuantity).unwrap();
    assert_ne!(
        now.canonical_sha256().unwrap(),
        legacy.canonical_sha256().unwrap()
    );
    assert_eq!(
        legacy.canonical_sha256().unwrap(),
        parse_source("x = (3 m)^2\n")
            .unwrap()
            .canonical_sha256()
            .unwrap()
    );
    for text in [
        "x = 3 m\n",
        "x = 2(1+2)\n",
        "x = 3^2\n",
        "mass = 1 kg\nenergy = mass*c^2\n",
    ] {
        assert_eq!(
            parse_source(text).unwrap().canonical_sha256().unwrap(),
            parse_source_with_mode(text, SyntaxMode::LegacyQuantity)
                .unwrap()
                .canonical_sha256()
                .unwrap()
        );
    }
}

#[test]
fn invalid_ambiguous_and_extreme_unit_suffixes_refuse() {
    let overlong = format!("x = 3 {}\n", vec!["m^0"; 100].join("*"));
    assert!(parse_source(&overlong).is_err());
    let too_deep = format!("x = 3 kg*{}m{}\n", "(".repeat(33), ")".repeat(33));
    assert!(parse_source(&too_deep).is_err());
    for text in [
        "x = 3 m^0.5\n",
        "x = 3 m^2.0\n",
        "x = 3 m^1e2\n",
        "x = 3 m^power\n",
        "x = 3 m^2^3\n",
        "x = 3 m^33\n",
        "x = 3 s^-33\n",
        "x = 3 m/s*kg\n",
        "x = 3 m/s/kg\n",
        "x = 3 kg/(m*unknown)\n",
        "x = 3 m^\n",
        "x = 3 m/()\n",
        "x = 3 kg*(m^32)^32\n",
        "x = 3 pF^32\n",
        "x = 3 kg*(km^32)^8\n",
    ] {
        assert!(parse_source(text).is_err(), "{text}");
    }
    // Ordinary undefined symbols and incompatible dimensions still refuse.
    for text in ["x = 3 m/unknown\n", "x = 1 m/s^2 + 1 s\n", "x = m/s^2\n"] {
        let parsed = parse_source(text).unwrap();
        assert!(
            Evaluation::new(".").eval_program(&parsed.program).is_err(),
            "{text}"
        );
    }
}

#[test]
fn both_engines_and_standalone_native_keep_exact_evidence() {
    let root = tempdir().unwrap();
    let source = root.path().join("units.gbl");
    fs::write(&source, PROGRAM).unwrap();
    let mut runs = vec![];
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
        assert_eq!(receipt["parser_policy"], PARSER_POLICY);
        assert!(verify_run(&run).unwrap().verified);
        runs.push(run);
    }
    let diff = diff_runs(&runs[0], &runs[1]).unwrap();
    assert!(diff.sealed_artifacts_same && diff.stdout_same && diff.parser_policy_same);
    let native = goblinpp::compiler::compile(
        &parse_source(PROGRAM).unwrap(),
        root.path().join("standalone"),
        &[],
    )
    .unwrap();
    let output = Command::new(native.binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("pressure = 7 kg/(m*s^2); speed = 3000 m/s")
    );
}

#[test]
fn historical_quantity_power_run_verifies_with_legacy_parser() {
    // Coherent synthetic historical evidence: execute the explicit old meaning,
    // then preserve its old spelling and old canonical AST. Outputs stay intact.
    let root = tempdir().unwrap();
    let source = root.path().join("old.gbl");
    fs::write(&source, "x = (3 m)^2\nprint(\"{x}\")\nseal x\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let old_text = "x = 3 m^2\nprint(\"{x}\")\nseal x\n";
    fs::write(run.join("old.gbl"), old_text).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    receipt["goblin_version"] = json!("0.1.0-alpha.24");
    receipt.as_object_mut().unwrap().remove("parser_policy");
    receipt["source"]["sha256"] = json!(sha256_bytes(old_text.as_bytes()));
    receipt["canonical_source"]["sha256"] = json!(
        parse_source_with_mode(old_text, SyntaxMode::LegacyQuantity)
            .unwrap()
            .canonical_sha256()
            .unwrap()
    );
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
    assert!(verify_run(&run).unwrap().verified);
    assert_eq!(
        fs::read_to_string(run.join("stdout.log")).unwrap(),
        "9 m^2\n"
    );
}

#[test]
fn legacy_freeze_verifies_but_execution_requires_explicit_revision() {
    let root = tempdir().unwrap();
    let source = root.path().join("old.gbl");
    let text = "GO_PARANOID\nx = 3 m^2\nseal x\n";
    fs::write(&source, text).unwrap();
    let (path, event) = create_freeze(&source).unwrap();
    let mut frozen: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    frozen["goblin_version"] = json!("0.1.0-alpha.24");
    frozen.as_object_mut().unwrap().remove("parser_policy");
    frozen["canonical_source"]["sha256"] = json!(
        parse_source_with_mode(text, SyntaxMode::LegacyQuantity)
            .unwrap()
            .canonical_sha256()
            .unwrap()
    );
    frozen
        .as_object_mut()
        .unwrap()
        .remove("freeze_receipt_sha256");
    frozen["freeze_receipt_sha256"] = json!(hash_canonical_json(&frozen).unwrap());
    let bytes = serde_json::to_vec_pretty(&frozen).unwrap();
    fs::write(&path, &bytes).unwrap();
    let mut payload = event.payload;
    payload["canonical_source_sha256"] = frozen["canonical_source"]["sha256"].clone();
    payload["freeze_receipt_core_sha256"] = frozen["freeze_receipt_sha256"].clone();
    payload["freeze_receipt_file_sha256"] = json!(sha256_bytes(&bytes));
    goblinpp::ledger::append(root.path(), "FREEZE", event.subject, payload).unwrap();
    assert!(verify_freeze(&source).verified);
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .arg("status")
        .arg(&source)
        .arg("--json")
        .output()
        .unwrap();
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status["state"], "FROZEN_PARSER_MIGRATION_REQUIRED");
    let compiled = std::process::Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .arg("compile")
        .arg(&source)
        .arg("--output")
        .arg(root.path().join("old-native"))
        .output()
        .unwrap();
    assert!(!compiled.status.success());
    assert!(
        String::from_utf8_lossy(&compiled.stderr).contains("PARSER_POLICY_CHANGED_AFTER_FREEZE")
    );
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
        assert!(verify_run(&run).unwrap().verified);
    }
    let child = root.path().join("new.gbl");
    create_revision(
        &source,
        &child,
        "reviewed coefficient versus whole-quantity power",
    )
    .unwrap();
    create_freeze(&child).unwrap();
    let run = run_file(&child, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&run).unwrap()["status"], "PASS");
}

#[test]
fn parser_policy_tampering_and_missing_current_policy_fail_verification() {
    for remove in [false, true] {
        let root = tempdir().unwrap();
        let source = root.path().join("current.gbl");
        fs::write(&source, "x = 3 m^2\nseal x\n").unwrap();
        let run = run_file(&source, &RunOptions::default()).unwrap();
        let mut receipt = read_receipt(&run).unwrap();
        if remove {
            receipt.as_object_mut().unwrap().remove("parser_policy");
        } else {
            receipt["parser_policy"] = json!("UNKNOWN_POLICY");
        }
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
        assert!(
            !report.verified
                && report
                    .checks
                    .iter()
                    .any(|c| c.check == "PARSER_POLICY_SUPPORTED" && !c.pass)
        );
    }
}

#[test]
fn imported_compound_units_use_the_same_policy_in_both_engines() {
    let root = tempdir().unwrap();
    fs::create_dir(root.path().join("lib")).unwrap();
    fs::write(
        root.path().join("lib/units.gbl"),
        "g_func get_area() { return 3 m^2 }\n",
    )
    .unwrap();
    let source = root.path().join("main.gbl");
    fs::write(
        &source,
        "import \"lib/units.gbl\"\nx = get_area()\nprint(\"{x}\")\nseal x\n",
    )
    .unwrap();
    for compile in [false, true] {
        let run = run_file(
            &source,
            &RunOptions {
                compile,
                ..RunOptions::default()
            },
        )
        .unwrap();
        assert_eq!(read_receipt(&run).unwrap()["status"], "PASS");
        assert!(verify_run(&run).unwrap().verified);
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "3 m^2\n"
        );
    }
}
