use goblinpp::audit::verify_run;
use goblinpp::batches::{Batches, MAX_BATCH_BYTES, MAX_FILE_BYTES, policy};
use goblinpp::custody::{create_freeze, freeze_path, verify_freeze};
use goblinpp::evaluator::Value;
use goblinpp::hashing::{hash_canonical_json, sha256_file};
use goblinpp::parser::parse_source;
use goblinpp::quantity::Quantity;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::json;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn number(n: u64) -> Value {
    Value::Quantity(Quantity::scalar(n as f64).unwrap())
}
fn columns(names: &[&str]) -> Value {
    Value::Array(names.iter().map(|s| text(s)).collect())
}
fn reader(b: &mut Batches, base: &std::path::Path, rows: u64, bytes: u64) -> Value {
    b.call(
        base,
        "csv_batch_open",
        &[
            text("data.csv"),
            columns(&["id", "value"]),
            number(rows),
            number(bytes),
            number(MAX_FILE_BYTES),
        ],
    )
    .unwrap()
}
fn reseal(run: &std::path::Path, mut receipt: serde_json::Value) {
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
fn run(base: &std::path::Path, source: &str, compile: bool) -> std::path::PathBuf {
    let path = base.join("main.gbl");
    fs::write(&path, source).unwrap();
    run_file(
        &path,
        &RunOptions {
            compile,
            ..RunOptions::default()
        },
    )
    .unwrap()
}
const COPY: &str = "GO_PARANOID\nreader = csv_batch_open(\"data.csv\", [\"id\",\"value\"], 2, 4096, 2147483648)\nwriter = csv_stream_open(\"copy.csv\", [\"id\",\"value\"], 2147483648)\ncount = 0\nwhile true {\n batch = batch_next(reader)\n if len(batch) == 0 { break }\n count = count + stream_write(writer, batch)\n batch = []\n}\nbatch_close(reader)\nstream_close(writer)\nprint(count)\nseal count";

#[test]
fn serial_copy_matches_both_engines_with_exact_ids_and_quoted_records() {
    let dir = tempdir().unwrap();
    let input = "id,value,unused\r\n9007199254740993,\"a,b\",x\r\n18446744073709551615,\"line\nπ\",x\r\n42,\"a\"\"b\",x\r\n0,,x\r\n";
    fs::write(dir.path().join("data.csv"), input).unwrap();
    let mut hashes = Vec::new();
    for compile in [false, true] {
        let path = run(dir.path(), COPY, compile);
        let receipt = read_receipt(&path).unwrap();
        assert_eq!(receipt["status"], "PASS", "{receipt}");
        let verification = verify_run(&path).unwrap();
        assert!(verification.verified, "{:?}", verification.checks);
        assert_eq!(receipt["batches"]["readers"][0]["rows"], 4);
        assert_eq!(receipt["batches"]["readers"][0]["batches"], 2);
        assert_eq!(
            receipt["generated_artifacts"][0]["metadata"]["complete"],
            true
        );
        let output = path.join("outputs/copy.csv");
        hashes.push(sha256_file(&output).unwrap());
        assert_eq!(
            fs::read_to_string(output).unwrap(),
            "id,value\n9007199254740993,\"a,b\"\n18446744073709551615,\"line\nπ\"\n42,\"a\"\"b\"\n0,\n"
        );
    }
    assert_eq!(hashes[0], hashes[1]);
}

#[test]
fn byte_limited_batches_keep_pending_rows_and_never_emit_false_eof() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("data.csv"),
        "value,id,unused\nπ,1,x\ntext,2,x\nlong,3,x\n",
    )
    .unwrap();
    let mut b = Batches::default();
    let handle = reader(&mut b, dir.path(), 100, 220);
    let mut collected = Vec::new();
    loop {
        let Value::Array(cells) = b
            .call(dir.path(), "batch_next", std::slice::from_ref(&handle))
            .unwrap()
        else {
            panic!()
        };
        if cells.is_empty() {
            break;
        }
        collected.extend(cells.into_iter().map(|v| v.render()));
    }
    b.call(dir.path(), "batch_close", &[handle]).unwrap();
    b.finish().unwrap();
    assert_eq!(collected, ["1", "π", "2", "text", "3", "long"]);
    assert!(
        b.evidence()["readers"][0]["peak_batch_bytes"]
            .as_u64()
            .unwrap()
            <= 220
    );
    assert!(b.evidence()["readers"][0]["batches"].as_u64().unwrap() > 1);
}

