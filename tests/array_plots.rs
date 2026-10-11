use goblinpp::audit::verify_run;
use goblinpp::custody::{create_freeze, freeze_path, verify_freeze};
use goblinpp::evaluator::{Evaluation, Value};
use goblinpp::hashing::{hash_canonical_json, sha256_bytes};
use goblinpp::output::artifact_descriptor;
use goblinpp::parser::parse_source;
use goblinpp::plots::{Axis, Input, POLICY, build, verify_pair};
use goblinpp::quantity::{DIMENSIONLESS, Quantity};
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::json;
use std::{fs, process::Command};
use tempfile::tempdir;

fn input(name: &str, kind: &str, x: &[f64], y: &[f64]) -> Input {
    let axis = |label: &str, values: &[f64]| Axis {
        label: label.into(),
        dimension: DIMENSIONLESS,
        values_bits: values
            .iter()
            .map(|v| format!("{:016x}", v.to_bits()))
            .collect(),
    };
    Input {
        figure: name.into(),
        kind: kind.into(),
        title: "Honest data".into(),
        x: axis("x", x),
        y: if kind == "histogram" {
            None
        } else {
            Some(axis("y", y))
        },
        bins: (kind == "histogram").then_some(2),
    }
}
fn document(output: &[goblinpp::output::GeneratedOutput]) -> serde_json::Value {
    serde_json::from_slice(&output[1].bytes).unwrap()
}
fn eval(code: &str) -> goblinpp::error::Result<Evaluation> {
    Evaluation::new(".").eval_program(&parse_source(code)?.program)
}
const SOURCE: &str = "GO_PARANOID\nx = [0 s, 1 s, 2 s]\ny = [0 m, 2 m, 4 m]\nplot_line(\"a.png\", x, y, \"Motion\", \"time\", \"distance\")\nplot_scatter(\"b.svg\", x, y, \"Motion\", \"time\", \"distance\")\nplot_histogram(\"c.png\", y, 2, \"Distance\", \"distance\")\nseal y\n";

