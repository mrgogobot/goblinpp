use goblinpp::audit::verify_run;
use goblinpp::compiler::compile;
use goblinpp::custody::create_freeze;
use goblinpp::evaluator::Evaluation;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use std::fs;
use tempfile::tempdir;

const BRANCHES: &str = "GO_PARANOID\nscore = 9\nlabel = \"unset\"\nif score < 5 {\n label = \"low\"\n} else if score >= 9 {\n label = \"high\"\n} else {\n label = \"middle\"\n}\nresult = 0\nswitch label {\n case \"low\" { result = 1 }\n case \"high\" { result = 2 }\n default { result = 3 }\n}\nprint(\"label = {label}; result = {result}\")\nseal label\nseal result\n";

#[test]
fn interpreted_and_compiled_branches_agree_and_verify() {
    let root = tempdir().unwrap();
    let source = root.path().join("branches.gbl");
    fs::write(&source, BRANCHES).unwrap();
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
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "label = high; result = 2\n"
        );
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn branches_are_lazy_and_switch_does_not_fall_through() {
    let program = "x = 0\nif true { x = 1 } else if missing == 1 { x = 2 } else { x = 3 }\nswitch 2 {\n case 1 { x = 10 }\n case 2 { x = x + 1 }\n case missing { x = 20 }\n default { x = 30 }\n}\nseal x\n";
    let parsed = parse_source(program).unwrap();
    let result = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(result.sealed["x"].render(), "2");
}

#[test]
fn default_and_nested_loop_branches_work() {
    let program = "sum = 0\nfor i in range(3) {\n if i == 1 {\n  switch i {\n   case 0 { sum = 100 }\n   default { sum = sum + 5 }\n  }\n } else {\n  sum = sum + 1\n }\n}\nseal sum\n";
    let parsed = parse_source(program).unwrap();
    let result = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(result.sealed["sum"].render(), "7");
    let root = tempdir().unwrap();
    let source = root.path().join("nested.gbl");
    fs::write(&source, program).unwrap();
    let run = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    assert_eq!(read_receipt(&run).unwrap()["status"], "PASS");
    assert!(verify_run(&run).unwrap().verified);
}

#[test]
fn boolean_conditions_and_dimension_aware_cases_fail_explicitly() {
    for (source, code) in [
        ("if 1 { x = 1 }\n", "G203"),
        ("if false { x = 1 } else if 2 { x = 2 }\n", "G203"),
        ("switch 1 kg { case 1 s { x = 1 } }\n", "G201"),
        ("switch 1 { case \"1\" { x = 1 } }\n", "G203"),
    ] {
        let parsed = parse_source(source).unwrap();
        let error = Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_err();
        assert_eq!(error.code, code, "{source}");
    }
}

#[test]
fn invalid_branch_syntax_is_rejected() {
    for source in [
        "if true { x = 1\n",
        "if true { x = 1 } else x = 2\n",
        "switch 1 { default { x = 1 } case 1 { x = 2 } }\n",
        "switch 1 { default { x = 1 } default { x = 2 } }\n",
        "switch 1 { }\n",
        "switch 1 { case 1 x = 2 }\n",
        "if true { GO_PARANOID }\n",
    ] {
        assert!(parse_source(source).is_err(), "accepted: {source}");
    }
}

#[test]
fn branch_failure_and_frozen_edits_are_preserved_and_verifiable() {
    let root = tempdir().unwrap();
    let source = root.path().join("branches.gbl");
    fs::write(&source, "GO_PARANOID\nif 1 { print(\"no\") }\n").unwrap();
    let failed = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&failed).unwrap()["status"], "MACHINERY_FAIL");
    assert!(verify_run(&failed).unwrap().verified);
    fs::write(&source, BRANCHES).unwrap();
    create_freeze(&source).unwrap();
    fs::write(&source, BRANCHES.replace("score = 9", "score = 8")).unwrap();
    let refused = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(
        read_receipt(&refused).unwrap()["status"],
        "PROTOCOL_VIOLATION"
    );
    assert!(verify_run(&refused).unwrap().verified);
}

#[test]
fn compiled_mode_refuses_data_calls_even_in_unreachable_branches() {
    let root = tempdir().unwrap();
    let parsed = parse_source("if false { x = fits_count(\"sample.fits\", 0) }\n").unwrap();
    let error = compile(&parsed, root.path().join("program"), &[]).unwrap_err();
    assert_eq!(error.code, "G501");
    assert!(error.message.contains("Native code generation for FITS"));
}
