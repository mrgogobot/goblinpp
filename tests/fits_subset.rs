use goblinpp::audit::verify_run;
use goblinpp::fits::{FilterValue, FitsFile, filter_from_text, filter_group, filter_where};
use goblinpp::hashing::sha256_bytes;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use std::{fs, path::PathBuf};
use tempfile::tempdir;

#[derive(Clone)]
struct Row {
    id: i64,
    z: f64,
    quality: i16,
    object: [u8; 8],
    valid: u8,
    scaled: i16,
}

fn rows() -> Vec<Row> {
    vec![
        Row {
            id: 9_007_199_254_740_993,
            z: 0.4,
            quality: 1,
            object: *b"GALAXY  ",
            valid: b'T',
            scaled: 2,
        },
        Row {
            id: 9_007_199_254_740_992,
            z: 0.7,
            quality: 0,
            object: *b"QSO     ",
            valid: b'F',
            scaled: 3,
        },
        Row {
            id: i64::MAX,
            z: 1.1,
            quality: 1,
            object: *b"GALAXY  ",
            valid: b'T',
            scaled: 4,
        },
        Row {
            id: i64::MIN,
            z: f64::NAN,
            quality: 1,
            object: *b"GALAXY  ",
            valid: b'T',
            scaled: 5,
        },
        Row {
            id: 9_007_199_254_740_994,
            z: 0.8,
            quality: -999,
            object: *b"GALAXY  ",
            valid: b'?',
            scaled: 6,
        },
        Row {
            id: 42,
            z: 0.9,
            quality: 1,
            object: *b"G,\"X    ",
            valid: b'T',
            scaled: -999,
        },
        Row {
            id: 43,
            z: 0.9,
            quality: 2,
            object: *b"GALAXY  ",
            valid: b'T',
            scaled: 8,
        },
        Row {
            id: 44,
            z: 0.9,
            quality: 1,
            object: *b"GALAXY  ",
            valid: b'T',
            scaled: 9,
        },
    ]
}

fn fits(rows: &[Row], extra: &[(&str, &str)]) -> FitsFile {
    FitsFile::from_bytes(PathBuf::from("catalog.fits"), table(rows, extra)).unwrap()
}

fn where_number(column: &str, op: &str, n: f64) -> String {
    filter_where(column, op, Some(FilterValue::Number(n))).unwrap()
}

fn cut() -> String {
    let either = filter_group(
        &[
            filter_where("OBJECT", "==", Some(FilterValue::Text("GALAXY".into()))).unwrap(),
            filter_where("SOURCE_ID", "==", Some(FilterValue::Text("42".into()))).unwrap(),
        ],
        false,
    )
    .unwrap();
    filter_group(
        &[
            where_number("Z", ">=", 0.4),
            where_number("Z", "<", 1.1),
            where_number("QUALITY", "==", 1.0),
            either,
        ],
        true,
    )
    .unwrap()
}

fn export(
    fits: &FitsFile,
    filter: &str,
    names: &[&str],
    tsv: bool,
) -> (goblinpp::output::GeneratedOutput, usize) {
    fits.export_subset(
        if tsv { "subset.tsv" } else { "subset.csv" },
        1,
        &names.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        &filter_from_text(filter).unwrap(),
        tsv,
    )
    .unwrap()
}

#[test]
fn combined_cuts_export_exact_ids_scaled_cells_and_quoted_text_in_original_order() {
    let fits = fits(&rows(), &[]);
    let (artifact, count) = export(
        &fits,
        &cut(),
        &["source_id", "Z", "OBJECT", "SCALED"],
        false,
    );
    assert_eq!(count, 3);
    assert_eq!(
        String::from_utf8(artifact.bytes).unwrap(),
        "SOURCE_ID,Z,OBJECT,SCALED\n9007199254740993,0.4,GALAXY,2\n42,0.9,\"G,\"\"X\",\\N\n44,0.9,GALAXY,5.5\n"
    );
    assert_eq!(artifact.metadata["input_rows"], 8);
    assert_eq!(artifact.metadata["selected_rows"], 3);
    assert_eq!(artifact.metadata["rejected_rows"], 5);
    assert_eq!(artifact.metadata["predicate_null_rows"], 2);
    assert_eq!(
        artifact.metadata["output_null_counts"],
        serde_json::json!([0, 0, 0, 1])
    );
    assert_eq!(artifact.metadata["projection"][3]["unit"], "m");
    assert_eq!(artifact.metadata["input_sha256"], fits.sha256);
    assert_eq!(artifact.metadata["physical_blinding"], false);
}

