use goblinpp::audit::verify_run;
use goblinpp::custody::create_freeze;
use goblinpp::evaluator::{Evaluation, Value};
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use std::fs;
use tempfile::tempdir;

const SCIENTIFIC_MATH: &str = r#"GO_PARANOID
root = sqrt(81)
length = sqrt((3 m)^2)
magnitude = abs(-4 kg)
low = min(3 m, 1 m, 2 m)
high = max(3 m, 1 m, 2 m)
diagonal = hypot(3 m, 4 m)
down = floor(2.9)
up = ceil(2.1)
nearest = round(-2.5)
exponential = exp(1)
natural = ln(exponential)
decades = log10(1000)
sine = sin(pi / 2)
cosine = cos(0)
tangent = tan(pi / 4)
arcsine = asin(1)
arccosine = acos(1)
arctangent = atan(1)
direction = atan2(1 m, 1 m)
print("root={root}; length={length}; diagonal={diagonal}; natural={natural}; decades={decades}")
seal root
seal length
seal magnitude
seal low
seal high
seal diagonal
seal down
seal up
seal nearest
seal exponential
seal natural
seal decades
seal sine
seal cosine
seal tangent
seal arcsine
seal arccosine
seal arctangent
seal direction
"#;

fn quantity<'a>(evaluation: &'a Evaluation, name: &str) -> &'a goblinpp::quantity::Quantity {
    match &evaluation.env[name] {
        Value::Quantity(value) => value,
        other => panic!("{name} is not a quantity: {other:?}"),
    }
}

#[test]
fn scientific_math_values_and_dimensions_are_explicit() {
    let parsed = parse_source(SCIENTIFIC_MATH).unwrap();
    let evaluation = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(quantity(&evaluation, "root").render(), "9");
    assert_eq!(quantity(&evaluation, "length").render(), "3 m");
    assert_eq!(quantity(&evaluation, "magnitude").render(), "4 kg");
    assert_eq!(quantity(&evaluation, "low").render(), "1 m");
    assert_eq!(quantity(&evaluation, "high").render(), "3 m");
    assert_eq!(quantity(&evaluation, "diagonal").render(), "5 m");
    assert_eq!(quantity(&evaluation, "down").render(), "2");
    assert_eq!(quantity(&evaluation, "up").render(), "3");
    assert_eq!(quantity(&evaluation, "nearest").render(), "-3");
    assert!((quantity(&evaluation, "natural").value_si - 1.0).abs() < 1e-14);
    assert_eq!(quantity(&evaluation, "decades").render(), "3");
    assert!((quantity(&evaluation, "sine").value_si - 1.0).abs() < 1e-15);
    assert!(
        (quantity(&evaluation, "arcsine").value_si - std::f64::consts::FRAC_PI_2).abs() < 1e-15
    );
    assert_eq!(quantity(&evaluation, "arccosine").render(), "0");
}

#[test]
fn scientific_math_interpreter_and_compiler_agree_and_verify() {
    let root = tempdir().unwrap();
    let source = root.path().join("scientific_math.gbl");
    fs::write(&source, SCIENTIFIC_MATH).unwrap();
    create_freeze(&source).unwrap();
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
            receipt["status"], "PASS",
            "compile={compile}; failure={:?}",
            receipt["failure"]
        );
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn invalid_math_domains_and_dimensions_are_refused() {
    for (source, code) in [
        ("x = sqrt(-1)\n", "G202"),
        ("x = sqrt(2 m)\n", "G201"),
        ("x = ln(0)\n", "G202"),
        ("x = log10(-10)\n", "G202"),
        ("x = exp(1000)\n", "G202"),
        ("x = sin(1 m)\n", "G201"),
        ("x = floor(1 m)\n", "G201"),
        ("x = asin(2)\n", "G202"),
        ("x = atan2(0, 0)\n", "G202"),
        ("x = atan2(1 m, 1 s)\n", "G201"),
        ("x = hypot(1 m, 1 s)\n", "G201"),
        ("x = min(1 m, 1 s)\n", "G201"),
        ("x = sqrt()\n", "G002"),
        ("x = min(1)\n", "G002"),
    ] {
        let parsed = parse_source(source).unwrap();
        let error = Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_err();
        assert_eq!(error.code, code, "source={source:?}; {error:?}");
    }
}

#[test]
fn math_failure_runs_are_preserved_and_verifiable_in_both_modes() {
    let root = tempdir().unwrap();
    let source = root.path().join("bad_math.gbl");
    fs::write(&source, "GO_PARANOID\nx = sqrt(-1)\nseal x\n").unwrap();
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
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn builtin_math_names_cannot_be_shadowed_by_g_func() {
    for name in ["sqrt", "sin", "min", "hypot"] {
        assert!(
            parse_source(&format!("g_func {name}(x) {{ return x }}\n")).is_err(),
            "accepted reserved builtin name {name}"
        );
    }
}
