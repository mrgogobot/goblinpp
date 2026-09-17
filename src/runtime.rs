use crate::ast::{Program, Stmt};
use crate::compiler::{Compilation, compile};
use crate::custody::{freeze_path, verify_freeze, verify_registration};
use crate::error::{GoblinError, Result};
use crate::evaluator::{DataImport, Evaluation, Value as EvalValue};
use crate::hashing::{hash_canonical_json, sha256_bytes, sha256_file};
use crate::interaction::InputPolicy;
use crate::ledger::{self, LedgerReport};
use crate::parser::{ParsedSource, parse_source};
use chrono::Utc;
use serde_json::{Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    pub runs_root: Option<PathBuf>,
    pub compile: bool,
    pub allowed_inline_rust: Vec<String>,
    pub program_args: Vec<String>,
    pub input_policy: InputPolicy,
}

pub fn run_file(source: impl AsRef<Path>, options: &RunOptions) -> Result<PathBuf> {
    let source = source.as_ref();
    if source.is_symlink() {
        return Err(GoblinError::new(
            "G000",
            "Refusing to run a symbolic-link source.",
        ));
    }
    if !source.is_file() {
        return Err(GoblinError::new(
            "G000",
            format!("Source file not found: {}", source.display()),
        ));
    }
    let source_bytes = fs::read(source)?;
    let source_sha = sha256_bytes(&source_bytes);
    let root = ledger::project_root_for(source)?;
    let source_parent = source
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let run_base = options
        .runs_root
        .clone()
        .unwrap_or_else(|| source_parent.join("runs"));
    fs::create_dir_all(&run_base)?;
    let canonical_run_base = run_base.canonicalize()?;
    if !canonical_run_base.starts_with(&root) {
        return Err(GoblinError::artifact(
            "Run evidence directory must remain inside the project.",
        ));
    }
    let run_dir = unique_run_dir(&run_base, &source_sha)?;
    let preserved_source = run_dir.join(source.file_name().unwrap());
    fs::write(&preserved_source, &source_bytes)?;
    let started = timestamp();
    let mut receipt = json!({
        "schema": "goblin.run-receipt.v2", "goblin_version": crate::VERSION,
        "status": "MACHINERY_FAIL", "started": started, "finished": Value::Null,
        "execution": { "engine": if options.compile { "rust-native-compiled" } else { "rust-interpreter" }, "compiler": Value::Null },
        "paranoid_mode": false, "paranoid_postflight": Value::Null,
        "source": { "path": source.file_name().unwrap().to_string_lossy(), "sha256": source_sha },
        "canonical_source": Value::Null, "constants_used": [], "data_imports": [], "inline_rust": [],
        "sealed_artifacts": [], "generated_artifacts": [], "stdout_sha256": Value::Null, "stderr_sha256": Value::Null,
        "interaction": Value::Null,
        "environment": { "runtime": format!("goblin++ {}", crate::VERSION), "os": std::env::consts::OS, "arch": std::env::consts::ARCH },
        "freeze": { "status": "NOT_FROZEN" }, "ledger": Value::Null,
        "protocol_violation": Value::Null, "failure": Value::Null,
    });
    let stdout_path = run_dir.join("stdout.log");
    let stderr_path = run_dir.join("stderr.log");
    let preflight_ledger = ledger::audit(&root);
    receipt["ledger"] = preserve_ledger_evidence(&root, &run_dir, &preflight_ledger)?;

    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut parsed: Option<ParsedSource> = None;
    let mut evaluation = Evaluation::new(source_parent);
    let mut outcome = (|| -> Result<()> {
        let mut argv = vec![source.to_string_lossy().to_string()];
        argv.extend(options.program_args.iter().cloned());
        evaluation.configure_interaction(argv, options.input_policy.clone())?;
        if preflight_ledger.exists && !preflight_ledger.verified {
            return Err(protocol(
                "LEDGER_INTEGRITY_FAILURE",
                "The project custody ledger or head checkpoint is damaged.",
            ));
        }
        let text = std::str::from_utf8(&source_bytes)
            .map_err(|_| GoblinError::lex("Source is not valid UTF-8."))?;
        let current = parse_source(text)?;
        current.require_executable_program()?;
        receipt["canonical_source"] = json!({ "sha256": current.canonical_sha256()? });
        receipt["paranoid_mode"] = Value::Bool(current.paranoid());
        receipt["inline_rust"] = json!(
            current
                .inline_rust
                .iter()
                .map(|block| json!({
                    "index": block.index, "sha256": block.sha256,
                    "authorized": options.allowed_inline_rust.contains(&block.sha256),
                }))
                .collect::<Vec<_>>()
        );
        enforce_freeze(source, &root, &run_dir, &mut receipt)?;
        if options.compile {
            crate::inline_rust::approved(&current.inline_rust, &options.allowed_inline_rust)?;
            let safe_program = Program {
                statements: current
                    .program
                    .statements
                    .iter()
                    .filter(|stmt| !matches!(stmt, Stmt::InlineRust { .. }))
                    .cloned()
                    .collect(),
            };
            evaluation.register_functions(&safe_program)?;
            for statement in &safe_program.statements {
                evaluation.eval_stmt(statement)?;
            }
            let binary = run_dir.join("program-native");
            let compilation = compile(&current, &binary, &options.allowed_inline_rust)?;
            receipt["execution"]["compiler"] = compilation_json(&compilation);
            let absolute_binary = binary.canonicalize()?;
            let native_results = run_dir.join("native-results.tsv");
            let absolute_native_results = run_dir.canonicalize()?.join("native-results.tsv");
            let mut child = Command::new(&absolute_binary)
                .args(&options.program_args)
                .current_dir(source_parent)
                .env("GOBLIN_NATIVE_RESULT_PATH", &absolute_native_results)
                .env("GOBLIN_SOURCE_ARGV0", source.to_string_lossy().as_ref())
                .env("GOBLIN_STDIN_REPLAY", "1")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| {
                    GoblinError::compile(format!("Unable to execute compiled program: {error}"))
                })?;
            let mut child_stdin = child.stdin.take().ok_or_else(|| {
                GoblinError::compile("Compiled program standard input pipe is unavailable.")
            })?;
            let replay = evaluation
                .interaction
                .input_events
                .iter()
                .filter_map(|event| event.response.as_ref())
                .map(|response| format!("{response}\n"))
                .collect::<String>();
            let writer = std::thread::spawn(move || child_stdin.write_all(replay.as_bytes()));
            let process = child.wait_with_output().map_err(|error| {
                GoblinError::compile(format!("Unable to wait for compiled program: {error}"))
            })?;
            stdout = String::from_utf8(process.stdout)
                .map_err(|_| GoblinError::compile("Compiled program wrote non-UTF-8 stdout."))?;
            stderr = String::from_utf8_lossy(&process.stderr).to_string();
            if !process.status.success() {
                return Err(GoblinError::compile(format!(
                    "Compiled program exited with {}.\n\n{}",
                    process.status,
                    stderr.trim()
                )));
            }
            writer
                .join()
                .map_err(|_| GoblinError::compile("Compiled stdin replay thread panicked."))?
                .map_err(|error| {
                    GoblinError::compile(format!("Compiled stdin replay failed: {error}"))
                })?;
            verify_native_results(&native_results, &evaluation.sealed)?;
            receipt["execution"]["compiler"]["native_result_manifest_path"] =
                Value::String("native-results.tsv".into());
            receipt["execution"]["compiler"]["native_result_manifest_sha256"] =
                Value::String(sha256_file(&native_results)?);
        } else {
            evaluation.register_functions(&current.program)?;
            for statement in &current.program.statements {
                evaluation.eval_stmt(statement)?;
            }
            stdout = lines_to_text(&evaluation.stdout);
        }
        parsed = Some(current);
        Ok(())
    })();

    if parsed.is_none()
        && let Ok(text) = std::str::from_utf8(&source_bytes)
        && let Ok(value) = parse_source(text)
    {
        parsed = Some(value);
    }
    let mut imports = evaluation.data_imports();
    if let Err(error) = preserve_data_evidence(&root, &run_dir, &evaluation, &mut imports) {
        outcome = Err(error);
    }
    receipt["data_imports"] = serde_json::to_value(&imports)?;
    if evaluation.interaction.evidence_needed() {
        let evidence_path = run_dir.join("interaction.json");
        let evidence = json!({
            "schema": "goblin.interaction.v1",
            "argv": &evaluation.interaction.argv,
            "input_events": &evaluation.interaction.input_events,
        });
        if let Err(error) = write_private_json_exclusive(&evidence_path, &evidence) {
            outcome = Err(error);
        } else {
            receipt["interaction"] = json!({
                "evidence_path": "interaction.json",
                "evidence_sha256": sha256_file(&evidence_path)?,
                "argc": evaluation.interaction.argv.len(),
                "input_count": evaluation.interaction.input_events.len(),
            });
        }
    }
    receipt["constants_used"] =
        serde_json::to_value(evaluation.constants_used.values().collect::<Vec<_>>())?;
    let mut generated_artifacts = Vec::new();
    if !evaluation.generated.is_empty() {
        let outputs_dir = run_dir.join("outputs");
        fs::create_dir(&outputs_dir)?;
        for artifact in evaluation.generated.values() {
            let path = outputs_dir.join(&artifact.name);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|error| {
                    GoblinError::artifact(format!(
                        "Unable to create generated output {}: {error}",
                        path.display()
                    ))
                })?;
            file.write_all(&artifact.bytes)?;
            file.sync_all()?;
            generated_artifacts.push(json!({
                "name": artifact.name,
                "path": format!("outputs/{}", artifact.name),
                "sha256": sha256_file(&path)?,
                "byte_count": artifact.bytes.len(),
                "media_type": artifact.media_type,
                "producer": artifact.producer,
                "metadata": artifact.metadata,
            }));
        }
    }
    receipt["generated_artifacts"] = Value::Array(generated_artifacts);
    let mut artifacts = Vec::new();
    for (name, value) in &evaluation.sealed {
        let path = run_dir.join(format!("{name}.json"));
        let artifact = json!({ "schema": "goblin.sealed-artifact.v1", "name": name, "value": value.to_json() });
        write_json_exclusive(&path, &artifact)?;
        artifacts.push(json!({ "name": name, "path": path.file_name().unwrap().to_string_lossy(), "sha256": sha256_file(&path)? }));
    }
    receipt["sealed_artifacts"] = Value::Array(artifacts);
    if receipt["paranoid_mode"] == true
        && let Err(error) = observe_paranoid_source(source, &source_sha, &run_dir, &mut receipt)
    {
        outcome = Err(error);
    }
    match outcome {
        Ok(()) => receipt["status"] = Value::String("PASS".into()),
        Err(error) => {
            if !stderr.is_empty() && !stderr.ends_with('\n') {
                stderr.push('\n');
            }
            stderr.push_str(&error.pretty());
            stderr.push('\n');
            receipt["status"] = Value::String(error.category.into());
            receipt["failure"] = json!({ "code": error.code, "type": "GoblinError", "message": error.message, "classification": error.classification });
            if error.category == "PROTOCOL_VIOLATION" {
                receipt["protocol_violation"] = json!({ "classification": error.classification });
            }
        }
    }
    fs::write(&stdout_path, stdout.as_bytes())?;
    fs::write(&stderr_path, stderr.as_bytes())?;
    receipt["finished"] = Value::String(timestamp());
    receipt["stdout_sha256"] = Value::String(sha256_file(&stdout_path)?);
    receipt["stderr_sha256"] = Value::String(sha256_file(&stderr_path)?);
    let core_hash = sealed_hash(&receipt, "receipt_core_sha256")?;
    receipt.as_object_mut().unwrap().insert(
        "receipt_core_sha256".into(),
        Value::String(core_hash.clone()),
    );
    let receipt_path = run_dir.join("receipt.json");
    write_json_exclusive(&receipt_path, &receipt)?;

    let mut paranoid_self_check_failed = false;
    if receipt["paranoid_mode"] == true {
        let self_check = crate::audit::verify_run(&run_dir);
        if !matches!(self_check, Ok(ref report) if report.verified) {
            paranoid_self_check_failed = true;
            let detail = match self_check {
                Ok(report) => report
                    .checks
                    .iter()
                    .filter(|check| !check.pass)
                    .map(|check| check.check.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                Err(error) => error.to_string(),
            };
            downgrade_self_check(&mut receipt, &receipt_path, &stderr_path, &detail)?;
        }
    }

    if !paranoid_self_check_failed && (!preflight_ledger.exists || preflight_ledger.verified) {
        let core_hash = receipt["receipt_core_sha256"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let event_type = if receipt["status"] == "PROTOCOL_VIOLATION" {
            "PROTOCOL_VIOLATION"
        } else {
            "RUN"
        };
        if let Err(error) = ledger::append(
            &root,
            event_type,
            ledger::subject_for(source, &root)?,
            json!({
                "status": receipt["status"], "source_sha256": source_sha,
                "canonical_source_sha256": parsed.as_ref().and_then(|value| value.canonical_sha256().ok()),
                "run_directory": ledger::subject_for(&run_dir, &root)?,
                "run_receipt_core_sha256": core_hash, "run_receipt_file_sha256": sha256_file(&receipt_path)?,
                "execution_engine": receipt["execution"]["engine"],
                "protocol_classification": receipt["protocol_violation"]["classification"],
            }),
        ) {
            let warning = format!("GOBLIN ERROR G404\n\nLEDGER REGISTRATION FAILED\n\n{error}\n");
            let mut current = fs::read_to_string(&stderr_path)?;
            current.push_str(&warning);
            fs::write(&stderr_path, current)?;
            receipt["status"] = Value::String("MACHINERY_FAIL".into());
            receipt["ledger"]["registration"] = Value::String("FAIL".into());
            receipt["failure"] = json!({
                "code": "G404", "type": "GoblinError", "message": error.to_string(),
                "classification": "LEDGER_REGISTRATION_FAILED",
            });
            receipt["finished"] = Value::String(timestamp());
            receipt["stderr_sha256"] = Value::String(sha256_file(&stderr_path)?);
            receipt
                .as_object_mut()
                .unwrap()
                .remove("receipt_core_sha256");
            let repaired_core = sealed_hash(&receipt, "receipt_core_sha256")?;
            receipt
                .as_object_mut()
                .unwrap()
                .insert("receipt_core_sha256".into(), Value::String(repaired_core));
            let mut bytes = serde_json::to_vec_pretty(&receipt)?;
            bytes.push(b'\n');
            fs::write(&receipt_path, bytes)?;
        }
    }
    Ok(run_dir)
}

fn observe_paranoid_source(
    source: &Path,
    initial_sha: &str,
    run_dir: &Path,
    receipt: &mut Value,
) -> Result<()> {
    let metadata = fs::symlink_metadata(source);
    let source_bytes = match metadata {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {
            fs::read(source)
        }
        Ok(_) => Err(std::io::Error::other(
            "source is no longer an ordinary non-symlink file",
        )),
        Err(error) => Err(error),
    };
    let source_bytes = match source_bytes {
        Ok(bytes) => bytes,
        Err(error) => {
            receipt["paranoid_postflight"] = json!({"status":"SOURCE_UNAVAILABLE", "source_start_sha256":initial_sha, "detail":error.to_string(), "self_verification_gate":"REQUIRED_BEFORE_LEDGER_REGISTRATION"});
            return Err(protocol(
                "SOURCE_UNAVAILABLE_AT_POSTFLIGHT",
                "GO_PARANOID could not re-read the source at postflight. Execution cannot be certified.",
            ));
        }
    };
    let end_sha = sha256_bytes(&source_bytes);
    let evidence_dir = run_dir.join("postflight");
    let evidence = evidence_dir.join("source.observed");
    let evidence_result = (|| -> std::io::Result<()> {
        fs::create_dir(&evidence_dir)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&evidence)?;
        file.write_all(&source_bytes)?;
        file.sync_all()
    })();
    if let Err(error) = evidence_result {
        receipt["paranoid_postflight"] = json!({
            "status":"EVIDENCE_UNAVAILABLE", "source_start_sha256":initial_sha,
            "source_end_sha256":end_sha, "detail":error.to_string(),
            "self_verification_gate":"REQUIRED_BEFORE_LEDGER_REGISTRATION"
        });
        return Err(protocol(
            "POSTFLIGHT_EVIDENCE_UNAVAILABLE",
            "GO_PARANOID could not preserve postflight source evidence. Execution cannot be certified.",
        ));
    }
    let same = end_sha == initial_sha;
    receipt["paranoid_postflight"] = json!({
        "status": if same {"PASS"} else {"SOURCE_CHANGED"},
        "source_start_sha256": initial_sha,
        "source_end_sha256": end_sha,
        "source_end_evidence_path": "postflight/source.observed",
        "source_end_evidence_sha256": sha256_file(&evidence)?,
        "source_bytes_equal_at_postflight": same,
        "self_verification_gate": "REQUIRED_BEFORE_LEDGER_REGISTRATION"
    });
    if same {
        Ok(())
    } else {
        Err(protocol(
            "SOURCE_CHANGED_AT_POSTFLIGHT",
            "GO_PARANOID observed different source bytes at postflight. Execution used the preserved starting source, but a PASS claim is refused.",
        ))
    }
}

