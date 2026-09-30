use goblinpp::audit::verify_run;
use goblinpp::electrical::REGISTRY_ID;
use goblinpp::evaluator::{Evaluation, Value};
use goblinpp::parser::parse_source;
use goblinpp::quantity::{CURRENT, ENERGY, POWER, RESISTANCE, TIME, VOLTAGE};
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use std::fs;
use tempfile::tempdir;

const ELECTRICAL_PROGRAM: &str = r#"GO_PARANOID
source_voltage = 12 V
load_resistance = 4.7 kohm
load_current = ee_current(source_voltage, load_resistance)
recovered_voltage = ee_voltage(load_current, load_resistance)
load_power = ee_power(source_voltage, load_current)
recovered_resistance = ee_resistance(source_voltage, load_current)
power_i2r = ee_power_i2r(load_current, load_resistance)
power_v2r = ee_power_v2r(source_voltage, load_resistance)
delivered_power = ee_power(12 V, -2 mA)
delivered_energy = ee_energy(delivered_power, 10 s)
series = ee_series_resistance([1 kohm, 2.2 kohm, 470 ohm])
parallel = ee_parallel_resistance([1 kohm, 1 kohm])
tau = ee_rc_time_constant(10 kohm, 100 uF)
capacitor_energy = ee_capacitor_energy(100 uF, 12 V)
inductor_energy = ee_inductor_energy(10 mH, 2 A)
charge = ee_charge(2 A, 3 s)
current_samples = [1 mA, 2 mA, 3 mA]
kcl = ee_kcl_residual([3 mA, -2 mA, -1 mA])
kvl = ee_kvl_residual([12 V, -7 V, -5 V])
kcl_ok = ee_kcl_balanced([3 mA, -2.001 mA, -1 mA], 0.01 mA)
kvl_ok = ee_kvl_balanced([12 V, -7 V, -5.001 V], 0.01 V)
load_kohm = ee_in_unit(load_resistance, "kΩ")

print("load current = {load_current}")
print("series = {series}; parallel = {parallel}")
print("tau = {tau}; KCL = {kcl}; KVL = {kvl}")
seal load_current
seal recovered_voltage
seal load_power
seal recovered_resistance
seal power_i2r
seal power_v2r
seal delivered_power
seal delivered_energy
seal series
seal parallel
seal tau
seal capacitor_energy
seal inductor_energy
seal charge
seal current_samples
seal kcl
seal kvl
seal kcl_ok
seal kvl_ok
seal load_kohm
"#;

fn quantity<'a>(evaluation: &'a Evaluation, name: &str) -> &'a goblinpp::quantity::Quantity {
    match &evaluation.env[name] {
        Value::Quantity(value) => value,
        other => panic!("{name} is not a quantity: {other:?}"),
    }
}

#[test]
fn electrical_helpers_enforce_si_dimensions_and_expected_values() {
    let parsed = parse_source(ELECTRICAL_PROGRAM).unwrap();
    let evaluation = Evaluation::new(".").eval_program(&parsed.program).unwrap();

    assert_eq!(quantity(&evaluation, "load_current").dimension, CURRENT);
    assert!((quantity(&evaluation, "load_current").value_si - 12.0 / 4_700.0).abs() < 1e-15);
    assert_eq!(
        quantity(&evaluation, "recovered_voltage").dimension,
        VOLTAGE
    );
    assert!((quantity(&evaluation, "recovered_voltage").value_si - 12.0).abs() < 1e-12);
    assert_eq!(quantity(&evaluation, "load_power").dimension, POWER);
    assert_eq!(quantity(&evaluation, "series").dimension, RESISTANCE);
    assert!((quantity(&evaluation, "series").value_si - 3_670.0).abs() < 1e-12);
    assert!((quantity(&evaluation, "parallel").value_si - 500.0).abs() < 1e-12);
    assert_eq!(quantity(&evaluation, "tau").dimension, TIME);
    assert!((quantity(&evaluation, "tau").value_si - 1.0).abs() < 1e-12);
    assert_eq!(quantity(&evaluation, "capacitor_energy").dimension, ENERGY);
    assert!((quantity(&evaluation, "capacitor_energy").value_si - 0.0072).abs() < 1e-15);
    assert!((quantity(&evaluation, "inductor_energy").value_si - 0.02).abs() < 1e-15);
    assert_eq!(quantity(&evaluation, "load_kohm").value_si, 4.7);
    assert_eq!(evaluation.env["kcl_ok"], Value::Bool(true));
    assert_eq!(evaluation.env["kvl_ok"], Value::Bool(true));
    assert!((quantity(&evaluation, "delivered_energy").value_si + 0.24).abs() < 1e-15);
}

