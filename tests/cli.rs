use goblinpp::custody::create_freeze;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn bare_gbl_argument_selects_interpreter() {
    let root = tempdir().unwrap();
    let source = root.path().join("hello.gbl");
    fs::write(&source, "GO_PARANOID\nprint(\"hello from Rust\")\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hello from Rust"));
    assert!(stdout.contains("EXECUTION_ENGINE=rust-interpreter"));
}

#[test]
fn bare_relative_filename_selects_interpreter() {
    let root = tempdir().unwrap();
    fs::write(
        root.path().join("hello.gbl"),
        "print(\"relative path works\")\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .current_dir(root.path())
        .arg("hello.gbl")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("relative path works")
    );
}

#[test]
fn check_prints_inline_digest_without_execution() {
    let root = tempdir().unwrap();
    let source = root.path().join("inline.gbl");
    fs::write(
        &source,
        "x = 1\nRUST_INLINE_BEGIN\npanic!(\"must not execute\");\nRUST_INLINE_END\nseal x\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["check", source.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("INLINE_RUST_BLOCKS=1"));
    assert!(stdout.contains("INLINE_RUST_1_SHA256="));
    assert!(stdout.contains("CHECK_STATUS=PASS"));
}

#[test]
fn compile_refuses_changed_frozen_source() {
    let root = tempdir().unwrap();
    let source = root.path().join("frozen.gbl");
    fs::write(&source, "x = 1\nseal x\n").unwrap();
    create_freeze(&source).unwrap();
    fs::write(&source, "x = 2\nseal x\n").unwrap();
    let binary = root.path().join("must-not-exist");
    let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args([
            "compile",
            source.to_str().unwrap(),
            "-o",
            binary.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!binary.exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("PROTOCOL VIOLATION"));
}

#[test]
fn fits_info_discovers_table_columns_without_creating_a_run() {
    let root = tempdir().unwrap();
    let file = root.path().join("catalog.fits");
    fs::write(&file, minimal_table_fits()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["fits-info", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("GOBLIN FITS INFO"));
    assert!(stdout.contains("HDUS=2"));
    assert!(stdout.contains("TYPE=BINTABLE"));
    assert!(stdout.contains("1 Z TFORM=1D TYPE=f64 REPEAT=1 UNIT=NONE READABLE=YES"));
    assert!(stdout.contains("AUTHORITY=INSPECTION_ONLY_NOT_RUN_EVIDENCE"));
    assert!(!root.path().join("runs").exists());

    let json_output = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["fits-info", file.to_str().unwrap(), "--json", "--quick"])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(report["hdu_count"], 2);
    assert_eq!(report["hdus"][1]["columns"][0]["name"], "Z");
    assert!(report["sha256"].is_null());
    assert_eq!(
        report["authority"],
        "QUICK_INSPECTION_UNHASHED_NOT_EVIDENCE"
    );
}

fn minimal_table_fits() -> Vec<u8> {
    fn card(key: &str, value: &str) -> Vec<u8> {
        let mut card = format!("{key:<8}= {value:>20}").into_bytes();
        card.resize(80, b' ');
        card
    }
    fn header(mut cards: Vec<Vec<u8>>) -> Vec<u8> {
        let mut end = format!("{:<8}", "END").into_bytes();
        end.resize(80, b' ');
        cards.push(end);
        let mut bytes = cards.into_iter().flatten().collect::<Vec<_>>();
        bytes.resize(bytes.len().div_ceil(2880) * 2880, b' ');
        bytes
    }
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
        card("NAXIS1", "8"),
        card("NAXIS2", "1"),
        card("PCOUNT", "0"),
        card("GCOUNT", "1"),
        card("TFIELDS", "1"),
        card("TTYPE1", "'Z'"),
        card("TFORM1", "'1D'"),
    ]));
    bytes.extend(0.25_f64.to_be_bytes());
    bytes.resize(bytes.len().div_ceil(2880) * 2880, 0);
    bytes
}