fn downgrade_self_check(
    receipt: &mut Value,
    receipt_path: &Path,
    stderr_path: &Path,
    detail: &str,
) -> Result<()> {
    let mut stderr = fs::read_to_string(stderr_path)?;
    if !stderr.is_empty() && !stderr.ends_with('\n') {
        stderr.push('\n');
    }
    stderr.push_str(&format!(
        "GOBLIN ERROR G405\n\nPARANOID SELF-VERIFICATION FAILED\n\n{detail}\n"
    ));
    fs::write(stderr_path, stderr)?;
    receipt["status"] = Value::String("MACHINERY_FAIL".into());
    receipt["failure"] = json!({"code":"G405", "type":"GoblinError", "message":format!("Paranoid self-verification failed: {detail}"), "classification":"PARANOID_SELF_VERIFICATION_FAILED"});
    receipt["finished"] = Value::String(timestamp());
    receipt["stderr_sha256"] = Value::String(sha256_file(stderr_path)?);
    receipt
        .as_object_mut()
        .unwrap()
        .remove("receipt_core_sha256");
    let core = sealed_hash(receipt, "receipt_core_sha256")?;
    receipt["receipt_core_sha256"] = Value::String(core);
    let mut bytes = serde_json::to_vec_pretty(receipt)?;
    bytes.push(b'\n');
    fs::write(receipt_path, bytes)?;
    Ok(())
}

