use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::custody::{create_freeze, create_revision, verify_freeze};
use goblinpp::hashing::{hash_canonical_json, sha256_file};
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use goblinpp::text_runtime::{SEMANTICS_POLICY, format_number, parse_number};
use std::{fs, process::Command};
use tempfile::tempdir;

fn corpus(count: usize) -> Vec<f64> {
    let mut values = vec![
        0.0,
        -0.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
        0.12345678901234567,
        1.0000000000000002,
        1e15,
        1e-4,
        f64::from_bits(1e15_f64.to_bits() - 1),
        f64::from_bits(1e-4_f64.to_bits() - 1),
        9007199254740992.0,
    ];
    // Deterministic test data generator, not a language RNG feature.
    let mut bits = 0x5eed_1234_abcd_9876_u64;
    while values.len() < count {
        bits ^= bits << 13;
        bits ^= bits >> 7;
        bits ^= bits << 17;
        let value = f64::from_bits(bits);
        if value.is_finite() {
            values.push(value);
        }
    }
    values
}

#[test]
fn finite_default_text_round_trips_bits_including_zero_subnormals_and_extremes() {
    for value in corpus(100_000) {
        let text = format_number(value);
        assert_eq!(
            text.parse::<f64>().unwrap().to_bits(),
            value.to_bits(),
            "{text}"
        );
        assert_eq!(
            parse_number(&text).unwrap().to_bits(),
            value.to_bits(),
            "{text}"
        );
        assert_eq!(goblinpp::quantity::format_number(value), text);
    }
    assert_eq!(format_number(-0.0), "-0");
    assert_eq!(format_number(100.0), "100");
    assert_eq!(format_number(1e15), "1e+15");
    assert_eq!(format_number(f64::from_bits(1)), "5e-324");
}

