use goblinpp::audit::verify_run;
use goblinpp::evaluator::{Evaluation, Value};
use goblinpp::parser::parse_source;
use goblinpp::quantity::Quantity;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use std::fs;
use tempfile::tempdir;

const SCIENCE_PROGRAM: &str = r#"GO_PARANOID
one_au = au2m(1)
au_roundtrip = m2au(one_au)
one_parsec = pc2m(1)
parsec_roundtrip = m2pc(one_parsec)
one_light_year = ly2m(1)
light_year_roundtrip = m2ly(one_light_year)

length = magnitude([3 m, 4 m])
projection = dot([1 m, 2 m, 3 m], [4 kg, 5 kg, 6 kg])
normal = cross([1 m, 0 m, 0 m], [0 kg, 1 kg, 0 kg])

polar = polar2cartesiand(2 m, 90)
polar_radians = polar2cartesianr(2 m, pi / 2)
radius = cartesian_radius([3 m, 4 m])
azimuth_degrees = cartesian_azimuthd([1 m, 1 m])
azimuth_radians = cartesian_azimuthr([1 m, 1 m])
spherical = spherical2cartesiand(2 m, 90, 0)
spherical_radians = spherical2cartesianr(2 m, pi / 2, 0)
inclination_degrees = cartesian_inclinationd([1 m, 0 m, 0 m])
inclination_radians = cartesian_inclinationr([1 m, 0 m, 0 m])

speed = velocity(10 m, 2 s)
vector_velocity = velocity([10 m, 4 m, 0 m], 2 s)
galilean = velocity_add_galilean(velocity(10 m, 2 s), velocity(6 m, 2 s))
galilean_vector = velocity_add_galilean(vector_velocity, velocity([2 m, 2 m, 2 m], 2 s))
half_c = c / 2
relativistic = velocity_add_relativistic_collinear(half_c, half_c)

omega = angular_velocityd(360, 2 s)
omega_radians = angular_velocityr(2 * pi, 2 s)
tangential = tangential_velocity(2 m, omega)
centripetal = centripetal_acceleration(2 m, omega)
zero_velocity = velocity(0 m, 1 s)
three_velocity = velocity(3 m, 1 s)
zero_momentum = 2 kg * zero_velocity
six_momentum = 2 kg * three_velocity
orbital_l = angular_momentum([1 m, 0 m, 0 m], [zero_momentum, six_momentum, zero_momentum])
orbital_l_velocity = angular_momentum_from_velocity([1 m, 0 m, 0 m], 2 kg, [zero_velocity, three_velocity, zero_velocity])

print("AU = {one_au}; speed = {speed}; omega = {omega}")
seal one_au
seal projection
seal normal
seal spherical
seal relativistic
seal orbital_l
"#;

fn quantity<'a>(evaluation: &'a Evaluation, name: &str) -> &'a Quantity {
    match &evaluation.env[name] {
        Value::Quantity(value) => value,
        other => panic!("{name} is not a quantity: {other:?}"),
    }
}

fn array<'a>(evaluation: &'a Evaluation, name: &str) -> Vec<&'a Quantity> {
    match &evaluation.env[name] {
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Quantity(value) => value,
                other => panic!("{name} contains a non-quantity: {other:?}"),
            })
            .collect(),
        other => panic!("{name} is not an array: {other:?}"),
    }
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual}; expected={expected}; tolerance={tolerance}"
    );
}