#[test]
fn quoted_record_crossing_io_buffer_bom_and_tsv_roundtrip() {
    let dir = tempdir().unwrap();
    let payload = "π".repeat(40_000) + "\tquote\"\nline";
    let input = format!(
        "\u{feff}id\tvalue\r\n18446744073709551615\t\"{}\"\r\n",
        payload.replace('"', "\"\"")
    );
    fs::write(dir.path().join("data.tsv"), input).unwrap();
    let source = COPY
        .replace("csv_batch_open", "tsv_batch_open")
        .replace("csv_stream_open", "tsv_stream_open")
        .replace("data.csv", "data.tsv")
        .replace("copy.csv", "copy.tsv")
        .replace("2, 4096", "2, 262144");
    for compile in [false, true] {
        let path = run(dir.path(), &source, compile);
        let receipt = read_receipt(&path).unwrap();
        assert_eq!(receipt["status"], "PASS", "{receipt}");
        assert!(verify_run(&path).unwrap().verified);
        let output = fs::read_to_string(path.join("outputs/copy.tsv")).unwrap();
        assert!(output.contains("18446744073709551615\t"));
        assert!(output.contains("quote\"\"\nline"));
    }
}

#[test]
fn late_malformed_row_preserves_partial_output_and_full_snapshot() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.csv"), "id,value\n1,one\n2,two\n3\n").unwrap();
    for compile in [false, true] {
        let path = run(dir.path(), COPY, compile);
        let receipt = read_receipt(&path).unwrap();
        assert_eq!(receipt["status"], "MACHINERY_FAIL", "{receipt}");
        assert!(verify_run(&path).unwrap().verified);
        assert_eq!(
            fs::read_to_string(path.join("outputs/copy.csv.partial")).unwrap(),
            "id,value\n1,one\n2,two\n"
        );
        assert_eq!(
            receipt["generated_artifacts"][0]["metadata"]["complete"],
            false
        );
        assert_eq!(receipt["batches"]["readers"][0]["failed"], true);
        assert_eq!(
            receipt["data_imports"][0]["sha256"],
            sha256_file(dir.path().join("data.csv")).unwrap()
        );
    }
}

#[test]
fn unfinished_lifecycles_and_explicit_failure_never_pass() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.csv"), "id,value\n1,one\n").unwrap();
    for source in [
        COPY.replace("batch_close(reader)", ""),
        COPY.replace("stream_close(writer)", ""),
        COPY.replace(
            "batch_close(reader)",
            "bad = parse_number(\"stop\")\nbatch_close(reader)",
        ),
    ] {
        let path = run(dir.path(), &source, false);
        let receipt = read_receipt(&path).unwrap();
        assert_eq!(receipt["status"], "MACHINERY_FAIL", "{receipt}");
        assert!(verify_run(&path).unwrap().verified);
        assert!(path.join("outputs/copy.csv.partial").exists());
        assert!(!path.join("outputs/copy.csv").exists());
    }
    let mut b = Batches::default();
    let h = reader(&mut b, dir.path(), 1, 4096);
    assert!(
        b.call(dir.path(), "batch_close", std::slice::from_ref(&h))
            .is_err()
    );
    assert!(b.finish().is_err());
}

#[test]
fn immutable_snapshot_survives_original_changes_and_deletion() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.csv"), "id,value\n1,original\n").unwrap();
    let mut b = Batches::default();
    let h = reader(&mut b, dir.path(), 2, 4096);
    fs::write(dir.path().join("data.csv"), "id,value\n9,changed\n").unwrap();
    assert_eq!(
        b.call(dir.path(), "batch_next", &[h]).unwrap().render(),
        "[1, original]"
    );
    let imports = b.imports();
    let snapshot = dir.path().join("saved.csv");
    b.copy_input(&imports[0].sha256, &snapshot)
        .unwrap()
        .unwrap();
    assert_eq!(
        fs::read_to_string(snapshot).unwrap(),
        "id,value\n1,original\n"
    );
    fs::write(dir.path().join("data.csv"), "id,value\n1,original\n").unwrap();
    let path = run(dir.path(), COPY, false);
    fs::remove_file(dir.path().join("data.csv")).unwrap();
    assert!(verify_run(path).unwrap().verified);
}

