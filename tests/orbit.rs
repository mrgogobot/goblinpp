#![cfg(any(target_os = "linux", target_os = "macos"))]
use goblinpp::hashing::sha256_bytes;
use goblinpp::orbit::{self, BaselineOptions, Classification, Limits, Observation};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

struct Lab {
    _temp: TempDir,
    base: PathBuf,
    root: PathBuf,
    hash: String,
}
impl Lab {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("selected");
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("empty")).unwrap();
        fs::write(root.join("a.txt"), b"abc").unwrap();
        fs::write(root.join("b.txt"), b"").unwrap();
        let hash =
            orbit::capture(&root, &base.join("baseline"), &Limits::default(), options()).unwrap();
        Self {
            _temp: temp,
            base,
            root,
            hash,
        }
    }
    fn check(&self, name: &str, limits: &Limits) -> orbit::CheckResult {
        orbit::check(
            &self.root,
            &self.base.join("baseline/baseline.json"),
            &self.hash,
            &self.base.join(name),
            limits,
            "Lab analyst",
            "Own disposable test directory only",
        )
        .unwrap()
    }
}
fn options() -> BaselineOptions<'static> {
    BaselineOptions {
        reason: "Initial synthetic baseline",
        trust_declaration: "Created fixture independently; this does not certify a real system",
        authorization: "Own disposable test directory",
        parent: None,
    }
}
fn load(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn save(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

#[test]
fn exact_hash_counts_empty_files_and_directories() {
    let lab = Lab::new();
    let baseline = orbit::verify_baseline(&lab.base.join("baseline"), &lab.hash).unwrap();
    assert_eq!(baseline.snapshot.bytes_read, 3);
    assert_eq!(baseline.snapshot.entries.len(), 3);
    assert_eq!(
        baseline.snapshot.entries[0].sha256.as_deref(),
        Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    );
    assert_eq!(
        baseline.snapshot.entries[1].sha256.as_deref(),
        Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    );
    let check = lab.check("match", &Limits::default());
    assert_eq!(check.check.status, "MATCH");
    assert!(
        check
            .check
            .results
            .iter()
            .all(|r| r.classification == Classification::Match)
    );
}

#[test]
fn changed_missing_added_and_type_changes_are_not_clean_verdicts() {
    let lab = Lab::new();
    fs::write(lab.root.join("a.txt"), b"altered").unwrap();
    fs::remove_file(lab.root.join("b.txt")).unwrap();
    fs::write(lab.root.join("new.txt"), b"new").unwrap();
    fs::remove_dir(lab.root.join("empty")).unwrap();
    fs::write(lab.root.join("empty"), b"now a file").unwrap();
    let result = lab.check("changes", &Limits::default());
    assert_eq!(result.check.status, "REVIEW_REQUIRED");
    assert!(result.check.complete);
    let rows = result.check.results;
    assert_eq!(rows[0].classification, Classification::ContentChanged);
    assert_eq!(rows[1].classification, Classification::Missing);
    assert_eq!(rows[2].classification, Classification::ContentChanged);
    assert_eq!(rows[3].classification, Classification::Added);
}

#[test]
fn reports_stand_alone_and_do_not_change_inputs() {
    let lab = Lab::new();
    let before = fs::metadata(lab.root.join("a.txt")).unwrap();
    let result = lab.check("report", &Limits::default());
    let after = fs::metadata(lab.root.join("a.txt")).unwrap();
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    assert_eq!(before.permissions().mode(), after.permissions().mode());
    assert_eq!(fs::read(lab.root.join("a.txt")).unwrap(), b"abc");
    assert_eq!(fs::read_dir(&lab.root).unwrap().count(), 3);
    fs::remove_dir_all(&lab.root).unwrap(); // Test-owned synthetic data only.
    fs::remove_dir_all(lab.base.join("baseline")).unwrap();
    let verified = orbit::verify(
        &result.directory,
        &lab.hash,
        Some(&result.receipt_core_sha256),
    )
    .unwrap();
    assert_eq!(verified.status, "MATCH");
    let case = load(&result.directory.join("case.json"));
    assert_eq!(case["findings"], json!([]));
    assert!(
        fs::read_to_string(result.directory.join("report/report.html"))
            .unwrap()
            .contains("not a clean-system verdict")
    );
}

#[test]
fn baseline_digest_anchor_rejects_modified_declarations() {
    let lab = Lab::new();
    let path = lab.base.join("baseline/baseline.json");
    let mut baseline = load(&path);
    baseline["trust_declaration"] = json!("Pretend this is trusted");
    save(&path, &baseline);
    assert!(orbit::verify_baseline(&lab.base.join("baseline"), &lab.hash).is_err());
    assert!(
        orbit::check(
            &lab.root,
            &path,
            &lab.hash,
            &lab.base.join("check"),
            &Limits::default(),
            "Lab",
            "Own lab"
        )
        .unwrap_err()
        .message
        .contains("SHA-256 mismatch")
    );
    assert!(!lab.base.join("check").exists());
}

#[test]
fn baseline_strict_schema_duplicates_unknown_paths_and_incomplete_claims_refuse() {
    let lab = Lab::new();
    let path = lab.base.join("baseline/baseline.json");
    let original = load(&path);
    for edit in 0..6 {
        let mut value = original.clone();
        match edit {
            0 => value["unknown"] = json!(true),
            1 => value["snapshot"]["entries"][0]["path"] = json!("../secret"),
            2 => value["snapshot"]["entries"][1]["path"] = json!("a.txt"),
            3 => value["snapshot"]["complete"] = json!(false),
            4 => value["snapshot"]["bytes_read"] = json!(4),
            _ => value["snapshot"]["entries"][0]["sha256"] = json!("z".repeat(64)),
        }
        save(&path, &value);
        let hash = sha256_bytes(&fs::read(&path).unwrap());
        assert!(orbit::verify_baseline(&lab.base.join("baseline"), &hash).is_err());
    }
    let text = serde_json::to_string(&original).unwrap();
    let duplicate = text.replacen(
        "\"schema\":",
        "\"schema\":\"goblin.orbit-baseline.v1\",\"schema\":",
        1,
    );
    fs::write(&path, duplicate.as_bytes()).unwrap();
    assert!(
        orbit::verify_baseline(
            &lab.base.join("baseline"),
            &sha256_bytes(duplicate.as_bytes())
        )
        .is_err()
    );
}

#[test]
fn observations_hash_and_classification_tampering_refuse() {
    let lab = Lab::new();
    let result = lab.check("check", &Limits::default());
    let path = result.directory.join("orbit-check.json");
    let mut check = load(&path);
    check["results"][0]["classification"] = json!("ADDED");
    save(&path, &check);
    assert!(
        orbit::verify(&result.directory, &lab.hash, None)
            .unwrap_err()
            .message
            .contains("classification")
    );
    let result = lab.check("other", &Limits::default());
    let path = result.directory.join("observations.json");
    let mut observation = load(&path);
    observation["entries"][0]["sha256"] = json!("0".repeat(64));
    save(&path, &observation);
    assert!(orbit::verify(&result.directory, &lab.hash, None).is_err());
}

#[test]
fn external_receipt_and_exact_copies_are_verified() {
    let lab = Lab::new();
    let result = lab.check("check", &Limits::default());
    assert!(orbit::verify(&result.directory, &lab.hash, Some(&"0".repeat(64))).is_err());
    fs::write(
        result.directory.join("report/evidence/0002.bin"),
        b"changed",
    )
    .unwrap();
    assert!(orbit::verify(&result.directory, &lab.hash, None).is_err());
}

#[test]
fn rebaseline_is_explicit_preserves_parent_and_requires_reason() {
    let lab = Lab::new();
    fs::write(lab.root.join("a.txt"), b"legitimate update").unwrap();
    let parent = lab.base.join("baseline/baseline.json");
    let original = fs::read(&parent).unwrap();
    let output = lab.base.join("baseline-r1");
    let hash = orbit::capture(
        &lab.root,
        &output,
        &Limits::default(),
        BaselineOptions {
            reason: "Reviewed legitimate update",
            parent: Some((&parent, &lab.hash)),
            ..options()
        },
    )
    .unwrap();
    assert_eq!(fs::read(&parent).unwrap(), original);
    assert_eq!(
        fs::read(output.join("parent-baseline.json")).unwrap(),
        original
    );
    let verified = orbit::verify_baseline(&output, &hash).unwrap();
    assert_eq!(
        verified.parent_baseline_sha256.as_deref(),
        Some(lab.hash.as_str())
    );
    fs::write(output.join("parent-baseline.json"), b"changed").unwrap();
    assert!(orbit::verify_baseline(&output, &hash).is_err());
    assert!(
        orbit::capture(
            &lab.root,
            &lab.base.join("bad"),
            &Limits::default(),
            BaselineOptions {
                reason: " ",
                ..options()
            }
        )
        .is_err()
    );
    assert!(!lab.base.join("bad").exists());
}

#[test]
fn file_and_total_byte_budgets_are_enforced_and_preserved() {
    let lab = Lab::new();
    for (name, limits) in [
        (
            "file",
            Limits {
                max_file_bytes: 2,
                ..Limits::default()
            },
        ),
        (
            "total",
            Limits {
                max_total_bytes: 2,
                ..Limits::default()
            },
        ),
    ] {
        let result = lab.check(name, &limits);
        assert_eq!(result.check.status, "INCOMPLETE");
        assert_eq!(
            result.check.results[0].classification,
            Classification::NotObserved
        );
        orbit::verify(
            &result.directory,
            &lab.hash,
            Some(&result.receipt_core_sha256),
        )
        .unwrap();
        let observed = load(&result.directory.join("observations.json"));
        assert_eq!(observed["bytes_read"], 0);
    }
}

#[test]
fn entry_and_depth_budgets_never_invent_missing_rows() {
    let lab = Lab::new();
    let result = lab.check(
        "few",
        &Limits {
            max_entries: 1,
            ..Limits::default()
        },
    );
    assert_eq!(result.check.status, "INCOMPLETE");
    assert!(
        result
            .check
            .results
            .iter()
            .all(|r| r.classification == Classification::NotObserved)
    );
    fs::create_dir_all(lab.root.join("nested/deeper")).unwrap();
    fs::write(lab.root.join("nested/deeper/hidden"), b"data").unwrap();
    let result = lab.check(
        "depth",
        &Limits {
            max_depth: 1,
            ..Limits::default()
        },
    );
    assert_eq!(result.check.status, "INCOMPLETE");
    assert!(
        !result
            .check
            .results
            .iter()
            .any(|r| r.path.ends_with("hidden"))
    );
    assert!(
        orbit::capture(
            &lab.root,
            &lab.base.join("bad"),
            &Limits {
                max_entries: 1,
                ..Limits::default()
            },
            options()
        )
        .is_err()
    );
    assert!(!lab.base.join("bad").exists());
}

#[test]
fn invalid_budgets_refuse_before_output() {
    let lab = Lab::new();
    for limits in [
        Limits {
            max_entries: 0,
            ..Limits::default()
        },
        Limits {
            max_depth: 33,
            ..Limits::default()
        },
        Limits {
            max_file_bytes: 1_073_741_825,
            ..Limits::default()
        },
        Limits {
            max_total_bytes: 8_589_934_593,
            ..Limits::default()
        },
        Limits {
            max_seconds: 3601,
            ..Limits::default()
        },
    ] {
        assert!(
            orbit::check(
                &lab.root,
                &lab.base.join("baseline/baseline.json"),
                &lab.hash,
                &lab.base.join("no"),
                &limits,
                "Lab",
                "Own lab"
            )
            .is_err()
        );
        assert!(!lab.base.join("no").exists());
    }
}

#[test]
fn symlinks_and_special_files_are_unsupported_never_followed_or_blocking() {
    let lab = Lab::new();
    let external = lab.base.join("secret");
    fs::write(&external, b"never read this outside scope").unwrap();
    symlink(&external, lab.root.join("link")).unwrap();
    symlink(&lab.base, lab.root.join("directory-link")).unwrap();
    let fifo =
        std::ffi::CString::new(lab.root.join("fifo").as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: valid terminated fixture path; create a disposable FIFO, never open it.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let result = lab.check("unsupported", &Limits::default());
    assert_eq!(result.check.status, "INCOMPLETE");
    let observed = load(&result.directory.join("observations.json"));
    assert_eq!(observed["bytes_read"], 3);
    assert_eq!(
        result
            .check
            .results
            .iter()
            .filter(|r| r.classification == Classification::Unsupported)
            .count(),
        3
    );
    assert!(
        orbit::capture(
            &lab.root,
            &lab.base.join("bad"),
            &Limits::default(),
            options()
        )
        .is_err()
    );
}

#[test]
fn unsafe_names_are_coverage_gaps_not_silent_omissions() {
    let lab = Lab::new();
    for name in ["newline\nfile", "colon:file", "bidi\u{202e}name"] {
        fs::write(lab.root.join(name), b"fixture").unwrap();
    }
    let result = lab.check("unsafe", &Limits::default());
    assert_eq!(result.check.status, "INCOMPLETE");
    let snapshot: orbit::Snapshot =
        serde_json::from_value(load(&result.directory.join("observations.json"))).unwrap();
    assert_eq!(snapshot.issues.len(), 3);
    assert!(!snapshot.entries.iter().any(|e| e.path.contains('\n')));
}

#[test]
fn unreadable_files_and_root_are_incomplete_with_reports() {
    // Actual permission denials are meaningful only when not running as root.
    // SAFETY: getuid has no arguments or side effects.
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let lab = Lab::new();
    fs::set_permissions(lab.root.join("a.txt"), fs::Permissions::from_mode(0o0)).unwrap();
    let result = lab.check("unreadable", &Limits::default());
    assert_eq!(result.check.status, "INCOMPLETE");
    assert_eq!(
        result.check.results[0].classification,
        Classification::Unreadable
    );
    fs::set_permissions(lab.root.join("a.txt"), fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(&lab.root, fs::Permissions::from_mode(0o0)).unwrap();
    let result = lab.check("root-unreadable", &Limits::default());
    fs::set_permissions(&lab.root, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(result.check.status, "INCOMPLETE");
    assert!(
        result
            .check
            .results
            .iter()
            .all(|r| r.classification == Classification::NotObserved)
    );
}

#[test]
fn destinations_baseline_inputs_and_paths_are_confined_and_exclusive() {
    let lab = Lab::new();
    assert!(
        orbit::capture(
            &lab.root,
            &lab.base.join("baseline"),
            &Limits::default(),
            options()
        )
        .is_err()
    );
    assert!(
        orbit::capture(
            &lab.root,
            &lab.root.join("baseline"),
            &Limits::default(),
            options()
        )
        .is_err()
    );
    assert!(
        orbit::capture(
            &lab.root.join("../selected"),
            &lab.base.join("bad"),
            &Limits::default(),
            options()
        )
        .is_err()
    );
    symlink(&lab.root, lab.base.join("root-link")).unwrap();
    assert!(orbit::scan(&lab.base.join("root-link"), &Limits::default()).is_err());
    symlink(lab.base.join("baseline"), lab.base.join("baseline-link")).unwrap();
    assert!(orbit::verify_baseline(&lab.base.join("baseline-link"), &lab.hash).is_err());
    lab.check("check", &Limits::default());
    let saved = fs::read(lab.base.join("check/orbit-check.json")).unwrap();
    assert!(
        orbit::check(
            &lab.root,
            &lab.base.join("baseline/baseline.json"),
            &lab.hash,
            &lab.base.join("check"),
            &Limits::default(),
            "Lab",
            "Own lab"
        )
        .is_err()
    );
    assert_eq!(
        fs::read(lab.base.join("check/orbit-check.json")).unwrap(),
        saved
    );
}

#[test]
fn root_relocation_and_filesystem_root_are_not_implicit_scope() {
    let lab = Lab::new();
    fs::create_dir(lab.base.join("other")).unwrap();
    assert!(
        orbit::check(
            &lab.base.join("other"),
            &lab.base.join("baseline/baseline.json"),
            &lab.hash,
            &lab.base.join("bad"),
            &Limits::default(),
            "Lab",
            "Own lab"
        )
        .is_err()
    );
    assert!(orbit::scan(Path::new("/"), &Limits::default()).is_err());
}

#[test]
fn reports_have_private_permissions_and_reject_extra_files() {
    let lab = Lab::new();
    let result = lab.check("check", &Limits::default());
    assert_eq!(
        fs::metadata(&result.directory)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(result.directory.join("orbit-check.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    fs::write(result.directory.join("unrecognised"), b"extra").unwrap();
    assert!(orbit::verify(&result.directory, &lab.hash, None).is_err());
}

#[test]
fn standalone_and_adapter_share_cli_results_and_exit_contract() {
    let lab = Lab::new();
    let mut previous = None;
    for (i, binary) in [
        env!("CARGO_BIN_EXE_goblinpp"),
        env!("CARGO_BIN_EXE_goblin-orbit"),
    ]
    .into_iter()
    .enumerate()
    {
        for (name, expected, limits) in [("match", 0, false), ("budget", 2, true)] {
            let output = lab.base.join(format!("cli-{i}-{name}"));
            let mut command = Command::new(binary);
            if i == 0 {
                command.arg("orbit");
            }
            command
                .args(["check"])
                .arg(&lab.root)
                .arg("--baseline")
                .arg(lab.base.join("baseline/baseline.json"))
                .arg("--expected-baseline-sha256")
                .arg(&lab.hash)
                .arg("--output")
                .arg(output)
                .args(["--author", "Lab", "--authorization", "Own lab", "--json"]);
            if limits {
                command.args(["--max-file-bytes", "2"]);
            }
            let result = command.output().unwrap();
            assert_eq!(
                result.status.code(),
                Some(expected),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let mut value: Value = serde_json::from_slice(&result.stdout).unwrap();
            if name == "match" {
                // Independent UTC observations legitimately have different digests.
                value["check"]
                    .as_object_mut()
                    .unwrap()
                    .remove("observation_sha256");
                if let Some(before) = &previous {
                    assert_eq!(&value["check"], before);
                }
                previous = Some(value["check"].clone());
            }
        }
    }
    // Real time differs between observations; their independent digest must differ.
}

#[test]
fn cli_requires_authorization_reason_trust_and_external_baseline_anchor() {
    let lab = Lab::new();
    for args in [
        vec!["baseline", lab.root.to_str().unwrap()],
        vec!["check", lab.root.to_str().unwrap()],
        vec!["verify", lab.base.to_str().unwrap()],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_goblin-orbit"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2));
    }
}

#[test]
fn cli_review_exit_is_one_not_success_or_machinery_error() {
    let lab = Lab::new();
    fs::write(lab.root.join("a.txt"), b"changed").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_goblin-orbit"))
        .arg("check")
        .arg(&lab.root)
        .arg("--baseline")
        .arg(lab.base.join("baseline/baseline.json"))
        .arg("--expected-baseline-sha256")
        .arg(&lab.hash)
        .arg("--output")
        .arg(lab.base.join("cli-review"))
        .args(["--author", "Lab", "--authorization", "Own lab"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("possible tampering or legitimate change")
    );
}

#[test]
fn interrupted_process_cannot_leave_a_success_receipt() {
    let lab = Lab::new();
    fs::File::create(lab.root.join("large"))
        .unwrap()
        .set_len(64 * 1024 * 1024)
        .unwrap();
    let output = lab.base.join("interrupted");
    let mut child = Command::new(env!("CARGO_BIN_EXE_goblin-orbit"))
        .arg("baseline")
        .arg(&lab.root)
        .arg("--output")
        .arg(&output)
        .args([
            "--reason",
            "Lab",
            "--trust-declaration",
            "Synthetic",
            "--authorization",
            "Own lab",
        ])
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    assert!(!output.join("baseline.json").exists());
}

#[test]
fn baseline_declarations_reject_control_and_bidi_content() {
    let lab = Lab::new();
    for reason in ["\n", "text\u{202e}text", "text\0text"] {
        assert!(
            orbit::capture(
                &lab.root,
                &lab.base.join("bad"),
                &Limits::default(),
                BaselineOptions {
                    reason,
                    ..options()
                }
            )
            .is_err()
        );
    }
}

#[test]
fn oversized_json_is_bounded_and_not_a_normal_file() {
    let lab = Lab::new();
    let path = lab.base.join("baseline/baseline.json");
    fs::File::create(&path)
        .unwrap()
        .set_len(8 * 1024 * 1024 + 1)
        .unwrap();
    assert!(
        orbit::verify_baseline(&lab.base.join("baseline"), &"0".repeat(64))
            .unwrap_err()
            .message
            .contains("8MiB")
    );
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(orbit::verify_baseline(&lab.base.join("baseline"), &lab.hash).is_err());
}

#[test]
fn permission_changes_are_not_silently_presented_as_security_audits() {
    let lab = Lab::new();
    fs::set_permissions(lab.root.join("a.txt"), fs::Permissions::from_mode(0o644)).unwrap();
    let result = lab.check("permissions", &Limits::default());
    assert_eq!(result.check.status, "MATCH"); // v1 policy compares bytes and entry kinds only.
    let observed: orbit::Snapshot =
        serde_json::from_value(load(&result.directory.join("observations.json"))).unwrap();
    assert_eq!(observed.entries[0].observation, Observation::Hashed);
    assert_eq!(
        observed.entries[0].before.as_ref().unwrap().mode & 0o777,
        0o644
    );
}
