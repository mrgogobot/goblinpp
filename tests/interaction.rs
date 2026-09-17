use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::interaction::InputPolicy;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use tempfile::tempdir;

const GREETING: &str = "GO_PARANOID\nname = input(\"Please enter your name.\\n\")\nprint(\"Hello, {name}!\")\nseal name\n";

#[test]
fn supplied_input_replays_in_both_engines_and_verifies() {
    let root = tempdir().unwrap();
    let source = root.path().join("greeting.gbl");
    fs::write(&source, GREETING).unwrap();
    let mut runs = Vec::new();
    for compile in [false, true] {
        let run = run_file(
            &source,
            &RunOptions {
                compile,
                input_policy: InputPolicy::Provided(vec!["Ada".into()]),
                ..RunOptions::default()
            },
        )
        .unwrap();
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "PASS", "{receipt:?}");
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "Hello, Ada!\n"
        );
        assert_eq!(receipt["interaction"]["input_count"], 1);
        let evidence: Value =
            serde_json::from_slice(&fs::read(run.join("interaction.json")).unwrap()).unwrap();
        assert_eq!(
            evidence["input_events"][0]["prompt"],
            "Please enter your name.\n"
        );
        assert_eq!(evidence["input_events"][0]["response"], "Ada");
        assert!(verify_run(&run).unwrap().verified);
        runs.push(run);
    }
    let diff = diff_runs(&runs[0], &runs[1]).unwrap();
    assert!(diff.interaction_same);
    assert_eq!(diff.classification, "IDENTICAL_RESULT");
}

#[test]
fn argc_argv_options_and_zero_index_work_in_both_engines() {
    let root = tempdir().unwrap();
    let source = root.path().join("args.gbl");
    fs::write(&source, "GO_PARANOID\nprogram = argv(0)\nflag = argv(1)\nname = argv(2)\nprint(\"argc = {argc}; flag = {flag}; name = {name}\")\nseal program\nseal name\n").unwrap();
    for compile in [false, true] {
        let run = run_file(
            &source,
            &RunOptions {
                compile,
                program_args: vec!["--name".into(), "Ada Lovelace".into()],
                ..RunOptions::default()
            },
        )
        .unwrap();
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "PASS", "{receipt:?}");
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "argc = 3; flag = --name; name = Ada Lovelace\n"
        );
        assert_eq!(receipt["interaction"]["argc"], 3);
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn missing_input_is_preserved_and_check_never_reads_stdin() {
    let root = tempdir().unwrap();
    let source = root.path().join("greeting.gbl");
    fs::write(&source, GREETING).unwrap();
    let run = run_file(
        &source,
        &RunOptions {
            input_policy: InputPolicy::Provided(vec![]),
            ..RunOptions::default()
        },
    )
    .unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["failure"]["code"], "G203");
    assert_eq!(receipt["interaction"]["input_count"], 1);
    assert!(verify_run(&run).unwrap().verified);

    let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["check", source.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "FAIL");
    assert_eq!(report["diagnostic"]["code"], "G203");
}

#[test]
fn interaction_tamper_and_different_answers_change_audit_result() {
    let root = tempdir().unwrap();
    let source = root.path().join("unused_answer.gbl");
    fs::write(&source, "x = input(\"name: \")\nprint(\"constant\")\n").unwrap();
    let make_run = |answer: &str| {
        run_file(
            &source,
            &RunOptions {
                input_policy: InputPolicy::Provided(vec![answer.into()]),
                ..RunOptions::default()
            },
        )
        .unwrap()
    };
    let a = make_run("Ada");
    let b = make_run("Grace");
    let diff = diff_runs(&a, &b).unwrap();
    assert!(!diff.interaction_same);
    assert_eq!(diff.classification, "SEMANTIC_OR_RESULT_CHANGE");
    fs::write(a.join("interaction.json"), b"tampered").unwrap();
    assert!(!verify_run(&a).unwrap().verified);
}

#[test]
fn invalid_indexes_and_read_only_argc_fail_explicitly() {
    assert!(parse_source("argc = 99\n").is_err());
    let root = tempdir().unwrap();
    let source = root.path().join("bad_argv.gbl");
    fs::write(&source, "x = argv(1)\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&run).unwrap()["failure"]["code"], "G203");
    assert!(verify_run(&run).unwrap().verified);
}

#[test]
fn two_stdin_lines_are_read_separately_and_preserved() {
    let root = tempdir().unwrap();
    let source = root.path().join("two_inputs.gbl");
    fs::write(
        &source,
        "a = input(\"first: \")\nb = input(\"second: \")\nprint(\"{a}/{b}\")\n",
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .arg(&source)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"Ada\nGrace\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Ada/Grace\n"));
    let run_dir = stdout
        .lines()
        .find_map(|line| line.strip_prefix("RUN_DIR="))
        .unwrap();
    let evidence: Value = serde_json::from_slice(
        &fs::read(std::path::Path::new(run_dir).join("interaction.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(evidence["input_events"][0]["response"], "Ada");
    assert_eq!(evidence["input_events"][1]["response"], "Grace");
}

#[test]
fn cli_double_dash_passes_program_options_unchanged() {
    let root = tempdir().unwrap();
    let source = root.path().join("cli_args.gbl");
    fs::write(&source, "print(\"argc = {argc}\")\nprint(argv(1))\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args([source.to_str().unwrap(), "--", "--name"])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("argc = 2\n--name\n"));
    assert!(stdout.contains("RUN_STATUS=PASS"));
}

#[test]
fn interactive_cli_prompts_once_and_keeps_json_stdout_clean() {
    let root = tempdir().unwrap();
    let source = root.path().join("hello.gbl");
    fs::write(
        &source,
        "name = input(\"Name? \")\nprint(\"Hello, {name}!\")\n",
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["run", source.to_str().unwrap(), "--json"])
        .env("GOBLIN_INPUT_PROMPT_PROTOCOL", "hex-v1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"Ada\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["receipt"]["status"], "PASS");
    let run_dir = report["run_dir"].as_str().unwrap();
    assert_eq!(
        fs::read_to_string(std::path::Path::new(run_dir).join("stdout.log")).unwrap(),
        "Hello, Ada!\n"
    );
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "GOBLIN_INPUT_PROMPT_V1\t4e616d653f20\n"
    );
}
