use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::hashing::{hash_canonical_json, sha256_bytes};
use goblinpp::parser::{self, SyntaxMode, parse_source, parse_source_with_mode};
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn run(root: &Path, text: &str, compile: bool) -> PathBuf {
    let source = root.join("main.gbl");
    fs::write(&source, text).unwrap();
    run_file(
        &source,
        &RunOptions {
            compile,
            ..RunOptions::default()
        },
    )
    .unwrap()
}
fn state(run: &Path) -> Value {
    serde_json::from_slice(&fs::read(run.join("final-state.json")).unwrap()).unwrap()
}
fn reseal(run: &Path, mut receipt: Value) {
    receipt
        .as_object_mut()
        .unwrap()
        .remove("receipt_core_sha256");
    receipt["receipt_core_sha256"] = json!(hash_canonical_json(&receipt).unwrap());
    fs::write(
        run.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
}

const PROGRAM: &str = r#"COMPLETELY_MAD
g_func double(value) {
    local_value = value * 2
    return local_value
}
mass = 2 kg
seal mass
mass = 3 kg
energy = mass * c^2
label = "captured mass = {mass}"
accepted = true
values = [1 m, 2 m]
values[0] = 9 m
empty = []
total = 0
for index in range(3) { total = total + double(index) }
if false { never_assigned = 99 }
if true { branch_value = 8 }
print(label)
"#;

#[test]
fn both_engines_capture_globals_without_overwriting_explicit_snapshots() {
    for compile in [false, true] {
        let root = tempdir().unwrap();
        let run = run(root.path(), PROGRAM, compile);
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "PASS", "{receipt:#}");
        assert_eq!(receipt["paranoid_mode"], true);
        assert_eq!(receipt["execution_profile"], "COMPLETELY_MAD");
        assert_eq!(receipt["final_state"]["completeness"], "COMPLETE");
        assert_eq!(receipt["sealed_artifacts"].as_array().unwrap().len(), 1);
        let explicit: Value =
            serde_json::from_slice(&fs::read(run.join("mass.json")).unwrap()).unwrap();
        assert_eq!(explicit["value"]["value_si"], 2.0);
        let final_state = state(&run);
        assert_eq!(final_state["values"]["mass"]["value_si"], 3.0);
        assert_eq!(final_state["values"]["label"], "captured mass = 3 kg");
        assert_eq!(final_state["values"]["accepted"], true);
        assert_eq!(final_state["values"]["values"]["items"][0]["value_si"], 9.0);
        assert_eq!(final_state["values"]["total"]["value_si"], 6.0);
        for excluded in [
            "index",
            "value",
            "local_value",
            "never_assigned",
            "c",
            "argc",
        ] {
            assert!(final_state["values"].get(excluded).is_none(), "{excluded}");
        }
        assert!(verify_run(&run).unwrap().verified, "{:?}", verify_run(&run));
        if compile {
            assert_eq!(receipt["final_state"]["native_parity"], "PASS");
            let standalone = std::process::Command::new(run.join("program-native"))
                .output()
                .unwrap();
            assert!(!standalone.status.success());
            assert!(String::from_utf8_lossy(&standalone.stderr).contains("audited run --compile"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(run.join("final-state.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
}

#[test]
fn explicit_global_reused_as_counter_is_still_a_global_binding() {
    let root = tempdir().unwrap();
    let run = run(
        root.path(),
        "COMPLETELY_MAD\nindex = 99\nfor index in range(2) { print(index) }",
        false,
    );
    assert_eq!(state(&run)["values"]["index"]["value_si"], 1.0);
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn failed_execution_captures_only_reached_globals_as_partial_reference_state() {
    for compile in [false, true] {
        let root = tempdir().unwrap();
        let run = run(
            root.path(),
            "COMPLETELY_MAD\ng_func fail(value) { local_value = value / 0\nreturn local_value }\nx = 7\nseal x\ny = fail(x)\nz = 9",
            compile,
        );
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "MACHINERY_FAIL");
        assert_eq!(receipt["final_state"]["completeness"], "PARTIAL");
        assert_eq!(receipt["final_state"]["variable_names"], json!(["x"]));
        assert_eq!(state(&run)["values"]["x"]["value_si"], 7.0);
        assert!(verify_run(run).unwrap().verified);
    }
}

#[test]
fn invalid_directives_and_inline_rust_refuse_before_user_effects() {
    for source in [
        "x = 1\nCOMPLETELY_MAD",
        "COMPLETELY_MAD\nCOMPLETELY_MAD",
        "COMPLETELY_MAD\nx = 1\nRUST_INLINE_BEGIN\nprintln!(\"unsafe effects\");\nRUST_INLINE_END",
    ] {
        let root = tempdir().unwrap();
        let run = run(root.path(), source, false);
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "MACHINERY_FAIL", "{receipt:#}");
        assert_eq!(receipt["final_state"]["capture_status"], "NOT_STARTED");
        assert!(!run.join("final-state.json").exists());
        assert!(verify_run(&run).unwrap().verified, "{:?}", verify_run(run));
    }
    for source in [
        "if true { COMPLETELY_MAD }",
        "g_func f() { COMPLETELY_MAD; return 1 }",
        "COMPLETELY_MAD = 2",
        "COMPLETELY_MAD()",
    ] {
        assert!(parse_source(source).is_err(), "{source}");
    }
}

#[test]
fn variable_and_serialization_budgets_refuse_without_truncation() {
    let mut many = String::from("COMPLETELY_MAD\n");
    for index in 0..257 {
        many.push_str(&format!("binding_{index} = {index}\n"));
    }
    let root = tempdir().unwrap();
    let limited_run = run(root.path(), &many, false);
    let receipt = read_receipt(&limited_run).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["final_state"]["capture_status"], "REFUSED");
    assert!(!limited_run.join("final-state.json").exists());
    assert!(verify_run(limited_run).unwrap().verified);
    // Several individually valid strings exceed the aggregate capture limit.
    let mut source = String::from(
        "COMPLETELY_MAD\ntext = \"0123456789abcdef\"\nfor i in range(15) { text = text + text }\n",
    );
    for index in 0..17 {
        source.push_str(&format!("copy_{index} = text\n"));
    }
    let root = tempdir().unwrap();
    let run = run(root.path(), &source, false);
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["final_state"]["capture_status"], "REFUSED");
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn damaged_or_missing_final_state_never_verifies() {
    for remove in [false, true] {
        let root = tempdir().unwrap();
        let run = run(root.path(), "COMPLETELY_MAD\nx = 1", false);
        if remove {
            fs::remove_file(run.join("final-state.json")).unwrap();
        } else {
            fs::write(run.join("final-state.json"), b"tampered").unwrap();
        }
        assert!(!verify_run(run).unwrap().verified);
    }
}

#[test]
fn stripping_or_downgrading_capture_policy_is_detected_even_after_core_rehash() {
    for field in [
        "remove",
        "policy",
        "requested",
        "capture_status",
        "completeness",
        "path",
        "variable_names",
        "origin",
    ] {
        let root = tempdir().unwrap();
        let run = run(root.path(), "COMPLETELY_MAD\nx = 1", false);
        let mut receipt = read_receipt(&run).unwrap();
        match field {
            "remove" => {
                receipt.as_object_mut().unwrap().remove("final_state");
            }
            "requested" => receipt["final_state"][field] = json!(false),
            "variable_names" => receipt["final_state"][field] = json!([]),
            _ => receipt["final_state"][field] = json!("wrong"),
        }
        reseal(&run, receipt);
        assert!(!verify_run(&run).unwrap().verified, "{field}");
    }
}

#[test]
fn removing_a_mandatory_global_from_rehashed_state_is_detected() {
    let root = tempdir().unwrap();
    let run = run(root.path(), "COMPLETELY_MAD\nx = 1\ny = 2", false);
    let mut document = state(&run);
    document["values"].as_object_mut().unwrap().remove("x");
    let bytes = serde_json::to_vec(&document).unwrap();
    fs::write(run.join("final-state.json"), &bytes).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    receipt["final_state"]["sha256"] = json!(sha256_bytes(&bytes));
    receipt["final_state"]["byte_count"] = json!(bytes.len());
    receipt["final_state"]["variable_count"] = json!(1);
    receipt["final_state"]["variable_names"] = json!(["y"]);
    reseal(&run, receipt);
    assert!(!verify_run(run).unwrap().verified);
}

#[test]
fn changed_final_native_values_fail_offline_verification() {
    let root = tempdir().unwrap();
    let run = run(root.path(), "COMPLETELY_MAD\nx = 1", true);
    let mut document = state(&run);
    document["values"]["x"]["value_si"] = json!(2.0);
    let bytes = serde_json::to_vec(&document).unwrap();
    fs::write(run.join("final-state.json"), &bytes).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    receipt["final_state"]["sha256"] = json!(sha256_bytes(&bytes));
    receipt["final_state"]["byte_count"] = json!(bytes.len());
    reseal(&run, receipt);
    assert!(!verify_run(run).unwrap().verified);
}

#[test]
fn everyday_and_paranoid_programs_do_not_gain_automatic_seals() {
    for directive in ["", "GO_PARANOID\n"] {
        let root = tempdir().unwrap();
        let run = run(root.path(), &format!("{directive}x = 1"), false);
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(receipt["status"], "PASS");
        assert_eq!(receipt["final_state"]["requested"], false);
        assert!(receipt["sealed_artifacts"].as_array().unwrap().is_empty());
        assert!(!run.join("final-state.json").exists());
        assert!(verify_run(run).unwrap().verified);
    }
}

#[test]
fn old_identifiers_keep_their_historical_canonical_meaning() {
    for mode in [
        SyntaxMode::PreMad,
        SyntaxMode::CompoundUnitsNoLoopBudget,
        SyntaxMode::LegacyQuantity,
    ] {
        for text in [
            "COMPLETELY_MAD = 2",
            "g_func COMPLETELY_MAD() { return 1 }",
            "g_func helper(COMPLETELY_MAD) { return COMPLETELY_MAD }",
            "for COMPLETELY_MAD in range(2) { x = COMPLETELY_MAD }",
        ] {
            assert!(
                parse_source_with_mode(text, mode).is_ok(),
                "{mode:?}: {text}"
            );
            assert!(parse_source(text).is_err(), "{text}");
        }
    }
    assert_eq!(parser::evidence_mode(&json!({"goblin_version":"0.1.0-alpha.38", "parser_policy":parser::PRE_MAD_PARSER_POLICY})).unwrap(), SyntaxMode::PreMad);
    assert!(parser::evidence_mode(&json!({"goblin_version":"0.1.0-alpha.39", "parser_policy":parser::PRE_MAD_PARSER_POLICY})).is_err());
}

#[test]
fn imported_function_locals_are_not_final_globals() {
    let root = tempdir().unwrap();
    fs::write(
        root.path().join("library.gbl"),
        "g_func double(value) { local_value = value * 2\nreturn local_value }",
    )
    .unwrap();
    let run = run(
        root.path(),
        "COMPLETELY_MAD\nimport \"library.gbl\"\nresult = double(3)",
        false,
    );
    assert_eq!(state(&run)["values"]["result"]["value_si"], 6.0);
    assert_eq!(
        read_receipt(&run).unwrap()["final_state"]["variable_names"],
        json!(["result"])
    );
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn diff_includes_automatic_final_values() {
    let root = tempdir().unwrap();
    let left = run(root.path(), "COMPLETELY_MAD\nx = 1", false);
    let right = run(root.path(), "COMPLETELY_MAD\nx = 2", false);
    assert!(!diff_runs(left, right).unwrap().sealed_artifacts_same);
}

#[test]
fn equal_final_values_compare_equal_across_engines() {
    let root = tempdir().unwrap();
    let left = run(root.path(), "COMPLETELY_MAD\nx = 1", false);
    let right = run(root.path(), "COMPLETELY_MAD\nx = 1", true);
    assert!(diff_runs(left, right).unwrap().sealed_artifacts_same);
}

#[test]
fn compiled_data_outputs_and_local_functions_keep_final_parity() {
    let root = tempdir().unwrap();
    let run = run(
        root.path(),
        "COMPLETELY_MAD\nx = 2 kg\nwrite_text(\"answer.txt\", \"mass = {x}\")\n",
        true,
    );
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PASS", "{receipt:#}");
    assert_eq!(state(&run)["values"]["x"]["value_si"], 2.0);
    assert_eq!(
        fs::read_to_string(run.join("outputs/answer.txt")).unwrap(),
        "mass = 2 kg\n"
    );
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn preview_is_explicitly_not_evidence_and_has_no_run_directory() {
    let root = tempdir().unwrap();
    let source = root.path().join("preview.gbl");
    fs::write(&source, "COMPLETELY_MAD\nx = 2\n").unwrap();
    let preview = std::process::Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["check", source.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let report: Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(report["execution_profile"], "COMPLETELY_MAD");
    assert_eq!(
        report["final_state_preview"]["authority"],
        "PREVIEW_ONLY_NOT_EVIDENCE"
    );
    assert_eq!(
        report["final_state_preview"]["values"]["x"]["value_si"],
        2.0
    );
    assert!(!root.path().join("runs").exists());
}

#[test]
fn cli_identifies_selected_profile_in_both_engines_and_verification() {
    for compile in [false, true] {
        for (directive, profile) in [
            ("COMPLETELY_MAD\n", "COMPLETELY_MAD"),
            ("GO_PARANOID\n", "GO_PARANOID"),
            ("", "EVERYDAY"),
        ] {
            let root = tempdir().unwrap();
            let source = root.path().join("main.gbl");
            fs::write(&source, format!("{directive}x = 2\nprint(x)\n")).unwrap();
            let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_goblinpp"));
            command.arg("run").arg(&source);
            if compile {
                command.arg("--compile");
            }
            let output = command.output().unwrap();
            let stdout = String::from_utf8(output.stdout).unwrap();
            assert!(output.status.success(), "{stdout}\n{:?}", output.stderr);
            let run = PathBuf::from(
                stdout
                    .lines()
                    .find_map(|line| line.strip_prefix("RUN_DIR="))
                    .unwrap(),
            );
            assert_eq!(read_receipt(&run).unwrap()["execution_profile"], profile);
            assert_eq!(fs::read_to_string(run.join("stdout.log")).unwrap(), "2\n");
            let verified = verify_run(&run).unwrap();
            assert!(verified.verified, "{verified:?}");
            match profile {
                "COMPLETELY_MAD" => {
                    assert!(stdout.starts_with("GOBLIN++ COMPLETELY_MAD MODE\n"));
                    assert!(stdout.contains("EXECUTION_PROFILE=COMPLETELY_MAD\n"));
                    assert!(stdout.contains("COMPLETELY_MAD_POSTFLIGHT=PASS\n"));
                    assert!(!stdout.contains("GOBLIN++ PARANOID MODE"));
                    assert!(!stdout.contains("PARANOID_POSTFLIGHT="));
                    assert!(
                        verified
                            .checks
                            .iter()
                            .any(|item| item.check == "COMPLETELY_MAD_POSTFLIGHT_EVIDENCE_SHA256")
                    );
                    assert!(
                        !verified
                            .checks
                            .iter()
                            .any(|item| item.check.starts_with("PARANOID_"))
                    );
                }
                "GO_PARANOID" => {
                    assert!(stdout.starts_with("GOBLIN++ PARANOID MODE\n"));
                    assert!(stdout.contains("PARANOID_POSTFLIGHT=PASS\n"));
                    assert!(!stdout.contains("COMPLETELY_MAD"));
                    assert!(
                        verified
                            .checks
                            .iter()
                            .any(|item| item.check == "PARANOID_POSTFLIGHT_EVIDENCE_SHA256")
                    );
                }
                _ => {
                    assert!(!stdout.contains("GOBLIN++"));
                    assert!(!stdout.contains("POSTFLIGHT="));
                }
            }
            let preview = std::process::Command::new(env!("CARGO_BIN_EXE_goblinpp"))
                .arg("check")
                .arg(&source)
                .arg("--json")
                .output()
                .unwrap();
            assert!(preview.status.success());
            let report: Value = serde_json::from_slice(&preview.stdout).unwrap();
            assert_eq!(report["execution_profile"], profile);
        }
    }
}

#[test]
fn incorrect_profile_refuses_even_with_rehashed_receipt() {
    for profile in [
        json!("GO_PARANOID"),
        json!("EVERYDAY"),
        json!("unknown"),
        Value::Null,
    ] {
        let root = tempdir().unwrap();
        let run = run(root.path(), "COMPLETELY_MAD\nx = 2", false);
        let mut receipt = read_receipt(&run).unwrap();
        receipt["execution_profile"] = profile;
        reseal(&run, receipt);
        let result = verify_run(&run).unwrap();
        assert!(!result.verified);
        assert!(
            result
                .checks
                .iter()
                .any(|item| item.check == "EXECUTION_PROFILE_MATCHES_SOURCE" && !item.pass)
        );
    }
}

#[test]
fn earlier_receipts_without_profile_remain_verifiable() {
    for directive in ["COMPLETELY_MAD", "GO_PARANOID", ""] {
        let root = tempdir().unwrap();
        let run = run(root.path(), &format!("{directive}\nx = 2"), false);
        let mut receipt = read_receipt(&run).unwrap();
        receipt.as_object_mut().unwrap().remove("execution_profile");
        reseal(&run, receipt);
        assert!(verify_run(run).unwrap().verified);
    }
}

#[test]
fn freeze_policy_is_pinned_and_frozen_run_verifies() {
    use goblinpp::custody::create_freeze;
    let root = tempdir().unwrap();
    let source = root.path().join("main.gbl");
    fs::write(&source, "COMPLETELY_MAD\nx = 2\n").unwrap();
    let (freeze_path, _) = create_freeze(&source).unwrap();
    let frozen: Value = serde_json::from_slice(&fs::read(freeze_path).unwrap()).unwrap();
    assert_eq!(frozen["final_state_policy"], goblinpp::mad::POLICY);
    let run_dir = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&run_dir).unwrap()["status"], "PASS");
    assert!(verify_run(run_dir).unwrap().verified);
}

#[test]
fn failed_import_still_records_requested_profile_and_verifies_as_failure() {
    let root = tempdir().unwrap();
    let run = run(
        root.path(),
        "COMPLETELY_MAD\nimport \"missing.gbl\"\nx = 1",
        false,
    );
    let receipt = read_receipt(&run).unwrap();
    assert_ne!(receipt["status"], "PASS");
    assert_eq!(receipt["final_state"]["capture_status"], "NOT_STARTED");
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn malformed_dimension_evidence_is_rejected_without_panicking() {
    let root = tempdir().unwrap();
    let run = run(root.path(), "COMPLETELY_MAD\nx = 1", false);
    let mut document = state(&run);
    document["values"]["x"]["dimension"][0] = json!(i32::MIN);
    let bytes = serde_json::to_vec(&document).unwrap();
    fs::write(run.join("final-state.json"), &bytes).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    receipt["final_state"]["sha256"] = json!(sha256_bytes(&bytes));
    receipt["final_state"]["byte_count"] = json!(bytes.len());
    reseal(&run, receipt);
    assert!(!verify_run(run).unwrap().verified);
}

#[test]
fn late_capture_refusal_marks_closed_streams_partial() {
    let root = tempdir().unwrap();
    let mut source = String::from(
        "COMPLETELY_MAD\nwriter = csv_stream_open(\"result.csv\", [\"value\"], 4096)\nstream_write(writer, [\"ok\"])\nstream_close(writer)\n",
    );
    for index in 0..256 {
        source.push_str(&format!("binding_{index} = {index}\n"));
    }
    let run = run(root.path(), &source, false);
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["final_state"]["capture_status"], "REFUSED");
    assert_eq!(
        receipt["generated_artifacts"][0]["metadata"]["complete"],
        false
    );
    assert_eq!(receipt["failure"]["code"], "G703");
    assert!(!run.join("outputs/result.csv").exists());
    assert_eq!(
        fs::read_to_string(run.join("outputs/result.csv.partial")).unwrap(),
        "value\nok\n"
    );
    assert!(verify_run(run).unwrap().verified);
}
