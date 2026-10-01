use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::custody::{create_freeze, verify_freeze};
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

fn run(source: &Path, compile: bool) -> std::path::PathBuf {
    run_file(
        source,
        &RunOptions {
            compile,
            ..Default::default()
        },
    )
    .unwrap()
}

fn fixture(root: &Path) {
    fs::write(
        root.join("sample.fits"),
        include_bytes!("../examples/sample.fits"),
    )
    .unwrap();
    fs::write(
        root.join("measurements.csv"),
        "\u{feff}name,length\r\n\"Ada, π\",10\r\n\"B\"\"ob\",11\r\n\"multi\nline\",12\r\n",
    )
    .unwrap();
    fs::write(root.join("measurements.tsv"), "label\tvalue\nx\t2\ny\t4\n").unwrap();
    fs::create_dir(root.join("lib")).unwrap();
    fs::write(
        root.join("lib/units.gbl"),
        "g_func length_of(number) { return number * 1 m }\n",
    )
    .unwrap();
    fs::write(
        root.join("lib/analysis.gbl"),
        r#"import "units.gbl"
g_func average_length(values) {
    lengths = []
    for value in values { lengths = append(lengths, length_of(value)) }
    return mean(lengths)
}
g_func report(value) { return write_text("function.txt", "value = {value}; argc = {argc}") }
"#,
    )
    .unwrap();
}

#[test]
fn all_three_features_agree_and_every_evidence_object_verifies() {
    let root = tempdir().unwrap();
    fixture(root.path());
    let source = root.path().join("all.gbl");
    let program = format!(
        r#"import "lib/analysis.gbl"
import "lib/units.gbl"
values = csv_numbers("measurements.csv", "length")
average = average_length(values)
names = csv_column("measurements.csv", "name")
headers = csv_headers("measurements.csv")
csv_count = csv_rows("measurements.csv")
csv_width = csv_columns("measurements.csv")
tsv_values = tsv_numbers("measurements.tsv", "value")
tsv_names = tsv_column("measurements.tsv", "label")
tsv_header = tsv_headers("measurements.tsv")
tsv_count = tsv_rows("measurements.tsv")
tsv_width = tsv_columns("measurements.tsv")
report(average)
print("average = {{average}}; rows = {{csv_count}}; columns = {{csv_width}}")
seal average
seal names
seal values
seal headers
seal tsv_values
seal tsv_names
seal tsv_header
seal tsv_count
seal tsv_width
{}
"#,
        include_str!("../examples/output_demo.gbl")
    );
    fs::write(&source, program).unwrap();
    create_freeze(&source).unwrap();
    let interpreted = run(&source, false);
    let native = run(&source, true);
    for directory in [&interpreted, &native] {
        let receipt = read_receipt(directory).unwrap();
        assert_eq!(
            receipt["status"],
            "PASS",
            "{}\n{}",
            receipt,
            fs::read_to_string(directory.join("stderr.log")).unwrap()
        );
        let report = verify_run(directory).unwrap();
        assert!(report.verified, "{report:?}");
        assert_eq!(receipt["module_imports"].as_array().unwrap().len(), 2);
        assert_eq!(receipt["data_imports"].as_array().unwrap().len(), 3);
        assert_eq!(
            fs::read_to_string(directory.join("outputs/function.txt")).unwrap(),
            "value = 11 m; argc = 1\n"
        );
    }
    let a = read_receipt(&interpreted).unwrap();
    let b = read_receipt(&native).unwrap();
    assert_eq!(a["sealed_artifacts"], b["sealed_artifacts"]);
    assert_eq!(a["generated_artifacts"], b["generated_artifacts"]);
    assert_eq!(
        fs::read(interpreted.join("stdout.log")).unwrap(),
        fs::read(native.join("stdout.log")).unwrap()
    );
    let main = fs::read_to_string(native.join("program-native.goblin.rs")).unwrap();
    assert!(!main.contains("parse_source("));
    assert!(!main.contains("Command::new"));
    // A generated executable continues to perform data I/O without the engine on PATH.
    let output = Command::new(native.join("program-native"))
        .current_dir(root.path())
        .env("PATH", "/no-goblin-engine")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, fs::read(native.join("stdout.log")).unwrap());
    let support = native.join("program-native.native-build/src/fits.rs");
    let saved = fs::read(&support).unwrap();
    fs::write(&support, "tampered support").unwrap();
    assert!(!verify_run(&native).unwrap().verified);
    fs::write(&support, saved).unwrap();
    let manifest = native.join("native-data.json");
    let saved = fs::read(&manifest).unwrap();
    fs::write(&manifest, "{}").unwrap();
    assert!(!verify_run(&native).unwrap().verified);
    fs::write(&manifest, saved).unwrap();
    assert!(verify_run(&native).unwrap().verified);
    // Native output evidence is checked separately, not rescued by intact interpreter artifacts.
    fs::write(native.join("native-outputs/summary.txt"), "tampered").unwrap();
    assert!(!verify_run(&native).unwrap().verified);
    let entry = a["data_imports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["format"] == "CSV-UTF8")
        .unwrap();
    let evidence = interpreted.join(entry["evidence_path"].as_str().unwrap());
    fs::remove_file(&evidence).unwrap();
    fs::write(&evidence, "tampered").unwrap();
    assert!(!verify_run(&interpreted).unwrap().verified);
}