fn enforce_freeze(source: &Path, root: &Path, run_dir: &Path, receipt: &mut Value) -> Result<()> {
    let path = freeze_path(source);
    if path.exists() {
        let report = verify_freeze(source);
        let evidence_path = receipt_evidence_name("freeze_receipt.observed.json");
        let evidence_file = run_dir.join(&evidence_path);
        fs::copy(&path, &evidence_file)?;
        let evidence_sha256 = sha256_file(&evidence_file)?;
        receipt["freeze"] = json!({
            "status": if report.verified { "PASS" } else { "FAIL" }, "classification": report.classification,
            "receipt_path": path.file_name().unwrap().to_string_lossy(), "receipt_file_sha256": report.receipt_file_sha256,
            "source_bytes_unchanged": report.checks.iter().find(|check| check.check == "SOURCE_SHA256").is_some_and(|check| check.pass),
            "canonical_program_unchanged": report.checks.iter().find(|check| check.check == "CANONICAL_SOURCE_SHA256").is_some_and(|check| check.pass),
            "constants_pinned": report.checks.iter().find(|check| check.check == "CONSTANT_REGISTRY_SHA256").is_some_and(|check| check.pass),
            "checks": report.checks, "evidence_path": evidence_path, "evidence_sha256": evidence_sha256,
        });
        if !report.verified {
            return Err(protocol(
                &report.classification,
                freeze_detail(&report.classification),
            ));
        }
        if !verify_registration(source, &report)? {
            return Err(protocol(
                "UNREGISTERED_FREEZE_RECEIPT",
                "The freeze receipt has no matching event in the clean custody ledger.",
            ));
        }
        return Ok(());
    }
    let subject = ledger::subject_for(source, root)?;
    let ledger_report = ledger::audit(root);
    if ledger_report
        .events
        .iter()
        .any(|event| event.subject == subject && event.event_type == "FREEZE")
    {
        return Err(protocol(
            "FREEZE_RECEIPT_MISSING",
            "The custody ledger records this source as frozen, but its freeze receipt is missing.",
        ));
    }
    Ok(())
}