#[test]
fn adjacent_large_ids_and_i64_extremes_never_pass_through_f64() {
    let fits = fits(&rows(), &[]);
    for (index, expected) in [
        (0, "9007199254740993"),
        (1, "9007199254740992"),
        (2, "9223372036854775807"),
        (3, "-9223372036854775808"),
    ] {
        assert_eq!(fits.column_text(1, "SOURCE_ID", index).unwrap(), expected);
        let filter =
            filter_where("SOURCE_ID", "==", Some(FilterValue::Text(expected.into()))).unwrap();
        let (artifact, count) = export(&fits, &filter, &["SOURCE_ID"], false);
        assert_eq!(count, 1);
        assert_eq!(
            String::from_utf8(artifact.bytes).unwrap(),
            format!("SOURCE_ID\n{expected}\n")
        );
        assert!(
            fits.column_value(1, "SOURCE_ID", index, None)
                .unwrap_err()
                .message
                .contains("fits_column_text")
        );
    }
    let filter = where_number("SOURCE_ID", ">", 9_007_199_254_740_992.0);
    assert!(
        fits.export_subset(
            "x.csv",
            1,
            &["SOURCE_ID".into()],
            &filter_from_text(&filter).unwrap(),
            false
        )
        .unwrap_err()
        .message
        .contains("quote large")
    );
}

#[test]
fn null_semantics_are_explicit_and_empty_selection_is_header_only() {
    let fits = fits(&rows(), &[]);
    let null = filter_where("Z", "is_null", None).unwrap();
    let (artifact, count) = export(&fits, &null, &["SOURCE_ID", "Z"], true);
    assert_eq!(count, 1);
    assert_eq!(
        String::from_utf8(artifact.bytes).unwrap(),
        "SOURCE_ID\tZ\n-9223372036854775808\t\\N\n"
    );
    assert!(
        fits.column_text(1, "Z", 3)
            .unwrap_err()
            .message
            .contains("null")
    );
    let (_, count) = export(&fits, &where_number("Z", "!=", 0.4), &["Z"], false);
    assert_eq!(count, 6); // NaN must not satisfy !=.
    let (artifact, count) = export(&fits, &where_number("Z", ">", 20.0), &["Z"], false);
    assert_eq!(count, 0);
    assert_eq!(artifact.bytes, b"Z\n");
    let bool_cut = filter_where("VALID", "==", Some(FilterValue::Boolean(true))).unwrap();
    assert_eq!(export(&fits, &bool_cut, &["VALID"], false).1, 6);
    assert_eq!(
        export(
            &fits,
            &filter_where("QUALITY", "is_valid", None).unwrap(),
            &["QUALITY"],
            false
        )
        .1,
        7
    );
    let all = filter_where("SOURCE_ID", "is_valid", None).unwrap();
    assert_eq!(export(&fits, &all, &["SOURCE_ID"], false).1, 8);
    let integer_null = FitsFile::from_bytes(
        PathBuf::from("ids.fits"),
        table(&rows(), &[("TNULL1", "42")]),
    )
    .unwrap();
    let null_id = filter_where("SOURCE_ID", "is_null", None).unwrap();
    let (artifact, count) = export(&integer_null, &null_id, &["SOURCE_ID"], false);
    assert_eq!(count, 1);
    assert_eq!(artifact.bytes, b"SOURCE_ID\n\\N\n");
}

