use goblinpp::{
    evaluator::Evaluation,
    interaction::InputPolicy,
    parser::{parse_source, validate_execution},
    runtime::{RunOptions, run_file},
};
use std::{
    fs,
    io::Write,
    process::{Command, Output, Stdio},
};
use tempfile::tempdir;
const KEY: &str = "public code fixture passphrase";
fn run(dir: &std::path::Path, args: &[&str], data: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(args)
        .current_dir(dir)
        .env("GOBLIN_SECRET_PROMPT_PROTOCOL", "hex-v1")
        .env(
            "GOBLIN_NATIVE_RESULT_PATH",
            dir.join("must-not-write-native-result"),
        )
        .env(
            "GOBLIN_NATIVE_DATA_PATH",
            dir.join("must-not-write-native-data"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(data).ok();
    child.wait_with_output().unwrap()
}
fn no_evidence(dir: &std::path::Path) {
    assert!(!dir.join("runs").exists());
    assert!(!dir.join(".goblin").exists());
    assert!(!dir.join("native-data.json").exists());
}
#[test]
fn language_roundtrip_runtime_secret_and_no_plaintext_evidence() {
    let d = tempdir().unwrap();
    fs::write(d.path().join("test.gbl"),"GO_PRIVATE\nkey = input_secret(\"Passphrase: \")\nencrypt_text(\"secret π {{literal}}\", key, \"message.age\")\ndecrypt_file(\"message.age\", key, \"restored.txt\")\nrestored = decrypt_text(\"message.age\", key)\nprint(restored)\nprint(read_text(\"restored.txt\"))\ndisplay_file(\"restored.txt\")\n").unwrap();
    let p = run(
        d.path(),
        &["run", "test.gbl"],
        format!("{KEY}\n").as_bytes(),
    );
    assert!(p.status.success(), "{}", String::from_utf8_lossy(&p.stderr));
    assert!(!String::from_utf8_lossy(&p.stdout).contains(KEY));
    assert!(String::from_utf8_lossy(&p.stdout).contains("secret π {literal}"));
    assert_eq!(
        fs::read_to_string(d.path().join("restored.txt")).unwrap(),
        "secret π {literal}"
    );
    no_evidence(d.path());
    let files = fs::read_dir(d.path())
        .unwrap()
        .map(|p| p.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 3);
}
#[test]
fn check_is_static_no_input_no_files_and_freeze_refused() {
    let d = tempdir().unwrap();
    fs::write(
        d.path().join("test.gbl"),
        "GO_PRIVATE\nkey = input_secret()\nencrypt_text(\"public\",key,\"must-not-exist.age\")\n",
    )
    .unwrap();
    let p = run(d.path(), &["check", "test.gbl"], b"");
    assert!(p.status.success());
    assert!(!d.path().join("must-not-exist.age").exists());
    assert!(!run(d.path(), &["freeze", "test.gbl"], b"").status.success());
    no_evidence(d.path());
}
#[test]
fn modes_evidence_calls_discarded_values_and_redefinitions_refuse_before_io() {
    for text in [
        "GO_PRIVATE\nGO_PARANOID\nprint(1)",
        "COMPLETELY_MAD\nGO_PRIVATE\nprint(1)",
        "GO_PRIVATE\nx=1\nseal x",
        "GO_PRIVATE\nGO_NUMERIC_AUDIT \"x.json\"\nprint(1)",
        "GO_PRIVATE\nwrite_text(\"x.txt\",\"p\")",
        "GO_PRIVATE\nread_text(\"x.txt\")",
        "key=input_secret()",
        "GO_PRIVATE\ng_func encrypt_file(x) { return x }",
        "GO_PRIVATE\nif true { GO_PRIVATE }",
        "x=1\nGO_PRIVATE",
    ] {
        let result = parse_source(text).and_then(|p| validate_execution(&p.program));
        assert!(result.is_err(), "{text}");
    }
}
#[test]
fn direct_audited_api_refuses_private_before_source_copy() {
    let d = tempdir().unwrap();
    let source = d.path().join("test.gbl");
    fs::write(&source, "GO_PRIVATE\nprint(\"private\")").unwrap();
    assert!(run_file(&source, &RunOptions::default()).is_err());
    no_evidence(d.path());
    fs::write(
        &source,
        "encrypt_text(\"must-not-copy\",\"public passphrase\",\"x.age\")",
    )
    .unwrap();
    assert!(run_file(&source, &RunOptions::default()).is_err());
    no_evidence(d.path());
}
#[test]
fn secrets_not_recorded_as_interaction_events_and_call_budget_applies() {
    let d = tempdir().unwrap();
    let mut e = Evaluation::new(d.path());
    e.configure_interaction(vec![], InputPolicy::Provided(vec![KEY.into()]))
        .unwrap();
    let p = parse_source("GO_PRIVATE\nkey=input_secret()\nx=len(key)\n").unwrap();
    let e = e.eval_program(&p.program).unwrap();
    assert!(e.interaction.input_events.is_empty());
    assert!(!e.interaction.evidence_needed());
    assert!(e.private);
    let mut interaction = goblinpp::interaction::Interaction::default();
    interaction
        .configure(
            vec![],
            InputPolicy::Provided(vec![
                KEY.into();
                goblinpp::interaction::MAX_INPUT_EVENTS + 1
            ]),
        )
        .unwrap();
    for _ in 0..goblinpp::interaction::MAX_INPUT_EVENTS {
        assert!(interaction.secret("Public prompt: ").is_ok());
    }
    assert!(interaction.secret("Public prompt: ").is_err());
    assert!(interaction.input_events.is_empty());
}
#[test]
fn redacted_failures_and_prior_completed_operations_are_explicit() {
    let d = tempdir().unwrap();
    fs::write(d.path().join("test.gbl"),format!("GO_PRIVATE\nencrypt_text(\"public fixture\",\"{KEY}\",\"one.age\")\ndecrypt_file(\"one.age\",\"wrong public passphrase\",\"must-not-exist.txt\")")).unwrap();
    let p = run(d.path(), &["test.gbl"], b"");
    assert!(!p.status.success());
    assert!(d.path().join("one.age").exists());
    assert!(!d.path().join("must-not-exist.txt").exists());
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&p.stdout),
        String::from_utf8_lossy(&p.stderr)
    );
    assert!(!log.contains(KEY));
    assert!(!log.contains("wrong public passphrase"));
    no_evidence(d.path());
}
#[test]
fn module_functions_binary_file_and_no_overwrite() {
    let d = tempdir().unwrap();
    let data = (0..10000).map(|n| (n % 251) as u8).collect::<Vec<_>>();
    fs::write(d.path().join("source.bin"), &data).unwrap();
    fs::write(
        d.path().join("library.gbl"),
        "g_func protect(source, key, target) { return encrypt_file(source, key, target) }",
    )
    .unwrap();
    fs::write(d.path().join("test.gbl"),format!("GO_PRIVATE\nimport \"library.gbl\"\nok=protect(\"source.bin\",\"{KEY}\",\"binary.age\")\ndecrypt_file(\"binary.age\",\"{KEY}\",\"copy.bin\")\n")).unwrap();
    let p = run(d.path(), &["test.gbl"], b"");
    assert!(p.status.success(), "{}", String::from_utf8_lossy(&p.stderr));
    assert_eq!(fs::read(d.path().join("copy.bin")).unwrap(), data);
    assert!(!run(d.path(), &["test.gbl"], b"").status.success());
    no_evidence(d.path());
}
#[test]
fn frozen_source_cannot_become_private_to_bypass_its_freeze() {
    let d = tempdir().unwrap();
    let path = d.path().join("test.gbl");
    fs::write(&path, "x=1\nseal x").unwrap();
    goblinpp::custody::create_freeze(&path).unwrap();
    let original = fs::read(d.path().join("test.gbl.freeze.json")).unwrap();
    fs::write(&path, "GO_PRIVATE\nprint(\"must not execute\")").unwrap();
    let p = run(d.path(), &["test.gbl"], b"");
    assert!(!p.status.success());
    assert!(!String::from_utf8_lossy(&p.stdout).contains("must not execute"));
    assert_eq!(
        fs::read(d.path().join("test.gbl.freeze.json")).unwrap(),
        original
    );
    assert!(!d.path().join("runs").exists());
}
#[test]
fn compiled_private_script_roundtrip_with_runtime_secret_and_no_manifests() {
    let d = tempdir().unwrap();
    fs::write(d.path().join("test.gbl"),"GO_PRIVATE\nkey=input_secret(\"Passphrase: \")\nencrypt_text(\"compiled public fixture\",key,\"native.age\")\ntext=decrypt_text(\"native.age\",key)\nprint(text)\n").unwrap();
    let p = run(
        d.path(),
        &["run", "test.gbl", "--compile"],
        format!("{KEY}\n").as_bytes(),
    );
    assert!(p.status.success(), "{}", String::from_utf8_lossy(&p.stderr));
    assert!(String::from_utf8_lossy(&p.stdout).contains("compiled public fixture"));
    assert!(!String::from_utf8_lossy(&p.stdout).contains(KEY));
    assert!(d.path().join("native.age").exists());
    no_evidence(d.path());
    assert_eq!(fs::read_dir(d.path()).unwrap().count(), 2);
}
