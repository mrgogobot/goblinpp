use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::custody::{create_freeze, freeze_path, verify_freeze};
use goblinpp::evaluator::Evaluation;
use goblinpp::hashing::{hash_canonical_json, sha256_file};
use goblinpp::parser::parse_source;
use goblinpp::resources;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use goblinpp::streaming::{MAX_FILE_BYTES, MAX_RECORD_BYTES, Scan};
use serde_json::{Value, json};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::process::Command;
use tempfile::tempdir;

fn reseal(run: &std::path::Path, mut receipt: Value) {
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

#[test]
fn single_top_level_integer_budget_only_and_modules_cannot_override() {
    for source in [
        "GO_LOOP_BUDGET 0",
        "GO_LOOP_BUDGET 50000001",
        "GO_LOOP_BUDGET -1",
        "GO_LOOP_BUDGET 2.5",
        "GO_LOOP_BUDGET 1e3",
        "GO_LOOP_BUDGET \"12\"",
        "GO_LOOP_BUDGET 3\nGO_LOOP_BUDGET 3",
        "if true { GO_LOOP_BUDGET 3 }",
        "g_func nested() { GO_LOOP_BUDGET 3\nreturn 1 }",
        "for index in range(1) { GO_LOOP_BUDGET 3 }",
    ] {
        assert!(parse_source(source).is_err(), "{source}");
    }
    assert!(parse_source("GO_LOOP_BUDGET 50000000\nx = 1").is_ok());
    let dir = tempdir().unwrap();
    let source = dir.path().join("main.gbl");
    fs::write(
        dir.path().join("library.gbl"),
        "GO_LOOP_BUDGET 50\ng_func helper() { return 1 }",
    )
    .unwrap();
    let text = "import \"library.gbl\"\nx = helper()";
    fs::write(&source, text).unwrap();
    assert!(goblinpp::modules::resolve(&source, text).is_err());
}

#[test]
fn nested_function_and_array_loops_share_one_counter() {
    let parsed=parse_source("GO_LOOP_BUDGET 9\ng_func helper() {\n for entry in [1,2] { if entry == 1 { continue } }\n return 1\n}\nfor outer in range(3) { result = helper() }\nwhile false { result = 999 }\nseal result").unwrap();
    let evaluation = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(
        evaluation.resource_evidence(),
        resources::evidence(Some(9), 9)
    );
    let stopped =
        parse_source("GO_LOOP_BUDGET 2\nfor index in range(100) { break }\nwhile true { break }")
            .unwrap();
    assert_eq!(
        Evaluation::new(".")
            .eval_program(&stopped.program)
            .unwrap()
            .resource_evidence(),
        resources::evidence(Some(2), 2)
    );
}

#[test]
fn larger_declared_budget_crosses_the_previous_fixed_limit() {
    let parsed =
        parse_source("GO_LOOP_BUDGET 1000001\nfor index in range(1000001) { value = 1 }").unwrap();
    let evaluation = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(
        evaluation.resource_evidence(),
        resources::evidence(Some(1000001), 1000001)
    );
}

#[test]
fn budget_refusals_are_preserved_and_verify_in_both_modes() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("fail.gbl");
    fs::write(&source,"GO_PARANOID\nGO_LOOP_BUDGET 4\nfor outer in range(2) { for inner in range(2) { value = 1 } }").unwrap();
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
        assert_eq!(receipt["status"], "MACHINERY_FAIL", "{receipt}");
        assert_eq!(receipt["failure"]["code"], "G203");
        assert_eq!(receipt["resources"], resources::evidence(Some(4), 4));
        assert!(verify_run(&run).unwrap().verified);
    }
}