#[test]
fn module_changes_are_frozen_and_preserved_runs_are_independent() {
    let root = tempdir().unwrap();
    fixture(root.path());
    let source = root.path().join("main.gbl");
    fs::write(
        &source,
        "GO_PARANOID\nimport \"lib/units.gbl\"\nx = length_of(2)\nseal x\n",
    )
    .unwrap();
    create_freeze(&source).unwrap();
    let good = run(&source, false);
    fs::write(
        root.path().join("lib/units.gbl"),
        "# only notation changed\ng_func length_of(number) { return number * 1 m }\n",
    )
    .unwrap();
    assert_eq!(
        verify_freeze(&source).classification,
        "MODULE_CHANGED_AFTER_FREEZE"
    );
    let refused = run(&source, false);
    assert_eq!(
        read_receipt(&refused).unwrap()["status"],
        "PROTOCOL_VIOLATION"
    );
    assert!(verify_run(&refused).unwrap().verified);
    fs::remove_file(root.path().join("lib/units.gbl")).unwrap();
    assert!(verify_run(&good).unwrap().verified);
    let receipt = read_receipt(&good).unwrap();
    let evidence = receipt["module_imports"][0]["evidence_path"]
        .as_str()
        .unwrap();
    fs::write(good.join(evidence), "tampered").unwrap();
    assert!(!verify_run(&good).unwrap().verified);
    assert!(!verify_freeze(&source).verified);
}

#[test]
fn dangerous_or_ambiguous_libraries_are_rejected() {
    for library in [
        "print(\"hidden execution\")",
        "GO_PARANOID",
        "x = 2",
        "import \"library.gbl\"",
        "g_func f(x) { return x }\ng_func f(y) { return y }",
    ] {
        let root = tempdir().unwrap();
        let source = root.path().join("main.gbl");
        fs::write(&source, "import \"library.gbl\"\nprint(1)\n").unwrap();
        fs::write(root.path().join("library.gbl"), library).unwrap();
        let failed = run(&source, false);
        assert_eq!(read_receipt(&failed).unwrap()["status"], "MACHINERY_FAIL");
        assert!(verify_run(failed).unwrap().verified);
    }
    for text in [
        "import \"../outside.gbl\"",
        "import \"/outside.gbl\"",
        "import \"file.txt\"",
        "import \"lib/./units.gbl\"",
        "if true { import \"library.gbl\" }",
        "g_func x() { import \"library.gbl\" }",
    ] {
        assert!(goblinpp::modules::resolve(Path::new("main.gbl"), text).is_err());
    }
}

#[test]
fn csv_errors_are_explicit_and_failed_numeric_reads_preserve_input() {
    for table in [
        "",
        "x,x\n1,2",
        "x,y\n1",
        "x\n\"unclosed",
        "x\n\"ok\"junk",
        "x\nwrong\"quote",
        "x\r1",
    ] {
        let root = tempdir().unwrap();
        fs::write(root.path().join("data.csv"), table).unwrap();
        assert!(goblinpp::delimited::Table::open(&root.path().join("data.csv"), b',').is_err());
    }
    for value in ["", "NaN", "inf", "1e999", "12 kg", "not a number"] {
        let root = tempdir().unwrap();
        fs::write(root.path().join("data.csv"), format!("x\n\"{value}\"\n")).unwrap();
        let source = root.path().join("main.gbl");
        fs::write(
            &source,
            "GO_PARANOID\nx = csv_numbers(\"data.csv\", \"x\")\n",
        )
        .unwrap();
        let failed = run(&source, false);
        let receipt = read_receipt(&failed).unwrap();
        assert_eq!(receipt["status"], "MACHINERY_FAIL");
        assert_eq!(receipt["data_imports"].as_array().unwrap().len(), 1);
        assert!(verify_run(failed).unwrap().verified);
    }
}