#[test]
fn every_electrical_unit_and_small_parallel_value_agrees_across_engines() {
    let root = tempdir().unwrap();
    let source = root.path().join("units.gbl");
    let mut program =
        String::from("tiny = ee_parallel_resistance([1e-310 ohm, 1e-310 ohm])\nseal tiny\n");
    for (index, unit) in goblinpp::quantity::UNITS.iter().enumerate().skip(21) {
        program.push_str(&format!(
            "q_{index} = 2 {}\nu_{index} = ee_in_unit(q_{index}, \"{}\")\nseal q_{index}\nseal u_{index}\n",
            unit.name, unit.name
        ));
    }
    fs::write(&source, &program).unwrap();
    let parsed = parse_source(&program).unwrap();
    let evaluation = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(quantity(&evaluation, "tiny").value_si, 5e-311);
    for (index, _) in goblinpp::quantity::UNITS.iter().enumerate().skip(21) {
        assert_eq!(quantity(&evaluation, &format!("u_{index}")).value_si, 2.0);
    }
    let interpreted = run_file(&source, &RunOptions::default()).unwrap();
    let compiled = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    let left = read_receipt(&interpreted).unwrap();
    let right = read_receipt(&compiled).unwrap();
    assert_eq!(left["status"], "PASS");
    assert_eq!(right["status"], "PASS", "{right}");
    assert_eq!(left["sealed_artifacts"], right["sealed_artifacts"]);
    assert!(verify_run(interpreted).unwrap().verified);
    assert!(verify_run(compiled).unwrap().verified);
}

#[test]
fn electrical_refusals_are_preserved_and_verifiable_in_both_modes() {
    let cases = [
        "x = ee_current(5 V, 0 ohm)\n",
        "x = ee_resistance(1 V, -1 A)\n",
        "x = ee_voltage(1 kg, 2 ohm)\n",
        "x = ee_series_resistance([-1 ohm])\n",
        "x = ee_parallel_resistance([])\n",
        "x = ee_rc_time_constant(1 ohm, -1 F)\n",
        "x = ee_capacitor_energy(-1 F, 1 V)\n",
        "x = ee_inductor_energy(-1 H, 1 A)\n",
        "x = ee_energy(1 W, -1 s)\n",
        "x = ee_charge(1 A, -1 s)\n",
        "x = ee_kcl_balanced([1 A], -1 A)\n",
        "x = ee_kvl_balanced([1 V], 1 A)\n",
        "x = ee_in_unit(1 V, \"mA\")\n",
        "x = ee_in_unit(1 V, \"unknown\")\n",
        "x = ee_power(1 V)\n",
        "x = ee_kcl_residual([1e308 A, 1e308 A, -1e308 A])\n",
        "x = ee_series_resistance([1e308 ohm, 1e308 ohm])\n",
    ];
    for (index, program) in cases.iter().enumerate() {
        let root = tempdir().unwrap();
        let source = root.path().join(format!("refusal-{index}.gbl"));
        fs::write(&source, format!("GO_PARANOID\n{program}")).unwrap();
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
            assert!(
                verify_run(run).unwrap().verified,
                "unverifiable refusal {program}"
            );
        }
    }
}

#[test]
fn electrical_builtins_cannot_be_shadowed_and_angular_display_is_preserved() {
    for name in goblinpp::electrical::FUNCTIONS {
        assert!(parse_source(&format!("g_func {name}() {{ return 1 }}\n")).is_err());
    }
    let parsed = parse_source("omega = angular_velocityr(1, 1 s)\n").unwrap();
    let evaluation = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(evaluation.env["omega"].render(), "1 1/s");
}

#[test]
fn electrical_helpers_refuse_dimensional_and_domain_nonsense() {
    for source in [
        "x = ee_current(5 V, 0 ohm)\n",
        "x = ee_voltage(1 kg, 2 ohm)\n",
        "x = ee_parallel_resistance([1 kohm, 0 ohm])\n",
        "x = ee_kcl_residual([1 A, 2 V])\n",
        "x = ee_in_unit(1 V, \"kΩ\")\n",
        "x = ee_in_unit(1 kg, \"kg\")\n",
    ] {
        let parsed = parse_source(source).unwrap();
        assert!(
            Evaluation::new(".").eval_program(&parsed.program).is_err(),
            "accepted {source:?}"
        );
    }
}

#[test]
fn interpreter_and_native_compiler_preserve_electrical_results() {
    let root = tempdir().unwrap();
    let source = root.path().join("electrical.gbl");
    fs::write(&source, ELECTRICAL_PROGRAM).unwrap();

    let interpreted = run_file(&source, &RunOptions::default()).unwrap();
    let compiled = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    for run in [&interpreted, &compiled] {
        assert!(verify_run(run).unwrap().verified);
    }
    let interpreted_receipt = read_receipt(interpreted).unwrap();
    let compiled_receipt = read_receipt(compiled).unwrap();
    assert_eq!(interpreted_receipt["status"], "PASS");
    assert_eq!(compiled_receipt["status"], "PASS", "{compiled_receipt}");
    assert_eq!(
        interpreted_receipt["scientific_registries"]["electrical"]["id"],
        REGISTRY_ID
    );
    assert_eq!(
        interpreted_receipt["sealed_artifacts"],
        compiled_receipt["sealed_artifacts"]
    );
}
