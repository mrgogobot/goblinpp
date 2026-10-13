use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::evaluator::{Evaluation, Value};
use goblinpp::hashing::hash_canonical_json;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::{Value as Json, json};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

const EXAMPLE: &str = include_str!("../examples/elementwise_arrays.gbl");

fn evaluate(source: &str) -> Evaluation {
    Evaluation::new(".")
        .eval_program(&parse_source(source).unwrap().program)
        .unwrap()
}

#[test]
fn hand_calculated_dimensions_shape_scalar_order_and_independent_copies() {
    let result = evaluate(EXAMPLE);
    for (name, expected) in [
        ("lengths", "[3 m, 4 m, 5 m]"),
        ("speeds", "[1.5 m/s, 2 m/s, 2.5 m/s]"),
        ("products", "[6, 20, 42]"),
        ("inner", "68"),
        ("offset", "[4 m, 5 m, 6 m]"),
        (
            "reciprocal",
            "[0.3333333333333333, 0.2, 0.14285714285714285]",
        ),
        ("input_copy", "[9 m^2, 16 m^2, 25 m^2]"),
        ("changed", "[99 m, 4 m, 5 m]"),
    ] {
        assert_eq!(result.sealed[name].render(), expected, "{name}");
    }
    assert_eq!(result.resource_evidence()["used_loop_iterations"], 0);
}

#[test]
fn every_numeric_operator_and_multi_argument_math_maps_in_supplied_order() {
    let result = evaluate(
        "a = [2, 4]\nb = [1, 2]\nsum_values = a + b\nminus = b - a\nmultiply = a * b\ndivide = a / b\nmodulo = a % 3\npowers = a ^ b\nnegative = -a\npositive = +a\nleft_minus = 10 - a\nleft_divide = 8 / a\nleft_power = 2 ^ b\nminimum = min(a, b, 3)\nmaximum = max(a, b, 3)\ndiagonal = hypot([3 m, 5 m], [4 m, 12 m])\n",
    );
    for (name, expected) in [
        ("sum_values", "[3, 6]"),
        ("minus", "[-1, -2]"),
        ("multiply", "[2, 8]"),
        ("divide", "[2, 2]"),
        ("modulo", "[2, 1]"),
        ("powers", "[2, 16]"),
        ("negative", "[-2, -4]"),
        ("positive", "[2, 4]"),
        ("left_minus", "[8, 6]"),
        ("left_divide", "[4, 2]"),
        ("left_power", "[2, 4]"),
        ("minimum", "[1, 2]"),
        ("maximum", "[3, 4]"),
        ("diagonal", "[5 m, 13 m]"),
    ] {
        assert_eq!(result.env[name].render(), expected, "{name}");
    }
}

#[test]
fn unary_math_uses_identical_scalar_rules_and_bits() {
    for (name, input) in [
        ("abs", "-2"),
        ("sqrt", "9"),
        ("floor", "1.2"),
        ("ceil", "1.2"),
        ("round", "1.5"),
        ("exp", "1"),
        ("ln", "2"),
        ("log10", "100"),
        ("sin", "0.3"),
        ("cos", "0.3"),
        ("tan", "0.3"),
        ("asin", "0.3"),
        ("acos", "0.3"),
        ("atan", "0.3"),
        ("sind", "30"),
        ("cosd", "30"),
        ("tand", "30"),
        ("sinr", "0.3"),
        ("cosr", "0.3"),
        ("tanr", "0.3"),
        ("asind", "0.3"),
        ("acosd", "0.3"),
        ("atand", "0.3"),
        ("asinr", "0.3"),
        ("acosr", "0.3"),
        ("atanr", "0.3"),
        ("deg2rad", "30"),
        ("rad2deg", "0.3"),
    ] {
        let result = evaluate(&format!(
            "scalar_value = {name}({input})\narray_value = {name}([{input}, {input}])\n"
        ));
        let Value::Quantity(expected) = result.env["scalar_value"] else {
            panic!("scalar");
        };
        let Value::Array(items) = &result.env["array_value"] else {
            panic!("array");
        };
        for item in items {
            let Value::Quantity(actual) = item else {
                panic!("item");
            };
            assert_eq!(
                actual.value_si.to_bits(),
                expected.value_si.to_bits(),
                "{name}"
            );
            assert_eq!(actual.dimension, expected.dimension, "{name}");
        }
        assert!(result.warnings.len() <= 1);
    }
    for name in ["atan2", "atan2r", "atan2d"] {
        let result = evaluate(&format!(
            "scalar_value = {name}(1 m, 2 m)\narray_value = {name}([1 m, 1 m], 2 m)\n"
        ));
        assert_eq!(
            result.env["array_value"].render(),
            format!("[{0}, {0}]", result.env["scalar_value"].render())
        );
    }
}