#[test]
fn declared_limits_and_types_refuse_before_unbounded_work() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.csv"), "id,value\n1,one\n").unwrap();
    for (index, value) in [
        (2, number(0)),
        (2, number(50001)),
        (3, number(MAX_BATCH_BYTES as u64 + 1)),
        (4, number(MAX_FILE_BYTES + 1)),
        (4, number(1)),
        (2, text("10")),
    ] {
        let mut args = vec![
            text("data.csv"),
            columns(&["id", "value"]),
            number(2),
            number(4096),
            number(4096),
        ];
        args[index] = value;
        assert!(
            Batches::default()
                .call(dir.path(), "csv_batch_open", &args)
                .is_err()
        );
    }
    for names in [columns(&[]), columns(&["id", "id"]), columns(&["missing"])] {
        assert!(
            Batches::default()
                .call(
                    dir.path(),
                    "csv_batch_open",
                    &[
                        text("data.csv"),
                        names,
                        number(2),
                        number(4096),
                        number(4096)
                    ]
                )
                .is_err()
        );
    }
    let mut b = Batches::default();
    let h = reader(&mut b, dir.path(), 2, 1);
    assert!(b.call(dir.path(), "batch_next", &[h]).is_err());
    let sparse = fs::File::create(dir.path().join("large.csv")).unwrap();
    sparse.set_len(2 * 1024 * 1024 * 1024).unwrap();
    assert!(
        Batches::default()
            .call(
                dir.path(),
                "csv_batch_open",
                &[
                    text("large.csv"),
                    columns(&["id"]),
                    number(2),
                    number(4096),
                    number(1024 * 1024 * 1024)
                ]
            )
            .is_err()
    );
}

#[test]
fn empty_tables_headers_and_bad_records_have_explicit_semantics() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.csv"), "id,value\n").unwrap();
    let path = run(dir.path(), COPY, false);
    let receipt = read_receipt(&path).unwrap();
    assert_eq!(receipt["status"], "PASS");
    assert!(verify_run(path).unwrap().verified);
    for input in [
        "",
        "id,id\n1,2\n",
        "id,value\n1,\"unclosed",
        "id,value\n1,\"a\"oops\n",
        "id,value\n1,x\n2,y,extra\n",
    ] {
        fs::write(dir.path().join("data.csv"), input).unwrap();
        let path = run(dir.path(), COPY, false);
        let receipt = read_receipt(&path).unwrap();
        assert_eq!(receipt["status"], "MACHINERY_FAIL", "{input}: {receipt}");
        assert!(verify_run(path).unwrap().verified);
    }
}

#[test]
fn incremental_output_limit_and_name_collisions_preserve_existing_rows() {
    let dir = tempdir().unwrap();
    let source = "writer = csv_stream_open(\"tiny.csv\",[\"id\"],5)\nstream_write(writer,[\"1\"])\nstream_write(writer,[\"2\"])\nstream_close(writer)";
    for compile in [false, true] {
        let path = run(dir.path(), source, compile);
        let receipt = read_receipt(&path).unwrap();
        assert_eq!(receipt["status"], "MACHINERY_FAIL");
        assert!(verify_run(&path).unwrap().verified);
        assert_eq!(
            fs::read_to_string(path.join("outputs/tiny.csv.partial")).unwrap(),
            "id\n1\n"
        );
    }
    for source in [
        "w=csv_stream_open(\"x.csv\",[\"id\"],100)\nx=write_csv(\"x.csv\",[\"id\"],[1])",
        "x=write_csv(\"x.csv\",[\"id\"],[1])\nw=csv_stream_open(\"x.csv\",[\"id\"],100)",
        "w=csv_stream_open(\"x.csv\",[\"id\"],100)\nw2=csv_stream_open(\"x.csv\",[\"id\"],100)",
    ] {
        let path = run(dir.path(), source, false);
        assert_ne!(read_receipt(path).unwrap()["status"], "PASS");
    }
}