fn preserve_ledger_evidence(root: &Path, run_dir: &Path, report: &LedgerReport) -> Result<Value> {
    let mut value = json!({ "status": report.status, "authentication": report.authentication, "event_count": report.events.len(), "head_event_sha256": report.head_event_sha256, "checks": report.checks });
    let path = ledger::ledger_path(root);
    if path.exists() {
        let destination = run_dir.join("custody-ledger.observed.jsonl");
        fs::copy(&path, &destination)?;
        value["evidence_path"] = Value::String(
            destination
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
        );
        value["evidence_sha256"] = Value::String(sha256_file(destination)?);
        let head = ledger::head_path(root);
        if head.exists() {
            let destination = run_dir.join("custody-ledger-head.observed.json");
            fs::copy(head, &destination)?;
            value["head_evidence_path"] = Value::String(
                destination
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
            );
            value["head_evidence_sha256"] = Value::String(sha256_file(destination)?);
        }
    }
    Ok(value)
}

fn preserve_data_evidence(
    root: &Path,
    run_dir: &Path,
    evaluation: &Evaluation,
    imports: &mut [DataImport],
) -> Result<()> {
    if imports.is_empty() {
        return Ok(());
    }
    let directory = run_dir.join("imports");
    fs::create_dir(&directory)?;
    let store = root.join(".goblin/imports");
    fs::create_dir_all(&store)?;
    let store_metadata = fs::symlink_metadata(&store)?;
    if store_metadata.file_type().is_symlink() || !store_metadata.is_dir() {
        return Err(GoblinError::data(format!(
            "FITS evidence store is not a trusted ordinary directory: {}.",
            store.display()
        )));
    }
    for import in imports {
        let name = format!("{}.fits", import.sha256);
        let stored = store.join(&name);
        let destination = directory.join(&name);
        if stored.exists() {
            let metadata = fs::symlink_metadata(&stored)?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(GoblinError::data(format!(
                    "Stored FITS evidence is not a trusted ordinary file: {}.",
                    stored.display()
                )));
            }
            let stored_hash = sha256_file(&stored)?;
            if stored_hash != import.sha256 {
                preserve_direct_import(evaluation, import, &destination, &name)?;
                return Err(GoblinError::data(format!(
                    "Stored FITS evidence {} is damaged; expected {}, observed {}.",
                    stored.display(),
                    import.sha256,
                    stored_hash
                )));
            }
        } else {
            let temporary =
                store.join(format!(".{}.partial-{}", import.sha256, std::process::id()));
            if temporary.exists() {
                fs::remove_file(&temporary)?;
            }
            let copied_hash = evaluation.copy_data_evidence(&import.sha256, &temporary)?;
            if copied_hash != import.sha256 {
                let _ = fs::remove_file(&temporary);
                return Err(GoblinError::data(format!(
                    "FITS input {} changed while Goblin++ was reading it; execution evidence is not stable.",
                    import.path
                )));
            }
            if let Err(error) = register_evidence_object(&temporary, &stored, &import.sha256) {
                if stored.exists() {
                    preserve_direct_import(evaluation, import, &destination, &name)?;
                }
                return Err(error);
            }
        }
        if let Err(link_error) = fs::hard_link(&stored, &destination) {
            fs::copy(&stored, &destination).map_err(|copy_error| {
                GoblinError::data(format!(
                    "Unable to attach FITS evidence to this run (link: {link_error}; copy: {copy_error})."
                ))
            })?;
        }
        let evidence_sha256 = sha256_file(&destination)?;
        import.evidence_path = Some(format!("imports/{name}"));
        import.evidence_sha256 = Some(evidence_sha256.clone());
        if evidence_sha256 != import.sha256 {
            return Err(GoblinError::data(format!(
                "FITS input {} changed while Goblin++ was reading it; execution evidence is not stable.",
                import.path
            )));
        }
    }
    Ok(())
}