#[test]
fn three_kinds_and_both_formats_reconstruct_exactly() {
    for kind in ["line", "scatter", "histogram"] {
        for extension in ["png", "svg"] {
            let outputs = build(&input(
                &format!("a.{extension}"),
                kind,
                &[0., 1., 2.],
                &[2., 0., 1.],
            ))
            .unwrap();
            verify_pair(
                &outputs[0].bytes,
                &outputs[1].bytes,
                &artifact_descriptor(&outputs[0]),
                &artifact_descriptor(&outputs[1]),
            )
            .unwrap();
            assert_eq!(document(&outputs)["schema"], POLICY);
            assert_eq!(document(&outputs)["derived"]["sampling"], "none");
            if extension == "png" {
                assert!(outputs[0].bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
            } else {
                assert!(String::from_utf8_lossy(&outputs[0].bytes).contains("<svg"));
            }
        }
    }
}
#[test]
fn exact_bits_signed_zero_and_order_are_preserved() {
    let values = [2., -0., 0., 0.12345678901234566];
    let output = build(&input("a.svg", "line", &values, &values)).unwrap();
    let data = document(&output);
    assert_eq!(data["input"]["x"]["values_bits"][1], "8000000000000000");
    assert_eq!(data["input"]["x"]["values_bits"][2], "0000000000000000");
    assert_eq!(
        data["input"]["x"]["values_bits"][3],
        format!("{:016x}", values[3].to_bits())
    );
    let reverse = build(&input("a.svg", "line", &[0., 2., 1.], &[0., 2., 1.])).unwrap();
    let sorted = build(&input("a.svg", "line", &[0., 1., 2.], &[0., 1., 2.])).unwrap();
    assert_ne!(reverse[0].bytes, sorted[0].bytes);
}
#[test]
fn compatible_units_normalize_each_axis_independently() {
    let e =
        eval("plot_line(\"a.svg\", [1 m, 2 km], [1 s, 2 s], \"Units\", \"distance\", \"time\")")
            .unwrap();
    let sidecar: serde_json::Value =
        serde_json::from_slice(&e.generated["a.svg.plot.json"].bytes).unwrap();
    assert_eq!(sidecar["derived"]["x_label"], "X: distance [m]");
    assert_eq!(sidecar["derived"]["y_label"], "Y: time [s]");
    assert_eq!(
        sidecar["input"]["x"]["values_bits"][1],
        format!("{:016x}", 2000_f64.to_bits())
    );
}
#[test]
fn empty_mismatched_nonnumeric_and_mixed_units_refuse() {
    for (x, y) in [
        ("[]", "[]"),
        ("[1]", "[1,2]"),
        ("[1 m, 2 s]", "[1,2]"),
        ("[true, false]", "[1,2]"),
        ("[\"a\",\"b\"]", "[1,2]"),
        ("[[1],[2]]", "[1,2]"),
        ("1", "[1,2]"),
    ] {
        assert!(
            eval(&format!(
                "plot_scatter(\"a.svg\", {x}, {y}, \"Title\", \"x\", \"y\")"
            ))
            .is_err(),
            "{x} {y}"
        );
    }
    assert!(build(&input("a.svg", "line", &[1.], &[2.])).is_err());
    assert!(build(&input("a.svg", "scatter", &[1.], &[2.])).is_ok());
}
#[test]
fn nonfinite_data_and_invalid_dimension_evidence_refuse() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(build(&input("a.svg", "scatter", &[value], &[1.])).is_err());
    }
    let mut data = input("a.svg", "scatter", &[1.], &[1.]);
    data.x.dimension[0] = i32::MIN;
    assert!(build(&data).is_err());
    data.x.dimension[0] = 0;
    data.x.values_bits[0] = "ABC".into();
    assert!(build(&data).is_err());
}
#[test]
fn direct_call_refuses_forged_nonfinite_quantity() {
    let q = Value::Quantity(Quantity {
        value_si: f64::INFINITY,
        dimension: DIMENSIONLESS,
    });
    let args = vec![
        Value::Text("a.svg".into()),
        Value::Array(vec![q]),
        Value::Quantity(Quantity::scalar(2.).unwrap()),
        Value::Text("t".into()),
        Value::Text("x".into()),
    ];
    assert!(goblinpp::plots::call("plot_histogram", &args).is_err());
}
#[test]
fn histogram_edges_include_every_value_and_final_endpoint() {
    let output = build(&input("a.svg", "histogram", &[0., 0.5, 1., 1.5, 2.], &[])).unwrap();
    assert_eq!(document(&output)["derived"]["bin_counts"], json!([2, 3]));
    assert_eq!(
        document(&output)["derived"]["bin_edges_bits"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        document(&output)["derived"]["y_bounds_bits"][0],
        "0000000000000000"
    );
}
#[test]
fn constant_axes_expand_without_losing_values() {
    for v in [0., 1., -1., f64::MAX, -f64::MAX] {
        let output = build(&input("a.svg", "histogram", &[v, v, v], &[])).unwrap();
        assert_eq!(
            document(&output)["derived"]["bin_counts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_u64().unwrap())
                .sum::<u64>(),
            3
        );
    }
}
#[test]
fn unrepresentable_spans_and_collapsed_bins_refuse() {
    assert!(
        build(&input(
            "a.svg",
            "scatter",
            &[-f64::MAX, f64::MAX],
            &[1., 2.]
        ))
        .is_err()
    );
    assert!(
        build(&input(
            "a.svg",
            "histogram",
            &[1., f64::from_bits(1_f64.to_bits() + 1)],
            &[]
        ))
        .is_err()
    );
}
#[test]
fn bin_count_requires_exact_dimensionless_integer() {
    for count in ["1", "501", "2.5", "2 m"] {
        assert!(
            eval(&format!(
                "plot_histogram(\"a.svg\", [1,2], {count}, \"t\", \"x\")"
            ))
            .is_err()
        );
    }
}
#[test]
fn no_traversal_or_unsupported_extension() {
    for name in ["../a.svg", "/a.png", "a.jpg", "a.txt"] {
        assert!(build(&input(name, "scatter", &[1.], &[2.])).is_err());
    }
}
#[test]
fn unicode_svg_escapes_markup_and_png_refuses_missing_glyphs() {
    let mut data = input("a.svg", "scatter", &[1.], &[2.]);
    data.title = "数据 <script> & \"quoted\"".into();
    let output = build(&data).unwrap();
    let svg = String::from_utf8_lossy(&output[0].bytes);
    assert!(svg.contains("数据 &lt;script&gt; &amp; &quot;quoted&quot;"));
    assert!(!svg.contains("<script>"));
    data.figure = "a.png".into();
    assert!(build(&data).unwrap_err().message.contains("Use SVG"));
}
#[test]
fn control_overlong_and_bidi_labels_refuse() {
    let mut data = input("a.svg", "scatter", &[1.], &[2.]);
    for text in ["bad\nlabel".into(), "x".repeat(71), "x\u{202e}y".into()] {
        data.title = text;
        assert!(build(&data).is_err());
    }
}
#[test]
fn png_preserves_unit_case() {
    let mut data = input("a.png", "scatter", &[1.], &[2.]);
    data.x.label = "m".into();
    let lower = build(&data).unwrap();
    data.x.label = "M".into();
    let upper = build(&data).unwrap();
    assert_ne!(lower[0].bytes, upper[0].bytes);
}
#[test]
fn malformed_kind_shape_and_unknown_fields_refuse() {
    let data = input("a.svg", "scatter", &[1.], &[2.]);
    let mut malformed = data.clone();
    malformed.kind = "pie".into();
    assert!(build(&malformed).is_err());
    malformed = data.clone();
    malformed.bins = Some(2);
    assert!(build(&malformed).is_err());
    malformed = data.clone();
    malformed.y = None;
    assert!(build(&malformed).is_err());
    let mut value = serde_json::to_value(&data).unwrap();
    value["unrecorded_setting"] = json!(true);
    assert!(serde_json::from_value::<Input>(value).is_err());
}
#[test]
fn arity_refuses_before_clock_or_io_side_effects() {
    let parsed = parse_source("plot_line(time_date())\n").unwrap();
    let dir = tempdir().unwrap();
    assert!(goblinpp::compiler::compile(&parsed, dir.path().join("native"), &[]).is_err());
    assert!(eval("plot_line(time_date())").is_err());
}
#[test]
fn function_names_cannot_be_shadowed() {
    assert!(eval("g_func plot_line(a) { return a }\n").is_err());
    assert!(eval("g_func test(plot_line) { return plot_line }\n").is_err());
}
#[test]
fn collision_refuses_both_halves_atomically_and_failure_is_preserved() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("a.gbl");
    fs::write(&source,"write_json(\"a.svg.plot.json\", \"old\", 1)\nplot_line(\"a.svg\", [1,2], [3,4], \"t\", \"x\", \"y\")\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&run).unwrap();
    assert_eq!(receipt["status"], "MACHINERY_FAIL");
    assert_eq!(receipt["generated_artifacts"].as_array().unwrap().len(), 1);
    assert!(!run.join("outputs/a.svg").exists());
    assert!(verify_run(&run).unwrap().verified);
}
#[test]
fn sidecar_data_settings_and_figure_tampering_refuse() {
    let output = build(&input("a.svg", "scatter", &[1., 2.], &[2., 3.])).unwrap();
    for path in ["title", "derived", "schema", "hash"] {
        let mut altered = document(&output);
        match path {
            "title" => altered["input"]["title"] = json!("Other"),
            "derived" => altered["derived"]["grid"] = json!(false),
            "schema" => altered["schema"] = json!("unknown"),
            _ => altered["figure_sha256"] = json!("bad"),
        };
        assert!(
            verify_pair(
                &output[0].bytes,
                &serde_json::to_vec_pretty(&altered).unwrap(),
                &artifact_descriptor(&output[0]),
                &artifact_descriptor(&output[1])
            )
            .is_err()
        );
    }
    let mut image = output[0].bytes.clone();
    image.push(b' ');
    assert!(
        verify_pair(
            &image,
            &output[1].bytes,
            &artifact_descriptor(&output[0]),
            &artifact_descriptor(&output[1])
        )
        .is_err()
    );
}
#[test]
fn resealed_figure_tamper_still_fails_reconstruction() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("a.gbl");
    fs::write(&source, SOURCE).unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    assert!(verify_run(&run).unwrap().verified);
    let path = run.join("outputs/a.png");
    let mut bytes = fs::read(&path).unwrap();
    bytes[100] ^= 1;
    fs::write(&path, &bytes).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    for a in receipt["generated_artifacts"].as_array_mut().unwrap() {
        if a["name"] == "a.png" {
            a["sha256"] = json!(sha256_bytes(&bytes));
        }
    }
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
    let verified = verify_run(&run).unwrap();
    assert!(!verified.verified);
    assert!(
        verified
            .checks
            .iter()
            .any(|c| c.check == "PLOT_RECONSTRUCTION:a.png" && !c.pass)
    );
}
#[test]
fn missing_sidecar_cannot_verify() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("a.gbl");
    fs::write(&source, SOURCE).unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    fs::rename(
        run.join("outputs/a.png.plot.json"),
        run.join("saved-sidecar.json"),
    )
    .unwrap();
    assert!(!verify_run(run).unwrap().verified);
}
#[test]
fn interpreted_and_native_outputs_and_evidence_are_identical() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("a.gbl");
    fs::write(&source, SOURCE).unwrap();
    let mut previous = None;
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
        assert_eq!(receipt["status"], "PASS", "{receipt:#}");
        assert_eq!(receipt["plot_policy"], POLICY);
        assert_eq!(receipt["generated_artifacts"].as_array().unwrap().len(), 6);
        assert!(
            verify_run(&run).unwrap().verified,
            "{:#?}",
            verify_run(&run)
        );
        if let Some(ref previous) = previous {
            assert_eq!(previous, &receipt["generated_artifacts"]);
        }
        previous = Some(receipt["generated_artifacts"].clone());
    }
}
#[test]
fn standalone_native_binary_preserves_reconstructable_pair() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("a.gbl");
    fs::write(
        &source,
        "plot_line(\"a.svg\", [1,2], [3,4], \"t\", \"x\", \"y\")\n",
    )
    .unwrap();
    let binary = dir.path().join("native");
    let parsed = parse_source(&fs::read_to_string(source).unwrap()).unwrap();
    goblinpp::compiler::compile(&parsed, &binary, &[]).unwrap();
    let output = Command::new(binary)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    let directory = stderr
        .lines()
        .find_map(|l| l.strip_prefix("NATIVE_DATA_DIR="))
        .unwrap();
    let root = dir.path().join(directory);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("native-data.json")).unwrap()).unwrap();
    let artifacts = manifest["generated_artifacts"].as_array().unwrap();
    assert_eq!(manifest["plot_policy"], POLICY);
    verify_pair(
        &fs::read(root.join("native-outputs/a.svg")).unwrap(),
        &fs::read(root.join("native-outputs/a.svg.plot.json")).unwrap(),
        &artifacts[0],
        &artifacts[1],
    )
    .unwrap();
}
#[test]
fn current_freeze_pins_plotting_and_unknown_policy_refuses() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("a.gbl");
    fs::write(&source, SOURCE).unwrap();
    create_freeze(&source).unwrap();
    let frozen = verify_freeze(&source).receipt.unwrap();
    assert_eq!(frozen["plot_policy"], POLICY);
    assert!(verify_freeze(&source).verified);
    assert_eq!(
        read_receipt(run_file(&source, &RunOptions::default()).unwrap()).unwrap()["status"],
        "PASS"
    );
    assert!(!goblinpp::plots::frozen_policy_matches(
        &json!({"goblin_version":goblinpp::VERSION,"plot_policy":"unknown"})
    ));
    assert!(freeze_path(source).exists());
}
#[test]
fn historical_nonplot_receipt_without_policy_still_verifies() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("a.gbl");
    fs::write(&source, "print(\"old\")\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let mut receipt = read_receipt(&run).unwrap();
    receipt["goblin_version"] = json!("0.1.0-alpha.29");
    receipt.as_object_mut().unwrap().remove("plot_policy");
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
    assert!(verify_run(run).unwrap().verified);
}