#[test]
fn writer_validates_shape_types_lifecycle_and_handle_identity() {
    let dir = tempdir().unwrap();
    let mut b = Batches::default();
    assert!(
        b.call(
            dir.path(),
            "csv_stream_open",
            &[text("../x.csv"), columns(&["id"]), number(100)]
        )
        .is_err()
    );
    assert!(
        b.call(
            dir.path(),
            "csv_stream_open",
            &[text("x.tsv"), columns(&["id"]), number(100)]
        )
        .is_err()
    );
    let w = b
        .call(
            dir.path(),
            "csv_stream_open",
            &[text("x.csv"), columns(&["id", "value"]), number(100)],
        )
        .unwrap();
    assert!(
        b.call(dir.path(), "stream_write", &[w.clone(), columns(&["1"])])
            .is_err()
    );
    assert!(b.call(dir.path(), "stream_close", &[w]).is_err());
    assert!(
        b.call(dir.path(), "batch_next", &[text("@goblin-reader-0")])
            .is_err()
    );
    assert!(
        b.call(dir.path(), "stream_close", &[text("@goblin-writer-00")])
            .is_err()
    );
    let w = b
        .call(
            dir.path(),
            "tsv_stream_open",
            &[text("empty.tsv"), columns(&["id"]), number(100)],
        )
        .unwrap();
    b.call(dir.path(), "stream_close", std::slice::from_ref(&w))
        .unwrap();
    assert!(b.call(dir.path(), "stream_close", &[w]).is_err());
}

#[test]
fn arity_and_discarded_results_are_checked_before_effects() {
    for source in [
        "csv_batch_open(\"x.csv\",[\"id\"],2,4096,100)",
        "batch_next(\"reader\")",
        "csv_stream_open(\"x.csv\",[\"id\"],100)",
    ] {
        let parsed = parse_source(source).unwrap();
        assert!(
            goblinpp::parser::validate_execution(&parsed.program).is_err(),
            "{source}"
        );
    }
    let parsed = parse_source("x = csv_batch_open(parse_number(\"effect\"))").unwrap();
    let dir = tempdir().unwrap();
    assert!(goblinpp::compiler::compile(&parsed, dir.path().join("invalid"), &[]).is_err());
    assert!(
        goblinpp::evaluator::Evaluation::new(".")
            .eval_program(&parsed.program)
            .unwrap_err()
            .message
            .contains("expects exactly")
    );
    assert!(parse_source("g_func batch_next(x) { return x }").is_err());
}

#[test]
fn mixed_eager_scan_and_batch_access_are_refused_both_directions() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.csv"), "id,value\n1,2\n").unwrap();
    let open = "r=csv_batch_open(\"data.csv\",[\"id\"],1,4096,100)";
    for eager in [
        "x=csv_numbers(\"data.csv\",\"value\")",
        "x=csv_scan_stats(\"data.csv\",\"value\")",
    ] {
        for source in [format!("{open}\n{eager}"), format!("{eager}\n{open}")] {
            let path = run(dir.path(), &source, false);
            assert_eq!(read_receipt(&path).unwrap()["status"], "MACHINERY_FAIL");
            assert!(verify_run(path).unwrap().verified);
        }
    }
}

#[test]
fn batch_policy_summary_output_and_native_tampering_are_detected() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.csv"), "id,value\n1,2\n").unwrap();
    let path = run(dir.path(), COPY, true);
    let original = read_receipt(&path).unwrap();
    assert_eq!(original["status"], "PASS");
    for altered in ["policy", "peak", "complete", "missing"] {
        let mut r = original.clone();
        match altered {
            "policy" => r["batch_policy"]["maximum_batch_bytes"] = json!(999),
            "peak" => r["batches"]["readers"][0]["peak_batch_bytes"] = json!(99999999),
            "complete" => r["generated_artifacts"][0]["metadata"]["complete"] = json!(false),
            _ => {
                r.as_object_mut().unwrap().remove("batches");
            }
        }
        reseal(&path, r);
        assert!(!verify_run(&path).unwrap().verified, "{altered}");
    }
    reseal(&path, original.clone());
    let native_path = path.join("native-data.json");
    let native_bytes = fs::read(&native_path).unwrap();
    let mut native: serde_json::Value = serde_json::from_slice(&native_bytes).unwrap();
    native["batches"]["readers"][0]["rows"] = json!(2);
    fs::write(&native_path, serde_json::to_vec_pretty(&native).unwrap()).unwrap();
    let mut rehashed = original.clone();
    rehashed["execution"]["compiler"]["native_data_manifest_sha256"] =
        json!(sha256_file(&native_path).unwrap());
    reseal(&path, rehashed);
    let verification = verify_run(&path).unwrap();
    assert!(!verification.verified);
    assert!(
        verification
            .checks
            .iter()
            .any(|c| c.check == "NATIVE_DATA_EVIDENCE_PARITY" && !c.pass)
    );
    fs::write(native_path, native_bytes).unwrap();
    reseal(&path, original);
    fs::write(path.join("outputs/copy.csv"), "tampered").unwrap();
    assert!(!verify_run(path).unwrap().verified);
}

