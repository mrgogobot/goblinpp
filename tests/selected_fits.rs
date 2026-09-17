use goblinpp::audit::verify_run;
use goblinpp::fits::FitsFile;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn selection_and_optional_weights_match_independent_arithmetic_and_verify() {
    let root = tempdir().unwrap();
    let catalog = root.path().join("catalog.fits");
    fs::write(&catalog, table(&sample_rows())).unwrap();
    let fits = FitsFile::open(&catalog).unwrap();
    let weighted = fits
        .selected_column_stats(1, "z", 0.5, 1.5, "signal", Some("weight"))
        .unwrap();
    assert_eq!(weighted.selected_rows, 4);
    assert_eq!(weighted.used_rows, 2);
    assert_eq!(weighted.weight_sum, 4.0);
    assert_eq!(weighted.mean, (2.0 * 1.0 + 4.0 * 3.0) / 4.0);
    let unweighted = fits
        .selected_column_stats(1, "Z", 0.5, 1.5, "SIGNAL", None)
        .unwrap();
    assert_eq!(unweighted.selected_rows, 4);
    assert_eq!(unweighted.used_rows, 3);
    assert_eq!(unweighted.weight_sum, 3.0);
    assert!((unweighted.mean - (2.0 + 4.0 + 8.0) / 3.0).abs() < 1e-12);

    let source = root.path().join("selection.gbl");
    fs::write(
        &source,
        "GO_PARANOID\nweighted = fits_select_stats(\"catalog.fits\", 1, \"Z\", 0.5, 1.5, \"SIGNAL\", \"WEIGHT\")\nplain = fits_select_stats(\"catalog.fits\", 1, \"Z\", 0.5, 1.5, \"SIGNAL\")\nselected = weighted[0]\nused = weighted[1]\nweight_sum = weighted[2]\nmean = weighted[3]\nprint(\"selected = {selected}; used = {used}; weight sum = {weight_sum}; mean = {mean}\")\nseal weighted\nseal plain\n",
    )
    .unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "PASS");
    assert_eq!(receipt["paranoid_postflight"]["status"], "PASS");
    assert_eq!(receipt["data_imports"][0]["sha256"], fits.sha256);
    let accesses = receipt["data_imports"][0]["access"].as_array().unwrap();
    assert_eq!(accesses.len(), 2);
    let parsed = accesses
        .iter()
        .map(|entry| serde_json::from_str::<Value>(entry.as_str().unwrap()).unwrap())
        .collect::<Vec<_>>();
    assert!(
        parsed
            .iter()
            .all(|entry| entry["operation"] == "fits_select_stats")
    );
    assert!(parsed.iter().all(|entry| entry["lower_inclusive"] == 0.5));
    assert!(parsed.iter().all(|entry| entry["upper_exclusive"] == 1.5));
    assert_eq!(
        fs::read_to_string(run.join("stdout.log")).unwrap(),
        "selected = 4; used = 2; weight sum = 4; mean = 3.5\n"
    );
    assert_eq!(receipt["sealed_artifacts"].as_array().unwrap().len(), 2);
    assert!(verify_run(&run).unwrap().verified);

    let compiled = run_file(
        &source,
        &RunOptions {
            compile: true,
            ..RunOptions::default()
        },
    )
    .unwrap();
    let compiled_receipt = read_receipt(&compiled).unwrap();
    assert_eq!(compiled_receipt["status"], "MACHINERY_FAIL");
    assert_eq!(compiled_receipt["failure"]["code"], "G501");
    assert!(verify_run(&compiled).unwrap().verified);
}

