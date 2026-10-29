use goblinpp::{
    audit::verify_run,
    compiler::compile,
    evaluator::{Evaluation, Value},
    hashing::{hash_canonical_json, sha256_bytes},
    parser::parse_source,
    quantity::Quantity,
    runtime::{RunOptions, read_receipt, run_file},
    scientific_io::{self, Inputs},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::tempdir;
fn fixture(group: &str, name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scientific_io")
        .join(group)
        .join(name)
}
fn get(group: &str, file: &str, op: &str, selector: Option<Value>) -> Value {
    Inputs::default()
        .call(&fixture(group, file), op, selector)
        .unwrap()
}
fn sel(s: &str) -> Option<Value> {
    Some(Value::Text(s.into()))
}
fn numbers(v: Value) -> Vec<f64> {
    if let Value::Array(a) = v {
        a.into_iter()
            .map(|v| v.quantity("test").unwrap().value_si)
            .collect()
    } else {
        panic!("array required")
    }
}
fn npy(header: &str, payload: &[u8]) -> Vec<u8> {
    let mut h = header.to_owned();
    while !(10 + h.len() + 1).is_multiple_of(64) {
        h.push(' ')
    }
    h.push('\n');
    let mut b = b"\x93NUMPY\x01\x00".to_vec();
    b.extend((h.len() as u16).to_le_bytes());
    b.extend(h.as_bytes());
    b.extend(payload);
    b
}
fn zip(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    use std::io::{Cursor, Write};
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (n, b) in entries {
        w.start_file(
            n,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        w.write_all(&b).unwrap();
    }
    w.finish().unwrap().into_inner()
}
fn fails(bytes: &[u8], op: &str, selector: Option<Value>) {
    let t = tempdir().unwrap();
    let p = t.path().join("input");
    fs::write(&p, bytes).unwrap();
    assert!(
        Inputs::default().call(&p, op, selector).is_err(),
        "unexpectedly accepted {op}"
    );
}
#[test]
fn upstream_fixture_manifests_match_bytes() {
    for group in ["numpy", "astropy"] {
        let r: serde_json::Value =
            serde_json::from_slice(&fs::read(fixture(group, "REFERENCE.json")).unwrap()).unwrap();
        let files = r["files"].as_object().unwrap();
        assert!(!files.is_empty());
        for (name, expected) in files {
            let actual = sha256_bytes(&fs::read(fixture(group, name)).unwrap());
            assert_eq!(
                expected
                    .as_str()
                    .or_else(|| expected["sha256"].as_str())
                    .unwrap(),
                actual
            );
        }
    }
}
#[test]
fn numpy_c_and_fortran_are_logically_equal() {
    for name in [
        "numeric_c.npy",
        "numeric_f.npy",
        "version2.npy",
        "version3.npy",
    ] {
        assert_eq!(
            numbers(get("numpy", name, "npy_read", None)),
            vec![1., 2., 3., 4., 5., 6.]
        );
        assert_eq!(numbers(get("numpy", name, "npy_shape", None)), vec![2., 3.]);
    }
    assert_eq!(
        get("numpy", "numeric_f.npy", "npy_order", None).render(),
        "F"
    );
}
#[test]
fn endian_float32_signed_zero_scalar_and_empty() {
    let a = numbers(get("numpy", "big_endian.npy", "npy_read", None));
    assert_eq!(a[0].to_bits(), (0.1f32 as f64).to_bits());
    assert_eq!(a[1].to_bits(), (-0.0f64).to_bits());
    assert_eq!(a[2].to_bits(), (1e-30f32 as f64).to_bits());
    assert_eq!(
        numbers(get("numpy", "scalar.npy", "npy_read", None)),
        vec![3.5]
    );
    assert!(numbers(get("numpy", "scalar.npy", "npy_shape", None)).is_empty());
    assert!(numbers(get("numpy", "empty.npy", "npy_read", None)).is_empty());
}
#[test]
fn exact_ids_never_become_rounded_floats() {
    let text = get("numpy", "ids.npy", "npy_text", None);
    assert_eq!(text.render(), "[9007199254740993, 18446744073709551615]");
    assert!(
        Inputs::default()
            .call(&fixture("numpy", "ids.npy"), "npy_read", None)
            .is_err()
    );
    assert!(
        Inputs::default()
            .call(&fixture("numpy", "numeric_c.npy"), "npy_text", None)
            .is_err()
    );
}
#[test]
fn unicode_and_boolean_dtypes_preserve_values() {
    assert_eq!(
        get("numpy", "unicode.npy", "npy_read", None).render(),
        "[Ada π, 科学, ]"
    );
    assert_eq!(
        get("numpy", "bytes.npy", "npy_text", None).render(),
        "[A, π]"
    );
    assert_eq!(
        get("numpy", "booleans.npy", "npy_read", None).render(),
        "[true, false]"
    );
    for file in ["objects.npy", "nonfinite.npy", "invalid_utf8.npy"] {
        assert!(
            Inputs::default()
                .call(&fixture("numpy", file), "npy_read", None)
                .is_err()
        );
    }
}
#[test]
fn malformed_numpy_headers_payloads_and_unsupported_types_refuse() {
    for header in [
        "{'descr':'<f8','fortran_order':False,'shape':(-1,)}",
        "{'descr':'<f8','fortran_order':False,'shape':(100001,)}",
        "{'descr':'<f8','fortran_order':False,'shape':(2)}",
        "{'descr':'<f8','descr':'<f8','fortran_order':False,'shape':()}",
        "{'descr': [('x','<f8')],'fortran_order':False,'shape':()}",
        "{'descr':'=f8','fortran_order':False,'shape':()}",
        "{'descr':'<c16','fortran_order':False,'shape':()}",
        "{'descr':'<f2','fortran_order':False,'shape':()}",
    ] {
        fails(&npy(header, &[0; 8]), "npy_read", None);
    }
    let good = fs::read(fixture("numpy", "numeric_c.npy")).unwrap();
    fails(&good[..good.len() - 1], "npy_read", None);
    let mut extra = good;
    extra.push(0);
    fails(&extra, "npy_read", None);
}
#[test]
fn npz_stored_and_deflate_match_upstream() {
    for name in ["stored.npz", "compressed.npz"] {
        assert_eq!(
            get("numpy", name, "npz_keys", None).render(),
            "[ids, labels, matrix]"
        );
        assert_eq!(
            numbers(get("numpy", name, "npz_read", sel("matrix"))),
            vec![1., 2., 3., 4., 5., 6.]
        );
        assert_eq!(get("numpy", name, "npz_order", sel("matrix")).render(), "F");
        assert_eq!(
            get("numpy", name, "npz_text", sel("ids")).render(),
            "[9007199254740993, 18446744073709551615]"
        );
    }
}
#[test]
fn zip_paths_prefixes_trailers_and_crc_tampering_refuse() {
    let b = fs::read(fixture("numpy", "numeric_c.npy")).unwrap();
    for name in ["../a.npy", "folder/a.npy", "a\\b.npy", "a.txt", ":x.npy"] {
        fails(&zip(vec![(name, b.clone())]), "npz_keys", None);
    }
    let mut archive = fs::read(fixture("numpy", "stored.npz")).unwrap();
    archive.push(1);
    fails(&archive, "npz_keys", None);
    let mut archive = fs::read(fixture("numpy", "stored.npz")).unwrap();
    let at = archive.windows(4).position(|x| x == b"PK\x01\x02").unwrap();
    archive[at + 16] ^= 1;
    fails(&archive, "npz_read", sel("matrix"));
}
#[test]
fn sparse_formats_match_scipy_dense_and_rows() {
    for file in ["csr.npz", "csc.npz", "coo.npz", "csr_array.npz"] {
        assert_eq!(
            numbers(get("numpy", file, "scipy_sparse_dense", None)),
            vec![0., 1., 0., 2., 3., 0., 0., 0., 0., 0., 4., 0.]
        );
        assert_eq!(
            numbers(get("numpy", file, "scipy_sparse_shape", None)),
            vec![3., 4.]
        );
        assert_eq!(get("numpy", file, "scipy_sparse_nnz", None).render(), "4");
        assert_eq!(
            numbers(get(
                "numpy",
                file,
                "scipy_sparse_row",
                Some(Value::Quantity(Quantity::scalar(1.).unwrap()))
            )),
            vec![3., 0., 0., 0.]
        );
    }
    assert!(
        Inputs::default()
            .call(
                &fixture("numpy", "duplicates.npz"),
                "scipy_sparse_dense",
                None
            )
            .is_err()
    );
}
#[test]
fn ecsv_astropy_space_and_comma_units_and_ids() {
    for file in ["space.ecsv", "comma.ecsv"] {
        assert_eq!(get("astropy", file, "ecsv_rows", None).render(), "2");
        assert_eq!(
            numbers(get("astropy", file, "ecsv_numbers", sel("distance"))),
            vec![1250., 2500.]
        );
        let a = get("astropy", file, "ecsv_numbers", sel("velocity"));
        if let Value::Array(a) = a {
            assert_eq!(
                a[0].quantity("test").unwrap().dimension,
                [0, 1, -1, 0, 0, 0]
            );
        } else {
            panic!()
        };
        assert_eq!(
            get("astropy", file, "ecsv_column", sel("source_id")).render(),
            "[9007199254740993, 18446744073709551615]"
        );
        assert_eq!(
            get("astropy", file, "ecsv_read", sel("accepted")).render(),
            "[true, false]"
        );
        assert!(
            Inputs::default()
                .call(&fixture("astropy", file), "ecsv_numbers", sel("source_id"))
                .is_err()
        );
    }
}
#[test]
fn ecsv_unsupported_units_masks_and_mixins_are_not_silently_discarded() {
    assert!(
        Inputs::default()
            .call(
                &fixture("astropy", "unsupported_unit.ecsv"),
                "ecsv_numbers",
                sel("flux")
            )
            .is_err()
    );
    for file in ["masked.ecsv", "mixin.ecsv"] {
        assert!(
            Inputs::default()
                .call(&fixture("astropy", file), "ecsv_rows", None)
                .is_err()
        );
    }
    for text in [
        "# %ECSV 1.0\n# ---\n# datatype:\n# - {name: x, datatype: float64, subtype: json}\nx\n1\n",
        "# %ECSV 1.0\n# ---\n# datatype:\n# - {name: x, datatype: float64}\n# meta: !!python/object:evil {}\nx\n1\n",
    ] {
        fails(text.as_bytes(), "ecsv_rows", None);
    }
}
#[test]
fn snapshots_are_immutable_and_failures_keep_original_bytes() {
    let t = tempdir().unwrap();
    let p = t.path().join("input.npy");
    let original = fs::read(fixture("numpy", "numeric_c.npy")).unwrap();
    fs::write(&p, &original).unwrap();
    let mut inputs = Inputs::default();
    let a = inputs.call(&p, "npy_read", None).unwrap();
    fs::write(&p, b"tampered").unwrap();
    assert_eq!(
        a.to_json(),
        inputs.call(&p, "npy_read", None).unwrap().to_json()
    );
    let dest = t.path().join("copy.npy");
    inputs
        .copy(&sha256_bytes(&original), &dest)
        .unwrap()
        .unwrap();
    assert_eq!(fs::read(dest).unwrap(), original);
    let mut invalid = Inputs::default();
    let p = fixture("numpy", "objects.npy");
    assert!(invalid.call(&p, "npy_read", None).is_err());
    assert_eq!(invalid.imports().len(), 1);
}
#[test]
fn wrong_arity_discarded_results_and_reserved_function_names_refuse() {
    for source in [
        "npy_read(\"x\")",
        "x=npy_read()",
        "x=npz_read(\"a\")",
        "g_func npy_read(x) { return x }",
    ] {
        let parsed = parse_source(&format!("{source}\n")).unwrap();
        assert!(
            Evaluation::new(".").eval_program(&parsed.program).is_err(),
            "{source}"
        );
    }
}
fn program(t: &Path) -> PathBuf {
    for (g, f) in [
        ("numpy", "numeric_f.npy"),
        ("numpy", "csr.npz"),
        ("astropy", "space.ecsv"),
    ] {
        fs::copy(fixture(g, f), t.join(f)).unwrap();
    }
    let p = t.join("read.gbl");
    fs::write(&p,"GO_PARANOID\na=npy_read(\"numeric_f.npy\")\nb=scipy_sparse_dense(\"csr.npz\")\nv=ecsv_numbers(\"space.ecsv\",\"velocity\")\nids=ecsv_column(\"space.ecsv\",\"source_id\")\nprint(\"{a}; {v}; {ids}\")\nseal a\nseal b\nseal v\nseal ids\n").unwrap();
    p
}
#[test]
fn both_engines_and_standalone_preserve_identical_results_and_inputs() {
    let t = tempdir().unwrap();
    let p = program(t.path());
    let mut values = Vec::new();
    for compile in [false, true] {
        let run = run_file(
            &p,
            &RunOptions {
                compile,
                ..Default::default()
            },
        )
        .unwrap();
        let r = read_receipt(&run).unwrap();
        assert_eq!(r["status"], "PASS", "{}", r["failure"]);
        assert!(verify_run(&run).unwrap().verified, "{run:?}");
        assert_eq!(r["data_imports"].as_array().unwrap().len(), 3);
        values.push((
            r["sealed_artifacts"].clone(),
            fs::read(run.join("stdout.log")).unwrap(),
            run,
        ));
    }
    assert_eq!(values[0].0, values[1].0);
    assert_eq!(values[0].1, values[1].1);
    let parsed = parse_source(&fs::read_to_string(&p).unwrap()).unwrap();
    let native = compile(&parsed, t.path().join("standalone"), &[]).unwrap();
    let output = Command::new(native.binary)
        .current_dir(t.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, values[0].1);
    for f in ["numeric_f.npy", "csr.npz", "space.ecsv"] {
        fs::remove_file(t.path().join(f)).unwrap();
    }
    for v in values {
        assert!(verify_run(v.2).unwrap().verified);
    }
}
#[test]
fn failed_reader_runs_verify_and_policy_access_tampering_is_detected() {
    let t = tempdir().unwrap();
    fs::copy(
        fixture("numpy", "objects.npy"),
        t.path().join("objects.npy"),
    )
    .unwrap();
    let p = t.path().join("bad.gbl");
    fs::write(&p, "GO_PARANOID\nx=npy_read(\"objects.npy\")\n").unwrap();
    for compile in [false, true] {
        let run = run_file(
            &p,
            &RunOptions {
                compile,
                ..Default::default()
            },
        )
        .unwrap();
        let mut r = read_receipt(&run).unwrap();
        assert_eq!(r["status"], "MACHINERY_FAIL");
        assert!(verify_run(&run).unwrap().verified);
        let mut a: serde_json::Value =
            serde_json::from_str(r["data_imports"][0]["access"][0].as_str().unwrap()).unwrap();
        a["error"] = serde_json::json!("fake");
        r["data_imports"][0]["access"][0] = serde_json::json!(a.to_string());
        assert!(!scientific_io::verify_access(&run, &r));
        r = read_receipt(&run).unwrap();
        r["scientific_input_policy"]["input_bytes"] = serde_json::json!(0);
        assert!(!scientific_io::verify_access(&run, &r));
    }
}
#[test]
fn successful_reader_cannot_hide_policy_by_version_downgrade() {
    let t = tempdir().unwrap();
    let p = program(t.path());
    let run = run_file(&p, &RunOptions::default()).unwrap();
    let mut r = read_receipt(&run).unwrap();
    r["goblin_version"] = serde_json::json!("0.1.0-alpha.40");
    r.as_object_mut().unwrap().remove("scientific_input_policy");
    r["data_imports"] = serde_json::json!([]);
    assert!(scientific_io::needs_policy(&run, &r));
    assert!(!scientific_io::verify_access(&run, &r));
    let _ = hash_canonical_json(&r).unwrap();
}

#[test]
fn freeze_pins_readers_and_historical_freeze_requires_explicit_revision() {
    use goblinpp::custody::{create_freeze, verify_freeze};
    use serde_json::json;
    for uses_readers in [false, true] {
        let t = tempdir().unwrap();
        let p = if uses_readers {
            program(t.path())
        } else {
            let p = t.path().join("old.gbl");
            fs::write(&p, "print(\"old\")\n").unwrap();
            p
        };
        let (path, event) = create_freeze(&p).unwrap();
        let mut frozen: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(frozen["scientific_input_policy"], scientific_io::policy());
        frozen["goblin_version"] = json!("0.1.0-alpha.40");
        frozen
            .as_object_mut()
            .unwrap()
            .remove("scientific_input_policy");
        frozen
            .as_object_mut()
            .unwrap()
            .remove("freeze_receipt_sha256");
        frozen["freeze_receipt_sha256"] = json!(hash_canonical_json(&frozen).unwrap());
        let bytes = serde_json::to_vec_pretty(&frozen).unwrap();
        fs::write(path, &bytes).unwrap();
        let mut payload = event.payload;
        payload["freeze_receipt_core_sha256"] = frozen["freeze_receipt_sha256"].clone();
        payload["freeze_receipt_file_sha256"] = json!(sha256_bytes(&bytes));
        goblinpp::ledger::append(
            goblinpp::ledger::project_root_for(&p).unwrap(),
            "FREEZE",
            event.subject,
            payload,
        )
        .unwrap();
        assert!(verify_freeze(&p).verified);
        let run = run_file(&p, &RunOptions::default()).unwrap();
        let r = read_receipt(&run).unwrap();
        assert_eq!(
            r["status"],
            if uses_readers {
                "PROTOCOL_VIOLATION"
            } else {
                "PASS"
            }
        );
        assert!(verify_run(run).unwrap().verified);
    }
}
#[test]
fn standalone_failure_keeps_original_input_and_replayable_error() {
    let t = tempdir().unwrap();
    let source = "x=npy_read(\"objects.npy\")\n";
    fs::copy(
        fixture("numpy", "objects.npy"),
        t.path().join("objects.npy"),
    )
    .unwrap();
    let parsed = parse_source(source).unwrap();
    let native = compile(&parsed, t.path().join("bad"), &[]).unwrap();
    let output = Command::new(native.binary)
        .current_dir(t.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    let dir = t.path().join(
        stderr
            .lines()
            .find_map(|s| s.strip_prefix("INCOMPLETE_DATA_DIR="))
            .unwrap(),
    );
    let r: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("batch-failure.json")).unwrap()).unwrap();
    assert_eq!(r["data_imports"].as_array().unwrap().len(), 1);
    assert!(scientific_io::verify_access(&dir, &r));
}
#[cfg(unix)]
#[test]
fn evidence_symlink_parents_and_original_symlinks_refuse() {
    use std::os::unix::fs::symlink;
    let t = tempdir().unwrap();
    let link = t.path().join("alias.npy");
    symlink(fixture("numpy", "numeric_c.npy"), &link).unwrap();
    assert!(Inputs::default().call(&link, "npy_read", None).is_err());
    let p = program(t.path());
    let run = run_file(&p, &RunOptions::default()).unwrap();
    let mut r = read_receipt(&run).unwrap();
    let source = r["data_imports"][0]["evidence_path"].as_str().unwrap();
    let target = run.join(source);
    let parent = target.parent().unwrap();
    let alias = run.join("alias");
    symlink(parent, &alias).unwrap();
    r["data_imports"][0]["evidence_path"] = serde_json::json!(format!(
        "alias/{}",
        target.file_name().unwrap().to_str().unwrap()
    ));
    assert!(!scientific_io::verify_access(&run, &r));
}
#[test]
fn metadata_listing_is_not_a_promise_that_all_payloads_are_valid() {
    let object = fs::read(fixture("numpy", "objects.npy")).unwrap();
    let archive = zip(vec![("object.npy", object)]);
    let t = tempdir().unwrap();
    let p = t.path().join("objects.npz");
    fs::write(&p, archive).unwrap();
    let mut inputs = Inputs::default();
    assert_eq!(
        inputs.call(&p, "npz_keys", None).unwrap().render(),
        "[object]"
    );
    assert!(inputs.call(&p, "npz_read", sel("object")).is_err());
}
#[test]
fn explicit_limits_and_malformed_sparse_pointers_refuse() {
    let scalar = |dtype: &str, payload: &[u8]| {
        npy(
            &format!("{{'descr':'{dtype}','fortran_order':False,'shape':()}}"),
            payload,
        )
    };
    let vector = |shape: usize, payload: &[u8]| {
        npy(
            &format!("{{'descr':'<i8','fortran_order':False,'shape':({shape},)}}"),
            payload,
        )
    };
    let i64bytes = |a: &[i64]| a.iter().flat_map(|n| n.to_le_bytes()).collect::<Vec<_>>();
    for pointers in [[0, 2, 1], [1, 1, 1], [0, 1, 2]] {
        let archive = zip(vec![
            ("format.npy", scalar("|S3", b"csr")),
            ("shape.npy", vector(2, &i64bytes(&[2, 2]))),
            ("data.npy", vector(1, &i64bytes(&[1]))),
            ("indices.npy", vector(1, &i64bytes(&[0]))),
            ("indptr.npy", vector(3, &i64bytes(&pointers))),
        ]);
        fails(&archive, "scipy_sparse_dense", None);
    }
    let t = tempdir().unwrap();
    let p = t.path().join("large");
    let file = fs::File::create(&p).unwrap();
    file.set_len(scientific_io::MAX_BYTES as u64 + 1).unwrap();
    assert!(Inputs::default().call(&p, "npy_read", None).is_err());
    let mut inputs = Inputs::default();
    for _ in 0..64 {
        inputs
            .call(&fixture("numpy", "scalar.npy"), "npy_shape", None)
            .unwrap();
    }
    assert!(
        inputs
            .call(&fixture("numpy", "scalar.npy"), "npy_shape", None)
            .is_err()
    );
}
