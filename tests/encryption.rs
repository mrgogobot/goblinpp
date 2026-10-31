use age::secrecy::SecretString;
use goblinpp::{
    audit::verify_run,
    evaluator::Evaluation,
    parser::parse_source,
    runtime::{RunOptions, run_file},
    text_runtime,
};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};
use tempfile::tempdir;

const PASSWORD: &str = "public test passphrase only";
fn cli(args: &[&str], stdin: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin).ok();
    child.wait_with_output().unwrap()
}
fn keyline() -> Vec<u8> {
    format!("{PASSWORD}\n").into_bytes()
}
fn fixture(plain: &[u8]) -> Vec<u8> {
    // Cheap test-only upstream age fixture. Production encryption always uses logN=18.
    let mut recipient = age::scrypt::Recipient::new(SecretString::from(PASSWORD.to_owned()));
    recipient.set_work_factor(2);
    let enc = age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
        .unwrap();
    let mut out = Vec::new();
    let mut writer = enc.wrap_output(&mut out).unwrap();
    writer.write_all(plain).unwrap();
    writer.finish().unwrap();
    out
}
fn dec(path: &Path, output: &Path) -> Output {
    cli(
        &[
            "decrypt",
            "--key-stdin",
            "--input",
            path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ],
        &keyline(),
    )
}
#[test]
fn standard_file_roundtrip_fresh_randomness_and_no_run_evidence() {
    let root = tempdir().unwrap();
    let plain = root.path().join("input.bin");
    let sealed = root.path().join("encrypted.age");
    let other = root.path().join("second.age");
    let restored = root.path().join("restored.bin");
    let bytes: Vec<u8> = (0..140_000).map(|n| (n % 251) as u8).collect();
    fs::write(&plain, &bytes).unwrap();
    for target in [&sealed, &other] {
        let result = cli(
            &[
                "-enc",
                "-of",
                plain.to_str().unwrap(),
                "-nf",
                target.to_str().unwrap(),
                "--key-stdin",
            ],
            &keyline(),
        );
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!String::from_utf8_lossy(&result.stdout).contains(PASSWORD));
    }
    let first = fs::read(&sealed).unwrap();
    assert!(first.starts_with(b"age-encryption.org/v1\n"));
    assert!(String::from_utf8_lossy(&first[..150]).contains(" 18\n"));
    assert_ne!(first, fs::read(other).unwrap());
    assert!(dec(&sealed, &restored).status.success());
    assert_eq!(fs::read(&restored).unwrap(), bytes);
    assert_eq!(fs::read(&plain).unwrap(), bytes);
    assert!(!root.path().join("runs").exists());
    assert!(!root.path().join(".goblin").exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(restored).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
#[test]
fn wrong_keys_damage_truncation_and_trailing_bytes_release_nothing() {
    let root = tempdir().unwrap();
    let input = root.path().join("input.age");
    let output = root.path().join("output.txt");
    let original = fixture(&vec![b'x'; 140_000]);
    let mut modified = original.clone();
    *modified.last_mut().unwrap() ^= 1;
    let mut appended = original.clone();
    appended.push(0);
    let huge_header = [
        b"age-encryption.org/v1\n-> scrypt ".as_slice(),
        &vec![b'a'; 65_537],
    ]
    .concat();
    let mut header = original.clone();
    header[0] ^= 1;
    for bytes in [
        &original[..0],
        &original[..20],
        &original[..original.len() - 1],
        &original[..original.len() - 30_000],
        &modified,
        &appended,
        &header,
        &huge_header,
    ] {
        fs::write(&input, bytes).unwrap();
        let result = dec(&input, &output);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert!(!output.exists());
        let result = cli(
            &[
                "decrypt",
                "--show",
                "--key-stdin",
                "--input",
                input.to_str().unwrap(),
            ],
            &keyline(),
        );
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert_eq!(
            fs::read_dir(root.path()).unwrap().count(),
            1,
            "private staging must clean up"
        );
    }
    fs::write(&input, original).unwrap();
    let result = cli(
        &[
            "decrypt",
            "--key-stdin",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ],
        b"wrong password\n",
    );
    assert!(!result.status.success());
    assert!(!output.exists());
    assert!(result.stdout.is_empty());
}
#[test]
fn existing_destinations_and_aliasing_are_refused_before_secrets() {
    let root = tempdir().unwrap();
    let input = root.path().join("i");
    let output = root.path().join("o");
    fs::write(&input, fixture(b"test")).unwrap();
    fs::write(&output, b"keep").unwrap();
    assert!(!dec(&input, &output).status.success());
    assert_eq!(fs::read(output).unwrap(), b"keep");
    let original = fs::read(&input).unwrap();
    assert!(!dec(&input, &input).status.success());
    assert_eq!(fs::read(input).unwrap(), original);
}
#[test]
fn new_text_pipe_and_screen_preview_do_not_interpolate_or_emit_controls() {
    let root = tempdir().unwrap();
    let input = root.path().join("unicode.age");
    let output = root.path().join("text");
    let result = cli(
        &[
            "encrypt",
            "--key-stdin",
            "--text-stdin",
            "--output",
            input.to_str().unwrap(),
        ],
        format!("{PASSWORD}\nhello π {{secret}}\n").as_bytes(),
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(dec(&input, &output).status.success());
    assert_eq!(fs::read(output).unwrap(), "hello π {secret}\n".as_bytes());
    fs::write(&input, fixture("hello\u{1b}[31m\u{202e}π\n".as_bytes())).unwrap();
    let result = cli(
        &[
            "-dec",
            "-of",
            input.to_str().unwrap(),
            "--show",
            "--key-stdin",
        ],
        &keyline(),
    );
    assert!(result.status.success());
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "hello\\u{1b}[31m\\u{202e}π\n"
    );
}
#[test]
fn empty_files_binary_previews_and_size_limits_are_explicit() {
    let root = tempdir().unwrap();
    let input = root.path().join("i");
    let output = root.path().join("o");
    fs::write(&input, fixture(b"")).unwrap();
    assert!(dec(&input, &output).status.success());
    assert_eq!(fs::read(output).unwrap(), b"");
    fs::write(&input, fixture(&[0xff, 0xfe])).unwrap();
    let result = cli(
        &[
            "decrypt",
            "--show",
            "--key-stdin",
            "--input",
            input.to_str().unwrap(),
        ],
        &keyline(),
    );
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    fs::write(&input, vec![b'a'; 1_048_577]).unwrap();
    assert!(
        !cli(&["read-file", input.to_str().unwrap()], b"")
            .status
            .success()
    );
    fs::write(&input, "你好\n").unwrap();
    assert_eq!(
        cli(&["-of", input.to_str().unwrap()], b"").stdout,
        "你好\n".as_bytes()
    );
    fs::OpenOptions::new()
        .write(true)
        .open(&input)
        .unwrap()
        .set_len(goblinpp::crypto_cli::MAX_CIPHERTEXT + 1)
        .unwrap();
    assert!(!dec(&input, &root.path().join("new")).status.success());
}
#[cfg(unix)]
#[test]
fn symlink_inputs_outputs_and_parents_are_refused() {
    use std::os::unix::fs::symlink;
    let root = tempdir().unwrap();
    let file = root.path().join("real");
    let link = root.path().join("link");
    fs::write(&file, fixture(b"test")).unwrap();
    symlink(&file, &link).unwrap();
    assert!(!dec(&link, &root.path().join("out")).status.success());
    assert!(!dec(&file, &link).status.success());
    assert!(
        !cli(&["read-file", link.to_str().unwrap()], b"")
            .status
            .success()
    );
    let parent = root.path().join("parent");
    symlink(root.path(), &parent).unwrap();
    assert!(!dec(&file, &parent.join("out")).status.success());
}
#[test]
fn malicious_kdf_cost_is_bounded_and_parse_errors_redact_values() {
    let root = tempdir().unwrap();
    let input = root.path().join("i");
    let mut bytes = fixture(b"test");
    let pos = bytes.windows(3).position(|s| s == b" 2\n").unwrap();
    bytes.splice(pos..pos + 3, b" 31\n".iter().copied());
    fs::write(&input, bytes).unwrap();
    assert!(!dec(&input, &root.path().join("out")).status.success());
    for args in [
        vec!["-enc", "-key", PASSWORD, "--unknown-secret-option"],
        vec![
            "encrypt",
            "--key",
            PASSWORD,
            "--input",
            "PRIVATE-FILENAME",
            "--output",
        ],
    ] {
        let result = cli(&args, b"");
        assert!(!result.status.success());
        let out = String::from_utf8_lossy(&result.stderr);
        assert!(!out.contains(PASSWORD));
        assert!(!out.contains("PRIVATE-FILENAME"));
    }
    assert!(
        !cli(
            &[
                "encrypt",
                "--key-stdin",
                "--text-stdin",
                "--output",
                root.path().join("bad").to_str().unwrap()
            ],
            b"short\nx"
        )
        .status
        .success()
    );
}
#[test]
fn base64_rfc_vectors_unicode_and_canonical_refusals() {
    for (plain, encoded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
        ("π", "z4A="),
    ] {
        assert_eq!(text_runtime::base64_text(plain, true).unwrap(), encoded);
        assert_eq!(text_runtime::base64_text(encoded, false).unwrap(), plain);
    }
    for bad in [
        "Zg", "Zg=", "Zg===", "Zh==", "Zm9=", "Zg==AAAA", "Zg==\n", "_w==", "/w==", "====",
    ] {
        assert!(text_runtime::base64_text(bad, false).is_err(), "{bad}");
    }
    assert!(text_runtime::base64_text(&"a".repeat(786_433), true).is_err());
    let source = "a = str_b64(\"hello π\")\nb = str_from_b64(a)\ncopied = str_to_b64(b)\n";
    let e = Evaluation::new(".")
        .eval_program(&parse_source(source).unwrap().program)
        .unwrap();
    assert_eq!(e.env["b"].render(), "hello π");
    assert_eq!(e.env["a"], e.env["copied"]);
    for bad in [
        "x = str_b64(1)\n",
        "x = str_from_b64()\n",
        "x = str_b64(\"a\",\"b\")\n",
    ] {
        assert!(
            Evaluation::new(".")
                .eval_program(&parse_source(bad).unwrap().program)
                .is_err()
        );
    }
}
#[test]
fn base64_native_interpreter_and_receipt_parity() {
    let root = tempdir().unwrap();
    let source = root.path().join("encoding.gbl");
    fs::write(&source,"GO_PARANOID\nencoded = str_b64(\"hello π\")\ndecoded = str_from_b64(encoded)\nprint(\"{encoded} / {decoded}\")\nseal encoded\nseal decoded\n").unwrap();
    for compile in [false, true] {
        let result = run_file(
            &source,
            &RunOptions {
                compile,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            goblinpp::runtime::read_receipt(&result).unwrap()["status"],
            "PASS"
        );
        assert!(verify_run(&result).unwrap().verified);
    }
}

#[test]
fn upstream_go_age_fixture_and_shortcut_arguments() {
    let root = tempdir().unwrap();
    let input = root.path().join("go.age");
    let output = root.path().join("restored.txt");
    fs::write(
        &input,
        include_bytes!("fixtures/encryption/go-age-v1.3.2.age"),
    )
    .unwrap();
    assert!(dec(&input, &output).status.success());
    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "Independent age fixture — π\n"
    );
    let mut args = [
        "goblin++",
        "-enc",
        "-key",
        "-nf",
        "-nf",
        "output.age",
        "text",
    ]
    .map(str::to_owned);
    goblinpp::crypto_cli::normalize_shortcuts(&mut args);
    assert_eq!(
        args,
        [
            "goblin++",
            "encrypt",
            "--key",
            "-nf",
            "--output",
            "output.age",
            "text"
        ]
    );
    let mut program = [
        "goblin++", "run", "test.gbl", "--", "-enc", "-key", "leave-me",
    ]
    .map(str::to_owned);
    let copy = program.clone();
    goblinpp::crypto_cli::normalize_shortcuts(&mut program);
    assert_eq!(program, copy);
}

#[cfg(unix)]
#[test]
fn invalid_utf8_arguments_cannot_leak_through_a_startup_panic() {
    use std::os::unix::ffi::OsStringExt;
    let secret = std::ffi::OsString::from_vec(b"private-argument-\xff".to_vec());
    let result = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["-enc", "-key"])
        .arg(secret)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert_eq!(
        String::from_utf8(result.stderr).unwrap(),
        "GOBLIN ERROR G000\n\nArguments must be valid UTF-8. Supplied values are omitted.\n"
    );
}
