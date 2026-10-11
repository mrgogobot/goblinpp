use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::custody::{create_freeze, freeze_path, verify_freeze};
use goblinpp::hashing::{hash_canonical_json, sha256_bytes};
use goblinpp::output::{GeneratedOutput, artifact_descriptor};
use goblinpp::plots::{Axis, Input, build, verify_embedded_provenance, verify_pair};
use goblinpp::quantity::DIMENSIONLESS;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::{Value, json};
use std::fs;
use tempfile::tempdir;

fn input(ext: &str) -> Input {
    let axis = |label: &str| Axis {
        label: label.into(),
        dimension: DIMENSIONLESS,
        values_bits: [0_f64, 1., 2.]
            .map(|v| format!("{:016x}", v.to_bits()))
            .to_vec(),
    };
    Input {
        figure: format!("a.{ext}"),
        kind: "line".into(),
        title: "goblin-provenance".into(),
        x: axis("time"),
        y: Some(axis("distance")),
        bins: None,
    }
}
fn doc(outputs: &[GeneratedOutput]) -> Value {
    serde_json::from_slice(&outputs[1].bytes).unwrap()
}
fn verify(outputs: &[GeneratedOutput], image: &[u8]) -> goblinpp::error::Result<()> {
    verify_pair(
        image,
        &outputs[1].bytes,
        &artifact_descriptor(&outputs[0]),
        &artifact_descriptor(&outputs[1]),
    )
}
fn png_metadata_end(bytes: &[u8]) -> usize {
    assert_eq!(&bytes[37..41], b"iTXt");
    33 + u32::from_be_bytes(bytes[33..37].try_into().unwrap()) as usize + 12
}
fn strip(bytes: &[u8], png: bool) -> Vec<u8> {
    if png {
        [&bytes[..33], &bytes[png_metadata_end(bytes)..]].concat()
    } else {
        let text = std::str::from_utf8(bytes).unwrap();
        let start = text.find("<metadata ").unwrap();
        let end = text.find("</metadata>\n").unwrap() + "</metadata>\n".len();
        format!("{}{}", &text[..start], &text[end..]).into_bytes()
    }
}

#[test]
fn canonical_core_is_independent_of_image_sidecar_and_receipt() {
    for ext in ["png", "svg"] {
        let outputs = build(&input(ext)).unwrap();
        let document = doc(&outputs);
        let core = json!({"schema":document["schema"],"input":document["input"],"derived":document["derived"]});
        let digest = hash_canonical_json(&core).unwrap();
        assert_eq!(document["manifest_sha256"], digest);
        assert_eq!(document["provenance_schema"], "goblin.plot-provenance.v1");
        assert_ne!(digest, sha256_bytes(&outputs[0].bytes));
        assert_ne!(digest, sha256_bytes(&outputs[1].bytes));
        assert!(outputs[0].bytes.windows(64).any(|b| b == digest.as_bytes()));
        assert_eq!(document["figure_sha256"], sha256_bytes(&outputs[0].bytes));
        assert_eq!(
            outputs[0].metadata["evidence_sha256"],
            sha256_bytes(&outputs[1].bytes)
        );
        for output in &outputs {
            assert_eq!(output.metadata["manifest_sha256"], digest);
        }
        verify(&outputs, &outputs[0].bytes).unwrap();
    }
}

#[test]
fn identical_inputs_produce_identical_bytes_and_metadata() {
    for ext in ["png", "svg"] {
        let a = build(&input(ext)).unwrap();
        let b = build(&input(ext)).unwrap();
        for (a, b) in a.iter().zip(b.iter()) {
            assert_eq!(a.bytes, b.bytes);
            assert_eq!(artifact_descriptor(a), artifact_descriptor(b));
        }
    }
}

#[test]
fn changed_data_or_settings_changes_embedded_manifest_link() {
    let original = input("svg");
    let original_hash = doc(&build(&original).unwrap())["manifest_sha256"].clone();
    for mutation in ["data", "label", "title", "order", "filename"] {
        let mut altered = original.clone();
        match mutation {
            "data" => altered.x.values_bits[1] = format!("{:016x}", 1.5_f64.to_bits()),
            "label" => altered.x.label = "other".into(),
            "title" => altered.title = "Other".into(),
            "order" => altered.x.values_bits.reverse(),
            _ => altered.figure = "other.svg".into(),
        }
        assert_ne!(
            doc(&build(&altered).unwrap())["manifest_sha256"],
            original_hash
        );
    }
}

#[test]
fn stripped_metadata_is_missing_not_verified() {
    for ext in ["png", "svg"] {
        let outputs = build(&input(ext)).unwrap();
        let bare = strip(&outputs[0].bytes, ext == "png");
        let error = verify_embedded_provenance(&bare, &doc(&outputs)).unwrap_err();
        assert!(error.to_string().contains("PROVENANCE_MISSING"), "{error}");
        assert!(verify(&outputs, &bare).is_err());
    }
}

