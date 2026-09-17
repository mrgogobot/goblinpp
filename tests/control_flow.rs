use goblinpp::audit::verify_run;
use goblinpp::custody::create_freeze;
use goblinpp::evaluator::Evaluation;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use std::fs;
use tempfile::tempdir;

const LOOPS: &str = "GO_PARANOID\nsum = 0\nfor i in range(1, 6) {\n    sum = sum + i\n}\nn = 3\nwhile n > 0 {\n    n = n - 1\n}\ndone = n == 0\nprint(\"sum = {sum}; done = {done}\")\nseal sum\nseal done\n";

#[test]
fn interpreted_and_compiled_loops_agree_and_verify() {
    let root = tempdir().unwrap();
    let source = root.path().join("loops.gbl");
    fs::write(&source, LOOPS).unwrap();
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
        assert_eq!(read_receipt(run).unwrap()["status"], "PASS");
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "sum = 15; done = true\n"
        );
        assert!(verify_run(run).unwrap().verified);
    }
    assert_eq!(
        read_receipt(&compiled).unwrap()["execution"]["engine"],
        "rust-native-compiled"
    );
}

#[test]
fn compiled_seal_inside_loop_captures_the_value_at_seal_time() {
    let root = tempdir().unwrap();
    let source = root.path().join("seals.gbl");
    fs::write(
        &source,
        "x = 0\nfor i in range(3) {\n x = i\n seal x\n}\nx = 99\n",
    )
    .unwrap();
    let run = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PASS");
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn interpreted_loop_can_process_real_fits_pixels_and_preserve_input_evidence() {
    let root = tempdir().unwrap();
    fs::write(
        root.path().join("sample.fits"),
        include_bytes!("../examples/sample.fits"),
    )
    .unwrap();
    let source = root.path().join("pixel_sum.gbl");
    fs::write(&source, "GO_PARANOID\nsum = 0\nfor i in range(4) {\n sum = sum + fits_pixel(\"sample.fits\", 0, i)\n}\nprint(\"pixel sum = {sum}\")\nseal sum\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PASS");
    assert_eq!(
        fs::read_to_string(run.join("stdout.log")).unwrap(),
        "pixel sum = 10\n"
    );
    assert_eq!(receipt["data_imports"].as_array().unwrap().len(), 1);
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn negative_steps_nested_loops_and_empty_ranges_work() {
    let source = "sum = 0\nfor i in range(5, 0, -2) {\n for j in range(2) {\n  sum = sum + i\n }\n}\nfor unused in range(0) {\n sum = 999\n}\nwhile false {\n sum = 999\n}\nseal sum\n";
    let parsed = parse_source(source).unwrap();
    let result = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(result.sealed["sum"].render(), "18");
    assert!(!result.env.contains_key("unused"));
}

#[test]
fn comparison_requires_matching_dimensions_and_boolean_while_condition() {
    for (source, code) in [
        ("x = 1 kg < 2 s\n", "G201"),
        ("while 1 {\n x = 2\n}\n", "G203"),
        ("for i in range(0, 5, 0) {\n x = i\n}\n", "G202"),
        ("for i in range(0, 2 kg) {\n x = i\n}\n", "G203"),
    ] {
        let parsed = parse_source(source).unwrap();
        let error = Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_err();
        assert_eq!(error.code, code, "unexpected error for {source:?}");
    }
}

#[test]
fn infinite_while_is_bounded_and_preserved_as_a_verifiable_failure() {
    let root = tempdir().unwrap();
    let source = root.path().join("infinite.gbl");
    fs::write(&source, "GO_PARANOID\nwhile true {\n x = 1\n}\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["failure"]["code"], "G203");
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn frozen_loop_edit_is_refused_before_execution() {
    let root = tempdir().unwrap();
    let source = root.path().join("loops.gbl");
    fs::write(&source, LOOPS).unwrap();
    create_freeze(&source).unwrap();
    fs::write(&source, LOOPS.replace("range(1, 6)", "range(1, 7)")).unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&run).unwrap()["status"], "PROTOCOL_VIOLATION");
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn syntax_errors_are_explicit() {
    for source in [
        "for i in range(3) {\n x = i\n",
        "for i in range(1, 2, 3, 4) { x = i }\n",
        "while true { x = 1 }}\n",
        "for i range(3) { x = i }\n",
    ] {
        assert!(
            parse_source(source).is_err(),
            "invalid syntax accepted: {source:?}"
        );
    }
}
