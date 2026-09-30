use goblinpp::audit::verify_run;
use goblinpp::chemistry::{CONCENTRATION, ELEMENTS, MASS_PER_AMOUNT, VOLUME};
use goblinpp::custody::create_freeze;
use goblinpp::evaluator::{Evaluation, Value};
use goblinpp::parser::parse_source;
use goblinpp::quantity::{AMOUNT, MASS};
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use std::fs;
use tempfile::tempdir;

const CHEMISTRY_PROGRAM: &str = r#"GO_PARANOID
registry = chem_registry_version()
carbon_number = chem_atomic_number("C")
oxygen_weight = chem_atomic_weight("O")
water_molar_mass = chem_molar_mass("H2O")
glucose_molar_mass = chem_molar_mass("C6H12O6")
calcium_hydroxide_molar_mass = chem_molar_mass("Ca(OH)2")

water_amount = chem_moles(36.03 g, water_molar_mass)
water_mass = chem_mass(water_amount, water_molar_mass)
stock = chem_concentration(0.1 mol, 100 mL)
diluted = chem_dilution(stock, 10 mL, 100 mL)

wavelength = 500 nm
sample_volume = 25 µL
line_wavelength = 5 Å
pressure = 1 atm
one_dalton = 1 Da
gas_energy = R * 300 K
atomic_mass = m_u

print("registry = {registry}")
print("H2O molar mass = {water_molar_mass}")
print("water amount = {water_amount}")
print("diluted concentration = {diluted}")
seal water_molar_mass
seal water_amount
seal stock
seal diluted
"#;

fn quantity<'a>(evaluation: &'a Evaluation, name: &str) -> &'a goblinpp::quantity::Quantity {
    match &evaluation.env[name] {
        Value::Quantity(value) => value,
        other => panic!("{name} is not a quantity: {other:?}"),
    }
}

#[test]
fn common_chemistry_helpers_are_dimension_checked() {
    let parsed = parse_source(CHEMISTRY_PROGRAM).unwrap();
    let evaluation = Evaluation::new(".").eval_program(&parsed.program).unwrap();

    assert_eq!(
        evaluation.env["registry"].render(),
        "IUPAC-2021-ABRIDGED-COMMON-v1"
    );
    assert_eq!(quantity(&evaluation, "carbon_number").value_si, 6.0);
    assert_eq!(quantity(&evaluation, "oxygen_weight").value_si, 15.999);
    assert_eq!(
        quantity(&evaluation, "water_molar_mass").dimension,
        MASS_PER_AMOUNT
    );
    assert!((quantity(&evaluation, "water_molar_mass").value_si - 0.018015).abs() < 1e-15);
    assert!((quantity(&evaluation, "glucose_molar_mass").value_si - 0.180156).abs() < 1e-15);
    assert!(
        (quantity(&evaluation, "calcium_hydroxide_molar_mass").value_si - 0.074092).abs() < 1e-15
    );
    assert_eq!(quantity(&evaluation, "water_amount").dimension, AMOUNT);
    assert!((quantity(&evaluation, "water_amount").value_si - 2.0).abs() < 1e-12);
    assert_eq!(quantity(&evaluation, "water_mass").dimension, MASS);
    assert!((quantity(&evaluation, "water_mass").value_si - 0.03603).abs() < 1e-15);
    assert_eq!(quantity(&evaluation, "stock").dimension, CONCENTRATION);
    assert!((quantity(&evaluation, "stock").value_si - 1_000.0).abs() < 1e-12);
    assert!((quantity(&evaluation, "diluted").value_si - 100.0).abs() < 1e-12);
    assert!((quantity(&evaluation, "wavelength").value_si - 5e-7).abs() < 1e-20);
    assert!((quantity(&evaluation, "sample_volume").value_si - 25e-9).abs() < 1e-21);
    assert!((quantity(&evaluation, "line_wavelength").value_si - 5e-10).abs() < 1e-23);
    assert_eq!(quantity(&evaluation, "pressure").value_si, 101_325.0);
    assert_eq!(quantity(&evaluation, "one_dalton").dimension, MASS);
    assert_eq!(
        quantity(&evaluation, "stock").dimension,
        [0, -3, 0, 0, 1, 0]
    );
    assert_eq!(VOLUME, [0, 3, 0, 0, 0, 0]);
}

#[test]
fn chemistry_refuses_unsupported_formulae_and_dimension_mistakes() {
    for source in [
        "x = chem_molar_mass(\"Na+\")\n",
        "x = chem_atomic_weight(\"carbon\")\n",
        "x = chem_moles(1 s, chem_molar_mass(\"H2O\"))\n",
        "x = chem_concentration(1 mol, 1 kg)\n",
        "c = chem_concentration(1 mol, 1 L)\nx = chem_dilution(c, 10 mL, 5 mL)\n",
    ] {
        let parsed = parse_source(source).unwrap();
        assert!(
            Evaluation::new(".").eval_program(&parsed.program).is_err(),
            "accepted {source:?}"
        );
    }
}

#[test]
fn interpreter_and_native_compiler_preserve_chemistry_results_and_registry() {
    let root = tempdir().unwrap();
    let source = root.path().join("chemistry.gbl");
    fs::write(&source, CHEMISTRY_PROGRAM).unwrap();

    let mut receipts = Vec::new();
    for compile in [false, true] {
        let run = run_file(
            &source,
            &RunOptions {
                compile,
                ..RunOptions::default()
            },
        )
        .unwrap();
        assert!(verify_run(&run).unwrap().verified);
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "PASS");
        assert_eq!(
            receipt["scientific_registries"]["chemistry"]["id"],
            "IUPAC-2021-ABRIDGED-COMMON-v1"
        );
        assert_eq!(
            receipt["scientific_registries"]["chemistry"]["sha256"]
                .as_str()
                .unwrap()
                .len(),
            64
        );
        receipts.push(receipt);
    }
    assert_eq!(
        receipts[0]["sealed_artifacts"],
        receipts[1]["sealed_artifacts"]
    );

    let (freeze_path, _) = create_freeze(&source).unwrap();
    let freeze: serde_json::Value =
        serde_json::from_slice(&fs::read(freeze_path).unwrap()).unwrap();
    assert_eq!(
        freeze["constant_registry"]["chemistry"]["id"],
        "IUPAC-2021-ABRIDGED-COMMON-v1"
    );
    assert_eq!(
        freeze["constant_registry"]["chemistry"]["element_count"],
        ELEMENTS.len()
    );
}

#[test]
fn every_registry_element_agrees_between_execution_engines() {
    let root = tempdir().unwrap();
    let source = root.path().join("all-elements.gbl");
    let mut program = String::new();
    for (index, element) in ELEMENTS.iter().enumerate() {
        program.push_str(&format!(
            "number_{index} = chem_atomic_number(\"{}\")\nweight_{index} = chem_atomic_weight(\"{}\")\nseal number_{index}\nseal weight_{index}\n",
            element.symbol, element.symbol
        ));
    }
    fs::write(&source, program).unwrap();

    let interpreted = run_file(&source, &RunOptions::default()).unwrap();
    let compiled = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    let interpreted_receipt = read_receipt(interpreted).unwrap();
    let compiled_receipt = read_receipt(compiled).unwrap();
    assert_eq!(
        interpreted_receipt["sealed_artifacts"],
        compiled_receipt["sealed_artifacts"]
    );
}
