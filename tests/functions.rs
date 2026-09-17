use goblinpp::audit::verify_run;
use goblinpp::compiler::compile;
use goblinpp::custody::create_freeze;
use goblinpp::evaluator::Evaluation;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use std::fs;
use tempfile::tempdir;

const FUNCTIONS: &str = r#"GO_PARANOID
g_func triple(value) {
    return value * 3
}
g_func first_above(items, threshold) {
    for i in range(len(items)) {
        if items[i] > threshold {
            return triple(items[i])
        }
    }
    return 0
}
values = [1, 2, 3]
result = first_above(values, 1)
print("result = {result}")
seal result
"#;

#[test]
fn g_func_interpreter_and_native_compiler_agree_and_verify() {
    let root = tempdir().unwrap();
    let source = root.path().join("functions.gbl");
    fs::write(&source, FUNCTIONS).unwrap();
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
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "result = 6\n"
        );
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn g_func_has_local_scope_and_copy_arguments() {
    let source = r#"g_func change(items) {
    items[0] = 9
    return items[0]
}
items = [1, 2]
inside = change(items)
outside = items[0]
seal inside
seal outside
"#;
    let parsed = parse_source(source).unwrap();
    let evaluated = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(evaluated.sealed["inside"].render(), "9");
    assert_eq!(evaluated.sealed["outside"].render(), "1");
    assert!(!evaluated.env.contains_key("change"));
    let root = tempdir().unwrap();
    let file = root.path().join("copies.gbl");
    fs::write(&file, source).unwrap();
    let run = run_file(
        &file,
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
fn forward_calls_and_zero_arg_functions_work() {
    let source = "answer = meaning()\ng_func meaning() { return 42 }\nseal answer\n";
    let parsed = parse_source(source).unwrap();
    let evaluated = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(evaluated.sealed["answer"].render(), "42");
    let root = tempdir().unwrap();
    let file = root.path().join("forward.gbl");
    fs::write(&file, source).unwrap();
    let run = run_file(
        &file,
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
fn g_func_failures_are_explicit_and_preserved() {
    let root = tempdir().unwrap();
    let source = root.path().join("bad.gbl");
    for (program, fragment) in [
        ("g_func f(x) { return x }\ny = f()\n", "expects 1 argument"),
        ("g_func f(x) { x = x + 1 }\ny = f(1)\n", "without return"),
        ("g_func f(x) { return f(x) }\ny = f(1)\n", "depth exceeded"),
        ("g_func f() { return missing }\ny = f()\n", "UNKNOWN SYMBOL"),
    ] {
        fs::write(&source, program).unwrap();
        let run = run_file(&source, &RunOptions::default()).unwrap();
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "MACHINERY_FAIL", "{program}");
        assert!(
            receipt["failure"].to_string().contains(fragment),
            "{receipt:?}"
        );
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn invalid_g_func_syntax_and_reserved_names_are_rejected() {
    for source in [
        "return 1\n",
        "if true { g_func f() { return 1 } }\n",
        "g_func print(x) { return x }\n",
        "g_func c(m, m) { return m }\n",
        "g_func c(argc) { return argc }\n",
        "g_func f() { seal x\n return 1 }\n",
        "g_func f() { return 1 }\ng_func f() { return 2 }\n",
    ] {
        assert!(parse_source(source).is_err(), "accepted {source:?}");
    }
}

#[test]
fn compiled_mode_refuses_data_calls_in_function_bodies() {
    let root = tempdir().unwrap();
    let parsed =
        parse_source("g_func f() { return fits_count(\"sample.fits\", 0) }\nx = 1\n").unwrap();
    let error = compile(&parsed, root.path().join("program"), &[]).unwrap_err();
    assert_eq!(error.code, "G501");
}

#[test]
fn interpreted_function_output_is_run_confined_and_verified() {
    let root = tempdir().unwrap();
    let source = root.path().join("output.gbl");
    fs::write(&source, "g_func report(x) {\n write_text(\"answer.txt\", \"x = {x}\")\n return x\n}\nvalue = report(7)\nseal value\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&run).unwrap()["status"], "PASS");
    assert_eq!(
        fs::read_to_string(run.join("outputs/answer.txt")).unwrap(),
        "x = 7\n"
    );
    assert!(verify_run(&run).unwrap().verified);
}