#[test]
fn compiled_counter_matches_reference_and_standalone_preserves_limit() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("budget.gbl");
    let text = "GO_LOOP_BUDGET 6\ng_func helper() { for index in range(2) { value = index }\n return 4 }\nfor outer in range(2) { result = helper() }\nprint(result)\nseal result";
    fs::write(&source, text).unwrap();
    let a = run_file(&source, &RunOptions::default()).unwrap();
    let b = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    for run in [&a, &b] {
        let receipt = read_receipt(run).unwrap();
        assert_eq!(receipt["status"], "PASS", "{receipt}");
        assert_eq!(receipt["resources"], resources::evidence(Some(6), 6));
        assert!(verify_run(run).unwrap().verified);
    }
    assert!(diff_runs(&a, &b).unwrap().resource_evidence_same);
    let invalid = parse_source("GO_LOOP_BUDGET 2\nfor index in range(3) { print(index) }").unwrap();
    let binary = goblinpp::compiler::compile(&invalid, dir.path().join("standalone"), &[]).unwrap();
    let output = Command::new(binary.binary).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("LOOP LIMIT EXCEEDED"));
}

#[test]
fn resource_tampering_is_detected_even_after_resealing() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("tamper.gbl");
    fs::write(
        &source,
        "GO_LOOP_BUDGET 4\nfor index in range(3) { value = index }\nseal value",
    )
    .unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let original = read_receipt(&run).unwrap();
    for replacement in [
        json!(null),
        json!({"requested_loop_budget":null,"effective_loop_budget":1000000,"used_loop_iterations":3}),
        json!({"requested_loop_budget":4,"effective_loop_budget":4,"used_loop_iterations":5}),
    ] {
        let mut receipt = original.clone();
        receipt["resources"] = replacement;
        reseal(&run, receipt);
        assert!(!verify_run(&run).unwrap().verified);
    }
    let mut receipt = original;
    receipt.as_object_mut().unwrap().remove("resource_policy");
    reseal(&run, receipt);
    assert!(!verify_run(&run).unwrap().verified);
}

#[test]
fn freeze_pins_resource_policy_and_old_evidence_stays_verifiable() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("frozen.gbl");
    fs::write(&source, "GO_LOOP_BUDGET 4\nx = 1\nseal x").unwrap();
    create_freeze(&source).unwrap();
    let frozen: Value = serde_json::from_slice(&fs::read(freeze_path(&source)).unwrap()).unwrap();
    assert_eq!(frozen["resource_policy"], resources::policy());
    assert!(verify_freeze(&source).verified);
    fs::write(&source, "GO_LOOP_BUDGET 5\nx = 1\nseal x").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&run).unwrap()["status"], "PROTOCOL_VIOLATION");
    assert!(verify_run(&run).unwrap().verified);
    assert!(freeze_path(&source).exists());
    assert!(!resources::requires_policy(
        &json!({"goblin_version":"0.1.0-alpha.26"})
    ));
    assert!(resources::requires_policy(
        &json!({"goblin_version":"0.1.0-alpha.27"})
    ));
    let legacydir = tempdir().unwrap();
    let legacy = legacydir.path().join("old.gbl");
    fs::write(&legacy, "x = 1\nseal x").unwrap();
    let run = run_file(&legacy, &RunOptions::default()).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    receipt["goblin_version"] = json!("0.1.0-alpha.26");
    receipt.as_object_mut().unwrap().remove("resource_policy");
    receipt.as_object_mut().unwrap().remove("resources");
    reseal(&run, receipt);
    assert!(verify_run(&run).unwrap().verified);
}

#[test]
fn streaming_quoted_multiline_bom_and_compensated_sum_preserve_exact_hash() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.csv");
    fs::write(
        &input,
        b"\xef\xbb\xbf\"value\",note\r\n1e16,\"a\nline\"\r\n1,\"a\"\"quote\"\r\n-1e16,end\r\n",
    )
    .unwrap();
    let scan = Scan::open(&input, b',', "value").unwrap();
    assert_eq!(scan.stats, [3.0, 1.0, 1.0 / 3.0, -1e16, 1e16]);
    assert_eq!(scan.sha256, sha256_file(&input).unwrap());
    let preserved = dir.path().join("copy.csv");
    assert_eq!(scan.copy_evidence(&preserved).unwrap(), scan.sha256);
    assert_eq!(fs::read(&input).unwrap(), fs::read(&preserved).unwrap());
    let input = dir.path().join("input.tsv");
    fs::write(&input, "n\tnote\n\"2\"\t\"a\tfield\"\n4\tlast").unwrap();
    assert_eq!(
        Scan::open(&input, b'\t', "n").unwrap().stats,
        [2.0, 6.0, 3.0, 2.0, 4.0]
    );
}