#[test]
fn empty_arrays_are_empty_but_do_not_hide_arity_shape_or_nonnumeric_scalars() {
    let result = evaluate(
        "a = sqrt([])\nb = [] + []\nd = [] / 0\ne = ln([])\nf = min([], 2)\ng = [] ^ 0.5\n",
    );
    for name in ["a", "b", "d", "e", "f", "g"] {
        assert_eq!(result.env[name].render(), "[]");
    }
    for source in [
        "x = sqrt([], [])",
        "x = min([])",
        "x = [] + [1]",
        "x = [] + \"s\"",
        "x = [] * true",
        "x = min([], false)",
    ] {
        assert!(
            Evaluation::new(".")
                .eval_program(&parse_source(source).unwrap().program)
                .is_err(),
            "{source}"
        );
    }
}

#[test]
fn failures_report_first_zero_based_element_and_do_not_replace_destination() {
    for (source, index, diagnostic) in [
        ("result = sqrt([9, -1, -4])", 1, "SQUARE ROOT DOMAIN"),
        ("result = [2, 4, 6] / [1, 0, 0]", 1, "DIVISION BY ZERO"),
        ("result = ln([1, 0, -1])", 1, "LOGARITHM DOMAIN"),
        ("result = exp([1, 1000])", 1, "NON-FINITE"),
        ("result = asind([0, 2])", 1, "INVERSE TRIGONOMETRIC"),
        ("result = sqrt([1 m, 2 m])", 0, "SQUARE ROOT DIMENSION"),
        (
            "result = [1 m, 2 m] + [1 s, 2 s]",
            0,
            "INCOMPATIBLE DIMENSIONS",
        ),
        (
            "result = [1 m, 2 m] ^ [1, 2]",
            1,
            "different result dimension",
        ),
        (
            "result = [1, 2] ^ [1, 0.5]",
            1,
            "integer dimensionless exponent",
        ),
        ("result = [1, 2] % [1, 0]", 1, "Remainder by zero"),
        ("result = atan2d([1 m, 0 m], [1 m, 0 m])", 1, "ATAN2 DOMAIN"),
        ("result = [1 m^2] ^ 2147483647", 0, "Unit exponent overflow"),
        ("result = [1 m] ^ -2147483648", 0, "Unit exponent overflow"),
    ] {
        let mut evaluation = Evaluation::new(".");
        let parsed = parse_source(&format!("result = 42\n{source}\n")).unwrap();
        evaluation.eval_stmt(&parsed.program.statements[0]).unwrap();
        let error = evaluation
            .eval_stmt(&parsed.program.statements[1])
            .unwrap_err();
        assert!(
            error
                .message
                .contains(&format!("element index {index} (zero-based)")),
            "{source}: {error}"
        );
        assert!(error.message.contains(diagnostic), "{source}: {error}");
        assert_eq!(evaluation.env["result"].render(), "42");
    }
}

#[test]
fn shape_and_numeric_type_refusals_are_not_vector_concatenation_or_masks() {
    for (source, diagnostic) in [
        ("x = [1, 2] + [3]", "array length mismatch"),
        ("x = hypot([1, 2], [3])", "array length mismatch"),
        ("x = [\"x\"] + [\"y\"]", "element index 0"),
        ("x = abs([true, false])", "element index 0"),
        ("x = [1] * \"2\"", "argument 2 must"),
        ("x = [1 m, 2 s]", "Array literal element index 1"),
    ] {
        let error = Evaluation::new(".")
            .eval_program(&parse_source(source).unwrap().program)
            .unwrap_err();
        assert!(error.message.contains(diagnostic), "{source}: {error}");
    }
    for source in [
        "x = [1] < [2]",
        "x = is_close([1], [1], 0, 0)",
        "x = same_bits([1], [1])",
    ] {
        assert!(
            Evaluation::new(".")
                .eval_program(&parse_source(source).unwrap().program)
                .is_err()
        );
    }
    assert_eq!(evaluate("x = [1, 2] == [1, 2]").env["x"].render(), "true");
}

