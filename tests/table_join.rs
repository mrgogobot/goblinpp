use goblinpp::{
    audit::verify_run,
    custody::{create_freeze, verify_freeze},
    delimited::Table,
    hashing::{hash_canonical_json, sha256_bytes},
    runtime::{RunOptions, read_receipt, run_file},
    table_join::{Rules, join},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn table(text: &str, delimiter: u8) -> Table {
    Table::from_bytes(Path::new("fixture"), text.as_bytes().to_vec(), delimiter).unwrap()
}
fn rules(kind: &str) -> Rules {
    Rules {
        function: "csv_join".into(),
        left_key: "id".into(),
        right_key: "id".into(),
        kind: kind.into(),
        left_prefix: "L_".into(),
        right_prefix: "R_".into(),
    }
}
const LEFT: &str = "id,value\n9007199254740993,a\n9007199254740992,b\n0007,c\n7,d\n";
const RIGHT: &str = "id,value\n7,four\n9007199254740992,two\n9007199254740993,one\n8,unused\n";
const CODE: &str = "GO_PARANOID\nrows = csv_join(\"joined.csv\", \"left.csv\", \"right.csv\", \"id\", \"id\", \"left\", \"L_\", \"R_\")\nprint(\"rows = {rows}\")\nseal rows\n";
fn setup(code: &str) -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("left.csv"), LEFT).unwrap();
    fs::write(d.path().join("right.csv"), RIGHT).unwrap();
    let path = d.path().join("main.gbl");
    fs::write(&path, code).unwrap();
    (d, path)
}
fn run(path: &Path, compile: bool) -> PathBuf {
    run_file(
        path,
        &RunOptions {
            compile,
            ..Default::default()
        },
    )
    .unwrap()
}
fn reseal(run: &Path, receipt: &mut Value) {
    receipt
        .as_object_mut()
        .unwrap()
        .remove("receipt_core_sha256");
    receipt["receipt_core_sha256"] = json!(hash_canonical_json(receipt).unwrap());
    fs::write(
        run.join("receipt.json"),
        serde_json::to_vec(receipt).unwrap(),
    )
    .unwrap();
}

