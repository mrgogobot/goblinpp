use goblinpp::audit::verify_run;
use goblinpp::custody::{create_freeze, create_revision, verify_freeze};
use goblinpp::hashing::sha256_file;
use goblinpp::ledger;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

const ENERGY: &str = "GO_PARANOID\n\nmass = 1 kg\nenergy = mass * c^2\n\nprint(\"Energy = {energy}\")\nseal energy\n";

#[test]
fn rust_interpreter_matches_reference_energy_contract() {
    let root = tempdir().unwrap();
    let source = root.path().join("energy.gbl");
    fs::write(&source, ENERGY).unwrap();
    let parsed = parse_source(ENERGY).unwrap();
    assert_eq!(
        parsed.canonical_sha256().unwrap(),
        "6a1d72b91490bfc247bdf22322f7eb2ec59d5a88344d6c13a469e92883185a2d"
    );
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PASS");
    assert_eq!(receipt["execution"]["engine"], "rust-interpreter");
    assert_eq!(
        fs::read_to_string(run.join("stdout.log")).unwrap(),
        "Energy = 8.98755178736818e+16 J\n"
    );
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn native_compile_run_is_sealed_and_verifiable() {
    let root = tempdir().unwrap();
    let source = root.path().join("ratio.gbl");
    fs::write(&source, "samples = 12\naccepted = 9\nfraction = accepted / samples\nprint(\"accepted fraction = {fraction:.3f}\")\nseal fraction\n").unwrap();
    let run = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PASS");
    assert_eq!(receipt["execution"]["engine"], "rust-native-compiled");
    assert_eq!(
        fs::read_to_string(run.join("stdout.log")).unwrap(),
        "accepted fraction = 0.750\n"
    );
    assert!(run.join("program-native").is_file());
    assert!(run.join("program-native.goblin.rs").is_file());
    assert!(run.join("native-results.tsv").is_file());
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn frozen_notation_change_is_refused_but_preserved_and_verifiable() {
    let root = tempdir().unwrap();
    let source = root.path().join("energy.gbl");
    fs::write(&source, ENERGY).unwrap();
    create_freeze(&source).unwrap();
    fs::write(&source, ENERGY.replace("mass * c^2", "mass c²")).unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PROTOCOL_VIOLATION");
    assert_eq!(
        receipt["protocol_violation"]["classification"],
        "NOTATION_ONLY_CHANGE_AFTER_FREEZE"
    );
    assert!(receipt["sealed_artifacts"].as_array().unwrap().is_empty());
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn freeze_receipt_tampering_is_detected() {
    let root = tempdir().unwrap();
    let source = root.path().join("energy.gbl");
    fs::write(&source, ENERGY).unwrap();
    let (receipt_path, _) = create_freeze(&source).unwrap();
    let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
    receipt["policy"] = Value::String("TRUST_ME_BRO".into());
    fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert_eq!(
        verify_freeze(&source).classification,
        "FREEZE_RECEIPT_TAMPERED"
    );
    let run = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(run).unwrap()["status"], "PROTOCOL_VIOLATION");
}

#[test]
fn explicit_revision_carries_frozen_parent_lineage() {
    let root = tempdir().unwrap();
    let parent = root.path().join("energy.gbl");
    fs::write(&parent, ENERGY).unwrap();
    create_freeze(&parent).unwrap();
    let child = root.path().join("energy_R1.gbl");
    let (_, lineage, _) = create_revision(&parent, &child, "notation experiment").unwrap();
    assert_eq!(sha256_file(&parent).unwrap(), sha256_file(&child).unwrap());
    fs::write(&child, ENERGY.replace("mass * c^2", "mass c²")).unwrap();
    create_freeze(&child).unwrap();
    assert!(verify_freeze(&child).verified);
    let document: Value = serde_json::from_slice(&fs::read(lineage).unwrap()).unwrap();
    assert_eq!(document["reason"], "notation experiment");
}

#[test]
fn inline_rust_is_default_deny_then_exact_hash_allowed() {
    let root = tempdir().unwrap();
    let source = root.path().join("inline.gbl");
    let program = "GO_PARANOID\nx = 1\nRUST_INLINE_BEGIN\nprintln!(\"native block ran\");\nprintln!(\"observed {} variable\", goblin_env.len());\nRUST_INLINE_END\nseal x\n";
    fs::write(&source, program).unwrap();
    let block_hash = parse_source(program).unwrap().inline_rust[0].sha256.clone();
    let refused = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    let refused_receipt = read_receipt(&refused).unwrap();
    assert_eq!(refused_receipt["status"], "PROTOCOL_VIOLATION");
    assert_eq!(
        refused_receipt["protocol_violation"]["classification"],
        "INLINE_RUST_NOT_AUTHORIZED"
    );
    assert!(verify_run(refused).unwrap().verified);
    let accepted = run_file(
        &source,
        &RunOptions {
            compile: true,
            allowed_inline_rust: vec![block_hash],
            ..RunOptions::default()
        },
    )
    .unwrap();
    assert_eq!(read_receipt(&accepted).unwrap()["status"], "PASS");
    assert_eq!(
        fs::read_to_string(accepted.join("stdout.log")).unwrap(),
        "native block ran\nobserved 1 variable\n"
    );
    assert!(verify_run(accepted).unwrap().verified);
}

#[test]
fn fits_import_is_native_read_only_and_copies_exact_evidence() {
    let root = tempdir().unwrap();
    let fits_path = root.path().join("sample.fits");
    fs::write(
        &fits_path,
        fits_i16(&[1, 2, 3, 4], &[2, 2], &[("EXPTIME", "12.5")]),
    )
    .unwrap();
    let source = root.path().join("fits.gbl");
    fs::write(&source, "GO_PARANOID\nexposure = fits_header(\"sample.fits\", \"EXPTIME\")\npixels = fits_count(\"sample.fits\")\nmean = fits_mean(\"sample.fits\")\nprint(\"exposure = {exposure}\")\nprint(\"pixels = {pixels}\")\nprint(\"mean = {mean}\")\nseal mean\n").unwrap();
    let original_hash = sha256_file(&fits_path).unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PASS");
    assert_eq!(receipt["data_imports"][0]["sha256"], original_hash);
    assert_eq!(
        fs::read_to_string(run.join("stdout.log")).unwrap(),
        "exposure = 12.5\npixels = 4\nmean = 2.5\n"
    );
    fs::write(&fits_path, b"changed after run").unwrap();
    assert!(
        verify_run(run).unwrap().verified,
        "verification must use preserved import evidence"
    );
}

#[test]
fn multi_hdu_binary_table_access_is_sealed_and_verifiable() {
    let root = tempdir().unwrap();
    let fits_path = root.path().join("catalog.fits");
    fs::write(&fits_path, fits_image_and_table()).unwrap();
    let source = root.path().join("catalog.gbl");
    fs::write(
        &source,
        "GO_PARANOID\nhdus = fits_hdu_count(\"catalog.fits\")\nname = fits_header(\"catalog.fits\", 1, \"EXTNAME\")\nrows = fits_rows(\"catalog.fits\", 1)\ncolumns = fits_columns(\"catalog.fits\", 1)\nfirst = fits_column(\"catalog.fits\", 1, \"OBJECT\", 0)\nvalid_z = fits_column_valid_count(\"catalog.fits\", 1, \"Z\")\nmean_z = fits_column_mean(\"catalog.fits\", 1, \"Z\")\nmin_z = fits_column_min(\"catalog.fits\", 1, \"Z\")\nmax_z = fits_column_max(\"catalog.fits\", 1, \"Z\")\nprint(\"hdus = {hdus}\")\nprint(\"table = {name}\")\nprint(\"rows = {rows}\")\nprint(\"columns = {columns}\")\nprint(\"first = {first}\")\nprint(\"valid z = {valid_z}\")\nprint(\"mean z = {mean_z}\")\nprint(\"range z = {min_z} to {max_z}\")\nseal mean_z\nseal first\n",
    )
    .unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PASS");
    assert_eq!(receipt["data_imports"][0]["format"], "FITS-multi-HDU");
    let import_hash = sha256_file(&fits_path).unwrap();
    assert_eq!(receipt["data_imports"][0]["sha256"], import_hash);
    assert_eq!(
        fs::read_to_string(run.join("stdout.log")).unwrap(),
        "hdus = 2\ntable = CATALOG\nrows = 3\ncolumns = 3\nfirst = GALAXY\nvalid z = 2\nmean z = 1.1875\nrange z = 0.125 to 2.25\n"
    );
    assert!(verify_run(run).unwrap().verified);
    let stored = root
        .path()
        .join(format!(".goblin/imports/{import_hash}.fits"));
    assert!(stored.is_file());
    let second = run_file(&source, &RunOptions::default()).unwrap();
    assert!(verify_run(&second).unwrap().verified);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let evidence = second.join(format!("imports/{import_hash}.fits"));
        assert_eq!(
            fs::metadata(stored).unwrap().ino(),
            fs::metadata(evidence).unwrap().ino()
        );
    }
}

#[test]
fn generated_text_tables_json_and_plots_are_deterministic_and_verifiable() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("catalog.fits"), fits_image_and_table()).unwrap();
    let source = root.path().join("outputs.gbl");
    fs::write(
        &source,
        "rows = fits_rows(\"catalog.fits\", 1)\nmean_z = fits_column_mean(\"catalog.fits\", 1, \"Z\")\nmass = 1 kg\nwrite_text(\"summary.txt\", \"rows = {rows}\", \"mean = {mean_z}\")\nwrite_csv(\"summary.csv\", 2, \"metric\", \"value\", \"rows\", rows, \"mean_z\", mean_z)\nwrite_tsv(\"summary.tsv\", 2, \"metric\", \"value\", \"rows\", rows)\nwrite_json(\"summary.json\", \"rows\", rows, \"mean_z\", mean_z, \"mass\", mass)\nplot_fits_histogram(\"z.svg\", \"catalog.fits\", 1, \"Z\", 8, 100, \"Z distribution\")\nplot_fits_histogram(\"z.png\", \"catalog.fits\", 1, \"Z\", 8, 100, \"Z distribution\")\nplot_fits_scatter(\"z_quality.png\", \"catalog.fits\", 1, \"Z\", \"QUALITY\", 100, \"Z and quality\")\nseal mean_z\n",
    )
    .unwrap();

    let first = run_file(&source, &RunOptions::default()).unwrap();
    let first_receipt = read_receipt(&first).unwrap();
    assert_eq!(first_receipt["status"], "PASS");
    assert_eq!(first_receipt["paranoid_mode"], false);
    assert_eq!(
        fs::read_to_string(first.join("outputs/summary.csv")).unwrap(),
        "metric,value\nrows,3\nmean_z,1.1875\n"
    );
    assert_eq!(
        fs::read_to_string(first.join("outputs/summary.txt")).unwrap(),
        "rows = 3\nmean = 1.1875\n"
    );
    let json: Value =
        serde_json::from_slice(&fs::read(first.join("outputs/summary.json")).unwrap()).unwrap();
    assert_eq!(json["rows"], 3.0);
    assert_eq!(json["mass"]["value_si"], 1.0);
    assert_eq!(json["mass"]["unit_si"], "kg");
    assert_eq!(
        &fs::read(first.join("outputs/z.png")).unwrap()[..8],
        b"\x89PNG\r\n\x1a\n"
    );
    assert!(
        fs::read_to_string(first.join("outputs/z.svg"))
            .unwrap()
            .starts_with("<svg")
    );
    assert_eq!(
        first_receipt["generated_artifacts"]
            .as_array()
            .unwrap()
            .len(),
        7
    );
    assert_eq!(
        first_receipt["generated_artifacts"][4]["metadata"]["sampling"]["method"],
        "deterministic-even-row-sample"
    );
    assert!(verify_run(&first).unwrap().verified);

    let second = run_file(&source, &RunOptions::default()).unwrap();
    let second_receipt = read_receipt(&second).unwrap();
    let hashes = |receipt: &Value| {
        receipt["generated_artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                (
                    item["name"].as_str().unwrap().to_string(),
                    item["sha256"].as_str().unwrap().to_string(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(hashes(&first_receipt), hashes(&second_receipt));
    assert!(verify_run(second).unwrap().verified);
}

#[test]
fn generated_artifact_tampering_is_detected() {
    let root = tempdir().unwrap();
    let source = root.path().join("output.gbl");
    fs::write(
        &source,
        "GO_PARANOID\nx = 2\nwrite_csv(\"result.csv\", 2, \"name\", \"value\", \"x\", x)\n",
    )
    .unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    assert!(verify_run(&run).unwrap().verified);
    fs::write(run.join("outputs/result.csv"), b"name,value\nx,999\n").unwrap();
    let report = verify_run(run).unwrap();
    assert!(!report.verified);
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.check == "GENERATED_ARTIFACT:result.csv" && !check.pass)
    );
}