#[test]
fn freeze_pins_batch_contract_without_changing_historical_policy() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("frozen.gbl");
    fs::write(&source, "x=1\nseal x").unwrap();
    create_freeze(&source).unwrap();
    let mut frozen: serde_json::Value =
        serde_json::from_slice(&fs::read(freeze_path(&source)).unwrap()).unwrap();
    assert_eq!(frozen["batch_policy"], policy());
    assert!(verify_freeze(&source).verified);
    frozen.as_object_mut().unwrap().remove("batch_policy");
    frozen
        .as_object_mut()
        .unwrap()
        .remove("freeze_receipt_sha256");
    frozen["freeze_receipt_sha256"] = json!(hash_canonical_json(&frozen).unwrap());
    fs::write(
        freeze_path(&source),
        serde_json::to_vec_pretty(&frozen).unwrap(),
    )
    .unwrap();
    assert!(!verify_freeze(&source).verified);
    let path = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&path).unwrap()["status"], "PROTOCOL_VIOLATION");
    assert!(verify_run(path).unwrap().verified);
    assert!(!goblinpp::batches::requires_policy(
        &json!({"goblin_version":"0.1.0-alpha.27"})
    ));
    assert!(goblinpp::batches::frozen_policy_matches(
        &json!({"goblin_version":"0.1.0-alpha.27"})
    ));
}

#[test]
fn standalone_success_and_failure_preserve_disk_outputs_and_input_snapshots() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.csv"), "id,value\n1,2\n").unwrap();
    for failure in [false, true] {
        let source = if failure {
            COPY.replace(
                "stream_close(writer)",
                "bad = parse_number(\"stop\")\nstream_close(writer)",
            )
        } else {
            COPY.to_owned()
        };
        let parsed = parse_source(&source).unwrap();
        let build = dir.path().join(if failure { "fail" } else { "good" });
        let binary = goblinpp::compiler::compile(&parsed, build, &[]).unwrap();
        let output = Command::new(binary.binary)
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            !failure,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        let prefix = if failure {
            "INCOMPLETE_DATA_DIR="
        } else {
            "NATIVE_DATA_DIR="
        };
        let saved = stderr.lines().find_map(|s| s.strip_prefix(prefix)).unwrap();
        let saved = dir.path().join(saved);
        assert!(saved.join("batch-inputs.json").exists());
        let output_path = if failure {
            "outputs/copy.csv.partial"
        } else {
            "native-outputs/copy.csv"
        };
        assert_eq!(
            fs::read_to_string(saved.join(output_path)).unwrap(),
            "id,value\n1,2\n"
        );
    }
}