#[test]
fn left_join_exact_ids_and_order_have_independent_literal_expectations() {
    let output = join(
        "joined.csv",
        &table(LEFT, b','),
        &table(RIGHT, b','),
        &rules("left"),
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(output.bytes).unwrap(),
        "__join_matched,L_id,L_value,R_id,R_value\ntrue,9007199254740993,a,9007199254740993,one\ntrue,9007199254740992,b,9007199254740992,two\nfalse,0007,c,,\ntrue,7,d,7,four\n"
    );
    assert_eq!(output.metadata["matched_rows"], 3);
    assert_eq!(output.metadata["unmatched_left_rows"], 1);
    assert_eq!(output.metadata["unmatched_right_rows"], 1);
    assert_eq!(output.metadata["dropped_left_rows"], 0);
}
#[test]
fn inner_join_counts_intentional_row_loss() {
    let output = join(
        "joined.csv",
        &table(LEFT, b','),
        &table(RIGHT, b','),
        &rules("inner"),
    )
    .unwrap();
    assert_eq!(output.metadata["output_rows"], 3);
    assert_eq!(output.metadata["dropped_left_rows"], 1);
    assert!(!String::from_utf8(output.bytes).unwrap().contains("0007"));
}
#[test]
fn ids_are_not_trimmed_case_folded_normalized_or_parsed_as_numbers() {
    let text = "id\n18446744073709551615\n001\n1\n 1\nA\na\né\né\n1.0\n1e0\n";
    let output = join(
        "j.csv",
        &table(text, b','),
        &table(text, b','),
        &rules("inner"),
    )
    .unwrap();
    assert_eq!(output.metadata["matched_rows"], 10);
    let output = join(
        "j.csv",
        &table("id\n001\n 1\nA\né\n1.0\n", b','),
        &table("id\n1\na\né\n1e0\n", b','),
        &rules("inner"),
    )
    .unwrap();
    assert_eq!(output.metadata["matched_rows"], 0);
}
#[test]
fn duplicates_are_errors_on_both_sides_even_if_unmatched() {
    for side in ["left", "right"] {
        let good = table("id\n1\n", b',');
        let bad = table("id\n9\n9\n", b',');
        let (left, right) = if side == "left" {
            (&bad, &good)
        } else {
            (&good, &bad)
        };
        let error = join("j.csv", left, right, &rules("inner"))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(side) && error.contains("rows 1 and 2") && error.contains("duplicate"),
            "{error}"
        );
    }
}
#[test]
fn missing_empty_and_fits_null_keys_are_refused_on_either_side() {
    for key in ["", "\\N"] {
        for side in ["left", "right"] {
            let bad = table(&format!("id,value\n{key},x\n"), b',');
            let good = table("id,value\n1,x\n", b',');
            let (left, right) = if side == "left" {
                (&bad, &good)
            } else {
                (&good, &bad)
            };
            let error = join("j.csv", left, right, &rules("left"))
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("missing key") && error.contains(side) && error.contains("row 1")
            );
        }
    }
}
#[test]
fn matched_empty_cells_are_distinguishable_from_missing_rows() {
    let output = join(
        "j.csv",
        &table("id\n1\n2\n", b','),
        &table("id,v\n1,\n", b','),
        &rules("left"),
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(output.bytes).unwrap(),
        "__join_matched,L_id,R_id,R_v\ntrue,1,1,\nfalse,2,,\n"
    );
}
#[test]
fn csv_and_tsv_quoting_preserve_multiline_tab_and_quote_cells() {
    for delimiter in [b',', b'\t'] {
        let d = delimiter as char;
        let text = format!("\u{feff}id{d}v\r\n1{d}\"tabs\tand,commas\r\nquotes\"\"π\"\r\n");
        let input = table(&text, delimiter);
        let mut r = rules("left");
        r.function = if delimiter == b',' {
            "csv_join"
        } else {
            "tsv_join"
        }
        .into();
        let output = join(
            if delimiter == b',' { "j.csv" } else { "j.tsv" },
            &input,
            &input,
            &r,
        )
        .unwrap();
        let parsed = Table::from_bytes(Path::new("joined"), output.bytes, delimiter).unwrap();
        assert_eq!(parsed.rows[0][2], input.rows[0][1]);
        assert_eq!(parsed.rows[0][4], input.rows[0][1]);
    }
}
#[test]
fn absent_headers_empty_tables_and_separate_key_names() {
    let mut r = rules("inner");
    r.right_key = "source_id".into();
    assert!(
        join(
            "j.csv",
            &table("id\n1\n", b','),
            &table("id\n1\n", b','),
            &r
        )
        .is_err()
    );
    let result = join(
        "j.csv",
        &table("id\n1\n", b','),
        &table("source_id\n1\n", b','),
        &r,
    )
    .unwrap();
    assert_eq!(result.metadata["matched_rows"], 1);
    for kind in ["inner", "left"] {
        let output = join(
            "j.csv",
            &table("id\n", b','),
            &table("id\n", b','),
            &rules(kind),
        )
        .unwrap();
        assert_eq!(output.metadata["output_rows"], 0);
        assert_eq!(
            String::from_utf8(output.bytes).unwrap(),
            "__join_matched,L_id,R_id\n"
        );
    }
}
#[test]
fn collision_mode_prefix_extension_and_path_rules_are_explicit() {
    let t = table("id\n1\n", b',');
    for kind in ["outer", "LEFT", ""] {
        assert!(join("j.csv", &t, &t, &rules(kind)).is_err());
    }
    for prefix in ["", "\n", &"x".repeat(129)] {
        let mut r = rules("left");
        r.left_prefix = prefix.into();
        assert!(join("j.csv", &t, &t, &r).is_err());
    }
    let mut r = rules("left");
    r.right_prefix = r.left_prefix.clone();
    assert!(
        join("j.csv", &t, &t, &r)
            .unwrap_err()
            .to_string()
            .contains("collide")
    );
    for output in ["j.tsv", "../j.csv", "/j.csv", "j.CSV"] {
        assert!(join(output, &t, &t, &rules("left")).is_err());
    }
    // Different prefixes can still collide with existing header text.
    r.left_prefix = "a".into();
    r.right_prefix = "ab".into();
    r.left_key = "bc".into();
    r.right_key = "c".into();
    assert!(join("j.csv", &table("bc\n1\n", b','), &table("c\n1\n", b','), &r).is_err());
}
#[test]
fn bounded_output_refuses_expansion_before_publishing() {
    let headers = (0..10)
        .map(|i| format!("v{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut text = format!("id,{headers}\n");
    for i in 0..50_000 {
        text.push_str(&format!("{i},0,0,0,0,0,0,0,0,0,0\n"));
    }
    let t = table(&text, b',');
    assert!(
        join("j.csv", &t, &t, &rules("left"))
            .unwrap_err()
            .to_string()
            .contains("1,000,000 cells")
    );
    let text = format!("id,v\n1,{}\n", "x".repeat(9 * 1024 * 1024));
    let t = table(&text, b',');
    assert!(
        join("j.csv", &t, &t, &rules("left"))
            .unwrap_err()
            .to_string()
            .contains("16 MiB")
    );
}
#[test]
fn engines_freezes_and_standalone_native_have_identical_join_outputs() {
    let (d, path) = setup(CODE);
    create_freeze(&path).unwrap();
    assert!(verify_freeze(&path).verified);
    let interpreted = run(&path, false);
    let native = run(&path, true);
    let a = read_receipt(&interpreted).unwrap();
    let b = read_receipt(&native).unwrap();
    for (dir, r) in [(&interpreted, &a), (&native, &b)] {
        assert_eq!(
            r["status"],
            "PASS",
            "{}",
            fs::read_to_string(dir.join("stderr.log")).unwrap()
        );
        assert!(verify_run(dir).unwrap().verified);
    }
    assert_eq!(a["generated_artifacts"], b["generated_artifacts"]);
    assert_eq!(a["sealed_artifacts"], b["sealed_artifacts"]);
    let expected = join(
        "joined.csv",
        &table(LEFT, b','),
        &table(RIGHT, b','),
        &rules("left"),
    )
    .unwrap()
    .bytes;
    assert_eq!(
        fs::read(native.join("outputs/joined.csv")).unwrap(),
        expected
    );
    let output = Command::new(native.join("program-native"))
        .current_dir(d.path())
        .env("PATH", "/no-engine")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"rows = 4\n");
    fs::remove_file(d.path().join("left.csv")).unwrap();
    fs::remove_file(d.path().join("right.csv")).unwrap();
    assert!(verify_run(&interpreted).unwrap().verified);
    assert!(verify_run(&native).unwrap().verified);
}
#[test]
fn tsv_native_join_retains_text_and_matches_interpreter() {
    let (d, path) = setup(&CODE.replace("csv_join", "tsv_join").replace(".csv", ".tsv"));
    fs::write(
        d.path().join("left.tsv"),
        "id\tv\n1\t\"line\nwith\ttabs\"\n",
    )
    .unwrap();
    fs::write(d.path().join("right.tsv"), "id\tv\n1\t\"quote\"\"π\"\n").unwrap();
    let a = run(&path, false);
    let b = run(&path, true);
    assert_eq!(
        read_receipt(&b).unwrap()["status"],
        "PASS",
        "{}",
        fs::read_to_string(b.join("stderr.log")).unwrap()
    );
    assert!(verify_run(&b).unwrap().verified);
    assert_eq!(
        fs::read(a.join("outputs/joined.tsv")).unwrap(),
        fs::read(b.join("outputs/joined.tsv")).unwrap()
    );
}
#[test]
fn failures_are_preserved_without_partial_join_outputs() {
    for right in [
        "id,value\n1,x\n1,y\n",
        "id,value\n,x\n",
        "not_id,value\n1,x\n",
    ] {
        let (d, path) = setup(CODE);
        fs::write(d.path().join("right.csv"), right).unwrap();
        let dir = run(&path, false);
        let r = read_receipt(&dir).unwrap();
        assert_eq!(r["status"], "MACHINERY_FAIL");
        assert_eq!(r["data_imports"].as_array().unwrap().len(), 2);
        assert!(r["generated_artifacts"].as_array().unwrap().is_empty());
        assert!(verify_run(&dir).unwrap().verified);
    }
}
#[test]
fn arity_is_checked_before_argument_effects_and_invalid_rules_before_input_reads() {
    for code in [
        "csv_join(write_text(\"bad.txt\",\"no\"))",
        "csv_join(\"j.csv\",\"missing.csv\",\"missing.csv\",\"id\",\"id\",\"outer\",\"L_\",\"R_\")",
        "csv_join(\"j.csv\",\"missing.csv\",\"missing.csv\",\"id\",\"id\",\"left\",1,\"R_\")",
    ] {
        let (_d, path) = setup(code);
        let dir = run(&path, false);
        let r = read_receipt(&dir).unwrap();
        assert_eq!(r["status"], "MACHINERY_FAIL");
        assert!(r["generated_artifacts"].as_array().unwrap().is_empty());
        assert!(r["data_imports"].as_array().unwrap().is_empty());
    }
}
#[test]
fn redefinition_is_execution_error_without_changing_historical_parsing() {
    let code = "g_func csv_join(x) { return x }\nx=csv_join(1)";
    assert!(goblinpp::parser::parse_source(code).is_ok());
    let (_d, path) = setup(code);
    let dir = run(&path, false);
    assert_eq!(read_receipt(&dir).unwrap()["status"], "MACHINERY_FAIL");
    assert!(verify_run(dir).unwrap().verified);
}
#[test]
fn duplicate_output_names_and_batch_input_mixing_are_refused() {
    for code in [
        format!("write_csv(\"joined.csv\",1,\"id\",\"1\")\n{CODE}"),
        format!("{CODE}\n{CODE}"),
        format!("cursor=csv_batch_open(\"left.csv\",[\"id\"],2)\n{CODE}"),
    ] {
        let (_d, path) = setup(&code);
        let dir = run(&path, false);
        assert_eq!(read_receipt(&dir).unwrap()["status"], "MACHINERY_FAIL");
    }
}
#[test]
fn offline_reconstruction_detects_rehashed_count_rules_output_and_descriptor_tampering() {
    for mutation in ["count", "rules", "bytes", "producer", "removed", "policy"] {
        let (_d, path) = setup(CODE);
        let dir = run(&path, false);
        let mut r = read_receipt(&dir).unwrap();
        match mutation {
            "count" => r["generated_artifacts"][0]["metadata"]["matched_rows"] = json!(4),
            "rules" => r["generated_artifacts"][0]["metadata"]["rules"]["kind"] = json!("inner"),
            "policy" => {
                r["generated_artifacts"][0]["metadata"]["table_join_policy"] = json!("future")
            }
            "producer" => r["generated_artifacts"][0]["producer"] = json!("write_csv"),
            "removed" => r["generated_artifacts"] = json!([]),
            "bytes" => {
                let bytes = b"id\nwrong\n";
                fs::write(dir.join("outputs/joined.csv"), bytes).unwrap();
                r["generated_artifacts"][0]["sha256"] = json!(sha256_bytes(bytes));
                r["generated_artifacts"][0]["byte_count"] = json!(bytes.len());
            }
            _ => unreachable!(),
        }
        reseal(&dir, &mut r);
        let v = verify_run(&dir).unwrap();
        assert!(!v.verified, "mutation {mutation}");
        assert!(
            v.checks
                .iter()
                .any(|c| c.check == "TABLE_JOIN_RECONSTRUCTION" && !c.pass)
        );
    }
}
#[test]
fn offline_reconstruction_checks_inputs_roles_and_safe_paths() {
    for mutation in ["input", "path", "role", "format"] {
        let (_d, path) = setup(CODE);
        let dir = run(&path, false);
        let mut r = read_receipt(&dir).unwrap();
        match mutation {
            "input" => {
                let input = r["data_imports"][0]["evidence_path"].as_str().unwrap();
                fs::remove_file(dir.join(input)).unwrap();
                fs::write(dir.join(input), "id,value\n9,x\n").unwrap();
            }
            "path" => r["data_imports"][0]["evidence_path"] = json!("../left.csv"),
            "role" => r["data_imports"][0]["access"] = json!([]),
            "format" => r["data_imports"][0]["format"] = json!("TSV-UTF8"),
            _ => unreachable!(),
        }
        reseal(&dir, &mut r);
        assert!(!verify_run(&dir).unwrap().verified);
    }
}
#[test]
fn self_join_and_eager_and_scan_cache_use_one_input_identity() {
    let (_d, path) = setup(
        "before=csv_rows(\"left.csv\")\nx=csv_join(\"joined.csv\",\"left.csv\",\"left.csv\",\"id\",\"id\",\"inner\",\"L_\",\"R_\")\nafter=csv_column(\"left.csv\",\"id\")\nseal x",
    );
    let dir = run(&path, false);
    let r = read_receipt(&dir).unwrap();
    assert_eq!(r["status"], "PASS");
    assert_eq!(r["data_imports"].as_array().unwrap().len(), 1);
    assert!(verify_run(dir).unwrap().verified);
}

#[test]
fn policy_is_mandatory_for_new_receipts_and_frozen_receipts_but_legacy_is_supported() {
    for mutation in [None, Some(json!("future"))] {
        let (_d, path) = setup(CODE);
        let dir = run(&path, false);
        let mut r = read_receipt(&dir).unwrap();
        match &mutation {
            None => {
                r.as_object_mut().unwrap().remove("table_join_policy");
            }
            Some(policy) => {
                r["table_join_policy"] = policy.clone();
            }
        }
        reseal(&dir, &mut r);
        assert!(!verify_run(&dir).unwrap().verified);
        let (freeze_path, _) = create_freeze(&path).unwrap();
        let mut frozen: Value = serde_json::from_slice(&fs::read(&freeze_path).unwrap()).unwrap();
        match &mutation {
            None => {
                frozen.as_object_mut().unwrap().remove("table_join_policy");
            }
            Some(policy) => {
                frozen["table_join_policy"] = policy.clone();
            }
        }
        frozen
            .as_object_mut()
            .unwrap()
            .remove("freeze_receipt_sha256");
        frozen["freeze_receipt_sha256"] = json!(hash_canonical_json(&frozen).unwrap());
        fs::write(&freeze_path, serde_json::to_vec(&frozen).unwrap()).unwrap();
        assert!(!verify_freeze(&path).verified);
        assert_eq!(
            read_receipt(run(&path, false)).unwrap()["status"],
            "PROTOCOL_VIOLATION"
        );
    }
    assert!(goblinpp::table_join::supports_receipt(
        &json!({"goblin_version":"0.1.0-alpha.37"})
    ));
    assert!(goblinpp::table_join::supports_receipt(
        &json!({"goblin_version":"0.0.3"})
    ));
}

#[test]
fn scan_and_join_share_snapshot_identity_and_delimiter_conflicts_are_refused() {
    let (d, path) = setup(&format!(
        "summary=csv_scan_stats(\"left.csv\",\"value\")\n{CODE}"
    ));
    fs::write(
        d.path().join("left.csv"),
        "id,value\n9007199254740993,1\n9007199254740992,2\n0007,3\n7,4\n",
    )
    .unwrap();
    let dir = run(&path, false);
    let r = read_receipt(&dir).unwrap();
    assert_eq!(
        r["status"],
        "PASS",
        "{}",
        fs::read_to_string(dir.join("stderr.log")).unwrap()
    );
    assert_eq!(r["data_imports"].as_array().unwrap().len(), 2);
    assert!(verify_run(dir).unwrap().verified);
    let (_d, path) = setup(
        "before=csv_rows(\"left.csv\")\nx=tsv_join(\"j.tsv\",\"left.csv\",\"right.csv\",\"id\",\"id\",\"left\",\"L_\",\"R_\")\n",
    );
    let dir = run(&path, false);
    assert_eq!(read_receipt(&dir).unwrap()["status"], "MACHINERY_FAIL");
    assert!(
        fs::read_to_string(dir.join("stderr.log"))
            .unwrap()
            .contains("delimiter")
    );
}

#[cfg(unix)]
#[test]
fn input_and_offline_evidence_symlinks_are_refused() {
    use std::os::unix::fs::symlink;
    let (d, path) = setup(CODE);
    let dir = run(&path, false);
    let r = read_receipt(&dir).unwrap();
    let snapshot = dir.join(r["data_imports"][0]["evidence_path"].as_str().unwrap());
    let saved = fs::read(&snapshot).unwrap();
    let replacement = d.path().join("replacement.csv");
    fs::write(&replacement, saved).unwrap();
    fs::remove_file(&snapshot).unwrap();
    symlink(&replacement, &snapshot).unwrap();
    assert!(!verify_run(&dir).unwrap().verified);
    fs::remove_file(d.path().join("left.csv")).unwrap();
    symlink(&replacement, d.path().join("left.csv")).unwrap();
    let dir = run(&path, false);
    assert_eq!(read_receipt(&dir).unwrap()["status"], "MACHINERY_FAIL");
}

#[test]
fn historical_receipts_may_predate_generated_files_and_data_imports() {
    let legacy = json!({"goblin_version":"0.0.2","status":"PASS"});
    assert!(goblinpp::table_join::verify(Path::new("unused"), &legacy).is_ok());
    for field in ["generated_artifacts", "data_imports"] {
        let mut malformed = legacy.clone();
        malformed[field] = json!({});
        assert!(goblinpp::table_join::verify(Path::new("unused"), &malformed).is_err());
    }
}