fn preserve_direct_import(
    evaluation: &Evaluation,
    import: &mut DataImport,
    destination: &Path,
    name: &str,
) -> Result<()> {
    let evidence_sha256 = evaluation.copy_data_evidence(&import.sha256, destination)?;
    import.evidence_path = Some(format!("imports/{name}"));
    import.evidence_sha256 = Some(evidence_sha256.clone());
    if evidence_sha256 != import.sha256 {
        return Err(GoblinError::data(format!(
            "FITS input {} changed while Goblin++ was reading it; execution evidence is not stable.",
            import.path
        )));
    }
    Ok(())
}

fn register_evidence_object(temporary: &Path, stored: &Path, expected: &str) -> Result<()> {
    match fs::hard_link(temporary, stored) {
        Ok(()) => {
            fs::remove_file(temporary)?;
            make_readonly(stored)?;
            return Ok(());
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            fs::remove_file(temporary)?;
            return verify_registered_object(stored, expected);
        }
        Err(_) => {}
    }

    let mut output = match OpenOptions::new().write(true).create_new(true).open(stored) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            fs::remove_file(temporary)?;
            return verify_registered_object(stored, expected);
        }
        Err(error) => {
            let _ = fs::remove_file(temporary);
            return Err(GoblinError::data(format!(
                "Unable to register FITS evidence object {}: {error}",
                stored.display()
            )));
        }
    };
    let copied = (|| -> std::io::Result<()> {
        let mut input = File::open(temporary)?;
        std::io::copy(&mut input, &mut output)?;
        output.flush()?;
        Ok(())
    })();
    let _ = fs::remove_file(temporary);
    if let Err(error) = copied {
        let _ = fs::remove_file(stored);
        return Err(GoblinError::data(format!(
            "Unable to register FITS evidence object {}: {error}",
            stored.display()
        )));
    }
    if let Err(error) = verify_registered_object(stored, expected) {
        let _ = fs::remove_file(stored);
        return Err(error);
    }
    make_readonly(stored)
}

