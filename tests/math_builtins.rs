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
sine_degrees = sind(30)
cosine_degrees = cosd(60)
tangent_degrees = tand(45)
sine_radians = sinr(pi / 2)
cosine_radians = cosr(0)
tangent_radians = tanr(pi / 4)
arcsine_degrees = asind(0.5)
arccosine_degrees = acosd(0.5)
arctangent_degrees = atand(1)
arcsine_radians = asinr(1)
arccosine_radians = acosr(1)
arctangent_radians = atanr(1)
direction_degrees = atan2d(1 m, 1 m)
direction_radians = atan2r(1 m, 1 m)
converted_radians = deg2rad(180)
converted_degrees = rad2deg(pi)
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
seal sine_degrees
seal cosine_degrees
seal tangent_degrees
seal sine_radians
seal cosine_radians
seal tangent_radians
seal arcsine_degrees
seal arccosine_degrees
seal arctangent_degrees
seal arcsine_radians
seal arccosine_radians
seal arctangent_radians
seal direction_degrees
seal direction_radians
seal converted_radians
seal converted_degrees
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
    assert!((quantity(&evaluation, "sine_degrees").value_si - 0.5).abs() < 1e-15);
    assert!((quantity(&evaluation, "cosine_degrees").value_si - 0.5).abs() < 1e-15);
    assert!((quantity(&evaluation, "tangent_degrees").value_si - 1.0).abs() < 1e-15);
    assert!((quantity(&evaluation, "sine_radians").value_si - 1.0).abs() < 1e-15);
    assert!(
        (quantity(&evaluation, "arcsine_radians").value_si - std::f64::consts::FRAC_PI_2).abs()
            < 1e-15
    );
    assert_eq!(quantity(&evaluation, "arccosine_radians").render(), "0");
    assert!((quantity(&evaluation, "arcsine_degrees").value_si - 30.0).abs() < 1e-12);
    assert!((quantity(&evaluation, "arccosine_degrees").value_si - 60.0).abs() < 1e-12);
    assert!((quantity(&evaluation, "arctangent_degrees").value_si - 45.0).abs() < 1e-12);
    assert!((quantity(&evaluation, "direction_degrees").value_si - 45.0).abs() < 1e-12);
    assert!(
        (quantity(&evaluation, "direction_radians").value_si - std::f64::consts::FRAC_PI_4).abs()
            < 1e-15
    );
    assert!(
        (quantity(&evaluation, "converted_radians").value_si - std::f64::consts::PI).abs() < 1e-15
    );
    assert!((quantity(&evaluation, "converted_degrees").value_si - 180.0).abs() < 1e-12);
    assert!(evaluation.warnings.is_empty());
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
        ("x = sind(1 m)\n", "G201"),
        ("x = floor(1 m)\n", "G201"),
        ("x = asind(2)\n", "G202"),
        ("x = atan2d(0, 0)\n", "G202"),
        ("x = atan2r(1 m, 1 s)\n", "G201"),
        ("x = deg2rad(1 m)\n", "G201"),
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
    for name in [
        "sqrt", "sin", "sind", "sinr", "asind", "asinr", "atan2d", "atan2r", "deg2rad", "rad2deg",
        "min", "hypot",
    ] {
        assert!(
            parse_source(&format!("g_func {name}(x) {{ return x }}\n")).is_err(),
            "accepted reserved builtin name {name}"
        );
    }
}

#[test]
fn legacy_radian_names_remain_compatible_and_warn_once_per_name() {
    let parsed =
        parse_source("a = sin(pi / 2)\nb = sin(0)\nangle = asin(1)\nd = atan2(1 m, 1 m)\n")
            .unwrap();
    let evaluation = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert!((quantity(&evaluation, "a").value_si - 1.0).abs() < 1e-15);
    assert_eq!(evaluation.warnings.len(), 3);
    assert!(evaluation.warnings[0].contains("sinr()"));
    assert!(evaluation.warnings[0].contains("sind()"));

    let root = tempdir().unwrap();
    let source = root.path().join("legacy_angles.gbl");
    fs::write(
        &source,
        "GO_PARANOID\na = sin(pi / 2)\nb = sin(0)\nseal a\n",
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
        let stderr = fs::read_to_string(run.join("stderr.log")).unwrap();
        assert_eq!(stderr.matches("GOBLIN WARNING G302").count(), 1);
        assert!(stderr.contains("sinr()"));
        assert!(stderr.contains("sind()"));
        assert!(verify_run(&run).unwrap().verified);
    }
}
