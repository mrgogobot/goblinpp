use goblinpp::audit::verify_run;
use goblinpp::custody::create_freeze;
use goblinpp::evaluator::{Evaluation, Value};
use goblinpp::parser::parse_source;
use goblinpp::quantity::{CURRENT, DIMENSIONLESS, LENGTH, MASS, Quantity};
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

const PROGRAM: &str = r#"
measurements = [10 m, 11 m, 12 m]
total = sum(measurements)
average = mean(measurements)
mixed_units = mean([1 km, 500 m])
mass_total = sum([1 kg, 500 g])
current_average = mean([1 mA, 2 mA, 3 mA])
negative = mean([-6, -3, 0])
zero = mean([0 m, 0 m])
singleton = sum([-2 kg])
singleton_mean = mean([-2 kg])
cancelled = sum([1e16, 1, -1e16])
cancelled_mean = mean([1e16, 1, -1e16])
large_mean = mean([1e308, 1e308])
tiny_mean = mean([1e-310, 1e-310])
g_func average_of_copy(values) { return mean(values) }
function_mean = average_of_copy(measurements[1:3])
sum = 7
mean = 8
print("total = {total}; average = {average}")
seal total
seal average
seal mixed_units
seal mass_total
seal current_average
seal negative
seal zero
seal singleton
seal singleton_mean
seal cancelled
seal cancelled_mean
seal large_mean
seal tiny_mean
seal function_mean
seal measurements
seal sum
seal mean
"#;

fn quantity<'a>(evaluation: &'a Evaluation, name: &str) -> &'a Quantity {
    match &evaluation.env[name] {
        Value::Quantity(value) => value,
        other => panic!("{name} is not a quantity: {other:?}"),
    }
}

#[test]
fn sums_and_means_preserve_dimensions_and_known_results() {
    let parsed = parse_source(PROGRAM).unwrap();
    let evaluation = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    for (name, expected, dimension) in [
        ("total", 33.0, LENGTH),
        ("average", 11.0, LENGTH),
        ("mixed_units", 750.0, LENGTH),
        ("mass_total", 1.5, MASS),
        ("current_average", 0.002, CURRENT),
        ("negative", -3.0, DIMENSIONLESS),
        ("zero", 0.0, LENGTH),
        ("singleton", -2.0, MASS),
        ("singleton_mean", -2.0, MASS),
        ("cancelled", 1.0, DIMENSIONLESS),
        ("large_mean", 1e308, DIMENSIONLESS),
        ("tiny_mean", 1e-310, DIMENSIONLESS),
        ("function_mean", 11.5, LENGTH),
    ] {
        assert_eq!(quantity(&evaluation, name).value_si, expected, "{name}");
        assert_eq!(quantity(&evaluation, name).dimension, dimension, "{name}");
    }
    assert!((quantity(&evaluation, "cancelled_mean").value_si - 1.0 / 3.0).abs() < 1e-15);
    assert_eq!(
        evaluation.env["measurements"].render(),
        "[10 m, 11 m, 12 m]"
    );
}

#[test]
fn interpreter_and_compiler_agree_with_and_without_paranoid_mode() {
    let root = tempdir().unwrap();
    let source = root.path().join("statistics.gbl");
    for paranoid in [false, true] {
        fs::write(
            &source,
            format!("{}{PROGRAM}", if paranoid { "GO_PARANOID\n" } else { "" }),
        )
        .unwrap();
        let mut receipts = Vec::new();
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
            outputs.push(fs::read_to_string(run.join("stdout.log")).unwrap());
            receipts.push(receipt);
        }
        assert_eq!(
            receipts[0]["sealed_artifacts"],
            receipts[1]["sealed_artifacts"]
        );
        assert_eq!(outputs[0], outputs[1]);
    }
}

#[test]
fn unsafe_statistics_are_refused_and_failure_evidence_verifies() {
    for (index, program) in [
        "x = sum([])",
        "x = mean([])",
        "x = sum(3)",
        "x = mean([true, false])",
        "x = sum([\"a\", \"b\"])",
        "x = mean([1 m, 2 s])",
        "x = sum([1e308, 1e308])",
        "x = sum([1e308, 1e308, -1e308])",
        "x = mean()",
        "x = sum([1], [2])",
    ]
    .iter()
    .enumerate()
    {
        let root = tempdir().unwrap();
        let source = root.path().join(format!("refusal-{index}.gbl"));
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
                "accepted {program}"
            );
            assert!(verify_run(&run).unwrap().verified, "unverifiable {program}");
            assert!(
                !fs::read_to_string(run.join("stderr.log"))
                    .unwrap()
                    .is_empty()
            );
        }
    }
}

#[test]
fn builtins_reserve_function_and_parameter_names() {
    for name in ["sum", "mean"] {
        assert!(parse_source(&format!("g_func {name}() {{ return 1 }}\n")).is_err());
        assert!(parse_source(&format!("g_func f({name}) {{ return 1 }}\n")).is_err());
    }
}

#[test]
fn frozen_statistics_source_is_enforced_in_both_modes() {
    let root = tempdir().unwrap();
    let source = root.path().join("frozen.gbl");
    let original = "GO_PARANOID\nx = mean([1 m, 3 m])\nseal x\n";
    fs::write(&source, original).unwrap();
    create_freeze(&source).unwrap();
    for compile in [false, true] {
        let options = RunOptions {
            compile,
            ..RunOptions::default()
        };
        let run = run_file(&source, &options).unwrap();
        assert_eq!(read_receipt(&run).unwrap()["status"], "PASS");
        assert!(verify_run(run).unwrap().verified);
    }
    fs::write(&source, format!("{original}# notation-only edit\n")).unwrap();
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
            "NOTATION_ONLY_CHANGE_AFTER_FREEZE"
        );
        assert!(verify_run(run).unwrap().verified);
    }
}

#[test]
fn capabilities_advertise_only_implemented_statistics() {
    let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["capabilities", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    for name in ["sum", "mean"] {
        assert!(
            report["science_functions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item == name)
        );
    }
    assert!(
        !report["science_functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "median")
    );
}