fn verify_registered_object(stored: &Path, expected: &str) -> Result<()> {
    let actual = sha256_file(stored)?;
    if actual == expected {
        Ok(())
    } else {
        Err(GoblinError::data(format!(
            "Concurrent FITS evidence registration produced a mismatched object at {}; expected {}, observed {}.",
            stored.display(),
            expected,
            actual
        )))
    }
}

fn make_readonly(path: &Path) -> Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn compilation_json(compilation: &Compilation) -> Value {
    json!({
        "rustc": compilation.rustc_version,
        "binary_path": compilation.binary.file_name().unwrap().to_string_lossy(), "binary_sha256": compilation.binary_sha256,
        "generated_source_path": compilation.generated_source.file_name().unwrap().to_string_lossy(), "generated_source_sha256": compilation.generated_source_sha256,
    })
}

fn verify_native_results(
    path: &Path,
    expected: &std::collections::BTreeMap<String, EvalValue>,
) -> Result<()> {
    let text = fs::read_to_string(path).map_err(|error| {
        GoblinError::compile(format!(
            "Native program did not produce its sealed-result manifest: {error}"
        ))
    })?;
    let mut observed = std::collections::BTreeMap::new();
    for (line_index, line) in text.lines().enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        let value = match fields.as_slice() {
            ["Q", name, bits, dimensions] => {
                let name = decode_hex_text(name)?;
                let bits = u64::from_str_radix(bits, 16).map_err(|_| {
                    GoblinError::compile(format!(
                        "Invalid native quantity bits at result line {}.",
                        line_index + 1
                    ))
                })?;
                let parts = dimensions
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|_| GoblinError::compile("Invalid native result dimension."))?;
                let dimension: [i32; 5] = parts.try_into().map_err(|_| {
                    GoblinError::compile("Native result dimension needs five axes.")
                })?;
                (
                    name,
                    EvalValue::Quantity(crate::quantity::Quantity::new(
                        f64::from_bits(bits),
                        dimension,
                    )?),
                )
            }
            ["T", name, value] => (
                decode_hex_text(name)?,
                EvalValue::Text(decode_hex_text(value)?),
            ),
            ["B", name, value] if *value == "true" || *value == "false" => {
                (decode_hex_text(name)?, EvalValue::Bool(*value == "true"))
            }
            ["A", name, value] => (
                decode_hex_text(name)?,
                EvalValue::Array(decode_native_array(value)?),
            ),
            _ => {
                return Err(GoblinError::compile(format!(
                    "Invalid native result manifest line {}.",
                    line_index + 1
                )));
            }
        };
        if observed.insert(value.0.clone(), value.1).is_some() {
            return Err(GoblinError::compile(format!(
                "Native result manifest repeats seal {}.",
                value.0
            )));
        }
    }
    if observed.len() != expected.len() {
        return Err(GoblinError::compile(format!(
            "NATIVE PARITY FAILURE\n\nInterpreter expected {} sealed value(s); compiled program reported {}.",
            expected.len(),
            observed.len()
        )));
    }
    for (name, expected_value) in expected {
        let actual = observed.get(name).ok_or_else(|| {
            GoblinError::compile(format!(
                "NATIVE PARITY FAILURE\n\nCompiled program omitted seal {name}."
            ))
        })?;
        let same = same_native_value(expected_value, actual);
        if !same {
            return Err(GoblinError::compile(format!(
                "NATIVE PARITY FAILURE\n\nCompiled seal {name} disagrees with the interpreter preflight."
            )));
        }
    }
    Ok(())
}