#[test]
fn invalid_selection_weights_and_columns_fail_explicitly() {
    let root = tempdir().unwrap();
    let catalog = root.path().join("catalog.fits");
    fs::write(&catalog, table(&sample_rows())).unwrap();
    let fits = FitsFile::open(&catalog).unwrap();
    for (lower, upper) in [(1.0, 1.0), (2.0, 1.0), (f64::NAN, 2.0)] {
        assert!(
            fits.selected_column_stats(1, "Z", lower, upper, "SIGNAL", None)
                .unwrap_err()
                .message
                .contains("lower < upper")
        );
    }
    assert!(
        fits.selected_column_stats(1, "Z", 4.0, 5.0, "SIGNAL", None)
            .unwrap_err()
            .message
            .contains("no rows")
    );
    assert!(
        fits.selected_column_stats(1, "SIGNAL", 0.0, 1.0, "Z", Some("OBJECT"))
            .unwrap_err()
            .message
            .contains("scalar real numeric")
    );

    let mut negative_rows = sample_rows();
    negative_rows[0].2 = -2;
    let negative =
        FitsFile::from_bytes(PathBuf::from("negative.fits"), table(&negative_rows)).unwrap();
    assert!(
        negative
            .selected_column_stats(1, "Z", 0.5, 1.5, "SIGNAL", Some("WEIGHT"))
            .unwrap_err()
            .message
            .contains("non-positive weight")
    );

    let mut bad_rows = sample_rows();
    bad_rows[0].2 = 0;
    fs::write(&catalog, table(&bad_rows)).unwrap();
    let source = root.path().join("bad_weight.gbl");
    fs::write(&source, "GO_PARANOID\nstats = fits_select_stats(\"catalog.fits\", 1, \"Z\", 0.5, 1.5, \"SIGNAL\", \"WEIGHT\")\nseal stats\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["failure"]["code"], "G601");
    assert!(
        receipt["failure"]["message"]
            .as_str()
            .unwrap()
            .contains("non-positive weight")
    );
    assert!(verify_run(&run).unwrap().verified);

    fs::write(
        &source,
        "stats = fits_select_stats(\"catalog.fits\", 1, \"Z\", 0.5 m, 1.5, \"SIGNAL\")\n",
    )
    .unwrap();
    let dimensioned = run_file(&source, &RunOptions::default()).unwrap();
    let dimensioned_receipt = read_receipt(&dimensioned).unwrap();
    assert_eq!(dimensioned_receipt["status"], "MACHINERY_FAIL");
    assert_eq!(dimensioned_receipt["failure"]["code"], "G601");
    assert!(verify_run(dimensioned).unwrap().verified);
}

#[test]
fn selection_crosses_the_eight_megabyte_chunk_boundary() {
    let rows = vec![(2_i16, 7.0_f64, 2_i32); 600_000];
    let fits = FitsFile::from_bytes(PathBuf::from("large.fits"), table(&rows)).unwrap();
    let stats = fits
        .selected_column_stats(1, "Z", 1.0, 1.5, "SIGNAL", Some("WEIGHT"))
        .unwrap();
    assert_eq!(stats.selected_rows, rows.len());
    assert_eq!(stats.used_rows, rows.len());
    assert_eq!(stats.weight_sum, 1_200_000.0);
    assert_eq!(stats.mean, 7.0);
}

fn sample_rows() -> Vec<(i16, f64, i32)> {
    vec![
        (1, 2.0, 1),
        (2, 4.0, 3),
        (3, 6.0, 2),
        (2, f64::NAN, 1),
        (-999, 10.0, 1),
        (2, 8.0, -999),
    ]
}

fn table(rows: &[(i16, f64, i32)]) -> Vec<u8> {
    let mut bytes = header(vec![
        card("SIMPLE", "T"),
        card("BITPIX", "8"),
        card("NAXIS", "0"),
        card("EXTEND", "T"),
    ]);
    bytes.extend(header(vec![
        card("XTENSION", "'BINTABLE'"),
        card("BITPIX", "8"),
        card("NAXIS", "2"),
        card("NAXIS1", "18"),
        card("NAXIS2", &rows.len().to_string()),
        card("PCOUNT", "0"),
        card("GCOUNT", "1"),
        card("TFIELDS", "4"),
        card("TTYPE1", "'Z'"),
        card("TFORM1", "'1I'"),
        card("TSCAL1", "0.5"),
        card("TNULL1", "-999"),
        card("TTYPE2", "'SIGNAL'"),
        card("TFORM2", "'1D'"),
        card("TTYPE3", "'WEIGHT'"),
        card("TFORM3", "'1J'"),
        card("TNULL3", "-999"),
        card("TTYPE4", "'OBJECT'"),
        card("TFORM4", "'4A'"),
    ]));
    for (z, signal, weight) in rows {
        bytes.extend(z.to_be_bytes());
        bytes.extend(signal.to_be_bytes());
        bytes.extend(weight.to_be_bytes());
        bytes.extend(b"OBJ ");
    }
    bytes.resize(bytes.len().div_ceil(2880) * 2880, 0);
    bytes
}

fn card(key: &str, value: &str) -> Vec<u8> {
    let mut bytes = format!("{key:<8}= {value:>20}").into_bytes();
    bytes.resize(80, b' ');
    bytes
}

fn header(mut cards: Vec<Vec<u8>>) -> Vec<u8> {
    let mut end = format!("{:<8}", "END").into_bytes();
    end.resize(80, b' ');
    cards.push(end);
    let mut bytes = cards.into_iter().flatten().collect::<Vec<_>>();
    bytes.resize(bytes.len().div_ceil(2880) * 2880, b' ');
    bytes
}