#[test]
fn historical_freeze_requires_revision_only_for_new_plot_calls() {
    for uses_plots in [false, true] {
        let dir = tempdir().unwrap();
        let source = dir.path().join("old.gbl");
        fs::write(
            &source,
            if uses_plots {
                SOURCE
            } else {
                "print(\"hello\")\n"
            },
        )
        .unwrap();
        let (path, event) = create_freeze(&source).unwrap();
        let mut frozen: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        frozen["goblin_version"] = json!("0.1.0-alpha.29");
        frozen.as_object_mut().unwrap().remove("plot_policy");
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
            goblinpp::ledger::project_root_for(&source).unwrap(),
            "FREEZE",
            event.subject,
            payload,
        )
        .unwrap();
        assert!(verify_freeze(&source).verified);
        let run = run_file(&source, &RunOptions::default()).unwrap();
        let receipt = read_receipt(&run).unwrap();
        assert_eq!(
            receipt["status"],
            if uses_plots {
                "PROTOCOL_VIOLATION"
            } else {
                "PASS"
            }
        );
        assert!(verify_run(run).unwrap().verified);
        if uses_plots {
            let compiled = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
                .arg("compile")
                .arg(&source)
                .arg("-o")
                .arg(dir.path().join("native"))
                .output()
                .unwrap();
            assert!(!compiled.status.success());
            assert!(String::from_utf8_lossy(&compiled.stderr).contains("Plotting is not pinned"));
        }
    }
}