#[test]
fn both_engines_preserve_numeric_bits_in_default_text_exports_json_and_seals() {
    let values = corpus(32);
    // Scientific literals avoid the deliberate exact-integer parsing guard.
    let literals = values
        .iter()
        .map(|value| format!("{value:e}"))
        .collect::<Vec<_>>()
        .join(", ");
    let source_text = format!(
        r#"GO_PARANOID
values = [{literals}]
restored = []
for value in values {{
    saved = to_text(value)
    restored = append(restored, parse_number(saved))
    print(value)
    print("{{value}}")
    print(saved)
}}
write_csv("values.csv", 1, "value", {cells})
write_tsv("values.tsv", 1, "value", {cells})
write_text("values.txt", {cells})
seal values
seal restored
"#,
        cells = (0..values.len())
            .map(|i| format!("values[{i}]"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let root = tempdir().unwrap();
    let source = root.path().join("numbers.gbl");
    fs::write(&source, source_text).unwrap();
    create_freeze(&source).unwrap();
    let expected = values.iter().map(|v| format_number(*v)).collect::<Vec<_>>();
    let mut runs = Vec::new();
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
        assert_eq!(receipt["language_semantics"], SEMANTICS_POLICY);
        let stdout = fs::read_to_string(run.join("stdout.log")).unwrap();
        assert_eq!(
            stdout.lines().collect::<Vec<_>>(),
            expected
                .iter()
                .flat_map(|text| [text.as_str(); 3])
                .collect::<Vec<_>>()
        );
        for name in ["values.csv", "values.tsv", "values.txt"] {
            let text = fs::read_to_string(run.join("outputs").join(name)).unwrap();
            let lines = text
                .lines()
                .skip(usize::from(name != "values.txt"))
                .collect::<Vec<_>>();
            assert_eq!(lines, expected, "{name}");
            for (line, original) in lines.iter().zip(&values) {
                assert_eq!(parse_number(line).unwrap().to_bits(), original.to_bits());
            }
        }
        for name in ["values", "restored"] {
            let json: serde_json::Value =
                serde_json::from_slice(&fs::read(run.join(format!("{name}.json"))).unwrap())
                    .unwrap();
            for (item, original) in json["value"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .zip(&values)
            {
                assert_eq!(
                    item["value_si"].as_f64().unwrap().to_bits(),
                    original.to_bits(),
                    "{name}"
                );
            }
            assert_eq!(
                json["value"]["items"].as_array().unwrap().len(),
                values.len()
            );
        }
        assert!(verify_run(&run).unwrap().verified);
        runs.push(run);
    }
    let diff = diff_runs(&runs[0], &runs[1]).unwrap();
    assert!(diff.stdout_same && diff.sealed_artifacts_same);
    for filename in ["values.csv", "values.tsv", "values.txt"] {
        assert_eq!(
            fs::read(runs[0].join("outputs").join(filename)).unwrap(),
            fs::read(runs[1].join("outputs").join(filename)).unwrap()
        );
    }
}

#[test]
fn explicit_precision_is_unchanged_and_numeric_json_keeps_full_precision() {
    let root = tempdir().unwrap();
    let source = root.path().join("formats.gbl");
    fs::write(&source, r#"GO_PARANOID
value = 0.12345678901234567
mass = value * 1 kg
negative_zero = -0
tiny = 5e-324
largest = 1.7976931348623157e308
print("{value:.3f}; {value:.2e}; {mass:.3f}")
print(mass)
write_json("numbers.json", "value", value, "mass", mass, "zero", negative_zero, "tiny", tiny, "largest", largest)
seal value
"#).unwrap();
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
        assert_eq!(
            fs::read_to_string(run.join("stdout.log")).unwrap(),
            "0.123; 1.23e-01; 0.123 kg\n0.12345678901234566 kg\n"
        );
        let json: serde_json::Value =
            serde_json::from_slice(&fs::read(run.join("outputs/numbers.json")).unwrap()).unwrap();
        for (key, value) in [
            ("value", 0.12345678901234567),
            ("zero", -0.0),
            ("tiny", f64::from_bits(1)),
            ("largest", f64::MAX),
        ] {
            assert_eq!(
                json[key].as_f64().unwrap().to_bits(),
                value.to_bits(),
                "{key}"
            );
        }
        assert_eq!(
            json["mass"]["value_si"].as_f64().unwrap().to_bits(),
            0.12345678901234567_f64.to_bits()
        );
        assert!(verify_run(&run).unwrap().verified);
    }
}

fn hash_without(document: &serde_json::Value, key: &str) -> String {
    let mut core = document.clone();
    core.as_object_mut().unwrap().remove(key);
    hash_canonical_json(&core).unwrap()
}

#[test]
fn alpha20_freeze_remains_verifiable_but_requires_explicit_revision_for_new_formatting() {
    let root = tempdir().unwrap();
    let source = root.path().join("parent.gbl");
    fs::write(
        &source,
        "GO_PARANOID\nvalue = 0.12345678901234567\nprint(value)\n",
    )
    .unwrap();
    let (freeze, _) = create_freeze(&source).unwrap();
    let mut receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(&freeze).unwrap()).unwrap();
    receipt["goblin_version"] = "0.1.0-alpha.20".into();
    receipt["language_semantics"] = "goblin.eager-text-and-protected-values.v1".into();
    receipt["freeze_receipt_sha256"] = hash_without(&receipt, "freeze_receipt_sha256").into();
    fs::write(&freeze, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    // Coherent synthetic historical evidence; never rewrite a user's freeze.
    let ledger = root.path().join(".goblin/custody-ledger.jsonl");
    let mut event: serde_json::Value =
        serde_json::from_str(fs::read_to_string(&ledger).unwrap().trim()).unwrap();
    event["payload"]["freeze_receipt_file_sha256"] = sha256_file(&freeze).unwrap().into();
    event["payload"]["freeze_receipt_core_sha256"] = receipt["freeze_receipt_sha256"].clone();
    event["event_sha256"] = hash_without(&event, "event_sha256").into();
    fs::write(
        &ledger,
        format!("{}\n", serde_json::to_string(&event).unwrap()),
    )
    .unwrap();
    let head_path = goblinpp::ledger::head_path(root.path());
    let mut head: serde_json::Value =
        serde_json::from_slice(&fs::read(&head_path).unwrap()).unwrap();
    head["head_event_sha256"] = event["event_sha256"].clone();
    head["ledger_file_sha256"] = sha256_file(&ledger).unwrap().into();
    fs::write(&head_path, serde_json::to_vec_pretty(&head).unwrap()).unwrap();
    assert!(verify_freeze(&source).verified);
    let command = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .arg("compile")
        .arg(&source)
        .arg("-o")
        .arg(root.path().join("refused"))
        .output()
        .unwrap();
    assert!(!command.status.success());
    assert!(
        String::from_utf8_lossy(&command.stderr)
            .contains("LANGUAGE_SEMANTICS_CHANGED_AFTER_FREEZE")
    );
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
        assert_eq!(receipt["status"], "PROTOCOL_VIOLATION");
        assert_eq!(
            receipt["freeze"]["classification"],
            "LANGUAGE_SEMANTICS_CHANGED_AFTER_FREEZE"
        );
        assert!(verify_run(&run).unwrap().verified);
    }
    let child = root.path().join("child.gbl");
    create_revision(&source, &child, "adopt lossless numeric text").unwrap();
    create_freeze(&child).unwrap();
    let run = run_file(&child, &RunOptions::default()).unwrap();
    assert_eq!(read_receipt(&run).unwrap()["status"], "PASS");
    assert!(verify_run(&run).unwrap().verified);
}

#[test]
fn nonfinite_and_nonzero_underflow_numeric_text_are_still_refused() {
    for text in ["NaN", "inf", "-inf", "1e309", "1e-9999", "9007199254740993"] {
        assert!(parse_number(text).is_err(), "{text}");
    }
}