#[test]
fn delimited_quotes_empty_fields_and_limits_are_not_guessed() {
    let rows = goblinpp::delimited::parse("a,b\n\"a,b\",\"a\"\"b\"\n,", b',').unwrap();
    assert_eq!(
        rows,
        vec![vec!["a", "b"], vec!["a,b", "a\"b"], vec!["", ""]]
    );
    assert!(goblinpp::delimited::parse(&"x\n".repeat(100_002), b',').is_err());
    assert!(goblinpp::delimited::parse(&format!("{}\n", "x,".repeat(1024)), b',').is_err());
}

#[test]
fn library_notation_changes_affect_source_identity_not_meaning() {
    let root = tempdir().unwrap();
    fixture(root.path());
    let source = root.path().join("main.gbl");
    fs::write(
        &source,
        "import \"lib/units.gbl\"\nx = length_of(2)\nseal x\n",
    )
    .unwrap();
    let before = run(&source, false);
    fs::write(
        root.path().join("lib/units.gbl"),
        "# new comment\ng_func length_of(number) { return number * 1 m }\n",
    )
    .unwrap();
    let after = run(&source, false);
    let diff = diff_runs(before, after).unwrap();
    assert!(!diff.source_bytes_same);
    assert!(diff.canonical_program_same);
    assert_eq!(diff.classification, "NOTATION_ONLY_CHANGE");
}

#[test]
fn native_names_do_not_shadow_user_functions_or_interpolation_variables() {
    let root = tempdir().unwrap();
    let source = root.path().join("main.gbl");
    fs::write(&source, "g_func fits_custom(x) { return x + 1 }\ng_func write_report(x) { return x + 2 }\n__goblin_data_arg_0 = 42\nanswer = write_report(fits_custom(4))\nwrite_text(\"answer.txt\", \"answer = {answer}; retained = {__goblin_data_arg_0}\")\nseal answer\n").unwrap();
    for compile in [false, true] {
        let directory = run(&source, compile);
        assert_eq!(read_receipt(&directory).unwrap()["status"], "PASS");
        assert_eq!(
            fs::read_to_string(directory.join("outputs/answer.txt")).unwrap(),
            "answer = 7; retained = 42\n"
        );
        assert!(verify_run(directory).unwrap().verified);
    }
}

#[test]
fn changed_library_at_postflight_refuses_pass_and_preserves_starting_evidence() {
    let root = tempdir().unwrap();
    let library = root.path().join("library.gbl");
    fs::write(&library, "g_func f() { return 7 }\n").unwrap();
    let source = root.path().join("main.gbl");
    let text = format!(
        "GO_PARANOID\nimport \"library.gbl\"\nx = f()\nseal x\nRUST_INLINE_BEGIN\nstd::fs::write({:?}, \"# changed\\ng_func f() {{ return 7 }}\\n\").unwrap();\nRUST_INLINE_END\n",
        library.to_string_lossy()
    );
    fs::write(&source, &text).unwrap();
    let digest = goblinpp::parser::parse_source(&text).unwrap().inline_rust[0]
        .sha256
        .clone();
    let directory = run_file(
        &source,
        &RunOptions {
            compile: true,
            allowed_inline_rust: vec![digest],
            ..Default::default()
        },
    )
    .unwrap();
    let receipt = read_receipt(&directory).unwrap();
    assert_eq!(receipt["status"], "PROTOCOL_VIOLATION");
    assert_eq!(
        receipt["protocol_violation"]["classification"],
        "MODULE_CHANGED_AT_POSTFLIGHT"
    );
    assert!(verify_run(directory).unwrap().verified);
}

#[cfg(unix)]
#[test]
fn symlink_inputs_and_modules_are_refused() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("real.gbl"), "g_func f(x) { return x }").unwrap();
    std::os::unix::fs::symlink(root.path().join("real.gbl"), root.path().join("link.gbl")).unwrap();
    assert!(
        goblinpp::modules::resolve(&root.path().join("main.gbl"), "import \"link.gbl\"").is_err()
    );
    fs::write(root.path().join("real.csv"), "x\n1\n").unwrap();
    std::os::unix::fs::symlink(root.path().join("real.csv"), root.path().join("link.csv")).unwrap();
    assert!(goblinpp::delimited::Table::open(&root.path().join("link.csv"), b',').is_err());
}