#[test]
fn mismatched_embedded_digest_even_with_valid_png_crc_refuses() {
    for ext in ["png", "svg"] {
        let outputs = build(&input(ext)).unwrap();
        let digest = doc(&outputs)["manifest_sha256"]
            .as_str()
            .unwrap()
            .as_bytes()
            .to_vec();
        let mut image = outputs[0].bytes.clone();
        let position = image
            .windows(digest.len())
            .position(|b| b == digest)
            .unwrap();
        image[position] = if image[position] == b'0' { b'1' } else { b'0' };
        if ext == "png" {
            let end = png_metadata_end(&image);
            let mut crc = u32::MAX;
            for b in &image[37..end - 4] {
                crc ^= *b as u32;
                for _ in 0..8 {
                    crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
                }
            }
            image[end - 4..end].copy_from_slice(&(!crc).to_be_bytes());
        }
        assert!(
            verify_embedded_provenance(&image, &doc(&outputs))
                .unwrap_err()
                .to_string()
                .contains("does not match")
        );
    }
}

#[test]
fn duplicate_metadata_refuses_without_choosing_one() {
    for ext in ["png", "svg"] {
        let outputs = build(&input(ext)).unwrap();
        let image = &outputs[0].bytes;
        let duplicate = if ext == "png" {
            let end = png_metadata_end(image);
            [&image[..end], &image[33..end], &image[end..]].concat()
        } else {
            let text = std::str::from_utf8(image).unwrap();
            let start = text.find("<metadata ").unwrap();
            let end = text.find("</metadata>\n").unwrap() + "</metadata>\n".len();
            format!("{}{}{}", &text[..end], &text[start..end], &text[end..]).into_bytes()
        };
        assert!(
            verify_embedded_provenance(&duplicate, &doc(&outputs))
                .unwrap_err()
                .to_string()
                .contains("Duplicate")
        );
    }
}

#[test]
fn svg_provenance_escapes_hostile_filenames_without_entity_expansion() {
    let mut data = input("svg");
    data.figure = "<&quot;\"'>&.svg".into();
    data.title = "科学 & <no script>".into();
    let output = build(&data).unwrap();
    let text = String::from_utf8(output[0].bytes.clone()).unwrap();
    assert!(!text.contains("<no script>"));
    assert!(!text.contains("<!DOCTYPE"));
    verify(&output, &output[0].bytes).unwrap();
}

#[test]
fn sidecar_or_receipt_manifest_digest_changes_refuse() {
    let outputs = build(&input("svg")).unwrap();
    let mut document = doc(&outputs);
    document["manifest_sha256"] = json!("0".repeat(64));
    assert!(verify_embedded_provenance(&outputs[0].bytes, &document).is_err());
    let mut descriptor = artifact_descriptor(&outputs[0]);
    descriptor["metadata"]["manifest_sha256"] = json!("0".repeat(64));
    assert!(
        verify_pair(
            &outputs[0].bytes,
            &outputs[1].bytes,
            &descriptor,
            &artifact_descriptor(&outputs[1])
        )
        .is_err()
    );
}

const SOURCE: &str = "GO_PARANOID\nplot_line(\"a.png\", [0,1,2], [2,3,4], \"Motion\", \"time\", \"distance\")\nplot_line(\"a.svg\", [0,1,2], [2,3,4], \"Motion\", \"time\", \"distance\")\n";
#[test]
fn separate_runs_keep_exact_figure_bytes_but_distinct_receipts() {
    let directory = tempdir().unwrap();
    let source = directory.path().join("a.gbl");
    fs::write(&source, SOURCE).unwrap();
    let left = run_file(&source, &RunOptions::default()).unwrap();
    let right = run_file(&source, &RunOptions::default()).unwrap();
    let a = read_receipt(&left).unwrap();
    let b = read_receipt(&right).unwrap();
    assert_ne!(a["receipt_core_sha256"], b["receipt_core_sha256"]);
    assert_eq!(a["generated_artifacts"], b["generated_artifacts"]);
    for root in [&left, &right] {
        let checks = verify_run(root).unwrap();
        assert!(checks.verified);
        assert!(
            checks
                .checks
                .iter()
                .any(|c| c.check == "PLOT_PROVENANCE:a.png" && c.pass)
        );
        assert!(
            checks
                .checks
                .iter()
                .any(|c| c.check == "PLOT_PROVENANCE:a.svg" && c.pass)
        );
    }
    assert!(diff_runs(left, right).unwrap().generated_artifacts_same);
}