#[test]
fn invalid_predicates_columns_shapes_and_encodings_fail_before_export() {
    let fits = fits(&[], &[]); // Binding must still reject invalid requests on an empty table.
    for (column, op, value) in [
        ("MISSING", "==", FilterValue::Number(1.0)),
        ("OBJECT", "==", FilterValue::Number(1.0)),
        ("VALID", ">", FilterValue::Boolean(true)),
        ("Z", "==", FilterValue::Text("0.4".into())),
        (
            "SOURCE_ID",
            "==",
            FilterValue::Text("9223372036854775808".into()),
        ),
        ("SOURCE_ID", "==", FilterValue::Number(0.5)),
    ] {
        let filter = filter_from_text(&filter_where(column, op, Some(value)).unwrap()).unwrap();
        assert!(
            fits.export_subset("x.csv", 1, &["Z".into()], &filter, false)
                .is_err()
        );
    }
    let filter = filter_from_text(&where_number("Z", ">", 0.0)).unwrap();
    for names in [vec![], vec!["Z".into(), "z".into()], vec!["MISSING".into()]] {
        assert!(
            fits.export_subset("x.csv", 1, &names, &filter, false)
                .is_err()
        );
    }
    for name in ["../x.csv", "/tmp/x.csv", "x.txt"] {
        assert!(
            fits.export_subset(name, 1, &["Z".into()], &filter, false)
                .is_err()
        );
    }
    assert!(
        fits.export_subset("x.csv", 0, &["Z".into()], &filter, false)
            .is_err()
    );
    assert!(filter_group(&[], true).is_err());
    assert!(filter_where("Z", "bogus", Some(FilterValue::Number(1.0))).is_err());
    assert!(filter_where("Z", "is_null", Some(FilterValue::Number(1.0))).is_err());
    assert!(filter_where("Z", "==", None).is_err());
    assert!(filter_where("Z", "==", Some(FilterValue::Number(f64::NAN))).is_err());
    assert!(filter_from_text("{}").is_err());
    let mut document: serde_json::Value =
        serde_json::from_str(&where_number("Z", ">", 0.0)).unwrap();
    document["schema"] = "future".into();
    assert!(filter_from_text(&document.to_string()).is_err());
    document["schema"] = "goblin.fits-filter.v1".into();
    document["unknown"] = true.into();
    assert!(filter_from_text(&document.to_string()).is_err());
}

#[test]
fn scaled_large_ids_unsupported_vectors_and_ambiguous_names_are_refused() {
    let filter = filter_from_text(&where_number("Z", ">=", 0.0)).unwrap();
    let projection = vec!["SOURCE_ID".into()];
    for extra in [
        vec![("TSCAL1", "2")],
        vec![("TZERO1", "1")],
        vec![("TFORM1", "'2J'")],
        vec![("TTYPE2", "'SOURCE_ID'")],
    ] {
        let fits = fits(&rows(), &extra);
        assert!(
            fits.export_subset("x.csv", 1, &projection, &filter, false)
                .is_err()
        );
    }
    let mut rows = rows();
    rows[0].object = *b"\\N      ";
    let reserved = fits(&rows, &[]);
    assert!(
        reserved
            .export_subset("x.csv", 1, &["OBJECT".into()], &filter, false)
            .unwrap_err()
            .message
            .contains("null marker")
    );
    rows[0].object = *b"a\tb     ";
    let tabs = fits(&rows, &[]);
    assert!(
        tabs.export_subset("x.tsv", 1, &["OBJECT".into()], &filter, true)
            .unwrap_err()
            .message
            .contains("use CSV")
    );
    assert!(
        tabs.export_subset("x.csv", 1, &["OBJECT".into()], &filter, false)
            .is_ok()
    );
    rows[0].z = f64::INFINITY;
    assert!(
        fits(&rows, &[])
            .export_subset("x.csv", 1, &["Z".into()], &filter, false)
            .is_err()
    );
}

#[test]
fn full_table_scan_crosses_chunk_boundary_without_array_or_loop_caps() {
    let rows = vec![rows()[0].clone(); 400_001];
    let fits = fits(&rows, &[]);
    let (artifact, count) = export(&fits, &cut(), &["SOURCE_ID"], false);
    assert_eq!(count, rows.len());
    assert_eq!(artifact.bytes.len(), b"SOURCE_ID\n".len() + 17 * rows.len());
    assert!(artifact.bytes.ends_with(b"9007199254740993\n"));
    assert_eq!(artifact.metadata["input_rows"], 400_001);
}

#[test]
fn filter_depth_and_node_budgets_are_explicit() {
    let leaf = where_number("Z", ">", 0.0);
    assert!(filter_group(&vec![leaf.clone(); 256], true).is_err());
    let mut nested = leaf;
    for _ in 0..15 {
        nested = filter_group(&[nested], true).unwrap();
    }
    assert!(filter_group(&[nested], true).is_err());
}