#[test]
fn array_limit_is_preserved_without_spending_user_loop_budget() {
    let mut result = Evaluation::new(".");
    let scalar = Value::Quantity(goblinpp::quantity::Quantity::scalar(2.).unwrap());
    result
        .env
        .insert("values".into(), Value::Array(vec![scalar; 100_000]));
    let parsed = parse_source("answer = values * 3\n").unwrap();
    result.eval_stmt(&parsed.program.statements[0]).unwrap();
    let Value::Array(items) = &result.env["answer"] else {
        panic!("array");
    };
    assert_eq!(items.len(), 100_000);
    assert_eq!(items[99_999].render(), "6");
    assert_eq!(result.resource_evidence()["used_loop_iterations"], 0);
    let extra = items[0].clone();
    let Value::Array(values) = result.env.get_mut("values").unwrap() else {
        panic!("array");
    };
    values.push(extra);
    assert!(
        result
            .eval_stmt(&parsed.program.statements[0])
            .unwrap_err()
            .message
            .contains("100000-item array limit")
    );
    assert_eq!(result.env["answer"].render().matches('6').count(), 100_000);
}

#[test]
fn interpreter_native_standalone_and_sealed_outputs_agree() {
    let root = tempdir().unwrap();
    let source = root.path().join("arrays.gbl");
    fs::write(&source, EXAMPLE).unwrap();
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
        assert_eq!(receipt["array_math_policy"], goblinpp::array_math::policy());
        assert!(verify_run(&run).unwrap().verified);
        if compile {
            let output = Command::new(run.join("program-native"))
                .current_dir(root.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, fs::read(run.join("stdout.log")).unwrap());
        }
        runs.push(run);
    }
    assert_eq!(
        diff_runs(&runs[0], &runs[1]).unwrap().classification,
        "IDENTICAL_RESULT"
    );
}

#[test]
fn data_bridge_arrays_exports_and_manifests_have_policy_and_equal_bytes() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("data.csv"), "area\n9\n16\n25\n").unwrap();
    let source = root.path().join("main.gbl");
    fs::write(&source, "GO_PARANOID\nareas = csv_numbers(\"data.csv\", \"area\") * (1 m)^2\nlengths = sqrt(areas)\nwrite_json(\"lengths.json\", \"lengths\", lengths)\nseal lengths\n").unwrap();
    let mut outputs = Vec::new();
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
        outputs.push(fs::read(run.join("outputs/lengths.json")).unwrap());
        if compile {
            let manifest: Json =
                serde_json::from_slice(&fs::read(run.join("native-data.json")).unwrap()).unwrap();
            assert_eq!(manifest["array_math_policy"], receipt["array_math_policy"]);
        }
    }
    assert_eq!(outputs[0], outputs[1]);
}

#[test]
fn native_failures_report_lane_not_panic_and_audited_failure_remains_verifiable() {
    let root = tempdir().unwrap();
    // Multiple branches exercise standalone generated code, not just the
    // interpreter reference (which intentionally refuses before audited compile).
    let source = "g_func test(which) {\nif which == 0 { return sqrt([1, -1]) }\nif which == 1 { return [1, 2] / [1, 0] }\nif which == 2 { return [1 m, 2 m] ^ [1, 2] }\nif which == 3 { return [1 m^2] ^ 2147483647 }\nif which == 4 { return [1, 2] + [3] }\nif which == 5 { return exp([1, 1000]) }\nreturn sqrt([1 m, 2 m])\n}\nanswer = test(parse_integer(argv(1)))\nprint(answer)\n";
    let parsed = parse_source(source).unwrap();
    let built = goblinpp::compiler::compile(&parsed, root.path().join("fail-native"), &[]).unwrap();
    for (argument, diagnostic) in [
        ("0", "element index 1"),
        ("1", "element index 1"),
        ("2", "element index 1"),
        ("3", "Unit exponent overflow"),
        ("4", "array length mismatch"),
        ("5", "element index 1"),
        ("6", "element index 0"),
    ] {
        let output = Command::new(&built.binary).arg(argument).output().unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(diagnostic), "{argument}: {stderr}");
        assert!(!stderr.contains("panicked"));
    }
    let file = root.path().join("failure.gbl");
    fs::write(
        &file,
        "GO_PARANOID\nprior = 7\nseal prior\nresult = sqrt([9, -1])\nseal result\n",
    )
    .unwrap();
    for compile in [false, true] {
        let run = run_file(
            &file,
            &RunOptions {
                compile,
                ..RunOptions::default()
            },
        )
        .unwrap();
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "MACHINERY_FAIL");
        assert_eq!(receipt["sealed_artifacts"].as_array().unwrap().len(), 1);
        assert!(verify_run(run).unwrap().verified);
    }
}

