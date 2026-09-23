use clap::{Args, Parser, Subcommand};
use goblinpp::ast::Stmt;
use goblinpp::audit::{diff_runs, verify_run};
use goblinpp::compiler;
use goblinpp::custody::{
    create_freeze, create_revision, freeze_path, lineage_chain, verify_freeze, verify_registration,
};
use goblinpp::error::{GoblinError, Result};
use goblinpp::evaluator::Evaluation;
use goblinpp::fits::FitsFile;
use goblinpp::hashing::sha256_bytes;
use goblinpp::interaction::InputPolicy;
use goblinpp::ledger;
use goblinpp::parser::parse_source;
use goblinpp::runtime::{RunOptions, read_receipt, run_file};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Parser)]
#[command(name = "goblin++", version = goblinpp::VERSION, about = "Evidence-first scientific interpreter and native compiler")]
struct Cli {
    #[command(subcommand)]
    command: CommandSet,
}

#[derive(Debug, Subcommand)]
enum CommandSet {
    Run(RunArgs),
    Check {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Compile(CompileArgs),
    Freeze {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Revise {
        parent: PathBuf,
        child: PathBuf,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        json: bool,
    },
    Verify {
        run_dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Diff {
        run_a: PathBuf,
        run_b: PathBuf,
        #[arg(long)]
        json: bool,
    },
    AuditLedger {
        #[arg(default_value = ".")]
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Status {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Lineage {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Doctor {
        #[arg(default_value = ".")]
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Capabilities {
        #[arg(long)]
        json: bool,
    },
    FitsInfo {
        file: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        quick: bool,
    },
}

#[derive(Debug, Args)]
struct RunArgs {
    file: PathBuf,
    #[arg(long)]
    compile: bool,
    #[arg(long = "allow-inline-rust", value_name = "SHA256")]
    allowed_inline_rust: Vec<String>,
    #[arg(long)]
    runs_root: Option<PathBuf>,
    #[arg(long)]
    json: bool,
    #[arg(last = true, allow_hyphen_values = true)]
    program_args: Vec<String>,
}

#[derive(Debug, Args)]
struct CompileArgs {
    file: PathBuf,
    #[arg(short, long)]
    output: PathBuf,
    #[arg(long = "allow-inline-rust", value_name = "SHA256")]
    allowed_inline_rust: Vec<String>,
    #[arg(long)]
    json: bool,
}

fn main() {
    let args = normalized_args();
    let exit = match Cli::try_parse_from(args) {
        Ok(cli) => match dispatch(cli.command) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("{}", error.pretty());
                2
            }
        },
        Err(error) => {
            let _ = error.print();
            error.exit_code()
        }
    };
    std::process::exit(exit);
}

fn normalized_args() -> Vec<String> {
    let mut args = std::env::args().collect::<Vec<_>>();
    let commands = [
        "run",
        "check",
        "compile",
        "freeze",
        "revise",
        "verify",
        "diff",
        "audit-ledger",
        "status",
        "lineage",
        "doctor",
        "capabilities",
        "fits-info",
        "help",
    ];
    if args
        .get(1)
        .is_some_and(|first| !first.starts_with('-') && !commands.contains(&first.as_str()))
    {
        args.insert(1, "run".into());
    }
    args
}

fn dispatch(command: CommandSet) -> Result<i32> {
    match command {
        CommandSet::Run(args) => command_run(args),
        CommandSet::Check { file, json } => command_check(file, json),
        CommandSet::Compile(args) => command_compile(args),
        CommandSet::Freeze { file, json } => command_freeze(file, json),
        CommandSet::Revise {
            parent,
            child,
            reason,
            json,
        } => command_revise(parent, child, &reason, json),
        CommandSet::Verify { run_dir, json } => command_verify(run_dir, json),
        CommandSet::Diff { run_a, run_b, json } => command_diff(run_a, run_b, json),
        CommandSet::AuditLedger { project, json } => command_audit_ledger(project, json),
        CommandSet::Status { file, json } => command_status(file, json),
        CommandSet::Lineage { file, json } => command_lineage(file, json),
        CommandSet::Doctor { project, json } => command_doctor(project, json),
        CommandSet::Capabilities { json } => command_capabilities(json),
        CommandSet::FitsInfo { file, json, quick } => command_fits_info(file, json, quick),
    }
}

fn command_run(args: RunArgs) -> Result<i32> {
    validate_hashes(&args.allowed_inline_rust)?;
    let run_dir = run_file(
        &args.file,
        &RunOptions {
            runs_root: args.runs_root,
            compile: args.compile,
            allowed_inline_rust: args.allowed_inline_rust,
            program_args: args.program_args,
            input_policy: InputPolicy::Interactive,
        },
    )?;
    let receipt = read_receipt(&run_dir)?;
    if args.json {
        emit_json(&json!({"run_dir": run_dir, "receipt": receipt}));
    } else {
        if receipt.get("paranoid_mode").and_then(Value::as_bool) == Some(true) {
            println!(
                "GOBLIN++ PARANOID MODE\nTRUST NOTHING. HASH EVERYTHING. OVERWRITE NOTHING.\n"
            );
        }
        if receipt.pointer("/freeze/status").and_then(Value::as_str) != Some("NOT_FROZEN") {
            println!(
                "FREEZE_RECEIPT={}",
                receipt
                    .pointer("/freeze/status")
                    .and_then(Value::as_str)
                    .unwrap_or("FAIL")
            );
            println!(
                "FREEZE_CLASSIFICATION={}\n",
                receipt
                    .pointer("/freeze/classification")
                    .and_then(Value::as_str)
                    .unwrap_or("UNKNOWN")
            );
        }
        let stdout = fs::read_to_string(run_dir.join("stdout.log"))?;
        if !stdout.is_empty() {
            print!("{stdout}");
        }
        println!(
            "RUN_STATUS={}",
            receipt["status"].as_str().unwrap_or("MACHINERY_FAIL")
        );
        if let Some(status) = receipt
            .pointer("/paranoid_postflight/status")
            .and_then(Value::as_str)
        {
            println!("PARANOID_POSTFLIGHT={status}");
        }
        println!(
            "EXECUTION_ENGINE={}",
            receipt
                .pointer("/execution/engine")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        );
        println!(
            "SOURCE_SHA256={}",
            receipt
                .pointer("/source/sha256")
                .and_then(Value::as_str)
                .unwrap_or("")
        );
        if let Some(hash) = receipt
            .pointer("/canonical_source/sha256")
            .and_then(Value::as_str)
        {
            println!("CANONICAL_SOURCE_SHA256={hash}");
        }
        println!(
            "RECEIPT_CORE_SHA256={}",
            receipt["receipt_core_sha256"].as_str().unwrap_or("")
        );
        println!("RUN_DIR={}", run_dir.display());
        if let Some(artifacts) = receipt.get("generated_artifacts").and_then(Value::as_array) {
            for artifact in artifacts {
                println!(
                    "GENERATED_ARTIFACT={} SHA256={} BYTES={} MEDIA_TYPE={}",
                    artifact.get("path").and_then(Value::as_str).unwrap_or(""),
                    artifact.get("sha256").and_then(Value::as_str).unwrap_or(""),
                    artifact
                        .get("byte_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    artifact
                        .get("media_type")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                );
            }
        }
        let root = ledger::project_root_for(&args.file)?;
        let report = ledger::audit(root);
        if report.verified
            && let Some(head) = report.head_event_sha256
        {
            println!("LEDGER_HEAD_SHA256={head}");
        }
        let stderr = fs::read_to_string(run_dir.join("stderr.log"))?;
        if !stderr.is_empty() {
            eprint!("{stderr}");
        }
    }
    Ok(if receipt["status"] == "PASS" { 0 } else { 1 })
}

fn command_check(file: PathBuf, json_output: bool) -> Result<i32> {
    if file.is_symlink() {
        return Err(GoblinError::new(
            "G000",
            "Refusing to check a symbolic-link source.",
        ));
    }
    let bytes = fs::read(&file)?;
    let source_sha = sha256_bytes(&bytes);
    let text =
        std::str::from_utf8(&bytes).map_err(|_| GoblinError::lex("Source is not valid UTF-8."))?;
    let parsed = parse_source(text)?;
    parsed.require_executable_program()?;
    let mut evaluation = Evaluation::new(
        file.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new(".")),
    );
    evaluation.configure_interaction(
        vec![file.to_string_lossy().to_string()],
        InputPolicy::Disabled,
    )?;
    let mut error = None;
    evaluation.register_functions(&parsed.program)?;
    for statement in &parsed.program.statements {
        if matches!(statement, Stmt::InlineRust { .. }) {
            continue;
        }
        if let Err(found) = evaluation.eval_stmt(statement) {
            error = Some(found);
            break;
        }
    }
    let report = json!({
        "schema": "goblin.check.v1", "goblin_version": goblinpp::VERSION,
        "status": if error.is_none() { "PASS" } else { "FAIL" }, "authority": "PREVIEW_ONLY_NOT_EVIDENCE",
        "evidence_created": false, "custody_checked": false,
        "source": {"path": file, "sha256": source_sha}, "canonical_source_sha256": parsed.canonical_sha256()?,
        "paranoid_mode": parsed.paranoid(), "inline_rust": parsed.inline_rust.iter().map(|block| json!({"sha256": block.sha256, "requires_compile": true})).collect::<Vec<_>>(),
        "stdout_preview": evaluation.stdout, "sealed_preview": evaluation.sealed.iter().map(|(name, value)| (name.clone(), value.to_json())).collect::<serde_json::Map<_,_>>(),
        "generated_output_preview": evaluation.generated.values().map(|artifact| json!({"name": artifact.name, "media_type": artifact.media_type, "byte_count": artifact.bytes.len(), "producer": artifact.producer, "metadata": artifact.metadata})).collect::<Vec<_>>(),
        "diagnostic": error.as_ref().map(|value| json!({"code": value.code, "category": value.category, "message": value.message, "classification": value.classification})),
    });
    if json_output {
        emit_json(&report);
    } else {
        println!(
            "GOBLIN CHECK\n\nSOURCE={}\nSOURCE_SHA256={}\nCANONICAL_SOURCE_SHA256={}\nINLINE_RUST_BLOCKS={}\nGENERATED_OUTPUTS={}\nAUTHORITY=PREVIEW_ONLY_NOT_EVIDENCE\nEVIDENCE_CREATED=NO\nCUSTODY_CHECKED=NO\nCHECK_STATUS={}",
            file.display(),
            source_sha,
            parsed.canonical_sha256()?,
            parsed.inline_rust.len(),
            evaluation.generated.len(),
            report["status"].as_str().unwrap()
        );
        for block in &parsed.inline_rust {
            println!("INLINE_RUST_{}_SHA256={}", block.index + 1, block.sha256);
        }
        if let Some(error) = error {
            eprintln!("{}", error.pretty());
        }
    }
    Ok(if report["status"] == "PASS" { 0 } else { 1 })
}

fn command_compile(args: CompileArgs) -> Result<i32> {
    validate_hashes(&args.allowed_inline_rust)?;
    let bytes = fs::read(&args.file)?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| GoblinError::lex("Source is not valid UTF-8."))?;
    let parsed = parse_source(text)?;
    parsed.require_executable_program()?;
    if freeze_path(&args.file).exists() {
        let freeze = verify_freeze(&args.file);
        if !freeze.verified {
            return Err(GoblinError::protocol(
                freeze.classification.clone(),
                format!(
                    "Frozen-source verification failed before compilation.\n\nClassification:\n{}\n\nCompilation refused.",
                    freeze.classification
                ),
            ));
        }
        if !verify_registration(&args.file, &freeze)? {
            return Err(GoblinError::protocol(
                "UNREGISTERED_FREEZE_RECEIPT",
                "The source freeze is not registered in a clean custody ledger. Compilation refused.",
            ));
        }
    }
    let compilation = compiler::compile(&parsed, &args.output, &args.allowed_inline_rust)?;
    let report = json!({ "status": "PASS", "source_sha256": sha256_bytes(&bytes), "canonical_source_sha256": parsed.canonical_sha256()?, "binary": compilation.binary, "binary_sha256": compilation.binary_sha256, "generated_source": compilation.generated_source, "generated_source_sha256": compilation.generated_source_sha256, "rustc": compilation.rustc_version, "inline_rust": parsed.inline_rust.iter().map(|block| &block.sha256).collect::<Vec<_>>() });
    if args.json {
        emit_json(&report);
    } else {
        println!(
            "GOBLIN NATIVE COMPILATION\n\nSOURCE_SHA256={}\nCANONICAL_SOURCE_SHA256={}\nBINARY={}\nBINARY_SHA256={}\nGENERATED_RUST={}\nGENERATED_RUST_SHA256={}\nRUSTC={}\nCOMPILE_STATUS=PASS",
            report["source_sha256"].as_str().unwrap(),
            report["canonical_source_sha256"].as_str().unwrap(),
            compilation.binary.display(),
            compilation.binary_sha256,
            compilation.generated_source.display(),
            compilation.generated_source_sha256,
            compilation.rustc_version
        );
    }
    Ok(0)
}

fn command_freeze(file: PathBuf, json_output: bool) -> Result<i32> {
    let (path, event) = create_freeze(&file)?;
    let receipt: Value = serde_json::from_slice(&fs::read(&path)?)?;
    if json_output {
        emit_json(&json!({"freeze_receipt": path, "receipt": receipt, "ledger_event": event}));
    } else {
        println!(
            "GOBLIN FREEZE\n\nSOURCE_SHA256={}\nCANONICAL_SOURCE_SHA256={}\nCONSTANT_REGISTRY_SHA256={}\nPROGRAM=FROZEN\nFREEZE_RECEIPT_SHA256={}\nFREEZE_RECEIPT={}\nLEDGER_HEAD_SHA256={}\n\nTHE GOBLIN WILL REMEMBER.",
            receipt
                .pointer("/source/sha256")
                .and_then(Value::as_str)
                .unwrap(),
            receipt
                .pointer("/canonical_source/sha256")
                .and_then(Value::as_str)
                .unwrap(),
            receipt
                .pointer("/constant_registry/sha256")
                .and_then(Value::as_str)
                .unwrap(),
            receipt["freeze_receipt_sha256"].as_str().unwrap(),
            path.display(),
            event.event_sha256
        );
    }
    Ok(0)
}

fn command_revise(parent: PathBuf, child: PathBuf, reason: &str, json_output: bool) -> Result<i32> {
    let (child, lineage, event) = create_revision(&parent, &child, reason)?;
    let receipt: Value = serde_json::from_slice(&fs::read(&lineage)?)?;
    if json_output {
        emit_json(
            &json!({"child": child, "lineage_receipt": lineage, "lineage": receipt, "ledger_event": event}),
        );
    } else {
        println!(
            "GOBLIN REVISION\n\nPARENT={}\nPARENT_SOURCE_SHA256={}\nPARENT_FREEZE_RECEIPT_SHA256={}\nCHILD={}\nREASON={}\nLINEAGE_RECEIPT_SHA256={}\nLINEAGE_RECEIPT={}\nLEDGER_HEAD_SHA256={}",
            parent.display(),
            receipt
                .pointer("/parent/source_sha256")
                .and_then(Value::as_str)
                .unwrap(),
            receipt
                .pointer("/parent/freeze_receipt/core_sha256")
                .and_then(Value::as_str)
                .unwrap(),
            child.display(),
            reason.trim(),
            receipt["lineage_receipt_sha256"].as_str().unwrap(),
            lineage.display(),
            event.event_sha256
        );
    }
    Ok(0)
}

fn command_verify(run_dir: PathBuf, json_output: bool) -> Result<i32> {
    let report = verify_run(&run_dir)?;
    if json_output {
        emit_json(&serde_json::to_value(&report)?);
    } else {
        println!("GOBLIN VERIFY\n");
        for item in &report.checks {
            println!(
                "{} {} {}",
                item.check,
                ".".repeat(usize::max(2, 31usize.saturating_sub(item.check.len()))),
                if item.pass { "PASS" } else { "FAIL" }
            );
        }
        println!("\nVERIFICATION_STATUS={}", report.status);
        if report.verified {
            println!("THE GOBLIN IS SATISFIED.");
        } else {
            println!("EVIDENCE CHANGED OR IS INCOMPLETE.");
        }
    }
    Ok(if report.verified { 0 } else { 1 })
}

fn command_diff(run_a: PathBuf, run_b: PathBuf, json_output: bool) -> Result<i32> {
    let report = diff_runs(run_a, run_b)?;
    if json_output {
        emit_json(&serde_json::to_value(&report)?);
    } else {
        println!(
            "GOBLIN DIFF\n\nSOURCE_BYTES_SAME ........... {}\nCANONICAL_PROGRAM_SAME ...... {}\nSEALED_ARTIFACTS_SAME ....... {}\nGENERATED_ARTIFACTS_SAME .... {}\nCONSTANTS_SAME .............. {}\nDATA_IMPORTS_SAME ........... {}\nINTERACTION_SAME ............ {}\nSTDOUT_SAME ................. {}\nSTDERR_SAME ................. {}\nSTATUS_SAME ................. {}\nENVIRONMENT_SAME ............ {}\nPARANOID_MODE_SAME .......... {}\nEXECUTION_ENGINE_SAME ....... {}\n\nCLASSIFICATION:\n{}",
            yn(report.source_bytes_same),
            yn(report.canonical_program_same),
            yn(report.sealed_artifacts_same),
            yn(report.generated_artifacts_same),
            yn(report.constants_same),
            yn(report.data_imports_same),
            yn(report.interaction_same),
            yn(report.stdout_same),
            yn(report.stderr_same),
            yn(report.status_same),
            yn(report.environment_same),
            yn(report.paranoid_mode_same),
            yn(report.execution_engine_same),
            report.classification
        );
    }
    Ok(0)
}

fn command_audit_ledger(project: PathBuf, json_output: bool) -> Result<i32> {
    let report = ledger::audit(&project);
    if json_output {
        emit_json(&serde_json::to_value(&report)?);
    } else {
        println!(
            "GOBLIN CUSTODY LEDGER AUDIT\n\nLEDGER={}\nEVENTS={}\nHEAD_EVENT_SHA256={}\nAUTHENTICATION={}\n\nLEDGER_STATUS={}",
            report.ledger_path,
            report.events.len(),
            report.head_event_sha256.as_deref().unwrap_or("NONE"),
            report.authentication,
            report.status
        );
        if report.verified {
            println!("HASH CHAIN VERIFIED; AUTHORSHIP NOT AUTHENTICATED.");
        }
    }
    Ok(if report.verified { 0 } else { 1 })
}

fn command_status(file: PathBuf, json_output: bool) -> Result<i32> {
    let root = ledger::project_root_for(&file)?;
    let ledger_report = ledger::audit(&root);
    let freeze = verify_freeze(&file);
    let freeze_registered = freeze.verified && verify_registration(&file, &freeze)?;
    let lineage_exists = goblinpp::custody::lineage_path(&file).exists();
    let state = if freeze_path(&file).exists() {
        if freeze.verified && freeze_registered {
            "FROZEN_VERIFIED"
        } else if freeze.verified {
            "FROZEN_UNREGISTERED"
        } else {
            "FROZEN_INVALID"
        }
    } else if lineage_exists {
        "REVISION_UNFROZEN"
    } else {
        "UNTRACKED"
    };
    let subject = ledger::subject_for(&file, &root)?;
    let events = ledger_report
        .events
        .iter()
        .filter(|event| event.subject == subject)
        .count();
    let report = json!({"source": file, "state": state, "freeze": freeze, "freeze_registered": freeze_registered, "ledger": ledger_report, "ledger_events_for_source": events});
    if json_output {
        emit_json(&report);
    } else {
        println!(
            "GOBLIN STATUS\n\nSOURCE={}\nSTATE={}\nLEDGER_STATUS={}\nLEDGER_EVENTS_FOR_SOURCE={}\nLEDGER_HEAD_SHA256={}\nAUTHENTICATION={}",
            file.display(),
            state,
            ledger_report.status,
            events,
            ledger_report.head_event_sha256.as_deref().unwrap_or("NONE"),
            ledger_report.authentication
        );
    }
    Ok(
        if matches!(state, "FROZEN_INVALID" | "FROZEN_UNREGISTERED") || !ledger_report.verified {
            1
        } else {
            0
        },
    )
}

fn command_lineage(file: PathBuf, json_output: bool) -> Result<i32> {
    let chain = lineage_chain(file)?;
    if json_output {
        emit_json(&json!({"chain": chain}));
    } else {
        println!("GOBLIN LINEAGE\n");
        for (index, node) in chain.iter().enumerate() {
            let prefix = if index == 0 {
                String::new()
            } else {
                format!("{}└── ", "  ".repeat(index - 1))
            };
            println!(
                "{}{} [{}]",
                prefix,
                node["path"].as_str().unwrap(),
                node["state"].as_str().unwrap()
            );
            if let Some(reason) = node.get("reason").and_then(Value::as_str) {
                println!("{}reason: {reason}", "  ".repeat(index));
            }
        }
    }
    Ok(0)
}

fn command_doctor(project: PathBuf, json_output: bool) -> Result<i32> {
    let rustc = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|value| value.status.success())
        .map(|value| String::from_utf8_lossy(&value.stdout).trim().to_string());
    let ledger = ledger::audit(&project);
    let writable = fs::metadata(&project)
        .map(|meta| !meta.permissions().readonly())
        .unwrap_or(false);
    let report = json!({ "schema": "goblin.doctor.v1", "project": project, "checks": [
        {"check":"RUSTC_AVAILABLE","pass":rustc.is_some(),"detail":rustc}, {"check":"PROJECT_READABLE","pass":project.is_dir()}, {"check":"PROJECT_WRITABLE","pass":writable},
        {"check":"CUSTODY_LEDGER","pass":ledger.verified,"detail":ledger.status}, {"check":"INLINE_RUST_DEFAULT_DENY","pass":true}, {"check":"FITS_NATIVE_RUST","pass":true}],
        "status": if rustc.is_some() && project.is_dir() && writable && ledger.verified {"PASS"} else {"WARN"} });
    if json_output {
        emit_json(&report);
    } else {
        println!("GOBLIN DOCTOR\n\nPROJECT={}", project.display());
        for item in report["checks"].as_array().unwrap() {
            println!(
                "{} {} {}",
                item["check"].as_str().unwrap(),
                ".".repeat(usize::max(
                    2,
                    31usize.saturating_sub(item["check"].as_str().unwrap().len())
                )),
                if item["pass"] == true { "PASS" } else { "WARN" }
            );
        }
        println!("\nDOCTOR_STATUS={}", report["status"].as_str().unwrap());
    }
    Ok(if report["status"] == "PASS" { 0 } else { 1 })
}

fn command_capabilities(json_output: bool) -> Result<i32> {
    let report = json!({
        "goblin_version": goblinpp::VERSION, "release_status": "ALPHA",
        "available": ["rust interpreter", "optional GO_PARANOID postflight evidence and self-verification", "optional seal value snapshots", "bounded range and direct-array for loops, while loops, break/continue, if/else-if/else, switch/case/default, and short-circuit Boolean logic", "g_func user-defined functions with local copy-value arguments and explicit return in both engines", "interactive input and argc/argv program arguments with hashed evidence", "text concatenation, Unicode-scalar length, checked number/integer conversion and g_strings built-ins in both engines", "copy-value arrays with indexing, half-open slices, append and len in both engines", "checked integer remainder in both engines", "native scalar and array compiler", "exact-hash inline Rust authorization", "native streaming FITS multi-HDU discovery", "native FITS image access", "native FITS binary-table scalar access and numeric statistics", "checksum-addressed deduplicated FITS evidence", "audited TXT, Markdown, CSV, TSV, and JSON output in either mode", "deterministic SVG and PNG FITS plots", "run receipts", "freeze enforcement", "revision lineage", "run verification", "semantic diff", "checksum custody ledger"],
        "string_functions": ["to_text", "parse_number", "parse_integer", "str_trim", "str_contains", "str_replace", "str_split", "str_join", "len"],
        "fits_functions": ["fits_hdu_count", "fits_header", "fits_axis", "fits_count", "fits_pixel", "fits_mean", "fits_rows", "fits_columns", "fits_column", "fits_column_valid_count", "fits_column_mean", "fits_column_min", "fits_column_max"],
        "output_functions": ["write_text", "write_csv", "write_tsv", "write_json", "plot_fits_histogram", "plot_fits_scatter"],
        "pending_from_python_reference": ["Ed25519 ledger signing", "interrupted-head recovery", "adopt legacy freeze", "normative corpus commands"],
        "known_limits": ["GO_PARANOID observes source bytes at postflight, not continuously, and is not an OS sandbox", "input and argument text is stored in run evidence; do not enter secrets", "check will not consume stdin and reports input required", "input and argv return text; parse_number converts finite unitless decimals and parse_integer accepts safe decimal integers only", "numbers remain f64 quantities, not a distinct exact integer type; integer parsing and remainder are limited to exactly representable values", "len(text) counts Unicode scalar values, not grapheme clusters; text operations do not normalize Unicode", "arrays are one-dimensional, homogeneous, and limited to 100000 items; slices, assignments, and direct iteration use independent values rather than shared Go-style backing storage", "loops share a 1000000-iteration ceiling", "g_func requires a return on the reached path, has no implicit global capture, and is limited to 16 active calls", "compiled FITS and output calls are refused, including inside functions", "switch uses exact equality, first match, and no fallthrough", "plots use explicitly labelled deterministic row samples rather than silently implying full-population rendering", "JPEG output is omitted because it is lossy", "inline Rust cannot mutate sealed Goblin variables", "ASCII tables are discovered but not read", "compressed images, random groups, bit, complex, and variable-array columns are not read", "FITS WCS and declared units are reported but not interpreted"]
    });
    if json_output {
        emit_json(&report);
    } else {
        println!(
            "GOBLIN++ CAPABILITIES {}\n\nSTATUS=ALPHA",
            goblinpp::VERSION
        );
        for item in report["available"].as_array().unwrap() {
            println!("PASS {}", item.as_str().unwrap());
        }
        println!("\nFITS FUNCTIONS");
        for item in report["fits_functions"].as_array().unwrap() {
            println!("- {}", item.as_str().unwrap());
        }
        println!("\nOUTPUT FUNCTIONS");
        for item in report["output_functions"].as_array().unwrap() {
            println!("- {}", item.as_str().unwrap());
        }
        println!("\nPENDING");
        for item in report["pending_from_python_reference"].as_array().unwrap() {
            println!("- {}", item.as_str().unwrap());
        }
    }
    Ok(0)
}

fn command_fits_info(file: PathBuf, json_output: bool, quick: bool) -> Result<i32> {
    let fits = FitsFile::inspect(&file, !quick)?;
    let report = fits.inspection();
    if json_output {
        emit_json(&report);
        return Ok(0);
    }
    println!(
        "GOBLIN FITS INFO\n\nFILE={}\nSHA256={}\nBYTES={}\nHDUS={}",
        file.display(),
        if fits.sha256.is_empty() {
            "NOT_COMPUTED (--quick)"
        } else {
            &fits.sha256
        },
        fits.byte_count,
        fits.hdu_count()
    );
    for hdu in &fits.hdus {
        println!("\nHDU={}\nTYPE={}", hdu.index, hdu.kind.label());
        if let Some(name) = &hdu.extension_name {
            println!("EXTNAME={name}");
        }
        println!(
            "AXES={}\nDATA_BYTES={}\nHEADER_CARDS={}",
            if hdu.axes.is_empty() {
                "NONE".into()
            } else {
                hdu.axes
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join("x")
            },
            hdu.data_length,
            hdu.cards.len().saturating_sub(1)
        );
        if let Some(rows) = hdu.rows {
            println!("ROWS={rows}");
        }
        if let Some(row_bytes) = hdu.row_bytes {
            println!("ROW_BYTES={row_bytes}");
        }
        if hdu.is_compressed_image() {
            println!("COMPRESSED_IMAGE=YES");
        }
        println!("HEADER");
        for card in hdu.cards.iter().filter(|card| card.keyword != "END") {
            let value = card
                .value
                .as_ref()
                .map(|value| format!(" = {}", value.render()))
                .unwrap_or_default();
            let comment = card
                .comment
                .as_ref()
                .map(|comment| format!(" / {comment}"))
                .unwrap_or_default();
            println!("  {}{}{}", card.keyword, value, comment);
        }
        let columns = hdu.columns();
        if !columns.is_empty() {
            println!("COLUMNS={}", columns.len());
            for column in columns {
                println!(
                    "  {} {} TFORM={} TYPE={} REPEAT={} UNIT={} READABLE={}",
                    column.index,
                    column.name,
                    column.format,
                    column.element_type,
                    column.repeat,
                    column.unit.as_deref().unwrap_or("NONE"),
                    if column.readable { "YES" } else { "NO" }
                );
            }
        }
    }
    println!(
        "\nAUTHORITY={}",
        if quick {
            "QUICK_INSPECTION_UNHASHED_NOT_EVIDENCE"
        } else {
            "INSPECTION_ONLY_NOT_RUN_EVIDENCE"
        }
    );
    Ok(0)
}

fn validate_hashes(values: &[String]) -> Result<()> {
    for value in values {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(GoblinError::protocol(
                "INVALID_INLINE_RUST_AUTHORIZATION",
                format!(
                    "Inline Rust authorization must be a full 64-character SHA-256 digest, got {value:?}."
                ),
            ));
        }
    }
    Ok(())
}
fn emit_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("serializable CLI report")
    );
}
fn yn(value: bool) -> &'static str {
    if value { "YES" } else { "NO" }
}