#[test]
fn output_paths_cannot_escape_the_run_directory() {
    let root = tempdir().unwrap();
    let source = root.path().join("escape.gbl");
    fs::write(&source, "write_text(\"../escape.txt\", \"no\")\n").unwrap();
    let escaped = root.path().parent().unwrap().join("escape.txt");
    assert!(!escaped.exists());
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["failure"]["code"], "G301");
    assert!(!escaped.exists());
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn generated_output_is_safe_without_paranoid_and_unique_names() {
    let root = tempdir().unwrap();
    let source = root.path().join("policy.gbl");
    fs::write(&source, "write_text(\"result.txt\", \"no\")\n").unwrap();
    let everyday = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&everyday).unwrap();
    assert_eq!(receipt["status"], "PASS");
    assert_eq!(receipt["paranoid_mode"], false);
    assert_eq!(
        fs::read_to_string(everyday.join("outputs/result.txt")).unwrap(),
        "no\n"
    );
    assert_eq!(receipt["generated_artifacts"].as_array().unwrap().len(), 1);
    assert!(verify_run(everyday).unwrap().verified);

    fs::write(
        &source,
        "write_text(\"result.txt\", \"first\")\nwrite_text(\"result.txt\", \"second\")\n",
    )
    .unwrap();
    let duplicate = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&duplicate).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["failure"]["code"], "G301");
    assert_eq!(receipt["generated_artifacts"].as_array().unwrap().len(), 1);
    assert!(verify_run(duplicate).unwrap().verified);
}