fn reseal(path: &std::path::Path, mut value: Json, field: &str) {
    value.as_object_mut().unwrap().remove(field);
    value[field] = json!(hash_canonical_json(&value).unwrap());
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

#[test]
fn independently_known_math_identities_hold_in_both_engines_and_standalone() {
    let root = tempdir().unwrap();
    // Exact small identities, not expected values generated by our engine.
    // One call exercises each mapped math function and every operator family.
    let text = r#"GO_PARANOID
roots = sqrt([9, 16])
rounded = floor([1.5, 2.5]) + ceil([1.5, 2.5]) + round([1.5, 2.5])
transforms = exp([0, 0]) + ln([1, 1]) + log10([100, 1000])
degrees = sind([0, 0]) + cosd([0, 0]) + tand([0, 0]) + asind([0, 0]) + acosd([1, 1]) + atand([0, 0])
radians = sinr([0, 0]) + cosr([0, 0]) + tanr([0, 0]) + asinr([0, 0]) + acosr([1, 1]) + atanr([0, 0])
legacy = sin([0, 0]) + cos([0, 0]) + tan([0, 0]) + asin([0, 0]) + acos([1, 1]) + atan([0, 0])
conversions = deg2rad([0, 0]) + rad2deg([0, 0])
directions = atan2d([1, 0], [1, 1])
other_directions = atan2([0, 0], 1) + atan2r([0, 0], 1)
diagonals = hypot([3, 5], [4, 12])
extrema = min([2, 4], [1, 8], 3) + max([2, 4], [1, 8], 3)
operators = +[2, 4] + (-[1, 2]) + ([1, 2] * [3, 4]) + (8 / [2, 4]) + ([2, 4] % 3) + ([2, 4] ^ [1, 2])
absolute = abs([-2, -3])
print(roots)
print(rounded)
print(transforms)
print(degrees)
print(radians)
print(legacy)
print(conversions)
print(directions)
print(other_directions)
print(diagonals)
print(extrema)
print(operators)
print(absolute)
seal roots
"#;
    let expected = "[3, 4]\n[5, 8]\n[3, 4]\n[1, 1]\n[1, 1]\n[1, 1]\n[0, 0]\n[45, 0]\n[0, 0]\n[5, 13]\n[4, 11]\n[12, 29]\n[2, 3]\n";
    let file = root.path().join("math.gbl");
    fs::write(&file, text).unwrap();
    for compile in [false, true] {
        let run = run_file(
            &file,
            &RunOptions {
                compile,
                ..RunOptions::default()
            },
        )
        .unwrap();
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "PASS", "{receipt}");
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            expected
        );
        assert!(verify_run(&run).unwrap().verified);
        if compile {
            let output = Command::new(run.join("program-native"))
                .current_dir(root.path())
                .output()
                .unwrap();
            assert!(output.status.success());
            assert_eq!(output.stdout, expected.as_bytes());
        }
    }
}

