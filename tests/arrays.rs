use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::evaluator::Evaluation;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use std::fs;
use tempfile::tempdir;

const ARRAYS: &str = "GO_PARANOID\nvalues = [1 kg, 2 kg, 3 kg, 4 kg]\npart = values[1:3]\npart[0] = 20 kg\ncopy = values\ncopy[0] = 99 kg\nmore = append(values, 5 kg)\nsum = 0 kg\nfor i in range(len(values)) {\n sum = sum + values[i]\n}\nprint(\"values = {values}\")\nprint(\"part = {part}; copy = {copy}\")\nprint(\"more = {more}; sum = {sum}\")\nseal values\nseal part\nseal more\nseal sum\n";

#[test]
fn arrays_copy_slice_index_append_and_loop_agree_across_engines() {
    let root = tempdir().unwrap();
    let source = root.path().join("arrays.gbl");
    fs::write(&source, ARRAYS).unwrap();
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
        assert_eq!(receipt["status"], "PASS", "{receipt:?}");
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "values = [1 kg, 2 kg, 3 kg, 4 kg]\npart = [20 kg, 3 kg]; copy = [99 kg, 2 kg, 3 kg, 4 kg]\nmore = [1 kg, 2 kg, 3 kg, 4 kg, 5 kg]; sum = 10 kg\n"
        );
        assert!(verify_run(&run).unwrap().verified);
        assert_eq!(receipt["sealed_artifacts"].as_array().unwrap().len(), 4);
        runs.push(run);
    }
    assert_eq!(
        diff_runs(&runs[0], &runs[1]).unwrap().classification,
        "IDENTICAL_RESULT"
    );
}

#[test]
fn empty_open_and_chained_slices_are_independent() {
    let parsed = parse_source("a = []\na = append(a, \"red\")\na = append(a, \"blue\")\nb = a[:]\nb[0] = \"green\"\nc = a[1:][0]\nseal a\nseal b\nseal c\n").unwrap();
    let result = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(result.sealed["a"].render(), "[red, blue]");
    assert_eq!(result.sealed["b"].render(), "[green, blue]");
    assert_eq!(result.sealed["c"].render(), "blue");
}

#[test]
fn array_type_index_and_slice_errors_are_explicit() {
    for source in [
        "a = [1 kg, 2 s]\n",
        "a = [1, \"x\"]\n",
        "a = [[1]]\n",
        "a = [1]\na[0] = \"x\"\n",
        "a = [1]\na = append(a, 2 kg)\n",
        "a = [1]\nx = a[1]\n",
        "a = [1]\nx = a[-1]\n",
        "a = [1]\nx = a[1:0]\n",
        "a = [1]\nx = a[0:2]\n",
        "x = len(1)\n",
    ] {
        let parsed = parse_source(source).unwrap();
        let error = Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_err();
        assert!(matches!(error.code, "G000" | "G203"), "{source}: {error:?}");
    }
}

#[test]
fn array_canonical_hash_and_frozen_edit_refusal() {
    let root = tempdir().unwrap();
    let source = root.path().join("a.gbl");
    fs::write(&source, "a = [1, 2]\nseal a\n").unwrap();
    let hash = parse_source(&fs::read_to_string(&source).unwrap())
        .unwrap()
        .canonical_sha256()
        .unwrap();
    let other = parse_source("a=[1,2]\nseal a\n")
        .unwrap()
        .canonical_sha256()
        .unwrap();
    assert_eq!(hash, other);
    goblinpp::custody::create_freeze(&source).unwrap();
    fs::write(&source, "a = [1, 3]\nseal a\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&run).unwrap()["status"], "PROTOCOL_VIOLATION");
    assert!(verify_run(&run).unwrap().verified);
}
