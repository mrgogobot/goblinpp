use chrono::{TimeZone, Utc};
use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::clock::{Clock, Evidence, Reading, verify_evidence};
use goblinpp::custody::{create_freeze, verify_freeze};
use goblinpp::evaluator::Evaluation;
use goblinpp::hashing::hash_canonical_json;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::json;
use std::{fs, process::Command};
use tempfile::tempdir;

fn reading(index: usize, mode: &str, offset: i32) -> Reading {
    Reading::at(
        index,
        mode,
        Utc.with_ymd_and_hms(2024, 2, 29, 23, 59, 58).unwrap(),
        offset,
    )
    .unwrap()
}
fn reseal(path: &std::path::Path, receipt: &mut serde_json::Value) {
    receipt
        .as_object_mut()
        .unwrap()
        .remove("receipt_core_sha256");
    receipt["receipt_core_sha256"] = json!(hash_canonical_json(receipt).unwrap());
    fs::write(
        path.join("receipt.json"),
        serde_json::to_vec_pretty(receipt).unwrap(),
    )
    .unwrap();
}
const SOURCE: &str = "GO_PARANOID\na = time_date()\nb = time_date(\"utc\")\nlocal_stamp = time_date(\"local\")\nprint(\"{a}; {b}; {local_stamp}\")\nwrite_text(\"clock.txt\", b)\nseal a\nseal b\n";

#[test]
fn leap_day_offsets_and_date_rollover() {
    assert_eq!(reading(0, "utc", 0).value, "29-02-2024:23:59:58");
    assert_eq!(reading(0, "local", 19800).value, "01-03-2024:05:29:58");
    assert_eq!(reading(0, "local", -18000).value, "29-02-2024:18:59:58");
    let before_epoch = Reading::at(
        0,
        "utc",
        Utc.with_ymd_and_hms(1969, 12, 31, 23, 59, 59).unwrap(),
        0,
    )
    .unwrap();
    assert_eq!(before_epoch.value, "31-12-1969:23:59:59");
}

#[test]
fn daylight_saving_repeat_preserves_distinct_instants() {
    let a = Reading::at(
        0,
        "local",
        Utc.with_ymd_and_hms(2026, 10, 25, 0, 30, 0).unwrap(),
        3600,
    )
    .unwrap();
    let b = Reading::at(
        1,
        "local",
        Utc.with_ymd_and_hms(2026, 10, 25, 1, 30, 0).unwrap(),
        0,
    )
    .unwrap();
    assert_eq!(a.value, b.value);
    assert_ne!(a.utc, b.utc);
    verify_evidence(&json!({"readings":[a,b]})).unwrap();
}

#[test]
fn unsupported_modes_offsets_and_years_refuse() {
    for mode in ["UTC", "", "GMT", "Europe/London"] {
        assert!(Clock::default().capture(mode).is_err());
    }
    assert!(Reading::at(0, "utc", Utc::now(), 3600).is_err());
    assert!(Reading::at(0, "local", Utc::now(), 86400).is_err());
    assert!(
        Reading::at(
            0,
            "utc",
            Utc.with_ymd_and_hms(10000, 1, 1, 0, 0, 0).unwrap(),
            0
        )
        .is_err()
    );
}

#[test]
fn replay_enforces_order_consumption_and_fixed_values() {
    let evidence = Evidence {
        readings: vec![reading(0, "utc", 0), reading(1, "local", 3600)],
    };
    let mut clock = Clock::replay(evidence.clone()).unwrap();
    assert!(clock.finish().is_err());
    assert!(clock.capture("local").is_err());
    assert_eq!(clock.capture("utc").unwrap(), evidence.readings[0].value);
    assert!(clock.finish().is_err());
    clock.capture("local").unwrap();
    clock.finish().unwrap();
    assert!(clock.capture("utc").is_err());
}

#[test]
fn clock_call_budget_is_bounded() {
    let mut clock = Clock::default();
    for _ in 0..goblinpp::clock::MAX_READINGS {
        clock.capture("utc").unwrap();
    }
    assert!(clock.capture("utc").unwrap_err().message.contains("limit"));
}

#[test]
fn evidence_rejects_tampered_fields_and_unknown_fields() {
    let evidence = json!({"readings":[reading(0,"utc",0)]});
    for (key, value) in [
        ("value", json!("wrong")),
        ("index", json!(1)),
        ("offset_seconds", json!(1)),
        ("timezone", json!("GMT")),
        ("utc", json!("invalid")),
        ("extra", json!(true)),
    ] {
        let mut altered = evidence.clone();
        altered["readings"][0][key] = value;
        assert!(verify_evidence(&altered).is_err(), "{key}");
    }
    assert!(verify_evidence(&json!({})).is_err());
}

