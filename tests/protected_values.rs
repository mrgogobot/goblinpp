use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::compiler::compile;
use goblinpp::constants::CONSTANTS;
use goblinpp::custody::{create_freeze, create_revision, verify_freeze};
use goblinpp::evaluator::Evaluation;
use goblinpp::interaction::InputPolicy;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use goblinpp::text_runtime::{self, MAX_TEXT_BYTES};
use std::{fs, process::Command};
use tempfile::tempdir;

fn seal(value: &serde_json::Value, field: &str) -> String {
    let mut core = value.clone();
    core.as_object_mut().unwrap().remove(field);
    goblinpp::hashing::hash_canonical_json(&core).unwrap()
}

#[test]
fn every_constant_alias_refuses_assignment_before_rhs_effects() {
    for constant in CONSTANTS {
        for alias in constant.aliases {
            let parsed = parse_source(&format!("{alias} = input(\"must not prompt\")\n")).unwrap();
            let error = Evaluation::new(".")
                .eval_program(&parsed.program)
                .unwrap_err();
            assert_eq!(error.code, "G002");
            assert!(
                error
                    .message
                    .contains(&format!("{alias} is a registered constant"))
            );
            let root = tempdir().unwrap();
            let error = compile(&parsed, root.path().join("program"), &[]).unwrap_err();
            assert!(error.message.contains("registered constant"));
            assert!(!root.path().join("program.goblin.rs").exists());
        }
    }
}

#[test]
fn nested_bindings_cannot_hide_constant_assignments() {
    for source in [
        "if false { h = 2 }\n",
        "g_func bad(value) { h = value\nreturn value }\n",
        "for i in range(0) { pi = 3 }\n",
        "switch 1 { case 2 { c = 4 } }\n",
    ] {
        let parsed = parse_source(source).unwrap();
        assert!(
            Evaluation::new(".")
                .eval_program(&parsed.program)
                .unwrap_err()
                .message
                .contains("registered constant")
        );
    }
}

#[test]
fn discarded_value_builtins_are_refused_but_copy_append_and_effectful_calls_work() {
    for source in [
        "a = []\nappend(a, 1)\n",
        "sqrt(9)\n",
        "str_trim(\" hi \")\n",
    ] {
        // The exact append acceptance case is also checked through both launchers below.
        let parsed = parse_source(source).unwrap();
        assert!(
            Evaluation::new(".")
                .eval_program(&parsed.program)
                .unwrap_err()
                .message
                .contains("Discarded result")
        );
    }
    let parsed = parse_source("a = []\nb = append(a, 1)\nprint(a, b)\n").unwrap();
    let evaluated = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(evaluated.stdout, ["[] [1]"]);
}

#[test]
fn priority_refusals_are_preserved_and_verifiable_in_both_modes() {
    for source_text in [
        "GO_PARANOID\nh = 2\n",
        "GO_PARANOID\na = []\nappend(a, 1)\n",
    ] {
        let root = tempdir().unwrap();
        let source = root.path().join("refuse.gbl");
        fs::write(&source, source_text).unwrap();
        assert!(create_freeze(&source).is_err());
        for compile in [false, true] {
            let run = run_file(
                &source,
                &RunOptions {
                    compile,
                    ..Default::default()
                },
            )
            .unwrap();
            let receipt = read_receipt(&run).unwrap();
            assert_eq!(receipt["status"], "MACHINERY_FAIL", "{receipt}");
            assert_eq!(receipt["failure"]["code"], "G002");
            assert!(receipt["sealed_artifacts"].as_array().unwrap().is_empty());
            assert!(verify_run(&run).unwrap().verified);
        }
    }
}

const SNAPSHOTS: &str = r#"GO_PARANOID
g_func captured(value) { return "saved = {value}" }
x = 1
s = "{x}"
rows = []
for i in range(3) { rows = append(rows, "row = {i}") }
returned = captured(x)
formatted = "{x:.3f}"
joined = s + "!"
literal = "{{x}}"
json_literal = "{\"outer\":{\"inner\":1}}"
x = 2
print(s)
print(rows)
print(returned)
print(formatted)
print(joined)
print(literal)
print(json_literal)
write_text("t.txt", s)
write_csv("rows.csv", 1, "row", rows[0], rows[1], rows[2])
write_tsv("literal.tsv", 1, literal, s)
write_json("captured.json", "saved", s, "literal", literal)
seal s
seal rows
"#;

