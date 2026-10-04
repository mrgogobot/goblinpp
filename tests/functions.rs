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
fn recursion_is_refused_on_a_small_stack_and_restores_the_caller() {
    // Linux's normal 2 MiB test-thread stack, explicitly imposed on macOS too.
    // Do not enlarge CI stacks to hide a broken call-depth refusal.
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for source in [
                "g_func f(x) { return f(x) }\ny = f(1)\n",
                "g_func f(x) { return again(x) }\ng_func again(x) { return f(x) }\ny = f(1)\n",
                "g_func f(x) { return abs(f(x)) }\ny = f(1)\n",
                "g_func f(x) { return len([f(x)]) }\ny = f(1)\n",
                "g_func f(x) { return fits_where(\"Z\", \"==\", f(x)) }\ny = f(1)\n",
            ] {
                eprintln!("small-stack recursion case: {source}");
                let parsed = parse_source(source).unwrap();
                let mut evaluation = Evaluation::new(".");
                evaluation.register_functions(&parsed.program).unwrap();
                let sentinel = parse_source("sentinel = 42\n").unwrap();
                evaluation
                    .eval_stmt(&sentinel.program.statements[0])
                    .unwrap();
                for statement in &parsed.program.statements[..parsed.program.statements.len() - 1] {
                    evaluation.eval_stmt(statement).unwrap();
                }
                let error = evaluation
                    .eval_stmt(parsed.program.statements.last().unwrap())
                    .unwrap_err();
                assert!(error.to_string().contains("depth exceeded"), "{error}");
                assert_eq!(evaluation.env["sentinel"].render(), "42");
                assert!(!evaluation.env.contains_key("x"));
                let recovery = parse_source("g_func recover(n) { if n == 0 { return 7 }\n return recover(n - 1) }\nrecovered = recover(15)\n").unwrap();
                evaluation.register_functions(&recovery.program).unwrap();
                evaluation.eval_stmt(recovery.program.statements.last().unwrap()).unwrap();
                assert_eq!(evaluation.env["recovered"].render(), "7");
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn recursive_depth_boundary_agrees_with_standalone_native_execution() {
    use goblinpp::evaluator::MAX_FUNCTION_DEPTH;
    let root = tempdir().unwrap();
    for (remaining, success) in [(MAX_FUNCTION_DEPTH - 1, true), (MAX_FUNCTION_DEPTH, false)] {
        let source = format!(
            "g_func descend(n) {{ if n == 0 {{ return 7 }}\n return abs(descend(n - 1)) }}\nanswer = descend({remaining})\nprint(answer)\nseal answer\n"
        );
        let parsed = parse_source(&source).unwrap();
        let interpreted = Evaluation::new(root.path()).eval_program(&parsed.program);
        assert_eq!(interpreted.is_ok(), success);
        if success {
            assert_eq!(interpreted.unwrap().sealed["answer"].render(), "7");
        } else {
            assert!(
                interpreted
                    .unwrap_err()
                    .to_string()
                    .contains("depth exceeded")
            );
        }
        // Run the binary directly, not the launcher's interpreter preflight.
        let compiled =
            compile(&parsed, root.path().join(format!("depth-{remaining}")), &[]).unwrap();
        let output = std::process::Command::new(compiled.binary)
            .output()
            .unwrap();
        assert_eq!(output.status.success(), success, "{output:?}");
        if success {
            assert_eq!(String::from_utf8(output.stdout).unwrap(), "7\n");
        } else {
            assert!(
                String::from_utf8(output.stderr)
                    .unwrap()
                    .contains("depth exceeded")
            );
        }
    }
}

#[test]
fn builtin_argument_validation_precedes_effects_and_values_are_evaluated_once() {
    for call in [
        "abs(tick(), tick())",
        "len(tick(), tick())",
        "fits_where(tick())",
        "write_text(tick())",
        "csv_rows(tick(), tick())",
        "not_a_function(tick())",
    ] {
        let parsed = parse_source(&format!(
            "g_func tick() {{ print(\"tick\")\n return 1 }}\nanswer = {call}\n"
        ))
        .unwrap();
        let mut evaluation = Evaluation::new(".");
        evaluation.register_functions(&parsed.program).unwrap();
        assert!(
            evaluation
                .eval_stmt(parsed.program.statements.last().unwrap())
                .is_err()
        );
        assert!(
            evaluation.stdout.is_empty(),
            "evaluated arguments of {call}"
        );
    }
    let source = "g_func tick(n) { print(n)\n return n }\nanswer = min(abs(tick(-2)), tick(3))\nprint(answer)\nseal answer\n";
    let parsed = parse_source(source).unwrap();
    let result = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(result.stdout, ["-2", "3", "2"]);
    assert_eq!(result.sealed["answer"].render(), "2");
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
fn compiled_mode_supports_uninvoked_data_functions() {
    let root = tempdir().unwrap();
    let parsed =
        parse_source("g_func f() { return fits_count(\"sample.fits\", 0) }\nx = 1\n").unwrap();
    let compiled = compile(&parsed, root.path().join("program"), &[]).unwrap();
    let output = std::process::Command::new(compiled.binary)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(output.stdout.is_empty());
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
