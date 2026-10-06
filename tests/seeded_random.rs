use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::custody::{create_freeze, verify_freeze, verify_registration};
use goblinpp::evaluator::{Evaluation, Value};
use goblinpp::hashing::{hash_canonical_json, sha256_bytes};
use goblinpp::parser::parse_source;
use goblinpp::quantity::Quantity;
use goblinpp::random::{Randomness, policy, verify_evidence};
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::json;
use std::{fs, process::Command};
use tempfile::tempdir;

fn number(n: f64) -> Value {
    Value::Quantity(Quantity::scalar(n).unwrap())
}
fn draw(rng: &mut Randomness, name: &str, args: &[Value]) -> f64 {
    let Value::Quantity(q) = rng.call(name, args).unwrap() else {
        panic!("not numeric")
    };
    q.value_si
}
const PROGRAM: &str = r#"GO_PARANOID
rng_seed(42, 54)
words = []
for i in range(6) { words = append(words, rng_word(54)) }
rng_seed("18446744073709551615")
uniforms = []
integers = []
g_func uniform_draw() { return rng_uniform() }
for i in range(12) {
    uniforms = append(uniforms, uniform_draw())
    integers = append(integers, rng_integer(-9007199254740991, 9007199254740991))
}
print("words = {words}")
print("uniforms = {uniforms}")
print("integers = {integers}")
seal words
seal uniforms
seal integers
"#;

#[test]
fn published_pcg_reference_vector_and_fixed_mappings() {
    let mut rng = Randomness::default();
    rng.call("rng_seed", &[number(42.0), number(54.0)]).unwrap();
    // PCG's published C demo, not generated from this implementation.
    for expected in [
        0xa15c02b7_u32,
        0x7b47f409,
        0xba1d3330,
        0x83d2f293,
        0xbfa4784b,
        0xcbed606e,
    ] {
        assert_eq!(draw(&mut rng, "rng_word", &[number(54.0)]), expected as f64);
    }
    let mut uniform = Randomness::default();
    uniform
        .call("rng_seed", &[number(42.0), number(54.0)])
        .unwrap();
    let numerator = ((0xa15c02b7_u64 >> 5) << 26) | (0x7b47f409_u64 >> 6);
    assert_eq!(
        draw(&mut uniform, "rng_uniform", &[number(54.0)]).to_bits(),
        (numerator as f64 / 9007199254740992.0).to_bits()
    );
    let mut integer = Randomness::default();
    integer
        .call("rng_seed", &[number(42.0), number(54.0)])
        .unwrap();
    let expected = -3 + (((0xa15c02b7_u64 << 32) | 0x7b47f409) % 10) as i64;
    assert_eq!(
        draw(
            &mut integer,
            "rng_integer",
            &[number(-3.0), number(7.0), number(54.0)]
        ),
        expected as f64
    );
    verify_evidence(&serde_json::to_value(rng.evidence()).unwrap()).unwrap();
}

#[test]
fn named_streams_are_unaffected_by_other_stream_interleaving() {
    let mut a = Randomness::default();
    let mut b = Randomness::default();
    for rng in [&mut a, &mut b] {
        for id in [0.0, 1.0] {
            rng.call("rng_seed", &[number(123.0), number(id)]).unwrap();
        }
    }
    for _ in 0..50 {
        assert_eq!(
            draw(&mut a, "rng_uniform", &[]),
            draw(&mut b, "rng_uniform", &[])
        );
        for _ in 0..3 {
            draw(&mut b, "rng_word", &[number(1.0)]);
        }
    }
    assert_eq!(a.evidence().streams[0], b.evidence().streams[0]);
    assert_ne!(a.evidence().streams[1], b.evidence().streams[1]);
}

#[test]
fn exact_seed_boundaries_and_transactional_refusals() {
    let mut rng = Randomness::default();
    let initial = rng.evidence();
    assert!(rng.call("rng_uniform", &[]).is_err());
    for seed in [
        number(-1.0),
        number(0.5),
        number(9007199254740992.0),
        Value::Text("01".into()),
        Value::Text("18446744073709551616".into()),
        Value::Bool(true),
    ] {
        assert!(rng.call("rng_seed", &[seed]).is_err());
        assert_eq!(rng.evidence(), initial);
    }
    rng.call(
        "rng_seed",
        &[
            Value::Text(u64::MAX.to_string()),
            Value::Text((u64::MAX >> 1).to_string()),
        ],
    )
    .unwrap();
    let initialized = rng.evidence();
    assert_eq!(initialized.usage, "INITIALIZED_NO_DRAWS");
    assert!(
        rng.call(
            "rng_seed",
            &[number(2.0), Value::Text((u64::MAX >> 1).to_string())]
        )
        .is_err()
    );
    assert!(
        rng.call(
            "rng_seed",
            &[number(2.0), Value::Text("9223372036854775808".into())]
        )
        .is_err()
    );
    assert_eq!(rng.evidence(), initialized);
    let id = Value::Text((u64::MAX >> 1).to_string());
    for (low, high) in [(1.0, 1.0), (2.0, 1.0), (0.5, 2.0)] {
        assert!(
            rng.call("rng_integer", &[number(low), number(high), id.clone()])
                .is_err()
        );
        assert_eq!(rng.evidence(), initialized);
    }
    for _ in 0..1000 {
        let u = draw(&mut rng, "rng_uniform", std::slice::from_ref(&id));
        assert!((0.0..1.0).contains(&u));
        let n = draw(
            &mut rng,
            "rng_integer",
            &[number(-2.0), number(3.0), id.clone()],
        );
        assert!((-2.0..3.0).contains(&n) && n.fract() == 0.0);
    }
    verify_evidence(&serde_json::to_value(rng.evidence()).unwrap()).unwrap();
}