fn same_native_value(left: &EvalValue, right: &EvalValue) -> bool {
    match (left, right) {
        (EvalValue::Quantity(a), EvalValue::Quantity(b)) => {
            a.value_si.to_bits() == b.value_si.to_bits() && a.dimension == b.dimension
        }
        (EvalValue::Text(a), EvalValue::Text(b)) => a == b,
        (EvalValue::Bool(a), EvalValue::Bool(b)) => a == b,
        (EvalValue::Array(a), EvalValue::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same_native_value(x, y))
        }
        _ => false,
    }
}

fn decode_native_array(encoded: &str) -> Result<Vec<EvalValue>> {
    let payload = decode_hex_text(encoded)?;
    if payload.is_empty() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    for line in payload.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        let item = match fields.as_slice() {
            ["Q", bits, dimensions] => {
                let bits = u64::from_str_radix(bits, 16)
                    .map_err(|_| GoblinError::compile("Invalid native array quantity bits."))?;
                let parts = dimensions
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|_| GoblinError::compile("Invalid native array dimensions."))?;
                let dimension: [i32; 5] = parts
                    .try_into()
                    .map_err(|_| GoblinError::compile("Native array dimensions need five axes."))?;
                EvalValue::Quantity(crate::quantity::Quantity::new(
                    f64::from_bits(bits),
                    dimension,
                )?)
            }
            ["T", text] => EvalValue::Text(decode_hex_text(text)?),
            ["B", "true"] => EvalValue::Bool(true),
            ["B", "false"] => EvalValue::Bool(false),
            _ => return Err(GoblinError::compile("Invalid native array item.")),
        };
        items.push(item);
        if items.len() > crate::evaluator::MAX_ARRAY_ITEMS {
            return Err(GoblinError::compile("Native array item limit exceeded."));
        }
    }
    Ok(items)
}