#[test]
fn fixed_batches_have_size_independent_owned_buffers_and_output_identity() {
    let dir = tempdir().unwrap();
    let mut peaks = Vec::new();
    for rows in [100, 10_000] {
        let contents = String::from("id,value\n") + &"1,2\n".repeat(rows);
        fs::write(dir.path().join("data.csv"), contents).unwrap();
        let mut b = Batches::default();
        let h = reader(&mut b, dir.path(), 17, 4096);
        let mut count = 0;
        loop {
            let Value::Array(cells) = b
                .call(dir.path(), "batch_next", std::slice::from_ref(&h))
                .unwrap()
            else {
                panic!()
            };
            if cells.is_empty() {
                break;
            }
            count += cells.len() / 2;
            drop(cells);
        }
        b.call(dir.path(), "batch_close", &[h]).unwrap();
        b.finish().unwrap();
        assert_eq!(count, rows);
        peaks.push(
            b.evidence()["readers"][0]["peak_batch_bytes"]
                .as_u64()
                .unwrap(),
        );
    }
    assert_eq!(peaks[0], peaks[1]);
    fs::write(dir.path().join("data.csv"), "id,value\n1,2\n3,4\n5,6\n").unwrap();
    let a = run(dir.path(), COPY, false);
    let c = run(dir.path(), &COPY.replace("2, 4096", "1, 4096"), false);
    assert_eq!(
        sha256_file(a.join("outputs/copy.csv")).unwrap(),
        sha256_file(c.join("outputs/copy.csv")).unwrap()
    );
}

#[test]
fn late_ledger_failure_marks_even_closed_native_outputs_incomplete() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.csv"), "id,value\n1,2\n").unwrap();
    fs::create_dir(dir.path().join(".goblin")).unwrap();
    fs::write(dir.path().join(".goblin/custody-ledger.lock"), b"held").unwrap();
    for compile in [false, true] {
        let path = run(dir.path(), COPY, compile);
        let r = read_receipt(&path).unwrap();
        assert_eq!(r["status"], "MACHINERY_FAIL");
        assert_eq!(r["failure"]["code"], "G404");
        assert_eq!(r["generated_artifacts"][0]["metadata"]["complete"], false);
        assert!(path.join("outputs/copy.csv.partial").exists());
        assert!(!path.join("outputs/copy.csv").exists());
        let verification = verify_run(path).unwrap();
        assert!(verification.verified, "{:?}", verification.checks);
    }
}

#[test]
fn reader_and_writer_handle_budgets_are_enforced_and_close_releases_slots() {
    let dir = tempdir().unwrap();
    let mut b = Batches::default();
    let mut handles = Vec::new();
    for i in 0..5 {
        let file = format!("data{i}.csv");
        fs::write(dir.path().join(&file), "id\n").unwrap();
        let result = b.call(
            dir.path(),
            "csv_batch_open",
            &[
                text(&file),
                columns(&["id"]),
                number(1),
                number(4096),
                number(100),
            ],
        );
        if i < 4 {
            handles.push(result.unwrap());
        } else {
            assert!(result.is_err());
        }
    }
    b.call(dir.path(), "batch_next", std::slice::from_ref(&handles[0]))
        .unwrap();
    b.call(dir.path(), "batch_close", std::slice::from_ref(&handles[0]))
        .unwrap();
    assert!(
        b.call(
            dir.path(),
            "csv_batch_open",
            &[
                text("data4.csv"),
                columns(&["id"]),
                number(1),
                number(4096),
                number(100)
            ]
        )
        .is_ok()
    );
    let mut w = Batches::default();
    for i in 0..128 {
        let h = w
            .call(
                dir.path(),
                "csv_stream_open",
                &[text(&format!("file{i}.csv")), columns(&["id"]), number(100)],
            )
            .unwrap();
        w.call(dir.path(), "stream_close", &[h]).unwrap();
    }
    assert!(
        w.call(
            dir.path(),
            "csv_stream_open",
            &[text("last.csv"), columns(&["id"]), number(100)]
        )
        .is_err()
    );
    w.finish().unwrap();
}

#[test]
fn preview_validates_lifecycle_and_returns_only_temporary_output_descriptors() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.csv"), "id,value\n1,2\n").unwrap();
    let source = dir.path().join("main.gbl");
    for incomplete in [false, true] {
        fs::write(
            &source,
            if incomplete {
                COPY.replace("stream_close(writer)", "")
            } else {
                COPY.to_owned()
            },
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
            .args(["check", source.to_str().unwrap(), "--json"])
            .output()
            .unwrap();
        let preview: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.status.success(), !incomplete);
        assert_eq!(preview["evidence_created"], false);
        assert_eq!(
            preview["generated_output_preview"][0]["metadata"]["complete"],
            !incomplete
        );
        assert!(!dir.path().join("runs").exists());
        assert!(!dir.path().join(".goblin").exists());
        assert!(!dir.path().join("outputs").exists());
    }
}