#[test]
fn empty_program_is_a_preserved_failure_not_a_pass() {
    let root = tempdir().unwrap();
    let source = root.path().join("empty.gbl");
    fs::write(&source, "").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["failure"]["code"], "G002");
    assert!(
        fs::read_to_string(run.join("stderr.log"))
            .unwrap()
            .contains("EMPTY PROGRAM")
    );
    assert!(verify_run(run).unwrap().verified);
}

#[cfg(unix)]
#[test]
fn damaged_shared_fits_evidence_is_refused_with_verifiable_failure_evidence() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempdir().unwrap();
    let fits_path = root.path().join("sample.fits");
    fs::write(&fits_path, fits_i16(&[1, 2], &[2], &[])).unwrap();
    let source = root.path().join("read.gbl");
    fs::write(
        &source,
        "GO_PARANOID\nmean = fits_mean(\"sample.fits\")\nseal mean\n",
    )
    .unwrap();
    let first = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(first).unwrap()["status"], "PASS");
    let hash = sha256_file(&fits_path).unwrap();
    let stored = root.path().join(format!(".goblin/imports/{hash}.fits"));
    fs::set_permissions(&stored, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&stored, b"damaged stored object").unwrap();

    let second = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&second).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["failure"]["code"], "G601");
    assert!(
        receipt["failure"]["message"]
            .as_str()
            .unwrap()
            .contains("is damaged")
    );
    assert!(verify_run(second).unwrap().verified);
}