#[test]
fn stored_timestamp_does_not_refresh_or_interpolate_again() {
    let parsed = parse_source(
        "a = time_date(\"utc\")\nb = time_date(\"local\")\nprint(a)\nprint(b)\nprint(a)\n",
    )
    .unwrap();
    let mut evaluation = Evaluation::new(".");
    evaluation.clock = Clock::replay(Evidence {
        readings: vec![reading(0, "utc", 0), reading(1, "local", 3600)],
    })
    .unwrap();
    let evaluation = evaluation.eval_program(&parsed.program).unwrap();
    assert_eq!(
        evaluation.stdout.join("\n"),
        "29-02-2024:23:59:58\n01-03-2024:00:59:58\n29-02-2024:23:59:58"
    );
}

#[test]
fn interpreter_without_paranoid_or_seals_records_clock() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("clock.gbl");
    fs::write(&source, "a = time_date()\nprint(a)\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PASS");
    assert_eq!(receipt["clock"]["readings"].as_array().unwrap().len(), 1);
    assert!(verify_run(&run).unwrap().verified);
}

#[test]
fn invalid_calls_and_shadowing_are_preserved_failures() {
    let dir = tempdir().unwrap();
    for (index, code) in [
        "a = time_date(1)",
        "a = time_date(\"UTC\")",
        "a = time_date(\"utc\", 1)",
        "time_date()",
        "g_func time_date() { return 1 }",
        "g_func f(time_date) { return time_date }",
    ]
    .iter()
    .enumerate()
    {
        let source = dir.path().join(format!("bad{index}.gbl"));
        fs::write(&source, code).unwrap();
        let run = run_file(&source, &RunOptions::default()).unwrap();
        assert_ne!(read_receipt(&run).unwrap()["status"], "PASS", "{code}");
        assert!(verify_run(&run).unwrap().verified, "{code}");
    }
}

#[test]
fn native_run_replays_reference_readings_and_exports_exactly() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("clock.gbl");
    fs::write(&source, SOURCE).unwrap();
    let run = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PASS", "{receipt:#}");
    assert_eq!(receipt["clock"]["readings"].as_array().unwrap().len(), 3);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(run.join("native-data.json")).unwrap()).unwrap();
    assert_eq!(manifest["clock"], receipt["clock"]);
    assert_eq!(
        fs::read_to_string(run.join("outputs/clock.txt")).unwrap(),
        format!(
            "{}\n",
            receipt["clock"]["readings"][1]["value"].as_str().unwrap()
        )
    );
    assert!(
        verify_run(&run).unwrap().verified,
        "{:#?}",
        verify_run(&run)
    );
}

#[test]
fn standalone_binary_reads_own_clock_and_preserves_manifest() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("standalone.gbl");
    fs::write(&source, "a = time_date(\"utc\")\nprint(a)\n").unwrap();
    let binary = dir.path().join("clock-native");
    let compiled = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["compile"])
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let run = Command::new(&binary)
        .current_dir(dir.path())
        .env_remove("GOBLIN_CLOCK_REPLAY_PATH")
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let output = String::from_utf8(run.stdout).unwrap();
    let diagnostics = String::from_utf8(run.stderr).unwrap();
    let path = diagnostics
        .lines()
        .find_map(|line| line.strip_prefix("NATIVE_DATA_DIR="))
        .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join(path).join("native-data.json")).unwrap())
            .unwrap();
    let evidence = verify_evidence(&manifest["clock"]).unwrap();
    assert!(output.contains(&evidence.readings[0].value));
}

#[test]
fn receipt_clock_tamper_fails_even_after_core_is_resealed() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("clock.gbl");
    fs::write(&source, SOURCE).unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    receipt["clock"]["readings"][0]["value"] = json!("wrong");
    reseal(&run, &mut receipt);
    assert!(!verify_run(&run).unwrap().verified);
}

#[test]
fn freeze_pins_clock_policy_and_old_non_clock_receipts_still_verify() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("clock.gbl");
    fs::write(&source, SOURCE).unwrap();
    create_freeze(&source).unwrap();
    assert!(verify_freeze(&source).verified);
    assert!(
        verify_run(run_file(&source, &RunOptions::default()).unwrap())
            .unwrap()
            .verified
    );
    let plain = dir.path().join("plain.gbl");
    fs::write(&plain, "print(\"hello\")\n").unwrap();
    let run = run_file(&plain, &RunOptions::default()).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    receipt["goblin_version"] = json!("0.1.0-alpha.28");
    receipt.as_object_mut().unwrap().remove("clock_policy");
    receipt.as_object_mut().unwrap().remove("clock");
    reseal(&run, &mut receipt);
    assert!(verify_run(&run).unwrap().verified);
}

#[test]
fn diff_labels_clock_inputs_without_claiming_runtime_divergence() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("clock.gbl");
    fs::write(&source, SOURCE).unwrap();
    let a = run_file(&source, &RunOptions::default()).unwrap();
    let b = run_file(&source, &RunOptions::default()).unwrap();
    let diff = diff_runs(&a, &b).unwrap();
    assert!(!diff.clock_readings_same);
    assert_eq!(diff.classification, "CLOCK_INPUT_CHANGE");
}