#[test]
fn refusal_runs_are_preserved_and_verify_in_both_modes() {
    let root = tempdir().unwrap();
    let source = root.path().join("refusal.gbl");
    for program in [
        "x = rng_uniform()",
        "rng_seed(42)\nrng_seed(42)",
        "rng_seed(42)\nrng_word()",
        "rng_seed(42)\nx = rng_integer(3, 3)",
        "rng_seed(42)\nx = rng_uniform(1, 2)",
        "rng_seed(1 kg)",
        "g_func rng_word() { return 1 }\nx = rng_word()",
        "g_func custom(rng_uniform) { return rng_uniform }\nx = custom(1)",
    ] {
        fs::write(&source, format!("GO_PARANOID\n{program}\n")).unwrap();
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
            assert_eq!(receipt["status"], "MACHINERY_FAIL", "{program}: {receipt}");
            assert!(verify_run(&run).unwrap().verified, "{program}");
        }
    }
}

#[test]
fn native_and_interpreted_evidence_is_exact_and_reportable() {
    let root = tempdir().unwrap();
    let source = root.path().join("random.gbl");
    fs::write(&source, PROGRAM).unwrap();
    let mut receipts = Vec::new();
    let mut runs = Vec::new();
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
        assert_eq!(receipt["status"], "PASS", "{receipt}");
        assert_eq!(receipt["rng_policy"], policy());
        assert!(verify_run(&run).unwrap().verified);
        receipts.push(receipt);
        runs.push(run);
    }
    assert_eq!(receipts[0]["rng"], receipts[1]["rng"]);
    assert_eq!(
        receipts[0]["sealed_artifacts"],
        receipts[1]["sealed_artifacts"]
    );
    assert_eq!(
        fs::read(runs[0].join("stdout.log")).unwrap(),
        fs::read(runs[1].join("stdout.log")).unwrap()
    );
    let diff = diff_runs(&runs[0], &runs[1]).unwrap();
    assert!(diff.rng_evidence_same && diff.rng_policy_same);
    if let Ok(path) = std::env::var("GOBLIN_RNG_REPORT_PATH") {
        let report = json!({"schema":"goblin.rng-platform-report.v1",
            "os": std::env::consts::OS, "arch": std::env::consts::ARCH, "goblin_version": goblinpp::VERSION,
            "source_sha256": sha256_bytes(PROGRAM.as_bytes()), "rng_policy":policy(),
            "interpreter": {"rng": receipts[0]["rng"], "sealed_artifacts": receipts[0]["sealed_artifacts"]},
            "native": {"rng": receipts[1]["rng"], "sealed_artifacts":receipts[1]["sealed_artifacts"]}});
        let path = std::path::Path::new(&path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    let parsed = parse_source(PROGRAM).unwrap();
    let native = goblinpp::compiler::compile(&parsed, root.path().join("standalone"), &[]).unwrap();
    let output = Command::new(native.binary)
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, fs::read(runs[0].join("stdout.log")).unwrap());
    let directory = fs::read_dir(root.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("goblin-native-outputs-")
        })
        .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("native-data.json")).unwrap()).unwrap();
    assert_eq!(manifest["rng"], receipts[0]["rng"]);
}

#[test]
fn replay_detects_tampering_even_after_receipt_core_is_resealed() {
    let root = tempdir().unwrap();
    let source = root.path().join("tamper.gbl");
    fs::write(&source, "rng_seed(42)\nx = rng_uniform()\nseal x\n").unwrap();
    let run = run_file(&source, &RunOptions::default()).unwrap();
    let original = read_receipt(&run).unwrap();
    for field in [
        "seed",
        "raw_words_sha256",
        "results_sha256",
        "final_state_hex",
        "operations",
    ] {
        let mut receipt = original.clone();
        receipt["rng"]["streams"][0][field] = if field == "operations" {
            json!(2)
        } else {
            json!("99")
        };
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
        assert!(!verify_run(&run).unwrap().verified, "{field}");
    }
    for remove in ["rng", "rng_policy"] {
        let mut receipt = original.clone();
        receipt.as_object_mut().unwrap().remove(remove);
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
        assert!(!verify_run(&run).unwrap().verified, "{remove}");
    }
}