#[cfg(unix)]
#[test]
fn fits_input_symlink_is_refused_and_failure_is_preserved() {
    use std::os::unix::fs::symlink;

    let root = tempdir().unwrap();
    fs::write(
        root.path().join("actual.fits"),
        fits_i16(&[1, 2], &[2], &[]),
    )
    .unwrap();
    symlink("actual.fits", root.path().join("linked.fits")).unwrap();
    let source = root.path().join("read.gbl");
    fs::write(
        &source,
        "GO_PARANOID\nmean = fits_mean(\"linked.fits\")\nseal mean\n",
    )
    .unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["failure"]["code"], "G601");
    assert!(
        receipt["failure"]["message"]
            .as_str()
            .unwrap()
            .contains("symbolic-link")
    );
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn damaged_ledger_blocks_execution_and_preserves_violation() {
    let root = tempdir().unwrap();
    let source = root.path().join("x.gbl");
    fs::write(&source, "x = 1\nseal x\n").unwrap();
    let first = run_file(&source, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(first).unwrap()["status"], "PASS");
    let path = ledger::ledger_path(root.path());
    let changed = fs::read_to_string(&path)
        .unwrap()
        .replace("\"status\":\"PASS\"", "\"status\":\"LIE\"");
    fs::write(path, changed).unwrap();
    let blocked = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&blocked).unwrap();
    assert_eq!(receipt["status"], "PROTOCOL_VIOLATION");
    assert_eq!(
        receipt["protocol_violation"]["classification"],
        "LEDGER_INTEGRITY_FAILURE"
    );
    assert!(verify_run(blocked).unwrap().verified);
}