#[test]
fn captured_clock_survives_later_execution_failure() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("fail.gbl");
    fs::write(&source, "a = time_date(\"utc\")\nx = 1 / 0\n").unwrap();
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
        assert_eq!(receipt["status"], "MACHINERY_FAIL");
        assert_eq!(receipt["clock"]["readings"].as_array().unwrap().len(), 1);
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn native_replay_tamper_or_removed_declaration_cannot_verify() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("clock.gbl");
    fs::write(&source, SOURCE).unwrap();
    let run = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    let original = read_receipt(&run).unwrap();
    assert_eq!(original["status"], "PASS");
    let mut removed = original.clone();
    removed["execution"]["compiler"]
        .as_object_mut()
        .unwrap()
        .remove("clock_replay_path");
    removed["execution"]["compiler"]
        .as_object_mut()
        .unwrap()
        .remove("clock_replay_sha256");
    reseal(&run, &mut removed);
    assert!(!verify_run(&run).unwrap().verified);
    let mut restored = original;
    reseal(&run, &mut restored);
    fs::write(run.join("clock-replay.json"), b"{\"readings\":[]}").unwrap();
    assert!(!verify_run(&run).unwrap().verified);
}

#[test]
fn historical_freezes_require_revision_only_when_clock_is_used() {
    for uses_clock in [false, true] {
        let dir = tempdir().unwrap();
        let source = dir.path().join("frozen.gbl");
        fs::write(
            &source,
            if uses_clock {
                "a = time_date(\"utc\")\nprint(a)\n"
            } else {
                "print(\"hello\")\n"
            },
        )
        .unwrap();
        let (path, event) = create_freeze(&source).unwrap();
        let mut frozen: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        frozen["goblin_version"] = json!("0.1.0-alpha.28");
        frozen.as_object_mut().unwrap().remove("clock_policy");
        frozen
            .as_object_mut()
            .unwrap()
            .remove("freeze_receipt_sha256");
        frozen["freeze_receipt_sha256"] = json!(hash_canonical_json(&frozen).unwrap());
        let bytes = serde_json::to_vec_pretty(&frozen).unwrap();
        fs::write(&path, &bytes).unwrap();
        let mut payload = event.payload;
        payload["freeze_receipt_core_sha256"] = frozen["freeze_receipt_sha256"].clone();
        payload["freeze_receipt_file_sha256"] = json!(goblinpp::hashing::sha256_bytes(&bytes));
        goblinpp::ledger::append(
            goblinpp::ledger::project_root_for(&source).unwrap(),
            "FREEZE",
            event.subject,
            payload,
        )
        .unwrap();
        assert!(verify_freeze(&source).verified);
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
                receipt["status"],
                if uses_clock {
                    "PROTOCOL_VIOLATION"
                } else {
                    "PASS"
                }
            );
            assert!(verify_run(&run).unwrap().verified);
        }
        let result = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
            .arg("compile")
            .arg(&source)
            .arg("-o")
            .arg(dir.path().join("native"))
            .output()
            .unwrap();
        assert_eq!(result.status.success(), !uses_clock);
    }
}

#[test]
fn preview_labels_clock_without_registering_a_run() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("preview.gbl");
    fs::write(&source, "a = time_date(\"utc\")\nprint(a)\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .arg("check")
        .arg(&source)
        .arg("--json")
        .output()
        .unwrap();
    assert!(result.status.success());
    let preview: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        preview["clock_preview"]["readings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(preview["clock_policy"]["authority"], "not-certified-time");
    assert!(!dir.path().join("runs").exists());
    assert!(!dir.path().join(".goblin").exists());
}

#[test]
fn replay_files_are_bounded_and_symlinks_refuse() {
    let dir = tempdir().unwrap();
    let replay = dir.path().join("replay.json");
    fs::write(
        &replay,
        serde_json::to_vec(&Evidence {
            readings: vec![reading(0, "utc", 0)],
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        goblinpp::clock::read_replay(&replay)
            .unwrap()
            .readings
            .len(),
        1
    );
    fs::OpenOptions::new()
        .write(true)
        .open(&replay)
        .unwrap()
        .set_len(goblinpp::clock::MAX_REPLAY_BYTES + 1)
        .unwrap();
    assert!(goblinpp::clock::read_replay(&replay).is_err());
    #[cfg(unix)]
    {
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&replay, &link).unwrap();
        assert!(goblinpp::clock::read_replay(&link).is_err());
    }
}

#[test]
fn wrong_arity_refuses_before_argument_effects_and_native_generation() {
    let parsed = parse_source("a = time_date(input(\"should not prompt\"), \"utc\")\n").unwrap();
    let mut evaluation = Evaluation::new(".");
    evaluation.register_functions(&parsed.program).unwrap();
    assert!(evaluation.eval_stmt(&parsed.program.statements[0]).is_err());
    assert!(evaluation.clock.evidence().readings.is_empty());
    assert!(evaluation.stdout.is_empty());
    let dir = tempdir().unwrap();
    assert!(goblinpp::compiler::compile(&parsed, dir.path().join("native"), &[]).is_err());
}