#[test]
fn freeze_policy_is_enforced_and_old_non_rng_freezes_remain_supported() {
    for legacy in [false, true] {
        let root = tempdir().unwrap();
        let source = root.path().join("frozen.gbl");
        fs::write(&source, "GO_PARANOID\nx = 1\nseal x\n").unwrap();
        let (path, event) = create_freeze(&source).unwrap();
        let mut frozen: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(frozen["rng_policy"], policy());
        if legacy {
            frozen["goblin_version"] = json!("0.1.0-alpha.25");
            frozen.as_object_mut().unwrap().remove("rng_policy");
        } else {
            frozen["rng_policy"]["implementation"] = json!("changed");
        }
        frozen
            .as_object_mut()
            .unwrap()
            .remove("freeze_receipt_sha256");
        frozen["freeze_receipt_sha256"] = json!(hash_canonical_json(&frozen).unwrap());
        let bytes = serde_json::to_vec_pretty(&frozen).unwrap();
        fs::write(&path, &bytes).unwrap();
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
        let freeze = verify_freeze(&source);
        assert!(freeze.verified && verify_registration(&source, &freeze).unwrap());
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
            assert_eq!(
                receipt["status"],
                if legacy { "PASS" } else { "PROTOCOL_VIOLATION" }
            );
            assert!(verify_run(&run).unwrap().verified);
        }
        let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
            .arg("compile")
            .arg(&source)
            .arg("-o")
            .arg(root.path().join("native"))
            .output()
            .unwrap();
        assert_eq!(output.status.success(), legacy);
    }
}

#[test]
fn seed_trace_changes_are_not_hidden_by_equal_sealed_values() {
    let root = tempdir().unwrap();
    let source = root.path().join("same.gbl");
    let mut runs = Vec::new();
    for seed in [1, 2] {
        fs::write(&source, format!("rng_seed({seed})\nx = 1\nseal x\n")).unwrap();
        runs.push(run_file(&source, &RunOptions::default()).unwrap());
    }
    let diff = diff_runs(&runs[0], &runs[1]).unwrap();
    assert!(diff.rng_policy_same && !diff.rng_evidence_same);
    assert_ne!(diff.classification, "NOTATION_ONLY_CHANGE");
}

#[test]
fn empty_and_malformed_traces_and_stream_limit_are_checked() {
    let mut rng = Randomness::default();
    verify_evidence(&serde_json::to_value(rng.evidence()).unwrap()).unwrap();
    for id in 0..goblinpp::random::MAX_STREAMS {
        rng.call("rng_seed", &[number(42.0), number(id as f64)])
            .unwrap();
    }
    let snapshot = rng.evidence();
    assert!(
        rng.call("rng_seed", &[number(42.0), number(128.0)])
            .is_err()
    );
    assert_eq!(snapshot, rng.evidence());
    let mut evidence = serde_json::to_value(snapshot).unwrap();
    evidence["streams"][0]["segments"] =
        json!([{"operation":"rng_uniform","low":null,"high":null,"count":1000001}]);
    assert!(verify_evidence(&evidence).is_err());
    evidence["streams"][0]["segments"][0]["count"] = json!(1);
    evidence["streams"][0]["segments"][0]["operation"] = json!("unknown");
    assert!(verify_evidence(&evidence).is_err());
}

#[test]
fn evaluation_captures_rng_inside_functions_and_plain_mode() {
    let parsed = parse_source(PROGRAM.strip_prefix("GO_PARANOID\n").unwrap()).unwrap();
    let evaluation = Evaluation::new(".").eval_program(&parsed.program).unwrap();
    assert_eq!(evaluation.randomness.evidence().usage, "USED");
    assert_eq!(evaluation.randomness.evidence().streams[0].operations, 24);
    assert_eq!(evaluation.randomness.evidence().streams[1].operations, 6);
}

#[test]
fn rng_arity_refuses_before_argument_effects_and_native_codegen() {
    let parsed = parse_source("x = rng_uniform(rng_seed(1), rng_seed(2, 1))\n").unwrap();
    let mut evaluation = Evaluation::new(".");
    assert!(evaluation.eval_stmt(&parsed.program.statements[0]).is_err());
    assert!(evaluation.randomness.evidence().streams.is_empty());
    let root = tempdir().unwrap();
    assert!(goblinpp::compiler::compile(&parsed, root.path().join("invalid"), &[]).is_err());
}