#[test]
fn library_arrays_and_scalar_arguments_are_evaluated_once_and_compile_preview_has_policy() {
    let root = tempdir().unwrap();
    let file = root.path().join("main.gbl");
    fs::write(root.path().join("library.gbl"), "g_func roots(values) { return sqrt(values) }\ng_func scalar() { print(\"called once\")\nreturn 2 }\n").unwrap();
    fs::write(
        &file,
        "import \"library.gbl\"\nvalues = roots([9, 16]) * scalar()\nprint(values)\nseal values\n",
    )
    .unwrap();
    let preview = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["check", file.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(preview.status.success());
    let preview: Json = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(preview["array_math_policy"], goblinpp::array_math::policy());
    assert_eq!(preview["evidence_created"], false);
    for compile in [false, true] {
        let run = run_file(
            &file,
            &RunOptions {
                compile,
                ..RunOptions::default()
            },
        )
        .unwrap();
        assert_eq!(read_receipt(&run).unwrap()["status"], "PASS");
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "called once\n[6, 8]\n"
        );
        assert!(verify_run(run).unwrap().verified);
    }
    let compilation = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args([
            "compile",
            file.to_str().unwrap(),
            "-o",
            root.path().join("standalone").to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let report: Json = serde_json::from_slice(&compilation.stdout).unwrap();
    assert_eq!(report["array_math_policy"], goblinpp::array_math::policy());
    assert!(std::path::Path::new(report["generated_source"].as_str().unwrap()).is_file());
}

#[test]
fn policy_cannot_be_removed_or_changed_even_if_receipt_is_rehashed() {
    let root = tempdir().unwrap();
    let source = root.path().join("main.gbl");
    fs::write(&source, "x = sqrt([9, 16])\nseal x").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let original = read_receipt(&run).unwrap();
    for change in [
        None,
        Some(json!({"id":"goblin.elementwise-numeric-arrays.v999"})),
    ] {
        let mut value = original.clone();
        if let Some(change) = change {
            value["array_math_policy"] = change;
        } else {
            value.as_object_mut().unwrap().remove("array_math_policy");
        }
        reseal(&run.join("receipt.json"), value, "receipt_core_sha256");
        let verification = verify_run(&run).unwrap();
        assert!(!verification.verified);
        assert!(
            verification
                .checks
                .iter()
                .any(|c| c.check == "ARRAY_MATH_POLICY_SUPPORTED" && !c.pass)
        );
    }
    reseal(
        &run.join("receipt.json"),
        original.clone(),
        "receipt_core_sha256",
    );
    assert!(verify_run(&run).unwrap().verified);
    // Synthetic legacy representation is not substituted for the real-package
    // compatibility gate. Its absence is accepted only on historical versions.
    let mut legacy = original;
    legacy["goblin_version"] = json!("0.1.0-alpha.32");
    legacy.as_object_mut().unwrap().remove("array_math_policy");
    reseal(&run.join("receipt.json"), legacy, "receipt_core_sha256");
    assert!(verify_run(&run).unwrap().verified);
    let current = run_file(&source, &RunOptions::default()).unwrap();
    let diff = diff_runs(&run, &current).unwrap();
    assert!(!diff.array_math_policy_same);
    assert_eq!(diff.classification, "ARRAY_MATH_POLICY_CHANGE");
}

#[test]
fn frozen_policy_and_historical_freeze_migration_are_explicit() {
    let root = tempdir().unwrap();
    let source = root.path().join("main.gbl");
    fs::write(&source, "x = sqrt([9, 16])\nseal x").unwrap();
    let (_, event) = goblinpp::custody::create_freeze(&source).unwrap();
    let freeze = goblinpp::custody::freeze_path(&source);
    let original: Json = serde_json::from_slice(&fs::read(&freeze).unwrap()).unwrap();
    assert_eq!(
        original["array_math_policy"],
        goblinpp::array_math::policy()
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
        assert_eq!(read_receipt(&run).unwrap()["status"], "PASS");
        assert!(verify_run(&run).unwrap().verified);
    }
    let mut missing = original.clone();
    missing.as_object_mut().unwrap().remove("array_math_policy");
    reseal(&freeze, missing, "freeze_receipt_sha256");
    assert!(!goblinpp::custody::verify_freeze(&source).verified);
    let mut old = original;
    old["goblin_version"] = json!("0.1.0-alpha.32");
    old.as_object_mut().unwrap().remove("array_math_policy");
    reseal(&freeze, old, "freeze_receipt_sha256");
    let registered: Json = serde_json::from_slice(&fs::read(&freeze).unwrap()).unwrap();
    let mut payload = event.payload;
    payload["freeze_receipt_core_sha256"] = registered["freeze_receipt_sha256"].clone();
    payload["freeze_receipt_file_sha256"] = json!(goblinpp::hashing::sha256_file(&freeze).unwrap());
    goblinpp::ledger::append(root.path(), "FREEZE", event.subject, payload).unwrap();
    assert!(goblinpp::custody::verify_freeze(&source).verified);
    let run = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&run).unwrap()["status"], "PROTOCOL_VIOLATION");
    assert_eq!(
        read_receipt(&run).unwrap()["protocol_violation"]["classification"],
        "ARRAY_MATH_POLICY_NOT_FROZEN"
    );
    assert!(verify_run(&run).unwrap().verified);
    let compile = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args([
            "compile",
            source.to_str().unwrap(),
            "-o",
            root.path().join("blocked").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!compile.status.success());
    assert!(!root.path().join("blocked").exists());
    let child = root.path().join("child.gbl");
    goblinpp::custody::create_revision(&source, &child, "Explicit array policy migration").unwrap();
    goblinpp::custody::create_freeze(&child).unwrap();
    let run = run_file(&child, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&run).unwrap()["status"], "PASS");
    assert!(verify_run(&run).unwrap().verified);
}