#[test]
fn snapshots_survive_assignment_loops_returns_arrays_concatenation_and_all_exports() {
    let root = tempdir().unwrap();
    let source = root.path().join("snapshots.gbl");
    fs::write(&source, SNAPSHOTS).unwrap();
    create_freeze(&source).unwrap();
    let mut runs = Vec::new();
    for compile in [false, true] {
        let run = run_file(
            &source,
            &RunOptions {
                compile,
                ..Default::default()
            },
        )
        .unwrap();
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "PASS", "{receipt}");
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "1\n[row = 0, row = 1, row = 2]\nsaved = 1\n1.000\n1!\n{x}\n{\"outer\":{\"inner\":1}}\n"
        );
        assert_eq!(
            fs::read_to_string(run.join("outputs/t.txt")).unwrap(),
            "1\n"
        );
        assert_eq!(
            fs::read_to_string(run.join("outputs/rows.csv")).unwrap(),
            "row\nrow = 0\nrow = 1\nrow = 2\n"
        );
        assert_eq!(
            fs::read_to_string(run.join("outputs/literal.tsv")).unwrap(),
            "{x}\n1\n"
        );
        let exported: serde_json::Value =
            serde_json::from_slice(&fs::read(run.join("outputs/captured.json")).unwrap()).unwrap();
        assert_eq!(exported["saved"], "1");
        assert_eq!(exported["literal"], "{x}");
        assert!(verify_run(&run).unwrap().verified);
        // Exercise the generated binary directly, not just the reference interpreter.
        if compile {
            let execution = Command::new(run.join("program-native"))
                .current_dir(root.path())
                .output()
                .unwrap();
            assert!(execution.status.success(), "{:?}", execution);
            assert_eq!(execution.stdout, fs::read(run.join("stdout.log")).unwrap());
        }
        runs.push(run);
    }
    assert!(diff_runs(&runs[0], &runs[1]).unwrap().sealed_artifacts_same);
}

#[test]
fn constants_have_one_meaning_in_expressions_templates_and_seals_and_are_recorded() {
    let root = tempdir().unwrap();
    let source = root.path().join("constants.gbl");
    fs::write(
        &source,
        "GO_PARANOID\nprint(h)\nprint(\"{h}\")\nprint(\"{π}; {ħ}\")\nseal h\nseal π\n",
    )
    .unwrap();
    for compile in [false, true] {
        let run = run_file(
            &source,
            &RunOptions {
                compile,
                ..Default::default()
            },
        )
        .unwrap();
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "PASS", "{receipt}");
        let stdout = fs::read_to_string(run.join("stdout.log")).unwrap();
        let lines = stdout.lines().collect::<Vec<_>>();
        assert_eq!(lines[0], lines[1]);
        assert!(lines[0].starts_with("6.62607015e-34"));
        assert_eq!(receipt["constants_used"].as_array().unwrap().len(), 3);
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn input_argv_and_data_braces_are_plain_text_not_executable_templates() {
    let root = tempdir().unwrap();
    let source = root.path().join("literal.gbl");
    fs::write(root.path().join("data.csv"), "name\n{missing}\n").unwrap();
    fs::write(
        &source,
        r#"x = 1
prompt = "value = {x}: "
x = 2
response = input(prompt)
argument = argv(1)
data = csv_column("data.csv", "name")
print(response)
print("response = {response}")
print(argument)
print(data[0])
write_text("literal.txt", response, argument, data[0])
"#,
    )
    .unwrap();
    for compile in [false, true] {
        let run = run_file(
            &source,
            &RunOptions {
                compile,
                input_policy: InputPolicy::Provided(vec!["{x}".into()]),
                program_args: vec!["{missing}".into()],
                ..Default::default()
            },
        )
        .unwrap();
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "PASS", "{receipt}");
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "{x}\nresponse = {x}\n{missing}\n{missing}\n"
        );
        assert_eq!(
            fs::read_to_string(run.join("outputs/literal.txt")).unwrap(),
            "{x}\n{missing}\n{missing}\n"
        );
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn snapshot_unknown_symbols_bad_formats_and_size_limits_fail_at_creation() {
    for text in ["{missing}", "{x", "{x:bad}"] {
        let parsed = parse_source(&format!("x = 1\ns = {text:?}\nx = 2\n")).unwrap();
        assert!(Evaluation::new(".").eval_program(&parsed.program).is_err());
    }
    assert!(text_runtime::snapshot("{x}", |_, _| Ok("a".repeat(MAX_TEXT_BYTES + 1))).is_err());
    assert_eq!(
        text_runtime::snapshot("{{x}}", |_, _| panic!("escaped field resolved")).unwrap(),
        "{x}"
    );
    assert_eq!(
        text_runtime::snapshot("{\"x\":{\"y\":1}}", |_, _| panic!("JSON resolved")).unwrap(),
        "{\"x\":{\"y\":1}}"
    );
}

#[test]
fn older_freeze_integrity_verifies_but_execution_requires_explicit_revision() {
    let root = tempdir().unwrap();
    let parent = root.path().join("parent.gbl");
    fs::write(
        &parent,
        "GO_PARANOID\nx = 1\ns = \"{x}\"\nx = 2\nprint(s)\n",
    )
    .unwrap();
    let (freeze, _) = create_freeze(&parent).unwrap();
    let mut receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(&freeze).unwrap()).unwrap();
    receipt
        .as_object_mut()
        .unwrap()
        .remove("language_semantics");
    receipt["goblin_version"] = "0.1.0-alpha.19".into();
    receipt["freeze_receipt_sha256"] = seal(&receipt, "freeze_receipt_sha256").into();
    fs::write(&freeze, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    // Rebuild a coherent synthetic historical ledger event (not a tampering test).
    let ledger_path = root.path().join(".goblin/custody-ledger.jsonl");
    let mut event: serde_json::Value =
        serde_json::from_str(fs::read_to_string(&ledger_path).unwrap().trim()).unwrap();
    event["payload"]["freeze_receipt_file_sha256"] =
        goblinpp::hashing::sha256_file(&freeze).unwrap().into();
    event["payload"]["freeze_receipt_core_sha256"] = receipt["freeze_receipt_sha256"].clone();
    // The ledger's field names and checkpoint format are asserted by its normal audit.
    event["event_sha256"] = seal(&event, "event_sha256").into();
    fs::write(
        &ledger_path,
        format!("{}\n", serde_json::to_string(&event).unwrap()),
    )
    .unwrap();
    let head_path = goblinpp::ledger::head_path(root.path());
    let mut head: serde_json::Value =
        serde_json::from_slice(&fs::read(&head_path).unwrap()).unwrap();
    head["head_event_sha256"] = event["event_sha256"].clone();
    head["ledger_file_sha256"] = goblinpp::hashing::sha256_file(&ledger_path).unwrap().into();
    fs::write(&head_path, serde_json::to_vec_pretty(&head).unwrap()).unwrap();
    assert!(verify_freeze(&parent).verified);
    let binary = root.path().join("refused-native");
    let compiled = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .arg("compile")
        .arg(&parent)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(!compiled.status.success());
    assert!(
        String::from_utf8_lossy(&compiled.stderr)
            .contains("LANGUAGE_SEMANTICS_CHANGED_AFTER_FREEZE")
    );
    assert!(!binary.exists());
    let refused = run_file(&parent, &RunOptions::default()).unwrap();
    let refused_receipt = read_receipt(&refused).unwrap();
    assert_eq!(
        refused_receipt["status"], "PROTOCOL_VIOLATION",
        "{refused_receipt}"
    );
    assert_eq!(
        refused_receipt["freeze"]["classification"],
        "LANGUAGE_SEMANTICS_CHANGED_AFTER_FREEZE"
    );
    assert!(verify_run(&refused).unwrap().verified);
    let child = root.path().join("child.gbl");
    create_revision(&parent, &child, "adopt immediate template capture").unwrap();
    create_freeze(&child).unwrap();
    let accepted = run_file(&child, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&accepted).unwrap()["status"], "PASS");
    assert_eq!(
        fs::read_to_string(accepted.join("stdout.log")).unwrap(),
        "1\n"
    );
}