#[test]
fn both_engines_seal_subset_provenance_and_detect_output_tampering() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("catalog.fits"), table(&rows(), &[])).unwrap();
    let source = root.path().join("subset.gbl");
    fs::write(&source, r#"GO_PARANOID
g_func selection() {
    zmin = fits_where("Z", ">=", 0.4)
    zmax = fits_where("Z", "<", 1.1)
    quality = fits_where("QUALITY", "==", 1)
    galaxy = fits_where("OBJECT", "==", "GALAXY")
    special = fits_where("SOURCE_ID", "==", "42")
    either = fits_any([galaxy, special])
    return fits_all([zmin, zmax, quality, either])
}
cut = selection()
count = fits_export_csv("subset.csv", "catalog.fits", 1, ["SOURCE_ID", "Z", "OBJECT", "SCALED"], cut)
fits_export_tsv("ids.tsv", "catalog.fits", 1, ["SOURCE_ID"], cut)
first_id = fits_column_text("catalog.fits", 1, "SOURCE_ID", 0)
print("rows = {count}; exact ID = {first_id}")
seal count
seal first_id
"#).unwrap();
    let mut evidence = Vec::new();
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
        assert_eq!(receipt["paranoid_postflight"]["status"], "PASS");
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "rows = 3; exact ID = 9007199254740993\n"
        );
        assert_eq!(
            receipt["data_imports"][0]["access"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        let generated = receipt["generated_artifacts"].as_array().unwrap();
        assert_eq!(generated.len(), 2);
        assert!(
            generated
                .iter()
                .all(|a| a["metadata"]["selected_rows"] == 3)
        );
        let payload = fs::read(run.join("outputs/subset.csv")).unwrap();
        let readback =
            goblinpp::delimited::Table::open(&run.join("outputs/subset.csv"), b',').unwrap();
        assert_eq!(readback.rows[0][0], "9007199254740993");
        assert_eq!(readback.rows[1][2], "G,\"X");
        assert_eq!(readback.rows[1][3], "\\N");
        let ids = goblinpp::delimited::Table::open(&run.join("outputs/ids.tsv"), b'\t').unwrap();
        assert_eq!(
            ids.rows,
            vec![vec!["9007199254740993"], vec!["42"], vec!["44"]]
        );
        assert_eq!(
            generated
                .iter()
                .find(|a| a["name"] == "subset.csv")
                .unwrap()["sha256"],
            sha256_bytes(&payload)
        );
        assert!(verify_run(&run).unwrap().verified);
        evidence.push((
            receipt["sealed_artifacts"].clone(),
            generated.clone(),
            payload,
        ));
        fs::write(run.join("outputs/subset.csv"), b"wrong ID\n").unwrap();
        assert!(!verify_run(&run).unwrap().verified);
    }
    assert_eq!(evidence[0], evidence[1]);
}

#[test]
fn both_modes_preserve_failures_without_partial_export_artifacts() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("catalog.fits"), table(&rows(), &[])).unwrap();
    let source = root.path().join("bad.gbl");
    for program in [
        "GO_PARANOID\ncut = fits_where(\"MISSING\", \">\", 0)\nfits_export_csv(\"bad.csv\", \"catalog.fits\", 1, [\"Z\"], cut)\n",
        "GO_PARANOID\ncut = fits_where(\"Z\", \">\", 1 m)\n",
        "GO_PARANOID\nx = fits_column(\"catalog.fits\", 1, \"SOURCE_ID\", 0)\n",
    ] {
        fs::write(&source, program).unwrap();
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
            assert_eq!(receipt["generated_artifacts"], serde_json::json!([]));
            assert!(!run.join("outputs/bad.csv").exists());
            assert!(verify_run(&run).unwrap().verified);
        }
    }
}

#[test]
fn new_pure_builtins_cannot_be_discarded_or_shadowed() {
    for source in [
        "fits_where(\"Z\", \">\", 0)\n",
        "g_func fits_all(x) { return x }\n",
    ] {
        let parsed = goblinpp::parser::parse_source(source).unwrap();
        assert!(
            goblinpp::evaluator::Evaluation::new(".")
                .eval_program(&parsed.program)
                .is_err()
        );
    }
}