#[test]
fn failed_ledger_registration_downgrades_run_without_breaking_receipt() {
    let root = tempdir().unwrap();
    let source = root.path().join("x.gbl");
    fs::write(&source, "x = 1\nseal x\n").unwrap();
    fs::create_dir(root.path().join(".goblin")).unwrap();
    fs::write(root.path().join(".goblin/custody-ledger.lock"), b"held").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["failure"]["code"], "G404");
    assert_eq!(receipt["ledger"]["registration"], "FAIL");
    assert!(verify_run(run).unwrap().verified);
}

fn fits_i16(values: &[i16], axes: &[usize], extra: &[(&str, &str)]) -> Vec<u8> {
    let mut cards = vec![
        fits_card("SIMPLE", "T"),
        fits_card("BITPIX", "16"),
        fits_card("NAXIS", &axes.len().to_string()),
    ];
    for (index, axis) in axes.iter().enumerate() {
        cards.push(fits_card(&format!("NAXIS{}", index + 1), &axis.to_string()));
    }
    for (key, value) in extra {
        cards.push(fits_card(key, value));
    }
    let mut bytes = fits_header(cards);
    for value in values {
        bytes.extend(value.to_be_bytes());
    }
    bytes.resize(bytes.len().div_ceil(2880) * 2880, 0);
    bytes
}

fn fits_image_and_table() -> Vec<u8> {
    let mut bytes = fits_i16(&[1, 2, 3, 4], &[2, 2], &[("ORIGIN", "'Goblin++'")]);
    bytes.extend(fits_header(vec![
        fits_card("XTENSION", "'BINTABLE'"),
        fits_card("BITPIX", "8"),
        fits_card("NAXIS", "2"),
        fits_card("NAXIS1", "20"),
        fits_card("NAXIS2", "3"),
        fits_card("PCOUNT", "0"),
        fits_card("GCOUNT", "1"),
        fits_card("TFIELDS", "3"),
        fits_card("EXTNAME", "'CATALOG'"),
        fits_card("TTYPE1", "'OBJECT'"),
        fits_card("TFORM1", "'8A'"),
        fits_card("TTYPE2", "'Z'"),
        fits_card("TFORM2", "'1D'"),
        fits_card("TTYPE3", "'QUALITY'"),
        fits_card("TFORM3", "'1J'"),
        fits_card("TNULL3", "-999"),
    ]));
    for (name, redshift, quality) in [
        ("GALAXY", 0.125_f64, 3_i32),
        ("QSO", 2.25_f64, 7_i32),
        ("STAR", f64::NAN, -999_i32),
    ] {
        let mut name = name.as_bytes().to_vec();
        name.resize(8, b' ');
        bytes.extend(name);
        bytes.extend(redshift.to_be_bytes());
        bytes.extend(quality.to_be_bytes());
    }
    bytes.resize(bytes.len().div_ceil(2880) * 2880, 0);
    bytes
}

fn fits_card(key: &str, value: &str) -> Vec<u8> {
    let mut card = format!("{key:<8}= {value:>20}").into_bytes();
    card.resize(80, b' ');
    card
}

fn fits_header(mut cards: Vec<Vec<u8>>) -> Vec<u8> {
    let mut end = format!("{:<8}", "END").into_bytes();
    end.resize(80, b' ');
    cards.push(end);
    let mut bytes = cards.into_iter().flatten().collect::<Vec<_>>();
    bytes.resize(bytes.len().div_ceil(2880) * 2880, b' ');
    bytes
}