#[test]
fn preview_reports_the_same_refusals_without_creating_evidence() {
    for text in [
        "h = 2\n",
        "a = []\nappend(a, 1)\n",
        "x = 1\ns = \"{missing}\"\n",
    ] {
        let root = tempdir().unwrap();
        let source = root.path().join("check.gbl");
        fs::write(&source, text).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
            .arg("check")
            .arg(&source)
            .arg("--json")
            .output()
            .unwrap();
        assert!(!result.status.success());
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["status"], "FAIL");
        assert_eq!(report["evidence_created"], false);
        assert!(!root.path().join("runs").exists());
    }
}

#[test]
fn historical_receipt_integrity_and_semantics_diffs_are_distinct() {
    let root = tempdir().unwrap();
    let source = root.path().join("simple.gbl");
    fs::write(&source, "x = 1\nprint(x)\n").unwrap();
    let current = run_file(&source, &RunOptions::default()).unwrap();
    // A coherent synthetic pre-policy receipt: its syntax/output did not change.
    let legacy = root.path().join("historical");
    fs::create_dir(&legacy).unwrap();
    for entry in fs::read_dir(&current).unwrap() {
        let entry = entry.unwrap();
        assert!(entry.file_type().unwrap().is_file());
        fs::copy(entry.path(), legacy.join(entry.file_name())).unwrap();
    }
    let mut receipt = read_receipt(&legacy).unwrap();
    receipt
        .as_object_mut()
        .unwrap()
        .remove("language_semantics");
    receipt["goblin_version"] = "0.1.0-alpha.19".into();
    receipt["receipt_core_sha256"] = seal(&receipt, "receipt_core_sha256").into();
    fs::write(
        legacy.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert!(verify_run(&legacy).unwrap().verified);
    let diff = diff_runs(&legacy, &current).unwrap();
    assert!(!diff.language_semantics_same);
    assert_eq!(diff.classification, "LANGUAGE_SEMANTICS_CHANGE");
    receipt["language_semantics"] = "unsealed-change".into();
    fs::write(
        legacy.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert!(!verify_run(&legacy).unwrap().verified);
}