#[test]
fn streaming_supports_over_16_mib_and_array_readers_remain_bounded() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("large.csv");
    {
        let mut output = BufWriter::new(File::create(&input).unwrap());
        output.write_all(b"value,note\n").unwrap();
        let row = format!("1,{}\n", "x".repeat(1024));
        for _ in 0..17000 {
            output.write_all(row.as_bytes()).unwrap();
        }
        output.flush().unwrap();
    }
    assert!(fs::metadata(&input).unwrap().len() > goblinpp::delimited::MAX_INPUT_BYTES);
    assert!(goblinpp::delimited::Table::open(&input, b',').is_err());
    let scan = Scan::open(&input, b',', "value").unwrap();
    assert_eq!(scan.stats, [17000.0, 17000.0, 1.0, 1.0, 1.0]);
    assert_eq!(scan.sha256, sha256_file(&input).unwrap());
}

#[test]
fn streaming_refuses_nulls_bad_headers_widths_quotes_and_limits() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("bad.csv");
    for text in [
        "",
        "n,n\n1,2",
        "n,\n1,2",
        "n\n",
        "n\n\"\"",
        "n\nNaN",
        "n\nInfinity",
        "n\n1,2",
        "n\n\"1\"junk",
        "n\n\"1",
        "n\n1\r2",
        "n\n1\0",
        "n\n1\"2",
    ] {
        fs::write(&input, text).unwrap();
        assert!(Scan::open(&input, b',', "n").is_err(), "{text:?}");
    }
    fs::write(&input, format!("n\n{}", "1".repeat(MAX_RECORD_BYTES + 1))).unwrap();
    assert!(Scan::open(&input, b',', "n").is_err());
    fs::write(
        &input,
        (0..1025)
            .map(|i| format!("c{i}"))
            .collect::<Vec<_>>()
            .join(","),
    )
    .unwrap();
    assert!(Scan::open(&input, b',', "c0").is_err());
    File::create(&input)
        .unwrap()
        .set_len(MAX_FILE_BYTES + 1)
        .unwrap();
    assert!(Scan::open(&input, b',', "n").is_err());
}

#[test]
fn changed_source_cannot_be_claimed_as_streaming_snapshot() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.csv");
    fs::write(&input, "n\n1\n").unwrap();
    let scan = Scan::open(&input, b',', "n").unwrap();
    fs::write(&input, "n\n2\n").unwrap();
    assert!(scan.copy_evidence(&dir.path().join("copy.csv")).is_err());
}

