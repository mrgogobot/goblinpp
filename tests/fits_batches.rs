use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::batches::{Batches, MAX_FILE_BYTES};
use goblinpp::custody::{create_freeze, freeze_path, verify_freeze};
use goblinpp::evaluator::Value;
use goblinpp::hashing::{hash_canonical_json, sha256_file};
use goblinpp::parser::parse_source;
use goblinpp::quantity::Quantity;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::{Value as Json, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

#[test]
fn preview_has_policy_schema_and_no_project_custody_writes() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.fits"), table(&rows(), &[])).unwrap();
    let source = dir.path().join("preview.gbl");
    fs::write(&source, COPY).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["check", source.to_str().unwrap(), "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(result.status.success(), "{:?}", result);
    let report: Json = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["status"], "PASS");
    assert_eq!(report["authority"], "PREVIEW_ONLY_NOT_EVIDENCE");
    assert_eq!(report["evidence_created"], false);
    assert_eq!(report["fits_batch_policy"], goblinpp::fits::batch::policy());
    assert_eq!(
        report["batches_preview"]["readers"][0]["fits"]["table_rows"],
        4
    );
    assert_eq!(report["generated_output_preview"][0]["name"], "subset.csv");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn huge_declared_padding_refuses_without_integer_wrap_or_panic() {
    let dir = tempdir().unwrap();
    let bytes = header(&[
        ("SIMPLE", "T".into()),
        ("BITPIX", "8".into()),
        ("NAXIS", "2".into()),
        ("NAXIS1", i64::MAX.to_string()),
        ("NAXIS2", "2".into()),
    ]);
    fs::write(dir.path().join("data.fits"), bytes).unwrap();
    let mut b = Batches::default();
    let error = open(&mut b, dir.path(), &["ID"], 1, 4096).unwrap_err();
    assert!(error.to_string().contains("overflows"), "{error}");
}

#[test]
fn unselected_vectors_and_one_mib_raw_rows_need_no_eager_array() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("data.fits"),
        table(&rows(), &[("TFORM3", "2J")]),
    )
    .unwrap();
    let mut b = Batches::default();
    let h = open(&mut b, dir.path(), &["ID", "Z"], 2, 4096).unwrap();
    assert_eq!(drain(&mut b, dir.path(), &h).len(), 8);
    let width = goblinpp::fits::batch::MAX_ROW_BYTES;
    let mut bytes = header(&[
        ("SIMPLE", "T".into()),
        ("BITPIX", "8".into()),
        ("NAXIS", "0".into()),
    ]);
    bytes.extend(header(&[
        ("XTENSION", "'BINTABLE'".into()),
        ("BITPIX", "8".into()),
        ("NAXIS", "2".into()),
        ("NAXIS1", width.to_string()),
        ("NAXIS2", "2".into()),
        ("PCOUNT", "0".into()),
        ("GCOUNT", "1".into()),
        ("TFIELDS", "3".into()),
        ("TTYPE1", "'ID'".into()),
        ("TFORM1", "'1K'".into()),
        ("TTYPE2", "'Z'".into()),
        ("TFORM2", "'1D'".into()),
        ("TTYPE3", "'UNSELECTED'".into()),
        ("TFORM3", format!("'{}A'", width - 16)),
    ]));
    for id in [9007199254740993_i64, i64::MAX] {
        bytes.extend(id.to_be_bytes());
        bytes.extend(2.5_f64.to_be_bytes());
        bytes.resize(bytes.len() + width - 16, b'x');
    }
    bytes.resize(bytes.len().div_ceil(2880) * 2880, 0);
    fs::write(dir.path().join("data.fits"), bytes).unwrap();
    let mut b = Batches::default();
    let h = open(&mut b, dir.path(), &["ID", "Z"], 2, 4096).unwrap();
    assert_eq!(
        drain(&mut b, dir.path(), &h),
        ["9007199254740993", "2.5", "9223372036854775807", "2.5"]
    );
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn number(n: u64) -> Value {
    Value::Quantity(Quantity::scalar(n as f64).unwrap())
}
fn columns(names: &[&str]) -> Value {
    Value::Array(names.iter().map(|n| text(n)).collect())
}
fn open(
    b: &mut Batches,
    root: &Path,
    names: &[&str],
    rows: u64,
    bytes: u64,
) -> goblinpp::error::Result<Value> {
    b.call(
        root,
        "fits_batch_open",
        &[
            text("data.fits"),
            number(1),
            columns(names),
            number(rows),
            number(bytes),
            number(MAX_FILE_BYTES),
        ],
    )
}
fn drain(b: &mut Batches, root: &Path, h: &Value) -> Vec<String> {
    let mut values = Vec::new();
    loop {
        let Value::Array(batch) = b.call(root, "batch_next", std::slice::from_ref(h)).unwrap()
        else {
            panic!()
        };
        if batch.is_empty() {
            break;
        }
        values.extend(batch.iter().map(Value::render));
    }
    b.call(root, "batch_close", std::slice::from_ref(h))
        .unwrap();
    b.finish().unwrap();
    values
}
fn header(cards: &[(&str, String)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (name, value) in cards {
        let s = format!("{name:<8}= {value:>20}");
        assert!(s.len() <= 80);
        bytes.extend(format!("{s:<80}").bytes());
    }
    bytes.extend(format!("{:<80}", "END").bytes());
    bytes.resize(bytes.len().div_ceil(2880) * 2880, b' ');
    bytes
}
fn table(rows: &[(i64, f64, &str, u8)], extra: &[(&str, &str)]) -> Vec<u8> {
    let mut bytes = header(&[
        ("SIMPLE", "T".into()),
        ("BITPIX", "8".into()),
        ("NAXIS", "0".into()),
    ]);
    let mut cards = vec![
        ("XTENSION", "'BINTABLE'".into()),
        ("BITPIX", "8".into()),
        ("NAXIS", "2".into()),
        ("NAXIS1", "25".into()),
        ("NAXIS2", rows.len().to_string()),
        ("PCOUNT", "0".into()),
        ("GCOUNT", "1".into()),
        ("TFIELDS", "4".into()),
        ("TTYPE1", "'ID'".into()),
        ("TFORM1", "'1K'".into()),
        ("TTYPE2", "'Z'".into()),
        ("TFORM2", "'1D'".into()),
        ("TUNIT2", "'m'".into()),
        ("TTYPE3", "'LABEL'".into()),
        ("TFORM3", "'8A'".into()),
        ("TTYPE4", "'VALID'".into()),
        ("TFORM4", "'1L'".into()),
    ];
    for (k, v) in extra {
        if let Some(card) = cards.iter_mut().find(|(key, _)| key == k) {
            card.1 = (*v).into();
        } else {
            cards.push((k, (*v).into()));
        }
    }
    bytes.extend(header(&cards));
    for (id, z, label, valid) in rows {
        bytes.extend(id.to_be_bytes());
        bytes.extend(z.to_be_bytes());
        let mut label_bytes = [b' '; 8];
        assert!(label.len() <= 8);
        label_bytes[..label.len()].copy_from_slice(label.as_bytes());
        bytes.extend(label_bytes);
        bytes.push(*valid);
    }
    bytes.resize(bytes.len().div_ceil(2880) * 2880, 0);
    bytes
}
fn rows() -> Vec<(i64, f64, &'static str, u8)> {
    vec![
        (9_007_199_254_740_993, 0.5, "a,b", b'T'),
        (i64::MAX, 1.25, "quote\"", b'F'),
        (i64::MIN, f64::NAN, "", b'?'),
        (42, -0.0, "last", b'T'),
    ]
}
const COPY: &str = "GO_PARANOID\nr= fits_batch_open(\"data.fits\",1,[\"ID\",\"Z\",\"LABEL\",\"VALID\"],2,4096,2147483648)\nw=csv_stream_open(\"subset.csv\",[\"ID\",\"Z\",\"LABEL\",\"VALID\"],2147483648)\ncount=0\nwhile true {\n batch=batch_next(r)\n if len(batch)==0 { break }\n count=count+stream_write(w,batch)\n batch=[]\n}\nbatch_close(r)\nstream_close(w)\nprint(count)\nseal count\n";
fn run(root: &Path, source: &str, compiled: bool) -> PathBuf {
    let file = root.join("main.gbl");
    fs::write(&file, source).unwrap();
    run_file(
        file,
        &RunOptions {
            compile: compiled,
            ..RunOptions::default()
        },
    )
    .unwrap()
}
fn reseal(run: &Path, mut r: Json) {
    r.as_object_mut().unwrap().remove("receipt_core_sha256");
    r["receipt_core_sha256"] = json!(hash_canonical_json(&r).unwrap());
    fs::write(
        run.join("receipt.json"),
        serde_json::to_vec_pretty(&r).unwrap(),
    )
    .unwrap();
}

#[test]
fn fits_batches_native_interpreter_csv_tsv_and_exact_id_parity() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.fits"), table(&rows(), &[])).unwrap();
    for tsv in [false, true] {
        let source = if tsv {
            COPY.replace("csv_stream_open", "tsv_stream_open")
                .replace("subset.csv", "subset.tsv")
        } else {
            COPY.into()
        };
        let mut hashes = Vec::new();
        for compiled in [false, true] {
            let path = run(dir.path(), &source, compiled);
            let r = read_receipt(&path).unwrap();
            assert_eq!(r["status"], "PASS", "{r}");
            let v = verify_run(&path).unwrap();
            assert!(v.verified, "{:?}", v.checks);
            assert_eq!(r["fits_batch_policy"], goblinpp::fits::batch::policy());
            assert_eq!(r["batches"]["readers"][0]["rows"], 4);
            assert_eq!(r["batches"]["readers"][0]["batches"], 2);
            assert_eq!(
                r["batches"]["readers"][0]["fits"]["columns"][1]["unit"],
                "m"
            );
            let output = path.join(if tsv {
                "outputs/subset.tsv"
            } else {
                "outputs/subset.csv"
            });
            hashes.push(sha256_file(&output).unwrap());
            let data = fs::read_to_string(output).unwrap();
            if !tsv {
                assert_eq!(
                    data,
                    "ID,Z,LABEL,VALID\n9007199254740993,0.5,\"a,b\",true\n9223372036854775807,1.25,\"quote\"\"\",false\n-9223372036854775808,\\N,,\\N\n42,-0,last,true\n"
                );
            }
            assert!(data.contains("9007199254740993"));
            assert!(data.contains("-9223372036854775808"));
        }
        assert_eq!(hashes[0], hashes[1]);
    }
}
#[test]
fn byte_bound_pending_rows_and_exact_requested_order() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.fits"), table(&rows(), &[])).unwrap();
    let mut b = Batches::default();
    let h = open(&mut b, dir.path(), &["LABEL", "ID"], 100, 220).unwrap();
    let values = drain(&mut b, dir.path(), &h);
    assert_eq!(
        &values[..4],
        ["a,b", "9007199254740993", "quote\"", "9223372036854775807"]
    );
    assert!(
        b.evidence()["readers"][0]["peak_batch_bytes"]
            .as_u64()
            .unwrap()
            <= 220
    );
    assert!(b.evidence()["readers"][0]["batches"].as_u64().unwrap() > 1);
}
#[test]
fn empty_table_observed_eof_is_success_not_missing_rows() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.fits"), table(&[], &[])).unwrap();
    let path = run(dir.path(), COPY, false);
    let r = read_receipt(&path).unwrap();
    assert_eq!(r["status"], "PASS");
    assert_eq!(r["batches"]["readers"][0]["rows"], 0);
    assert!(verify_run(path).unwrap().verified);
}
#[test]
fn tnull_nan_scaled_values_and_float_roundtrip_bits() {
    let dir = tempdir().unwrap();
    for (extra, expected) in [
        (vec![("TNULL1", "9223372036854775807")], "\\N"),
        (vec![("TSCAL2", "2"), ("TZERO2", "1")], "2"),
    ] {
        fs::write(dir.path().join("data.fits"), table(&rows(), &extra)).unwrap();
        let mut b = Batches::default();
        let h = open(&mut b, dir.path(), &["ID", "Z"], 2, 4096).unwrap();
        let values = drain(&mut b, dir.path(), &h);
        if expected == "\\N" {
            assert_eq!(values[2], expected);
        } else {
            assert_eq!(values[1], expected);
        }
    }
    for n in [-0.0, f64::from_bits(1), f64::MAX, 1.2345678901234567] {
        fs::write(
            dir.path().join("data.fits"),
            table(&[(42, n, "x", b'T')], &[]),
        )
        .unwrap();
        let mut b = Batches::default();
        let h = open(&mut b, dir.path(), &["Z"], 1, 4096).unwrap();
        assert_eq!(
            drain(&mut b, dir.path(), &h)[0]
                .parse::<f64>()
                .unwrap()
                .to_bits(),
            n.to_bits()
        );
    }
}
#[test]
fn late_invalid_logical_preserves_full_input_and_partial_output_both_engines() {
    let dir = tempdir().unwrap();
    let mut data = rows();
    data[2].3 = b'X';
    fs::write(dir.path().join("data.fits"), table(&data, &[])).unwrap();
    for compiled in [false, true] {
        let path = run(dir.path(), COPY, compiled);
        let r = read_receipt(&path).unwrap();
        assert_eq!(r["status"], "MACHINERY_FAIL", "{r}");
        assert!(verify_run(&path).unwrap().verified);
        assert!(path.join("outputs/subset.csv.partial").exists());
        assert!(!path.join("outputs/subset.csv").exists());
        assert_eq!(r["batches"]["readers"][0]["rows"], 2);
        assert_eq!(r["batches"]["readers"][0]["failed"], true);
        assert!(r["failure"]["message"].as_str().unwrap().contains("row 2"));
    }
}
#[test]
fn explicit_open_budgets_case_names_and_hdus_refuse() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.fits"), table(&rows(), &[])).unwrap();
    for source in [
        COPY.replace("1,[\"ID\"", "0,[\"ID\""),
        COPY.replace("[\"ID\"", "[\"id\""),
        COPY.replace("[\"ID\"", "[\"MISSING\""),
        COPY.replace("2,4096", "2,1"),
        COPY.replace("2,4096", "0,4096"),
        COPY.replace("2147483648)", "10)"),
        COPY.replace("[\"ID\",\"Z\"", "[\"ID\",\"ID\""),
        COPY.replace("1,[\"ID\"", "128,[\"ID\""),
        COPY.replace("1,[\"ID\"", "-1,[\"ID\""),
        COPY.replace("2,4096", "2,8388609"),
        COPY.replace("1,[\"ID\"", "1 m,[\"ID\""),
    ] {
        let path = run(dir.path(), &source, false);
        let r = read_receipt(&path).unwrap();
        assert_eq!(r["status"], "MACHINERY_FAIL", "{source}: {r}");
        let v = verify_run(&path).unwrap();
        assert!(v.verified, "{:?}", v.checks);
    }
}
#[test]
fn unsupported_vector_scaled_id_null_marker_and_nonfinite_refuse_no_skips() {
    let dir = tempdir().unwrap();
    for (data, extra) in [
        (table(&rows(), &[("TFORM1", "'2J'")]), "ID"),
        (table(&rows(), &[("TFORM1", "'1P'")]), "ID"),
        (table(&rows(), &[("TSCAL1", "2")]), "ID"),
        (table(&[(1, f64::INFINITY, "x", b'T')], &[]), "Z"),
        (table(&[(1, 1.0, "\\N", b'T')], &[]), "LABEL"),
        (table(&rows(), &[("TTYPE2", "'ID'")]), "ID"),
        (table(&rows(), &[("ZIMAGE", "T")]), "ID"),
    ] {
        fs::write(dir.path().join("data.fits"), data).unwrap();
        let mut b = Batches::default();
        if let Ok(h) = open(&mut b, dir.path(), &[extra], 1, 4096) {
            assert!(b.call(dir.path(), "batch_next", &[h]).is_err());
        }
        assert!(b.finish().is_err());
    }
}
#[test]
fn snapshot_survives_original_deletion_and_mutation() {
    let dir = tempdir().unwrap();
    let original = table(&rows(), &[]);
    let path = dir.path().join("data.fits");
    fs::write(&path, &original).unwrap();
    let mut b = Batches::default();
    let h = open(&mut b, dir.path(), &["ID"], 2, 4096).unwrap();
    fs::write(&path, b"changed").unwrap();
    fs::remove_file(&path).unwrap();
    assert_eq!(drain(&mut b, dir.path(), &h)[0], "9007199254740993");
    let destination = dir.path().join("evidence.fits");
    let hash = b.imports()[0].sha256.clone();
    b.copy_input(&hash, &destination).unwrap().unwrap();
    assert_eq!(fs::read(destination).unwrap(), original);
}
#[test]
fn incomplete_eof_close_and_duplicate_access_fail_closed() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.fits"), table(&rows(), &[])).unwrap();
    for source in [
        COPY.replace("batch_close(r)", ""),
        COPY.replace("stream_close(w)", ""),
        COPY.replace("count=0", "batch_close(r)\ncount=0"),
        COPY.replace("count=0", "x=fits_rows(\"data.fits\",1)\ncount=0"),
        COPY.replace(
            "r= fits_batch_open",
            "x=fits_rows(\"data.fits\",1)\nr= fits_batch_open",
        ),
        COPY.replace(
            "count=0",
            "r2=fits_batch_open(\"data.fits\",1,[\"Z\"],2,4096,2147483648)\ncount=0",
        ),
    ] {
        let path = run(dir.path(), &source, false);
        assert_ne!(read_receipt(&path).unwrap()["status"], "PASS");
        assert!(verify_run(path).unwrap().verified);
    }
}
#[test]
fn metadata_headers_rows_and_truncation_are_bounded() {
    let dir = tempdir().unwrap();
    let mut variants = vec![
        vec![b' '; 1024 * 1024 + 2880],
        table(&rows(), &[("NAXIS1", "1048577"), ("NAXIS2", "0")]),
    ];
    let mut duplicates = table(&[], &[]);
    duplicates[17 * 80 + 2880..18 * 80 + 2880]
        .copy_from_slice(format!("{:<80}", "NAXIS1  =                   25").as_bytes());
    variants.push(duplicates);
    let mut many = Vec::new();
    for _ in 0..129 {
        many.extend(header(&[
            ("SIMPLE", "T".into()),
            ("XTENSION", "'IMAGE'".into()),
            ("BITPIX", "8".into()),
            ("NAXIS", "0".into()),
        ]));
    }
    variants.push(many);
    let mut truncated = table(&rows(), &[]);
    truncated.pop();
    variants.push(truncated);
    for bytes in variants {
        fs::write(dir.path().join("data.fits"), bytes).unwrap();
        let mut b = Batches::default();
        assert!(open(&mut b, dir.path(), &["ID"], 1, 4096).is_err());
    }
}
#[test]
fn policy_and_schema_resealed_tampering_cannot_downgrade_or_change_units() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.fits"), table(&rows(), &[])).unwrap();
    let path = run(dir.path(), COPY, true);
    let original = read_receipt(&path).unwrap();
    assert_eq!(original["status"], "PASS");
    for kind in ["missing", "policy", "units", "rows", "hdu", "columns"] {
        let mut r = original.clone();
        match kind {
            "missing" => {
                r.as_object_mut().unwrap().remove("fits_batch_policy");
            }
            "policy" => r["fits_batch_policy"]["null_marker"] = json!(""),
            "units" => r["batches"]["readers"][0]["fits"]["columns"][1]["unit"] = json!("s"),
            "rows" => r["batches"]["readers"][0]["fits"]["table_rows"] = json!(500),
            "hdu" => r["batches"]["readers"][0]["fits"]["hdu"] = json!(0),
            _ => r["batches"]["readers"][0]["columns"] = json!(["Z", "ID", "LABEL", "VALID"]),
        }
        // Even rebind the access summary: the preserved FITS schema disagrees.
        let summary = r["batches"]["readers"][0].clone();
        r["data_imports"][0]["access"]=json!([json!({"operation":"fits_batch_read","policy":goblinpp::fits::batch::POLICY_ID,"summary":summary}).to_string()]);
        reseal(&path, r);
        assert!(!verify_run(&path).unwrap().verified, "{kind}");
    }
    reseal(&path, original.clone());
    let mut native: Json =
        serde_json::from_slice(&fs::read(path.join("native-data.json")).unwrap()).unwrap();
    native.as_object_mut().unwrap().remove("fits_batch_policy");
    fs::write(
        path.join("native-data.json"),
        serde_json::to_vec_pretty(&native).unwrap(),
    )
    .unwrap();
    let mut r = original;
    r["execution"]["compiler"]["native_data_manifest_sha256"] =
        json!(sha256_file(path.join("native-data.json")).unwrap());
    reseal(&path, r);
    assert!(!verify_run(path).unwrap().verified);
}
#[test]
fn freeze_pins_new_policy_and_old_nonfits_freeze_stays_usable() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("main.gbl");
    fs::write(&source, "x=1\nseal x").unwrap();
    create_freeze(&source).unwrap();
    let frozen: Json = serde_json::from_slice(&fs::read(freeze_path(&source)).unwrap()).unwrap();
    assert_eq!(frozen["fits_batch_policy"], goblinpp::fits::batch::policy());
    let mut old = frozen;
    old["goblin_version"] = json!("0.1.0-alpha.31");
    old.as_object_mut().unwrap().remove("fits_batch_policy");
    old.as_object_mut().unwrap().remove("freeze_receipt_sha256");
    old["freeze_receipt_sha256"] = json!(hash_canonical_json(&old).unwrap());
    fs::write(
        freeze_path(&source),
        serde_json::to_vec_pretty(&old).unwrap(),
    )
    .unwrap();
    assert!(verify_freeze(&source).verified);
    // Register this synthetic historical receipt in the disposable fixture's
    // custody chain. Rehashing a receipt alone correctly does not authorize it.
    goblinpp::ledger::append(dir.path(),"FREEZE",goblinpp::ledger::subject_for(&source,dir.path()).unwrap(),
        json!({"source_sha256":old["source"]["sha256"],"canonical_source_sha256":old["canonical_source"]["sha256"],
            "freeze_receipt_path":"main.gbl.freeze.json","freeze_receipt_file_sha256":sha256_file(freeze_path(&source)).unwrap(),
            "freeze_receipt_core_sha256":old["freeze_receipt_sha256"]})).unwrap();
    let path = run_file(&source, &RunOptions::default()).unwrap();
    let r = read_receipt(&path).unwrap();
    assert_eq!(r["status"], "PASS", "{r}");
}
#[test]
fn old_freeze_cannot_adopt_fits_batches_without_revision() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.fits"), table(&rows(), &[])).unwrap();
    let source = dir.path().join("main.gbl");
    fs::write(&source, COPY).unwrap();
    create_freeze(&source).unwrap();
    let mut old: Json = serde_json::from_slice(&fs::read(freeze_path(&source)).unwrap()).unwrap();
    old["goblin_version"] = json!("0.1.0-alpha.31");
    old.as_object_mut().unwrap().remove("fits_batch_policy");
    old.as_object_mut().unwrap().remove("freeze_receipt_sha256");
    old["freeze_receipt_sha256"] = json!(hash_canonical_json(&old).unwrap());
    fs::write(
        freeze_path(&source),
        serde_json::to_vec_pretty(&old).unwrap(),
    )
    .unwrap();
    goblinpp::ledger::append(dir.path(),"FREEZE",goblinpp::ledger::subject_for(&source,dir.path()).unwrap(),
        json!({"source_sha256":old["source"]["sha256"],"canonical_source_sha256":old["canonical_source"]["sha256"],
            "freeze_receipt_path":"main.gbl.freeze.json","freeze_receipt_file_sha256":sha256_file(freeze_path(&source)).unwrap(),
            "freeze_receipt_core_sha256":old["freeze_receipt_sha256"]})).unwrap();
    let path = run_file(&source, &RunOptions::default()).unwrap();
    let r = read_receipt(&path).unwrap();
    assert_eq!(r["status"], "PROTOCOL_VIOLATION");
    assert_eq!(
        r["protocol_violation"]["classification"],
        "FITS_BATCH_POLICY_NOT_FROZEN"
    );
    assert!(verify_run(path).unwrap().verified);
    let parsed = parse_source(COPY).unwrap();
    // Current freezes contain the new contract, and refuse resealed removal.
    let child = dir.path().join("child.gbl");
    fs::write(&child, COPY).unwrap();
    create_freeze(&child).unwrap();
    let passed = run_file(&child, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&passed).unwrap()["status"], "PASS");
    assert!(goblinpp::fits::batch::program_uses(&parsed.program));
    let mut frozen: Json = serde_json::from_slice(&fs::read(freeze_path(&child)).unwrap()).unwrap();
    frozen.as_object_mut().unwrap().remove("fits_batch_policy");
    frozen
        .as_object_mut()
        .unwrap()
        .remove("freeze_receipt_sha256");
    frozen["freeze_receipt_sha256"] = json!(hash_canonical_json(&frozen).unwrap());
    fs::write(
        freeze_path(&child),
        serde_json::to_vec_pretty(&frozen).unwrap(),
    )
    .unwrap();
    assert!(!verify_freeze(child).verified);
}
#[test]
fn new_names_preserve_historical_canonical_parse_but_refuse_execution_collision() {
    for source in [
        "g_func fits_batch_open(x) { return x }\ny=fits_batch_open(1)",
        "g_func f(fits_batch_open) {return fits_batch_open}",
        "fits_batch_open(\"x\",1,[\"Z\"],1,4096,5000)",
    ] {
        let parsed = parse_source(source).unwrap();
        assert!(goblinpp::parser::validate_execution(&parsed.program).is_err());
    }
    let mut b = Batches::default();
    assert!(b.call(Path::new("."), "fits_batch_open", &[]).is_err());
}
#[test]
fn historical_alpha31_nonfits_receipt_verifies_and_policy_diff_is_explicit() {
    let dir = tempdir().unwrap();
    let path = run(dir.path(), "x=1\nseal x", false);
    let original = read_receipt(&path).unwrap();
    let mut old = original.clone();
    old["goblin_version"] = json!("0.1.0-alpha.31");
    old.as_object_mut().unwrap().remove("fits_batch_policy");
    reseal(&path, old);
    assert!(verify_run(&path).unwrap().verified);
    let current = run(dir.path(), "x=1\nseal x", false);
    assert_eq!(
        diff_runs(path, current).unwrap().classification,
        "FITS_BATCH_POLICY_CHANGE"
    );
}
#[test]
fn standalone_native_preserves_snapshot_and_partial_failure_without_a_launcher() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("data.fits"), table(&rows(), &[])).unwrap();
    for failure in [false, true] {
        let source = if failure {
            COPY.replace(
                "stream_close(w)",
                "bad=parse_number(\"stop\")\nstream_close(w)",
            )
        } else {
            COPY.into()
        };
        let parsed = parse_source(&source).unwrap();
        let output = dir.path().join(if failure { "bad" } else { "good" });
        let compiled = goblinpp::compiler::compile(&parsed, &output, &[]).unwrap();
        let run = Command::new(compiled.binary)
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert_eq!(run.status.success(), !failure);
        let stderr = String::from_utf8(run.stderr).unwrap();
        let prefix = if failure {
            "INCOMPLETE_DATA_DIR="
        } else {
            "NATIVE_DATA_DIR="
        };
        let directory = dir
            .path()
            .join(stderr.lines().find_map(|s| s.strip_prefix(prefix)).unwrap());
        let evidence: Json = serde_json::from_slice(
            &fs::read(directory.join(if failure {
                "batch-failure.json"
            } else {
                "native-data.json"
            }))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            evidence["fits_batch_policy"],
            goblinpp::fits::batch::policy()
        );
        let inputs: Json =
            serde_json::from_slice(&fs::read(directory.join("batch-inputs.json")).unwrap())
                .unwrap();
        assert_eq!(
            sha256_file(directory.join(inputs[0]["path"].as_str().unwrap())).unwrap(),
            inputs[0]["sha256"]
        );
        assert!(
            directory
                .join(if failure {
                    "outputs/subset.csv.partial"
                } else {
                    "native-outputs/subset.csv"
                })
                .exists()
        );
    }
}