#[test]
fn conversions_vectors_coordinates_and_motion_are_dimensionally_explicit() {
    let parsed = parse_source(SCIENCE_PROGRAM).unwrap();
    let evaluation = Evaluation::new(".").eval_program(&parsed.program).unwrap();

    assert_eq!(quantity(&evaluation, "one_au").render(), "149597870700 m");
    close(quantity(&evaluation, "au_roundtrip").value_si, 1.0, 1e-15);
    close(
        quantity(&evaluation, "one_parsec").value_si,
        30_856_775_814_913_672.0,
        4.0,
    );
    close(
        quantity(&evaluation, "parsec_roundtrip").value_si,
        1.0,
        1e-15,
    );
    assert_eq!(
        quantity(&evaluation, "one_light_year").value_si,
        9_460_730_472_580_800.0
    );
    close(
        quantity(&evaluation, "light_year_roundtrip").value_si,
        1.0,
        1e-15,
    );
    assert_eq!(quantity(&evaluation, "length").render(), "5 m");
    assert_eq!(quantity(&evaluation, "projection").render(), "32 kg*m");
    let normal = array(&evaluation, "normal");
    assert_eq!(normal[0].render(), "0 kg*m");
    assert_eq!(normal[1].render(), "0 kg*m");
    assert_eq!(normal[2].render(), "1 kg*m");

    for name in ["polar", "polar_radians"] {
        let vector = array(&evaluation, name);
        close(vector[0].value_si, 0.0, 1e-12);
        close(vector[1].value_si, 2.0, 1e-12);
    }
    assert_eq!(quantity(&evaluation, "radius").render(), "5 m");
    close(
        quantity(&evaluation, "azimuth_degrees").value_si,
        45.0,
        1e-12,
    );
    close(
        quantity(&evaluation, "azimuth_radians").value_si,
        std::f64::consts::FRAC_PI_4,
        1e-15,
    );
    for name in ["spherical", "spherical_radians"] {
        let vector = array(&evaluation, name);
        close(vector[0].value_si, 2.0, 1e-12);
        close(vector[1].value_si, 0.0, 1e-12);
        close(vector[2].value_si, 0.0, 1e-12);
    }
    close(
        quantity(&evaluation, "inclination_degrees").value_si,
        90.0,
        1e-12,
    );
    close(
        quantity(&evaluation, "inclination_radians").value_si,
        std::f64::consts::FRAC_PI_2,
        1e-15,
    );

    assert_eq!(quantity(&evaluation, "speed").render(), "5 m/s");
    assert_eq!(quantity(&evaluation, "galilean").render(), "8 m/s");
    let vector_velocity = array(&evaluation, "vector_velocity");
    assert_eq!(vector_velocity[0].render(), "5 m/s");
    assert_eq!(vector_velocity[1].render(), "2 m/s");
    let galilean_vector = array(&evaluation, "galilean_vector");
    assert_eq!(galilean_vector[0].render(), "6 m/s");
    assert_eq!(galilean_vector[1].render(), "3 m/s");
    assert_eq!(galilean_vector[2].render(), "1 m/s");
    close(
        quantity(&evaluation, "relativistic").value_si,
        0.8 * 299_792_458.0,
        1e-7,
    );

    close(
        quantity(&evaluation, "omega").value_si,
        std::f64::consts::PI,
        1e-15,
    );
    close(
        quantity(&evaluation, "omega_radians").value_si,
        std::f64::consts::PI,
        1e-15,
    );
    close(
        quantity(&evaluation, "tangential").value_si,
        2.0 * std::f64::consts::PI,
        1e-14,
    );
    close(
        quantity(&evaluation, "centripetal").value_si,
        2.0 * std::f64::consts::PI.powi(2),
        1e-13,
    );
    for name in ["orbital_l", "orbital_l_velocity"] {
        let angular_momentum = array(&evaluation, name);
        assert_eq!(angular_momentum[0].render(), "0 kg*m^2/s");
        assert_eq!(angular_momentum[1].render(), "0 kg*m^2/s");
        assert_eq!(angular_momentum[2].render(), "6 kg*m^2/s");
    }
}

#[test]
fn science_helpers_run_verify_and_compile_with_matching_outputs() {
    let root = tempdir().unwrap();
    let source = root.path().join("science.gbl");
    fs::write(&source, SCIENCE_PROGRAM).unwrap();
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
        assert_eq!(
            receipt["status"], "PASS",
            "compile={compile}; failure={:?}",
            receipt["failure"]
        );
        assert!(verify_run(&run).unwrap().verified);
        outputs.push(fs::read_to_string(run.join("stdout.log")).unwrap());
    }
    assert_eq!(outputs[0], outputs[1]);
}

#[test]
fn unsafe_or_ambiguous_science_inputs_are_refused() {
    for (source, code) in [
        ("x = au2m(1 m)\n", "G201"),
        ("x = m2au(1)\n", "G201"),
        ("x = magnitude([])\n", "G203"),
        ("x = dot([1 m], [1 m, 2 m])\n", "G203"),
        ("x = cross([1 m, 2 m], [3 m, 4 m])\n", "G203"),
        ("x = polar2cartesiand(-1 m, 0)\n", "G202"),
        ("x = cartesian_azimuthd([0 m, 0 m])\n", "G202"),
        ("x = cartesian_inclinationd([0 m, 0 m, 0 m])\n", "G202"),
        ("x = spherical2cartesiand(1 m, 181, 0)\n", "G202"),
        ("x = velocity(1 m, 0 s)\n", "G202"),
        ("x = velocity(1 kg, 1 s)\n", "G201"),
        (
            "x = velocity_add_galilean(1 m / 1 s, [1 m / 1 s])\n",
            "G203",
        ),
        (
            "x = velocity_add_relativistic_collinear(c * 2, 0 m / 1 s)\n",
            "G202",
        ),
        ("x = angular_velocityd(90, 1 m)\n", "G201"),
        (
            "omega = angular_velocityr(1, 1 s)\nx = tangential_velocity(-1 m, omega)\n",
            "G202",
        ),
        (
            "zero = velocity(0 m, 1 s)\none = velocity(1 m, 1 s)\nx = angular_momentum([1 m, 0 m, 0 m], [zero, one, zero])\n",
            "G201",
        ),
        ("x = dot([1 m])\n", "G002"),
    ] {
        let parsed = parse_source(source).unwrap();
        let error = Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_err();
        assert_eq!(error.code, code, "source={source:?}; {error:?}");
    }
}

#[test]
fn science_failure_runs_are_preserved_and_verifiable_in_both_modes() {
    let root = tempdir().unwrap();
    let source = root.path().join("bad_science.gbl");
    fs::write(
        &source,
        "GO_PARANOID\nx = velocity_add_relativistic_collinear(c * 2, 0 m / 1 s)\nseal x\n",
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
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "MACHINERY_FAIL");
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn science_builtin_names_cannot_be_shadowed() {
    for name in [
        "au2m",
        "dot",
        "cross",
        "spherical2cartesiand",
        "velocity",
        "velocity_add_galilean",
        "velocity_add_relativistic_collinear",
        "angular_velocityd",
        "angular_momentum",
    ] {
        assert!(
            parse_source(&format!("g_func {name}(x) {{ return x }}\n")).is_err(),
            "accepted reserved builtin name {name}"
        );
    }
}