#[test]
fn streaming_native_interpreter_and_custody_are_exact_with_repeated_columns() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("input.csv"),
        "value,second,note\n1,10,a\n2,20,b\n3,30,c\n",
    )
    .unwrap();
    let source = dir.path().join("stream.gbl");
    fs::write(&source,"GO_PARANOID\nGO_LOOP_BUDGET 4\nsummary = csv_scan_stats(\"input.csv\", \"value\")\nsecond = csv_scan_stats(\"input.csv\", \"second\")\nfor index in range(4) { result = summary[2] }\nprint(summary)\nseal summary\nseal second\nseal result").unwrap();
    let a = run_file(&source, &RunOptions::default()).unwrap();
    let b = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    for run in [&a, &b] {
        let receipt = read_receipt(run).unwrap();
        assert_eq!(receipt["status"], "PASS", "{receipt}");
        assert_eq!(receipt["data_imports"].as_array().unwrap().len(), 1);
        assert_eq!(
            receipt["data_imports"][0]["access"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(
            verify_run(run).unwrap().verified,
            "{:?}",
            verify_run(run).unwrap()
        );
    }
    let diff = diff_runs(&a, &b).unwrap();
    assert!(diff.data_imports_same && diff.sealed_artifacts_same && diff.resource_evidence_same);
    fs::remove_file(dir.path().join("input.csv")).unwrap();
    assert!(verify_run(&a).unwrap().verified);
    assert!(verify_run(&b).unwrap().verified);
}

#[test]
fn streaming_arity_discard_and_historical_function_name_are_explicit() {
    for text in [
        "csv_scan_stats(\"x.csv\",\"n\")",
        "x = csv_scan_stats()",
        "g_func csv_scan_stats() { return 1 }\nx = csv_scan_stats()",
    ] {
        let parsed = parse_source(text).unwrap();
        assert!(Evaluation::new(".").eval_program(&parsed.program).is_err());
    }
    assert!(
        parse_source("g_func csv_scan_stats() { return 1 }\nx = csv_scan_stats()")
            .unwrap()
            .canonical_sha256()
            .is_ok()
    );
}

#[test]
fn mixed_array_and_scan_access_is_one_import_without_delimiter_or_hash_switches() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.csv");
    fs::write(&input, "n\n1\n2\n").unwrap();
    for text in [
        "a = csv_numbers(\"input.csv\",\"n\")\nb = csv_scan_stats(\"input.csv\",\"n\")",
        "b = csv_scan_stats(\"input.csv\",\"n\")\na = csv_numbers(\"input.csv\",\"n\")",
    ] {
        let parsed = parse_source(text).unwrap();
        let evaluation = Evaluation::new(dir.path())
            .eval_program(&parsed.program)
            .unwrap();
        assert_eq!(evaluation.data_imports().len(), 1);
        assert_eq!(evaluation.data_imports()[0].access.len(), 2);
    }
    let parsed =
        parse_source("a = csv_scan_stats(\"input.csv\",\"n\")\nb = tsv_rows(\"input.csv\")")
            .unwrap();
    assert!(
        Evaluation::new(dir.path())
            .eval_program(&parsed.program)
            .is_err()
    );
    let mut evaluation = Evaluation::new(dir.path());
    let parsed = parse_source("a = csv_scan_stats(\"input.csv\",\"n\")").unwrap();
    evaluation.register_functions(&parsed.program).unwrap();
    evaluation.eval_stmt(&parsed.program.statements[0]).unwrap();
    fs::write(&input, "n\n3\n4\n").unwrap();
    let next = parse_source("b = csv_numbers(\"input.csv\",\"n\")").unwrap();
    assert!(evaluation.eval_stmt(&next.program.statements[0]).is_err());
}

#[test]
fn native_budget_counter_tamper_and_standalone_streaming_metadata_are_checked() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("input.csv"), "n\n1\n2\n").unwrap();
    let text = "GO_LOOP_BUDGET 4\nx = csv_scan_stats(\"input.csv\",\"n\")\nfor index in range(3) { value = index }\nseal x";
    let source = dir.path().join("native.gbl");
    fs::write(&source, text).unwrap();
    let run = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PASS", "{receipt}");
    receipt["resources"]["used_loop_iterations"] = json!(2);
    reseal(&run, receipt);
    assert!(!verify_run(&run).unwrap().verified);
    let parsed = parse_source(text).unwrap();
    let built = goblinpp::compiler::compile(&parsed, dir.path().join("standalone"), &[]).unwrap();
    let output = Command::new(built.binary)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let directory = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("goblin-native-outputs-")
        })
        .unwrap();
    let manifest: Value =
        serde_json::from_slice(&fs::read(directory.join("native-data.json")).unwrap()).unwrap();
    assert_eq!(manifest["resource_policy"], resources::policy());
    assert_eq!(manifest["resources"], resources::evidence(Some(4), 3));
    let imported = read_receipt(&run).unwrap();
    let snapshot = run.join(
        imported["data_imports"][0]["evidence_path"]
            .as_str()
            .unwrap(),
    );
    // Replace the run's hard link, without changing the immutable shared object.
    fs::remove_file(&snapshot).unwrap();
    fs::write(snapshot, "n\n9\n").unwrap();
    assert!(!verify_run(&run).unwrap().verified);
}