fn decode_hex_text(value: &str) -> Result<String> {
    if !value.len().is_multiple_of(2) {
        return Err(GoblinError::compile(
            "Invalid odd-length text in native result manifest.",
        ));
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| GoblinError::compile("Invalid hexadecimal text in native result manifest."))?;
    String::from_utf8(bytes)
        .map_err(|_| GoblinError::compile("Native result manifest contains non-UTF-8 text."))
}

fn unique_run_dir(base: &Path, source_sha: &str) -> Result<PathBuf> {
    let stamp = Utc::now().format("%Y%m%dT%H%M%S%.6fZ").to_string();
    let stem = format!("{stamp}_{}", &source_sha[..12]);
    for suffix in 0..10_000 {
        let name = if suffix == 0 {
            stem.clone()
        } else {
            format!("{stem}_R{suffix}")
        };
        let path = base.join(name);
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(GoblinError::artifact(
        "Unable to allocate a unique run directory.",
    ))
}

fn protocol(classification: &str, detail: &str) -> GoblinError {
    GoblinError::protocol(
        classification,
        format!(
            "{detail}\n\nClassification:\n{classification}\n\nExecution refused.\n\nCreate a revision if this change is intentional.\n\nWHO PAID FOR THIS ASSUMPTION?"
        ),
    )
}
fn freeze_detail(classification: &str) -> &'static str {
    match classification {
        "NOTATION_ONLY_CHANGE_AFTER_FREEZE" => {
            "Frozen source bytes changed. Semantic meaning is unchanged, but exact frozen bytes changed."
        }
        "SEMANTIC_CHANGE_AFTER_FREEZE" => "Frozen source and its canonical program changed.",
        "FREEZE_RECEIPT_TAMPERED" => "The freeze receipt seal is invalid or unreadable.",
        "CONSTANT_REGISTRY_CHANGED_AFTER_FREEZE" => {
            "The pinned scientific constant registry changed."
        }
        "LINEAGE_RECEIPT_TAMPERED" => "The revision lineage receipt is missing or invalid.",
        _ => "Frozen program custody verification failed.",
    }
}
fn lines_to_text(lines: &[String]) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}
fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}
fn sealed_hash(value: &Value, field: &str) -> Result<String> {
    let mut core = value.clone();
    core.as_object_mut().unwrap().remove(field);
    hash_canonical_json(&core)
}
fn write_json_exclusive(path: &Path, value: &Value) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
fn write_private_json_exclusive(path: &Path, value: &Value) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
fn receipt_evidence_name(name: &str) -> String {
    name.to_string()
}

pub fn read_receipt(run_dir: impl AsRef<Path>) -> Result<Value> {
    let value: Value = serde_json::from_slice(&fs::read(run_dir.as_ref().join("receipt.json"))?)?;
    if !value.is_object() {
        return Err(GoblinError::new(
            "G999",
            "Run receipt is not a JSON object.",
        ));
    }
    Ok(value)
}