#[test]
fn maximum_array_is_preserved_and_over_budget_refuses() {
    let values = vec![0.; 100000];
    let output = build(&input("a.svg", "histogram", &values, &[])).unwrap();
    assert_eq!(
        document(&output)["input"]["x"]["values_bits"]
            .as_array()
            .unwrap()
            .len(),
        100000
    );
    assert!(build(&input("a.svg", "histogram", &vec![0.; 100001], &[])).is_err());
}

#[test]
fn removed_current_policy_and_bad_partner_metadata_fail() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("a.gbl");
    fs::write(&source, SOURCE).unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let original = read_receipt(&run).unwrap();
    for changed in ["policy", "partner"] {
        let mut receipt = original.clone();
        if changed == "policy" {
            receipt.as_object_mut().unwrap().remove("plot_policy");
        } else {
            receipt["generated_artifacts"][0]["metadata"]["evidence_name"] =
                json!("../escape.json");
        }
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
        assert!(!verify_run(&run).unwrap().verified);
    }
}

#[test]
fn narrow_axes_use_distinct_full_precision_tick_labels() {
    let values = [1., 1.00000000001];
    let outputs = build(&input("a.png", "scatter", &values, &values)).unwrap();
    let doc = document(&outputs);
    let ticks = doc["derived"]["x_ticks"].as_array().unwrap();
    assert!(ticks.windows(2).all(|t| t[0]["label"] != t[1]["label"]));
    assert_eq!(doc["derived"]["x_tick_font_scale"], 1);
}

#[test]
fn adjacent_values_keep_endpoint_ticks_without_phantom_values() {
    let values = [1., f64::from_bits(1_f64.to_bits() + 1)];
    let outputs = build(&input("a.svg", "scatter", &values, &values)).unwrap();
    let doc = document(&outputs);
    let ticks = doc["derived"]["x_ticks"].as_array().unwrap();
    assert_eq!(ticks.len(), 2);
    assert_ne!(ticks[0]["label"], ticks[1]["label"]);
}
