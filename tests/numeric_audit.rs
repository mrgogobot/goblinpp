use goblinpp::{
    audit::verify_run,
    custody::{create_freeze, verify_freeze},
    hashing::hash_canonical_json,
    numeric_audit::{self, Config},
    parser::parse_source,
    runtime::{RunOptions, read_receipt, run_file},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

const CODE: &str = "GO_PARANOID\nGO_NUMERIC_AUDIT \"audit.json\"\nx = 3 m\nresult = x * x\nprint(\"result = {result}\")\nseal result\n";
fn setup(code: &str, names: &[&str]) -> (TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("main.gbl");
    fs::write(&p, code).unwrap();
    fs::write(d.path().join("audit.json"),serde_json::to_vec(&json!({"schema":"goblin.numeric-audit-config.v1","comparison":"interpreter-native-same-bits","quantities":names})).unwrap()).unwrap();
    (d, p)
}
fn run(p: &Path, compiled: bool) -> PathBuf {
    run_file(
        p,
        &RunOptions {
            compile: compiled,
            ..Default::default()
        },
    )
    .unwrap()
}
fn reseal(run: &Path, r: &mut Value) {
    r.as_object_mut().unwrap().remove("receipt_core_sha256");
    r["receipt_core_sha256"] = json!(hash_canonical_json(r).unwrap());
    fs::write(run.join("receipt.json"), serde_json::to_vec(r).unwrap()).unwrap();
}

#[test]
fn compiled_scalar_audit_passes_and_offline_verifies() {
    let (d, p) = setup(CODE, &["result"]);
    let dir = run(&p, true);
    let r = read_receipt(&dir).unwrap();
    assert_eq!(r["status"], "PASS");
    assert_eq!(r["numeric_audit"]["status"], "PASS");
    assert_eq!(
        r["numeric_audit"]["comparisons"][0]["interpreter"]["dimension"],
        json!([0, 2, 0, 0, 0, 0])
    );
    assert!(verify_run(&dir).unwrap().verified);
    fs::remove_file(d.path().join("audit.json")).unwrap();
    assert!(verify_run(&dir).unwrap().verified);
}
#[test]
fn interpreted_only_is_preserved_error_not_unchecked_success() {
    let (_d, p) = setup(CODE, &["result"]);
    let dir = run(&p, false);
    let r = read_receipt(&dir).unwrap();
    assert_eq!(r["status"], "MACHINERY_FAIL");
    assert_eq!(r["numeric_audit"]["status"], "ERROR");
    assert!(r["sealed_artifacts"].as_array().unwrap().is_empty());
    assert!(verify_run(dir).unwrap().verified);
}
#[test]
fn missing_scalar_is_inconclusive_and_cannot_pass_run() {
    let (_d, p) = setup(CODE, &["absent"]);
    let dir = run(&p, true);
    let r = read_receipt(&dir).unwrap();
    assert_eq!(r["status"], "MACHINERY_FAIL");
    assert_eq!(r["numeric_audit"]["status"], "INCONCLUSIVE");
    assert!(verify_run(dir).unwrap().verified);
}
#[test]
fn arrays_are_not_silently_reduced_to_scalar() {
    let (_d, p) = setup(
        "GO_NUMERIC_AUDIT \"audit.json\"\nresult = [1,2]\nseal result\n",
        &["result"],
    );
    let dir = run(&p, true);
    assert_eq!(
        read_receipt(&dir).unwrap()["numeric_audit"]["status"],
        "INCONCLUSIVE"
    );
    assert!(verify_run(dir).unwrap().verified);
}
#[test]
fn missing_config_is_error_with_valid_failure_evidence() {
    let (d, p) = setup(CODE, &["result"]);
    fs::remove_file(d.path().join("audit.json")).unwrap();
    let dir = run(&p, true);
    assert_eq!(
        read_receipt(&dir).unwrap()["numeric_audit"]["status"],
        "ERROR"
    );
    assert!(verify_run(dir).unwrap().verified);
}
#[test]
fn configuration_requires_explicit_supported_mode_and_unique_names() {
    for config in [
        json!({}),
        json!({"schema":"goblin.numeric-audit-config.v1","comparison":"tolerance","quantities":["result"]}),
        json!({"schema":"goblin.numeric-audit-config.v1","comparison":"interpreter-native-same-bits","quantities":[]}),
        json!({"schema":"goblin.numeric-audit-config.v1","comparison":"interpreter-native-same-bits","quantities":["result","result"]}),
        json!({"schema":"goblin.numeric-audit-config.v1","comparison":"interpreter-native-same-bits","quantities":["result"],"relative_tolerance":1.0}),
    ] {
        let (d, p) = setup(CODE, &["result"]);
        fs::write(
            d.path().join("audit.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let dir = run(&p, true);
        assert_ne!(read_receipt(&dir).unwrap()["status"], "PASS");
        assert!(verify_run(dir).unwrap().verified);
    }
}
#[test]
fn config_bounds_duplicate_json_and_symlink_refuse() {
    for bytes in [
        vec![b' '; 16385],
        b"{\"schema\":\"x\",\"schema\":\"x\"}".to_vec(),
    ] {
        let (d, p) = setup(CODE, &["result"]);
        fs::write(d.path().join("audit.json"), bytes).unwrap();
        assert!(numeric_audit::snapshot(&p, &parse_source(CODE).unwrap().program).is_err());
    }
    #[cfg(unix)]
    {
        let (d, p) = setup(CODE, &["result"]);
        fs::rename(d.path().join("audit.json"), d.path().join("actual.json")).unwrap();
        std::os::unix::fs::symlink("actual.json", d.path().join("audit.json")).unwrap();
        assert!(numeric_audit::snapshot(&p, &parse_source(CODE).unwrap().program).is_err());
    }
}
#[test]
fn directive_is_top_level_once_and_local_only() {
    for code in [
        "GO_NUMERIC_AUDIT \"../a.json\"",
        "GO_NUMERIC_AUDIT \"/tmp/a.json\"",
        "GO_NUMERIC_AUDIT \"a.json\"\nGO_NUMERIC_AUDIT \"b.json\"",
        "if true { GO_NUMERIC_AUDIT \"a.json\" }",
        "g_func f() { GO_NUMERIC_AUDIT \"a.json\" return 1 }",
    ] {
        assert!(parse_source(code).is_err(), "{code}");
    }
}
#[test]
fn libraries_cannot_introduce_audit_effects() {
    let (d, p) = setup("import \"lib.gbl\"\nx = 1\n", &["x"]);
    fs::write(
        d.path().join("lib.gbl"),
        "GO_NUMERIC_AUDIT \"audit.json\"\n",
    )
    .unwrap();
    let dir = run(&p, true);
    assert_eq!(read_receipt(&dir).unwrap()["status"], "MACHINERY_FAIL");
    assert!(verify_run(dir).unwrap().verified);
}
#[test]
fn freeze_pins_exact_config_and_refuses_changed_rules() {
    let (d, p) = setup(CODE, &["result"]);
    create_freeze(&p).unwrap();
    assert!(verify_freeze(&p).verified);
    fs::write(d.path().join("audit.json"), "{}\n").unwrap();
    assert_eq!(
        verify_freeze(&p).classification,
        "NUMERIC_AUDIT_CONFIG_CHANGED_AFTER_FREEZE"
    );
    let dir = run(&p, true);
    let r = read_receipt(&dir).unwrap();
    assert_eq!(r["status"], "PROTOCOL_VIOLATION");
    assert_eq!(r["numeric_audit"]["status"], "ERROR");
    assert!(verify_run(dir).unwrap().verified);
}
#[test]
fn tampered_audit_rows_and_removed_policy_refuse_even_resealed() {
    let (_d, p) = setup(CODE, &["result"]);
    let dir = run(&p, true);
    let original = read_receipt(&dir).unwrap();
    for mutation in [0, 1, 2, 3] {
        let mut r = original.clone();
        match mutation {
            0 => {
                r.as_object_mut().unwrap().remove("numeric_audit");
            }
            1 => {
                r["numeric_audit"]["comparisons"][0]["native"]["bits_hex"] =
                    json!("0000000000000000")
            }
            2 => r["numeric_audit"]["status"] = json!("FAIL"),
            _ => r["numeric_audit"]["policy"] = json!("unknown"),
        };
        reseal(&dir, &mut r);
        assert!(!verify_run(&dir).unwrap().verified);
    }
}
#[test]
fn bad_config_snapshot_cannot_be_hidden_by_core_rehash() {
    let (_d, p) = setup(CODE, &["result"]);
    let dir = run(&p, true);
    fs::write(dir.join(numeric_audit::CONFIG_FILE), "{}").unwrap();
    assert!(!verify_run(dir).unwrap().verified);
}
#[test]
fn requested_audit_and_program_failure_remain_error() {
    let (_d, p) = setup(
        "GO_PARANOID\nGO_NUMERIC_AUDIT \"audit.json\"\nresult = 1 / 0\nseal result\n",
        &["result"],
    );
    let dir = run(&p, true);
    let r = read_receipt(&dir).unwrap();
    assert_eq!(r["numeric_audit"]["status"], "ERROR");
    assert!(verify_run(dir).unwrap().verified);
}
#[test]
fn missing_compiler_and_standalone_compilation_never_claim_audit() {
    let (d, p) = setup(CODE, &["result"]);
    let bin = env!("CARGO_BIN_EXE_goblinpp");
    let r = Command::new(bin)
        .arg("run")
        .arg(&p)
        .arg("--compile")
        .arg("--json")
        .env("PATH", d.path().join("no-tools"))
        .output()
        .unwrap();
    assert!(!r.status.success());
    let v: Value = serde_json::from_slice(&r.stdout).unwrap();
    assert_eq!(v["receipt"]["numeric_audit"]["status"], "ERROR");
    let c = Command::new(bin)
        .arg("compile")
        .arg(&p)
        .arg("-o")
        .arg(d.path().join("must-not-exist"))
        .output()
        .unwrap();
    assert!(!c.status.success());
    assert!(!d.path().join("must-not-exist").exists());
}
#[test]
fn preview_explains_no_comparison_and_creates_no_run() {
    let (d, p) = setup(CODE, &["result"]);
    let out = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .arg("check")
        .arg(&p)
        .arg("--json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v["numeric_audit_preview"]["status"],
        "NOT_PERFORMED_PREVIEW_ONLY"
    );
    assert!(!d.path().join("runs").exists());
    let human = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .arg("check")
        .arg(&p)
        .output()
        .unwrap();
    assert!(human.status.success());
    assert!(
        String::from_utf8(human.stdout)
            .unwrap()
            .contains("NUMERIC_AUDIT_STATUS=NOT_PERFORMED_PREVIEW_ONLY")
    );
}
#[test]
fn contextual_directive_does_not_reinterpret_historical_identifier() {
    assert!(parse_source("GO_NUMERIC_AUDIT = 2\nseal GO_NUMERIC_AUDIT\n").is_ok());
}
#[test]
fn synthetic_mismatch_signed_zero_and_dimension_disagreement_are_failures() {
    use goblinpp::{evaluator::Value as E, quantity::Quantity};
    let config = Config {
        schema: "goblin.numeric-audit-config.v1".into(),
        comparison: "interpreter-native-same-bits".into(),
        quantities: vec!["x".into()],
    };
    let base = numeric_audit::initial(Some(
        &parse_source("GO_NUMERIC_AUDIT \"a.json\"").unwrap().program,
    ));
    for (a, b, dim) in [
        (1.0, 2.0, [0; 6]),
        (0.0, -0.0, [0; 6]),
        (1.0, 1.0, [1, 0, 0, 0, 0, 0]),
    ] {
        let left = BTreeMap::from([("x".into(), E::Quantity(Quantity::scalar(a).unwrap()))]);
        let right = BTreeMap::from([("x".into(), E::Quantity(Quantity::new(b, dim).unwrap()))]);
        assert_eq!(
            numeric_audit::assess(&config, &left, &right, &base)["status"],
            "FAIL"
        );
    }
}

#[test]
fn historical_non_audit_receipt_and_freeze_remain_usable() {
    let (_d, p) = setup("GO_PARANOID\nx = 1\nseal x\n", &["x"]);
    let dir = run(&p, false);
    let current = read_receipt(&dir).unwrap();
    let mut historical = current.clone();
    historical["goblin_version"] = json!("0.1.0-alpha.36");
    historical.as_object_mut().unwrap().remove("numeric_audit");
    reseal(&dir, &mut historical);
    assert!(verify_run(&dir).unwrap().verified);
    assert_eq!(
        numeric_audit::comparison_evidence(&historical),
        numeric_audit::comparison_evidence(&current)
    );
    let (path, _) = create_freeze(&p).unwrap();
    let mut frozen: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    frozen.as_object_mut().unwrap().remove("numeric_audit");
    frozen
        .as_object_mut()
        .unwrap()
        .remove("freeze_receipt_sha256");
    frozen["freeze_receipt_sha256"] = json!(hash_canonical_json(&frozen).unwrap());
    fs::write(path, serde_json::to_vec(&frozen).unwrap()).unwrap();
    assert!(verify_freeze(p).verified);
}

#[test]
fn non_audit_marker_cannot_be_removed_from_new_receipts() {
    let (_d, p) = setup("x = 1\nseal x\n", &["x"]);
    let dir = run(&p, false);
    let mut r = read_receipt(&dir).unwrap();
    r.as_object_mut().unwrap().remove("numeric_audit");
    reseal(&dir, &mut r);
    assert!(!verify_run(dir).unwrap().verified);
}

#[test]
fn audit_diff_detects_changed_named_quantities() {
    let (d, p) = setup(CODE, &["result"]);
    let a = run(&p, true);
    let b = run(&p, true);
    let equal = goblinpp::audit::diff_runs(&a, &b).unwrap();
    assert!(equal.numeric_audit_same);
    assert_eq!(equal.classification, "IDENTICAL_RESULT");
    fs::write(d.path().join("audit.json"), serde_json::to_vec(&json!({"schema":"goblin.numeric-audit-config.v1","comparison":"interpreter-native-same-bits","quantities":["absent"]})).unwrap()).unwrap();
    let c = run(&p, true);
    let changed = goblinpp::audit::diff_runs(&a, &c).unwrap();
    assert!(!changed.numeric_audit_same);
    assert_eq!(changed.classification, "NUMERIC_AUDIT_CHANGE");
}

#[test]
fn audit_reuses_seeded_rng_and_clock_replay_contracts() {
    let (_d, p) = setup(
        "GO_PARANOID\nGO_NUMERIC_AUDIT \"audit.json\"\nrng_seed(42, 7)\nresult = rng_uniform(7)\nstamp = time_date(\"utc\")\nprint(\"{stamp}: {result}\")\nseal result\n",
        &["result"],
    );
    let dir = run(&p, true);
    let r = read_receipt(&dir).unwrap();
    assert_eq!(r["status"], "PASS", "{r}");
    assert_eq!(r["numeric_audit"]["status"], "PASS");
    assert!(verify_run(dir).unwrap().verified);
}

#[test]
fn late_failure_downgrades_audit_success_but_not_a_real_mismatch() {
    for (status, expected) in [
        ("PASS", "ERROR"),
        ("FAIL", "FAIL"),
        ("INCONCLUSIVE", "INCONCLUSIVE"),
    ] {
        let mut receipt = json!({"numeric_audit":{"status":status}});
        numeric_audit::run_failed(&mut receipt);
        assert_eq!(receipt["numeric_audit"]["status"], expected);
    }
}

#[test]
fn inline_rust_is_refused_before_any_reference_execution() {
    let (_d, p) = setup(
        "GO_NUMERIC_AUDIT \"audit.json\"\nresult = 9\nseal result\nRUST_INLINE_BEGIN\npanic!(\"must never execute\");\nRUST_INLINE_END\n",
        &["result"],
    );
    let dir = run(&p, true);
    let r = read_receipt(&dir).unwrap();
    assert_eq!(r["numeric_audit"]["status"], "ERROR");
    assert!(r["sealed_artifacts"].as_array().unwrap().is_empty());
    assert!(verify_run(dir).unwrap().verified);
}

#[test]
fn synthetic_failed_comparison_reconstructs_without_becoming_success() {
    // Synthetic corrupted native result: not an observed platform discrepancy.
    let (_d, p) = setup(CODE, &["result"]);
    let dir = run(&p, true);
    let mut r = read_receipt(&dir).unwrap();
    let path = dir.join("native-results.tsv");
    let original = fs::read_to_string(&path).unwrap();
    let altered = original
        .lines()
        .map(|line| {
            if line.starts_with("Q\t726573756c74\t") {
                format!("Q\t726573756c74\t{:016x}\t0,2,0,0,0,0", 10.0_f64.to_bits())
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_ne!(original, altered);
    fs::write(&path, altered).unwrap();
    r["status"] = json!("MACHINERY_FAIL");
    r["numeric_audit"]["status"] = json!("FAIL");
    r["numeric_audit"]["comparisons"][0]["status"] = json!("FAIL");
    r["numeric_audit"]["comparisons"][0]["native"]["value_si"] = json!(10.0);
    r["numeric_audit"]["comparisons"][0]["native"]["bits_hex"] =
        json!(format!("{:016x}", 10.0_f64.to_bits()));
    assert!(numeric_audit::verify(&dir, &r).is_ok());
    r["status"] = json!("PASS");
    assert!(numeric_audit::verify(&dir, &r).is_err());
    r["status"] = json!("MACHINERY_FAIL");
    r["numeric_audit"]["status"] = json!("PASS");
    assert!(numeric_audit::verify(&dir, &r).is_err());
}

#[test]
fn audited_table_and_library_use_preserved_data_and_named_scalar() {
    let (d, p) = setup(
        "GO_PARANOID\nGO_NUMERIC_AUDIT \"audit.json\"\nimport \"analysis.gbl\"\nvalues = csv_numbers(\"data.csv\", \"length\")\nresult = measured_mean(values)\nprint(\"result = {result}\")\nseal result\n",
        &["result"],
    );
    fs::write(
        d.path().join("analysis.gbl"),
        "g_func measured_mean(values) { return mean(values) * 1 m }\n",
    )
    .unwrap();
    fs::write(d.path().join("data.csv"), "length\n10\n11\n12\n").unwrap();
    let dir = run(&p, true);
    let r = read_receipt(&dir).unwrap();
    assert_eq!(r["status"], "PASS", "{r}");
    assert_eq!(
        r["numeric_audit"]["comparisons"][0]["native"]["value_si"],
        11.0
    );
    assert_eq!(r["data_imports"].as_array().unwrap().len(), 1);
    fs::remove_file(d.path().join("data.csv")).unwrap();
    fs::remove_file(d.path().join("analysis.gbl")).unwrap();
    assert!(verify_run(dir).unwrap().verified);
}