#[test]
fn old_freeze_remains_inspectable_but_new_plot_policy_requires_revision() {
    let directory = tempdir().unwrap();
    let source = directory.path().join("a.gbl");
    fs::write(&source, SOURCE).unwrap();
    let (_, event) = create_freeze(&source).unwrap();
    let mut frozen = verify_freeze(&source).receipt.unwrap();
    frozen["goblin_version"] = json!("0.1.0-alpha.30");
    frozen["plot_policy"] = json!(goblinpp::plots::LEGACY_POLICY);
    frozen
        .as_object_mut()
        .unwrap()
        .remove("freeze_receipt_sha256");
    frozen["freeze_receipt_sha256"] = json!(hash_canonical_json(&frozen).unwrap());
    let bytes = serde_json::to_vec_pretty(&frozen).unwrap();
    fs::write(freeze_path(&source), &bytes).unwrap();
    // Register the synthetic historical fixture; do not test a mere ledger
    // mismatch as if it proved policy migration enforcement.
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
    assert!(goblinpp::plots::frozen_policy_matches(&frozen));
    assert!(verify_freeze(&source).verified);
    let result = run_file(&source, &RunOptions::default()).unwrap();
    let receipt = read_receipt(&result).unwrap();
    assert_eq!(receipt["status"], "PROTOCOL_VIOLATION");
    assert!(
        receipt["generated_artifacts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        fs::read_to_string(result.join("stderr.log"))
            .unwrap()
            .contains("current plot provenance policy")
    );
    assert!(verify_run(result).unwrap().verified);
    let compiled = std::process::Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .arg("compile")
        .arg(&source)
        .arg("-o")
        .arg(directory.path().join("native"))
        .output()
        .unwrap();
    assert!(!compiled.status.success());
    assert!(String::from_utf8_lossy(&compiled.stderr).contains("Plotting is not pinned"));
}

#[test]
fn receipt_policy_downgrade_without_pair_downgrade_fails() {
    let directory = tempdir().unwrap();
    let source = directory.path().join("a.gbl");
    fs::write(&source, SOURCE).unwrap();
    let root = run_file(&source, &RunOptions::default()).unwrap();
    let mut receipt = read_receipt(&root).unwrap();
    receipt["plot_policy"] = json!(goblinpp::plots::LEGACY_POLICY);
    receipt
        .as_object_mut()
        .unwrap()
        .remove("receipt_core_sha256");
    receipt["receipt_core_sha256"] = json!(hash_canonical_json(&receipt).unwrap());
    fs::write(
        root.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    let verified = verify_run(&root).unwrap();
    assert!(!verified.verified);
    assert!(
        verified
            .checks
            .iter()
            .any(|c| c.check == "PLOT_POLICY_MATCH:a.svg" && !c.pass)
    );
}

#[test]
fn stripped_metadata_cannot_be_rescued_by_rehashing_image_sidecar_and_receipt() {
    for ext in ["png", "svg"] {
        let directory = tempdir().unwrap();
        let source = directory.path().join("a.gbl");
        fs::write(&source, SOURCE).unwrap();
        let root = run_file(&source, &RunOptions::default()).unwrap();
        let name = format!("a.{ext}");
        let image_path = root.join("outputs").join(&name);
        let image = strip(&fs::read(&image_path).unwrap(), ext == "png");
        fs::write(&image_path, &image).unwrap();
        let sidecar_path = root.join("outputs").join(format!("{name}.plot.json"));
        let mut sidecar: Value = serde_json::from_slice(&fs::read(&sidecar_path).unwrap()).unwrap();
        sidecar["figure_sha256"] = json!(sha256_bytes(&image));
        let sidecar_bytes = serde_json::to_vec_pretty(&sidecar).unwrap();
        fs::write(sidecar_path, &sidecar_bytes).unwrap();
        let mut receipt = read_receipt(&root).unwrap();
        for descriptor in receipt["generated_artifacts"].as_array_mut().unwrap() {
            if descriptor["name"] == name {
                descriptor["sha256"] = json!(sha256_bytes(&image));
                descriptor["byte_count"] = json!(image.len());
                descriptor["metadata"]["evidence_sha256"] = json!(sha256_bytes(&sidecar_bytes));
            } else if descriptor["name"] == format!("{name}.plot.json") {
                descriptor["sha256"] = json!(sha256_bytes(&sidecar_bytes));
                descriptor["byte_count"] = json!(sidecar_bytes.len());
                descriptor["metadata"]["figure_sha256"] = json!(sha256_bytes(&image));
            }
        }
        receipt
            .as_object_mut()
            .unwrap()
            .remove("receipt_core_sha256");
        receipt["receipt_core_sha256"] = json!(hash_canonical_json(&receipt).unwrap());
        fs::write(
            root.join("receipt.json"),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
        let verified = verify_run(root).unwrap();
        assert!(!verified.verified);
        assert!(
            verified
                .checks
                .iter()
                .any(|c| c.check == format!("PLOT_PROVENANCE:{name}")
                    && !c.pass
                    && c.detail.as_ref().unwrap().contains("PROVENANCE_MISSING"))
        );
        assert!(
            verified
                .checks
                .iter()
                .any(|c| c.check == format!("GENERATED_ARTIFACT:{name}") && c.pass)
        );
    }
}