#[test]
fn standalone_native_failures_do_not_depend_on_interpreter_preflight() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("catalog.fits"), table(&rows(), &[])).unwrap();
    for (index, source, expected) in [
        (
            0,
            "cut = fits_where(\"MISSING\", \">\", 0)\nfits_export_csv(\"bad.csv\", \"catalog.fits\", 1, [\"Z\"], cut)\n",
            "no MISSING column",
        ),
        (
            1,
            "x = fits_column(\"catalog.fits\", 1, \"SOURCE_ID\", 0)\n",
            "safe f64 integer range",
        ),
        (2, "cut = fits_where(\"Z\", \">\", 1 m)\n", "dimensionless"),
    ] {
        let parsed = goblinpp::parser::parse_source(source).unwrap();
        let binary = root.path().join(format!("bad{index}"));
        goblinpp::compiler::compile(&parsed, &binary, &[]).unwrap();
        let output = std::process::Command::new(&binary)
            .current_dir(root.path())
            .env("GOBLIN_NATIVE_DATA_BASE", root.path())
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(expected), "{stderr}");
        assert!(!root.path().join("outputs/bad.csv").exists());
    }
}

#[test]
fn subset_float_text_round_trips_bits_without_quantizing() {
    for n in [-0.0, f64::from_bits(1), f64::MAX, 1.2345678901234567] {
        let mut rows = rows();
        rows[0].z = n;
        let fits = fits(&rows[..1], &[]);
        let cut = filter_where("Z", "is_valid", None).unwrap();
        let (artifact, _) = export(&fits, &cut, &["Z"], false);
        let text = String::from_utf8(artifact.bytes).unwrap();
        assert_eq!(
            text.lines()
                .nth(1)
                .unwrap()
                .parse::<f64>()
                .unwrap()
                .to_bits(),
            n.to_bits()
        );
    }
}

fn table(rows: &[Row], extra: &[(&str, &str)]) -> Vec<u8> {
    let mut bytes = header(&[
        ("SIMPLE", "T"),
        ("BITPIX", "8"),
        ("NAXIS", "0"),
        ("EXTEND", "T"),
    ]);
    let mut cards = vec![
        ("XTENSION", "'BINTABLE'".to_string()),
        ("BITPIX", "8".into()),
        ("NAXIS", "2".into()),
        ("NAXIS1", "29".into()),
        ("NAXIS2", rows.len().to_string()),
        ("PCOUNT", "0".into()),
        ("GCOUNT", "1".into()),
        ("TFIELDS", "6".into()),
        ("TTYPE1", "'SOURCE_ID'".into()),
        ("TFORM1", "'1K'".into()),
        ("TTYPE2", "'Z'".into()),
        ("TFORM2", "'1D'".into()),
        ("TTYPE3", "'QUALITY'".into()),
        ("TFORM3", "'1I'".into()),
        ("TNULL3", "-999".into()),
        ("TTYPE4", "'OBJECT'".into()),
        ("TFORM4", "'8A'".into()),
        ("TTYPE5", "'VALID'".into()),
        ("TFORM5", "'1L'".into()),
        ("TTYPE6", "'SCALED'".into()),
        ("TFORM6", "'1I'".into()),
        ("TNULL6", "-999".into()),
        ("TSCAL6", "0.5".into()),
        ("TZERO6", "1".into()),
        ("TUNIT6", "'m'".into()),
    ];
    for (key, value) in extra {
        if let Some(card) = cards.iter_mut().find(|card| card.0 == *key) {
            card.1 = value.to_string();
        } else {
            cards.push((key, value.to_string()));
        }
    }
    bytes.extend(header(
        &cards
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect::<Vec<_>>(),
    ));
    for row in rows {
        bytes.extend(row.id.to_be_bytes());
        bytes.extend(row.z.to_be_bytes());
        bytes.extend(row.quality.to_be_bytes());
        bytes.extend(row.object);
        bytes.push(row.valid);
        bytes.extend(row.scaled.to_be_bytes());
    }
    bytes.resize(bytes.len().div_ceil(2880) * 2880, 0);
    bytes
}

fn header(cards: &[(&str, &str)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (key, value) in cards {
        let mut card = format!("{key:<8}= {value:>20}").into_bytes();
        card.resize(80, b' ');
        bytes.extend(card);
    }
    let mut end = b"END".to_vec();
    end.resize(80, b' ');
    bytes.extend(end);
    bytes.resize(bytes.len().div_ceil(2880) * 2880, b' ');
    bytes
}
