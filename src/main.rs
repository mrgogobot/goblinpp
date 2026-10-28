#![recursion_limit = "256"]
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
    /// Explicit bounded public HTTPS acquisition; never decode or execute the payload.
    #[cfg(unix)]
    Fetch(goblinpp::acquisition_cli::FetchArgs),
    /// Verify an acquisition receipt and its payload offline, including preserved failures.
    #[cfg(unix)]
    VerifyFetch {
        directory: PathBuf,
        #[arg(long)]
        expected_receipt_sha256: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Read-only, explicitly scoped integrity checking against a pinned baseline.
    Orbit(goblinpp::orbit_cli::OrbitArgs),
    /// Offline reports from declared findings and hash-pinned evidence copies.
    EvidenceReport {
        case_file: PathBuf,
        /// New directory; its parent must already exist. Never overwritten.
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Verify report copies, exact renderings, and optional trusted receipt hash.
    VerifyReport {
        directory: PathBuf,
        #[arg(long)]
        expected_receipt_sha256: Option<String>,
        #[arg(long)]
        json: bool,
    },
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
    /// Compare one sealed scalar; exact verification is mandatory and unchanged.
    CompareValue {
        run_a: PathBuf,
        run_b: PathBuf,
        name: String,
        /// Absolute tolerance in the sealed quantity's SI units (required).
        #[arg(long, allow_hyphen_values = true)]
        absolute_tolerance_si: f64,
        /// Dimensionless relative tolerance (required; no implicit defaults).
        #[arg(long, allow_hyphen_values = true)]
        relative_tolerance: f64,
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
    // Clap can echo unexpected argument values. Fetch errors must not repeat a URL/query.
    let redact_fetch_arguments = args.get(1).is_some_and(|arg| arg == "fetch");
    let exit = match Cli::try_parse_from(args) {
        Ok(cli) => match dispatch(cli.command) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("{}", error.pretty());
                2
            }
        },
        Err(error) => {
            if redact_fetch_arguments && error.exit_code() != 0 {
                eprintln!(
                    "GOBLIN ERROR G901\n\nInvalid fetch arguments. Use 'goblin++ fetch --help'; provide --allow-host, --output, and exactly one of --sha256/--record-only. Supplied values are omitted."
                );
            } else {
                let _ = error.print();
            }
            error.exit_code()
        }
    };
    std::process::exit(exit);
}

fn normalized_args() -> Vec<String> {
    let mut args = std::env::args().collect::<Vec<_>>();
    let commands = [
        "fetch",
        "verify-fetch",
        "run",
        "check",
        "compile",
        "freeze",
        "revise",
        "verify",
        "diff",
        "compare-value",
        "audit-ledger",
        "status",
        "lineage",
        "doctor",
        "capabilities",
        "fits-info",
        "evidence-report",
        "verify-report",
        "orbit",
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
        #[cfg(unix)]
        CommandSet::Fetch(args) => goblinpp::acquisition_cli::dispatch(args),
        #[cfg(unix)]
        CommandSet::VerifyFetch {
            directory,
            expected_receipt_sha256,
            json,
        } => {
            goblinpp::acquisition_cli::verify(&directory, expected_receipt_sha256.as_deref(), json)
        }
        CommandSet::Orbit(args) => goblinpp::orbit_cli::dispatch(args),
        CommandSet::EvidenceReport {
            case_file,
            output,
            json,
        } => {
            let receipt = goblinpp::evidence_report::create_report(&case_file, &output)?;
            if json {
                emit_json(&serde_json::to_value(&receipt)?);
            } else {
                println!(
                    "GOBLIN OFFLINE EVIDENCE REPORT\n\nREPORT_DIRECTORY={}\nREPORT_STATUS=PASS\nRECEIPT_CORE_SHA256={}\nAUTHENTICATION=CHECKSUM_ONLY\n\nEvidence copies and report bytes verified; findings and permission are analyst declarations. Review plaintext evidence before sharing.",
                    output.display(),
                    receipt.receipt_core_sha256
                );
            }
            Ok(0)
        }
        CommandSet::VerifyReport {
            directory,
            expected_receipt_sha256,
            json,
        } => {
            let receipt = goblinpp::evidence_report::verify_report(
                &directory,
                expected_receipt_sha256.as_deref(),
            )?;
            if json {
                emit_json(
                    &serde_json::json!({"status":"PASS", "verified":true, "authentication":"CHECKSUM_ONLY", "trusted_receipt_matched":expected_receipt_sha256.is_some(), "receipt_core_sha256":receipt.receipt_core_sha256}),
                );
            } else {
                println!(
                    "GOBLIN REPORT VERIFICATION\n\nVERIFICATION_STATUS=PASS\nRECEIPT_CORE_SHA256={}\nTRUSTED_RECEIPT_MATCH={}\nAUTHENTICATION=CHECKSUM_ONLY\n\nEvidence copies and independently reconstructed report bytes match. Findings are not certified.",
                    receipt.receipt_core_sha256,
                    if expected_receipt_sha256.is_some() {
                        "PASS"
                    } else {
                        "NOT_REQUESTED"
                    }
                );
            }
            Ok(0)
        }
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
        CommandSet::CompareValue {
            run_a,
            run_b,
            name,
            absolute_tolerance_si,
            relative_tolerance,
            json,
        } => {
            let report = goblinpp::comparison::compare_sealed_quantities(
                run_a,
                run_b,
                &name,
                absolute_tolerance_si,
                relative_tolerance,
            )?;
            if json {
                emit_json(&report);
            } else {
                println!(
                    "GOBLIN NUMERIC COMPARISON\n\nVALUE={}\nEVIDENCE_VERIFICATION=EXACT_SHA256_PASS\nABSOLUTE_TOLERANCE_SI={}\nRELATIVE_TOLERANCE={}\nSI_UNIT={}\nBITS_EQUAL={}\nCLASSIFICATION={}\n\nNamed scalar only; this does not prove scientific validity or equivalence of entire runs.",
                    name,
                    report["absolute_tolerance_si"],
                    report["relative_tolerance"],
                    report["unit_si"].as_str().unwrap(),
                    yn(report["bits_equal"].as_bool().unwrap()),
                    report["classification"].as_str().unwrap()
                );
            }
            Ok(if report["within_declared_tolerance"] == true {
                0
            } else {
                1
            })
        }
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
        let completely_mad = receipt["execution_profile"] == "COMPLETELY_MAD";
        if completely_mad {
            println!(
                "GOBLIN++ COMPLETELY_MAD MODE\nTRUST NOTHING. HASH EVERYTHING. OVERWRITE NOTHING.\n"
            );
            println!("EXECUTION_PROFILE=COMPLETELY_MAD");
        } else if receipt.get("paranoid_mode").and_then(Value::as_bool) == Some(true) {
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
        if receipt["numeric_audit"]["requested"] == true {
            println!(
                "NUMERIC_AUDIT_STATUS={}",
                receipt["numeric_audit"]["status"]
                    .as_str()
                    .unwrap_or("ERROR")
            );
        }
        if receipt["final_state"]["requested"] == true {
            println!(
                "FINAL_STATE_CAPTURE={}",
                receipt["final_state"]["capture_status"]
                    .as_str()
                    .unwrap_or("ERROR")
            );
            if let Some(completeness) = receipt["final_state"]["completeness"].as_str() {
                println!("FINAL_STATE_COMPLETENESS={completeness}");
            }
        }
        if let Some(status) = receipt
            .pointer("/paranoid_postflight/status")
            .and_then(Value::as_str)
        {
            if completely_mad {
                println!("COMPLETELY_MAD_POSTFLIGHT={status}");
            } else {
                println!("PARANOID_POSTFLIGHT={status}");
            }
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
    let parsed = goblinpp::modules::resolve(&file, text)?.parsed;
    parsed.require_executable_program()?;
    let numeric_audit_preview = goblinpp::numeric_audit::snapshot(&file, &parsed.program)?;
    let mut evaluation = Evaluation::new(
        file.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new(".")),
    );
    evaluation.configure_interaction(
        vec![file.to_string_lossy().to_string()],
        InputPolicy::Disabled,
    )?;
    let mut error = evaluation.register_functions(&parsed.program).err();
    for statement in &parsed.program.statements {
        if error.is_some() {
            break;
        }
        if matches!(statement, Stmt::InlineRust { .. }) {
            continue;
        }
        if let Err(found) = evaluation.eval_stmt(statement) {
            error = Some(found);
            break;
        }
    }
    if error.is_none() {
        error = evaluation.batches.finish().err();
    }
    let final_state_preview = if evaluation.completely_mad {
        match goblinpp::mad::values(&evaluation) {
            Ok(values) => {
                json!({"policy": goblinpp::mad::POLICY, "authority":"PREVIEW_ONLY_NOT_EVIDENCE", "completeness":if error.is_none() {"COMPLETE"} else {"PARTIAL"}, "values":values.iter().map(|(n,v)| (n.clone(),v.to_json())).collect::<serde_json::Map<_,_>>()})
            }
            Err(found) => {
                error = Some(found);
                json!({"capture_status":"REFUSED"})
            }
        }
    } else {
        Value::Null
    };
    let mut output_preview=evaluation.generated.values().map(|artifact| json!({"name":artifact.name,"media_type":artifact.media_type,"byte_count":artifact.bytes.len(),"producer":artifact.producer,"metadata":artifact.metadata})).collect::<Vec<_>>();
    match evaluation.batches.descriptors(error.is_none()) {
        Ok(outputs) => output_preview.extend(outputs),
        Err(found) => error = Some(found),
    }
    let report = json!({
        "schema": "goblin.check.v1", "goblin_version": goblinpp::VERSION,
        "language_semantics": goblinpp::text_runtime::SEMANTICS_POLICY,
        "parser_policy": goblinpp::parser::PARSER_POLICY,
        "math_policy": goblinpp::math_policy::policy(),
        "array_math_policy": goblinpp::array_math::policy(), "table_join_policy": goblinpp::table_join::POLICY,
        "rng_policy": goblinpp::random::policy(),
        "rng_preview": evaluation.randomness.evidence(),
        "inference_policy": goblinpp::inference_policy::policy(),
        "inference_preview": evaluation.inference.evidence(),
        "resource_policy": goblinpp::resources::policy(),
        "resources_preview": evaluation.resource_evidence(),
        "batch_policy": goblinpp::batches::policy(),
        "fits_batch_policy": goblinpp::fits::batch::policy(),
        "clock_policy": goblinpp::clock::policy(),
        "plot_policy": goblinpp::plots::POLICY,
        "clock_preview": evaluation.clock.evidence(),
        "batches_preview": evaluation.batches.evidence(),
        "status": if error.is_none() { "PASS" } else { "FAIL" }, "authority": "PREVIEW_ONLY_NOT_EVIDENCE",
        "evidence_created": false, "custody_checked": false,
        "numeric_audit_preview": {"configuration":numeric_audit_preview,"status":"NOT_PERFORMED_PREVIEW_ONLY"},
        "final_state_preview": final_state_preview,
        "source": {"path": file, "sha256": source_sha}, "canonical_source_sha256": parsed.canonical_sha256()?,
        "execution_profile": parsed.execution_profile(),
        "paranoid_mode": parsed.paranoid(), "inline_rust": parsed.inline_rust.iter().map(|block| json!({"sha256": block.sha256, "requires_compile": true})).collect::<Vec<_>>(),
        "stdout_preview": evaluation.stdout, "sealed_preview": evaluation.sealed.iter().map(|(name, value)| (name.clone(), value.to_json())).collect::<serde_json::Map<_,_>>(),
        "generated_output_preview": output_preview,
        "warnings": evaluation.warnings,
        "diagnostic": error.as_ref().map(|value| json!({"code": value.code, "category": value.category, "message": value.message, "classification": value.classification})),
    });
    if json_output {
        emit_json(&report);
    } else {
        if parsed.execution_profile() == "COMPLETELY_MAD" {
            println!("EXECUTION_PROFILE=COMPLETELY_MAD");
        }
        println!(
            "GOBLIN CHECK\n\nSOURCE={}\nSOURCE_SHA256={}\nCANONICAL_SOURCE_SHA256={}\nINLINE_RUST_BLOCKS={}\nGENERATED_OUTPUTS={}\nAUTHORITY=PREVIEW_ONLY_NOT_EVIDENCE\nEVIDENCE_CREATED=NO\nCUSTODY_CHECKED=NO\nCHECK_STATUS={}",
            file.display(),
            source_sha,
            parsed.canonical_sha256()?,
            parsed.inline_rust.len(),
            output_preview.len(),
            report["status"].as_str().unwrap()
        );
        for block in &parsed.inline_rust {
            println!("INLINE_RUST_{}_SHA256={}", block.index + 1, block.sha256);
        }
        if goblinpp::numeric_audit::requested(&parsed.program).is_some() {
            println!("NUMERIC_AUDIT_STATUS=NOT_PERFORMED_PREVIEW_ONLY");
        }
        for warning in &evaluation.warnings {
            eprintln!("{warning}");
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
    let parsed = goblinpp::modules::resolve(&args.file, text)?.parsed;
    parsed.require_executable_program()?;
    if goblinpp::numeric_audit::requested(&parsed.program).is_some() {
        return Err(GoblinError::new(
            "G702",
            "GO_NUMERIC_AUDIT cannot be fulfilled by standalone compile. Use goblin++ run FILE --compile for preserved audit evidence.",
        ));
    }
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
        if freeze
            .receipt
            .as_ref()
            .and_then(|r| r["parser_policy"].as_str())
            != Some(goblinpp::parser::PARSER_POLICY)
        {
            return Err(GoblinError::protocol(
                "PARSER_POLICY_CHANGED_AFTER_FREEZE",
                "The freeze predates compound-unit semantics.\n\nClassification:\nPARSER_POLICY_CHANGED_AFTER_FREEZE\n\nCompilation refused. Review quantity powers, create an explicit revision and freeze the child to adopt alpha.25 semantics.",
            ));
        }
        if freeze
            .receipt
            .as_ref()
            .and_then(|value| value["language_semantics"].as_str())
            != Some(goblinpp::text_runtime::SEMANTICS_POLICY)
        {
            return Err(GoblinError::protocol(
                "LANGUAGE_SEMANTICS_CHANGED_AFTER_FREEZE",
                "The freeze uses different language semantics.\n\nClassification:\nLANGUAGE_SEMANTICS_CHANGED_AFTER_FREEZE\n\nCompilation refused. Create an explicit revision and freeze the child to adopt the current semantics.",
            ));
        }
        if !goblinpp::math_policy::frozen_policy_matches(freeze.receipt.as_ref().unwrap()) {
            return Err(GoblinError::protocol(
                "MATH_POLICY_CHANGED_AFTER_FREEZE",
                "The declared math policy differs from the frozen policy. Compilation refused. Create an explicit revision to adopt a changed policy.",
            ));
        }
        if !goblinpp::science::frozen_statistics_policy_matches(freeze.receipt.as_ref().unwrap()) {
            return Err(GoblinError::protocol(
                "STATISTICS_POLICY_CHANGED_AFTER_FREEZE",
                "The statistics policy differs from the frozen policy. Compilation refused. Create an explicit revision to adopt the changed policy.",
            ));
        }
        if !goblinpp::random::frozen_policy_matches(freeze.receipt.as_ref().unwrap()) {
            return Err(GoblinError::protocol(
                "RNG_POLICY_CHANGED_AFTER_FREEZE",
                "The RNG policy differs from the freeze. Compilation refused; create an explicit revision.",
            ));
        }
        if !goblinpp::inference_policy::frozen_policy_matches(freeze.receipt.as_ref().unwrap()) {
            return Err(GoblinError::protocol(
                "INFERENCE_POLICY_CHANGED_AFTER_FREEZE",
                "The scientific helper policy differs from the freeze. Compilation refused; create an explicit revision.",
            ));
        }
        if !goblinpp::resources::frozen_policy_matches(freeze.receipt.as_ref().unwrap()) {
            return Err(GoblinError::protocol(
                "RESOURCE_POLICY_CHANGED_AFTER_FREEZE",
                "The resource policy differs from the freeze. Compilation refused; create an explicit revision.",
            ));
        }
        let frozen = freeze.receipt.as_ref().unwrap();
        if frozen["array_math_policy"] != goblinpp::array_math::policy() {
            return Err(GoblinError::protocol(
                "ARRAY_MATH_POLICY_NOT_FROZEN",
                "This freeze did not pin the current array arithmetic semantics. Compilation refused; create an explicit revision and freeze the child.",
            ));
        }
        if !goblinpp::plots::frozen_policy_matches(frozen)
            || (frozen["plot_policy"] != goblinpp::plots::POLICY
                && goblinpp::plots::program_uses_plots(&parsed.program))
        {
            return Err(GoblinError::protocol(
                "PLOT_POLICY_CHANGED_AFTER_FREEZE",
                "Plotting is not pinned by this freeze. Compilation refused; create a revision and freeze the child.",
            ));
        }
        if !goblinpp::clock::frozen_policy_matches(frozen)
            || (frozen.get("clock_policy").is_none()
                && goblinpp::clock::program_uses_clock(&parsed.program))
        {
            return Err(GoblinError::protocol(
                "CLOCK_POLICY_CHANGED_AFTER_FREEZE",
                "Clock inputs are not pinned by this freeze. Compilation refused; create an explicit revision and freeze the child.",
            ));
        }
        if !goblinpp::batches::frozen_policy_matches(frozen)
            || (frozen.get("batch_policy").is_none()
                && goblinpp::batches::program_uses_batches(&parsed.program))
        {
            return Err(GoblinError::protocol(
                "BATCH_POLICY_CHANGED_AFTER_FREEZE",
                "Batch processing is not pinned by this freeze. Compilation refused; create an explicit revision and freeze the child.",
            ));
        }
        if !goblinpp::fits::batch::frozen_policy_matches(frozen)
            || (frozen.get("fits_batch_policy").is_none()
                && goblinpp::fits::batch::program_uses(&parsed.program))
        {
            return Err(GoblinError::protocol(
                "FITS_BATCH_POLICY_CHANGED_AFTER_FREEZE",
                "FITS batches are not pinned by this freeze. Create a revision and freeze the child.",
            ));
        }
    }
    let compilation = compiler::compile(&parsed, &args.output, &args.allowed_inline_rust)?;
    let report = json!({ "status": "PASS", "array_math_policy": goblinpp::array_math::policy(), "table_join_policy": goblinpp::table_join::POLICY, "parser_policy": goblinpp::parser::PARSER_POLICY, "math_policy": goblinpp::math_policy::policy(), "rng_policy": goblinpp::random::policy(), "inference_policy":goblinpp::inference_policy::policy(), "resource_policy":goblinpp::resources::policy(), "clock_policy":goblinpp::clock::policy(), "plot_policy":goblinpp::plots::POLICY, "declared_loop_budget":goblinpp::resources::requested(&parsed.program), "execution_status":"NOT_RUN", "launcher_math_environment": goblinpp::math_policy::environment()?, "source_sha256": sha256_bytes(&bytes), "canonical_source_sha256": parsed.canonical_sha256()?, "binary": compilation.binary, "binary_sha256": compilation.binary_sha256, "generated_source": compilation.generated_source, "generated_source_sha256": compilation.generated_source_sha256, "rustc": compilation.rustc_version, "inline_rust": parsed.inline_rust.iter().map(|block| &block.sha256).collect::<Vec<_>>() });
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
        println!(
            "LANGUAGE_SEMANTICS_SAME ....... {}",
            yn(report.language_semantics_same)
        );
        println!(
            "PARSER_POLICY_SAME ........... {}",
            yn(report.parser_policy_same)
        );
        println!(
            "MATH_POLICY_SAME ............. {}\nMATH_ENVIRONMENT_SAME ........ {}\nCOMPARISON_MODE=EXACT_EVIDENCE",
            yn(report.math_policy_same),
            yn(report.math_environment_same)
        );
        println!(
            "STATISTICS_POLICY_SAME ....... {}",
            yn(report.statistics_policy_same)
        );
        println!(
            "ARRAY_MATH_POLICY_SAME ....... {}",
            yn(report.array_math_policy_same)
        );
        println!(
            "TABLE_JOIN_POLICY_SAME ....... {}",
            yn(report.table_join_policy_same)
        );
        println!(
            "RNG_POLICY_SAME .............. {}\nRNG_EVIDENCE_SAME ............ {}",
            yn(report.rng_policy_same),
            yn(report.rng_evidence_same)
        );
        println!(
            "INFERENCE_POLICY_SAME ........ {}\nINFERENCE_EVIDENCE_SAME ...... {}\nRESOURCE_POLICY_SAME ......... {}\nRESOURCE_EVIDENCE_SAME ....... {}",
            yn(report.inference_policy_same),
            yn(report.inference_evidence_same),
            yn(report.resource_policy_same),
            yn(report.resource_evidence_same)
        );
        println!(
            "BATCH_POLICY_SAME ............ {}\nBATCH_EVIDENCE_SAME .......... {}",
            yn(report.batch_policy_same),
            yn(report.batch_evidence_same)
        );
        println!(
            "FITS_BATCH_POLICY_SAME ....... {}",
            yn(report.fits_batch_policy_same)
        );
        println!(
            "CLOCK_POLICY_SAME ............ {}\nCLOCK_READINGS_SAME .......... {}",
            yn(report.clock_policy_same),
            yn(report.clock_readings_same)
        );
        println!(
            "PLOT_POLICY_SAME ............. {}",
            yn(report.plot_policy_same)
        );
        println!(
            "NUMERIC_AUDIT_SAME ........... {}",
            yn(report.numeric_audit_same)
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
            if freeze
                .receipt
                .as_ref()
                .and_then(|r| r["parser_policy"].as_str())
                == Some(goblinpp::parser::PARSER_POLICY)
            {
                "FROZEN_VERIFIED"
            } else {
                "FROZEN_PARSER_MIGRATION_REQUIRED"
            }
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
    #[cfg(unix)]
    let network_acquisition = json!({"policy":goblinpp::acquisition::POLICY,
        "commands":["fetch", "verify-fetch"], "platforms":"Unix; local validation macOS arm64, cross-platform CI required",
        "https_only":true, "port":443, "explicit_host_permission":true, "public_dns_addresses_only":true,
        "redirects":0, "proxies":false, "http_content_decoding":false, "automatic_gbl_network":false,
        "default_max_payload_bytes":goblinpp::acquisition::DEFAULT_MAX_BYTES,
        "hard_max_payload_bytes":goblinpp::acquisition::MAX_BYTES,
        "expected_hash_or_explicit_record_only":true, "offline_verification":true,
        "server_authentication":"RUSTLS_PLATFORM_TRUST", "content_authentication":"CHECKSUM_ONLY",
        "uploads":false, "servers":false, "auth_headers":false, "cube_loading":false});
    #[cfg(not(unix))]
    let network_acquisition =
        json!({"available":false,"reason":"Initial acquisition storage backend requires Unix"});
    let report = json!({
        "goblin_version": goblinpp::VERSION, "release_status": "ALPHA",
        "network_acquisition": network_acquisition,
        "completely_mad": {"policy":goblinpp::mad::POLICY, "stage":"BOUNDED_FINAL_STATE_EVIDENCE", "implies_go_paranoid":true, "max_variables":goblinpp::mad::MAX_VARIABLES, "max_bytes":goblinpp::mad::MAX_BYTES, "inline_rust":false, "standalone_native":false, "encryption":false, "scientific_validity_claim":false},
        "orbit": {"policy":goblinpp::orbit::POLICY,"commands":["orbit baseline", "orbit verify-baseline", "orbit check", "orbit verify"], "standalone":"goblin-orbit", "authentication":"CHECKSUM_ONLY", "scope":"EXPLICIT_LOCAL_DIRECTORY", "comparison":"FILE_BYTES_AND_ENTRY_KIND_ONLY", "symlinks":"NEVER_FOLLOWED", "platforms":"Unix descriptor traversal; locally tested macOS arm64", "repairs":false, "media_detection":false},
        "evidence_report": {"policy":goblinpp::evidence_report::POLICY,"commands":["evidence-report", "verify-report"],"case_max_bytes":goblinpp::evidence_report::CASE_BYTES,"evidence_file_max_bytes":goblinpp::evidence_report::FILE_BYTES,"evidence_total_max_bytes":goblinpp::evidence_report::TOTAL_BYTES,"authentication":"CHECKSUM_ONLY","network_requests":false,"findings":"ANALYST_DECLARATIONS_NOT_CERTIFIED"},
        "available": ["rust interpreter", "explicitly seeded PCG32 v1 scientific randomness in both engines with replayable evidence; not cryptographic", "optional GO_PARANOID postflight evidence and self-verification", "optional seal value snapshots", "bounded range and direct-array for loops, while loops, break/continue, if/else-if/else, switch/case/default, and short-circuit Boolean logic", "g_func user-defined functions with local copy-value arguments and explicit return in both engines", "interactive input and argc/argv program arguments with hashed evidence", "text concatenation, Unicode-scalar length, checked number/integer conversion and g_strings built-ins in both engines", "six-axis SI dimensional analysis including electric current", "dimension-aware scientific math built-ins in both engines", "astronomical distance conversion, homogeneous vector algebra, explicit coordinate conversion, linear kinematics, and rotational mechanics helpers in both engines", "versioned common-element chemistry registry, formula molar mass, moles/mass/concentration/dilution helpers, and laboratory units in both engines", "strict SI electrical units, Ohm and passive-component helpers, unit reporting, and explicit Kirchhoff residual checks in both engines", "copy-value arrays with indexing, half-open slices, append and len in both engines", "checked integer remainder in both engines", "native scalar and array compiler", "strict CSV and TSV reading with explicit text or numeric columns", "static local imports of reusable g_func libraries", "compiled FITS, tables and files with native/reference evidence parity", "exact-hash inline Rust authorization", "native streaming FITS multi-HDU discovery", "native FITS image access", "native FITS binary-table scalar access and numeric statistics", "checksum-addressed deduplicated FITS evidence", "audited TXT, Markdown, CSV, TSV, and JSON output in either mode", "deterministic SVG and PNG FITS plots", "run receipts", "freeze enforcement", "revision lineage", "run verification", "semantic diff", "checksum custody ledger"],
        "fluid_functions": goblinpp::fluids::FUNCTIONS,
        "string_functions": ["str_upper", "str_lower", "str_bytes_len", "sizeof", "str_compare", "to_base", "from_base", "str_to_hex", "str_from_hex", "str_to_octal", "str_from_octal", "str_to_binary", "str_from_binary", "to_text", "parse_number", "parse_integer", "str_trim", "str_contains", "str_replace", "str_split", "str_join", "len"],
        "clock_functions": ["time_date"],
        "plot_functions": ["plot_line", "plot_scatter", "plot_histogram"],
        "plot_policy": goblinpp::plots::POLICY,
        "clock_policy": goblinpp::clock::policy(),
        "math_functions": ["abs", "sqrt", "min", "max", "floor", "ceil", "round", "exp", "ln", "log10", "sind", "cosd", "tand", "sinr", "cosr", "tanr", "asind", "acosd", "atand", "asinr", "acosr", "atanr", "atan2d", "atan2r", "deg2rad", "rad2deg", "hypot"],
        "numeric_comparison_functions": ["is_close", "same_bits"],
        "numeric_audit": {"directive":"GO_NUMERIC_AUDIT", "policy":goblinpp::numeric_audit::POLICY,
            "requires":"run --compile with explicit JSON configuration", "quantities":"1..16 named finite numeric scalar seals",
            "comparison":"exact f64 bits and six dimensions; same machine only", "preview":"NOT_PERFORMED_PREVIEW_ONLY",
            "higher_precision":false, "scientific_accuracy_certificate":false, "cross_platform_guarantee":false,
            "completely_mad_profile":"PROPOSED_NOT_IMPLEMENTED"},
        "array_math_policy": goblinpp::array_math::policy(), "table_join_policy": goblinpp::table_join::POLICY,
        "math_policy": goblinpp::math_policy::policy(),
        "science_functions": goblinpp::science::FUNCTIONS,
        "parser_policy": goblinpp::parser::PARSER_POLICY,
        "compound_unit_limits": {"unit_exponent_abs_max":32,"dimension_exponent_abs_max":256,"max_group_depth":32,"max_suffix_tokens":256,"multiple_denominator_factors":"PARENTHESES_REQUIRED"},
        "statistics_policy": goblinpp::science::statistics_policy(),
        "inference_policy": goblinpp::inference_policy::policy(),
        "resource_policy": goblinpp::resources::policy(),
        "inference_functions": ["resample", "bootstrap_mean", "bootstrap_median", "rng_normal", "rng_normal_array", "matrix_transpose", "matrix_multiply", "vector_norm", "vector_unit", "covariance", "cholesky", "mvnormal"],
        "streaming_functions": ["csv_scan_stats", "tsv_scan_stats"],
        "rng_policy": goblinpp::random::policy(),
        "chemistry_functions": goblinpp::chemistry::FUNCTIONS,
        "rng_functions": ["rng_seed", "rng_word", "rng_uniform", "rng_integer"],
        "chemistry_registry": {"id": goblinpp::chemistry::REGISTRY_ID, "sha256": goblinpp::chemistry::registry_sha256()?, "elements": goblinpp::chemistry::ELEMENTS.len()},
        "electrical_functions": goblinpp::electrical::FUNCTIONS,
        "electrical_registry": {"id": goblinpp::electrical::REGISTRY_ID, "sha256": goblinpp::electrical::registry_sha256()?},
        "deprecated_math_aliases": ["sin", "cos", "tan", "asin", "acos", "atan", "atan2"],
        "fits_functions": ["fits_hdu_count", "fits_header", "fits_axis", "fits_count", "fits_pixel", "fits_mean", "fits_rows", "fits_columns", "fits_column", "fits_column_valid_count", "fits_column_mean", "fits_column_min", "fits_column_max", "fits_select_stats", "fits_column_text", "fits_where", "fits_all", "fits_any", "fits_export_csv", "fits_export_tsv"],
        "fits_subset_policy": {"schema":"goblin.fits-subset.v1", "exact_ids":"unscaled signed i64 decimal text, not arithmetic", "null_marker":"\\N", "physical_blinding":false, "chunk_bytes_limit":8388608, "output_bytes_limit":67108864, "filter_depth_limit":16, "filter_node_limit":256},
        "output_functions": ["write_text", "write_csv", "write_tsv", "write_json", "plot_fits_histogram", "plot_fits_scatter", "plot_line", "plot_scatter", "plot_histogram"],
        "table_functions": ["csv_rows", "csv_columns", "csv_headers", "csv_column", "csv_numbers", "tsv_rows", "tsv_columns", "tsv_headers", "tsv_column", "tsv_numbers"],
        "module_limits": {"max_files":32,"max_bytes_per_file":1048576,"definitions_only":true},
        "table_limits": {"max_bytes":16777216,"max_rows":100000,"max_columns":1024,"max_cells":1000000,"missing_numeric_policy":"REFUSE"},
        "pending_from_python_reference": ["Ed25519 ledger signing", "interrupted-head recovery", "adopt legacy freeze", "normative corpus commands"],
        "known_limits": ["GO_PARANOID observes source bytes at postflight, not continuously, and is not an OS sandbox", "input and argument text is stored in run evidence; do not enter secrets", "check will not consume stdin and reports input required", "input and argv return text; parse_number converts finite unitless decimals and parse_integer accepts safe decimal integers only", "numbers remain f64 quantities, not a distinct exact integer type; integer parsing and remainder are limited to exactly representable values", "len(text) counts Unicode scalar values, not grapheme clusters; text operations do not normalize Unicode", "arrays are one-dimensional, homogeneous, and limited to 100000 items; slices, assignments, and direct iteration use independent values rather than shared Go-style backing storage", "loops share a default 1000000-body-entry budget; GO_LOOP_BUDGET can declare up to 50000000", "g_func requires a return on the reached path, has no implicit global capture, and is limited to 16 active calls", "coordinate helpers use Euclidean Cartesian geometry, inclination from +z, and azimuth from +x toward +y", "relativistic velocity addition is collinear special relativity only; reference frames, tensors, curved spacetime, orbital propagation, and uncertainty propagation are not implemented", "chemistry uses an explicitly scoped abridged common-element registry; isotope-specific masses, charge, hydrates, reactions, pH, kinetics, equilibria, and biological sequence semantics are not inferred", "electrical helpers cover scalar DC/passive calculations and explicit KCL/KVL residuals; they do not infer circuit topology, solve simultaneous networks, model AC phasors, semiconductors, tolerances, or uncertainty", "ee_power uses the passive sign convention, so negative power represents delivery rather than invalid data", "data-using compilation requires Cargo and cached locked crates; standalone binaries lack launcher custody and freeze checks", "switch uses exact equality, first match, and no fallthrough", "FITS plots use explicit deterministic row samples; array plots use all finite supplied points and preserve exact data/settings sidecars", "JPEG output is omitted because it is lossy", "inline Rust cannot mutate sealed Goblin variables", "ASCII tables are discovered but not read", "compressed images, random groups, bit, complex, and variable-array columns are not read", "FITS WCS and declared units are reported but not interpreted"]
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
        println!("\nMATH FUNCTIONS");
        for item in report["math_functions"].as_array().unwrap() {
            println!("- {}", item.as_str().unwrap());
        }
        println!("\nSCIENCE FUNCTIONS");
        for item in report["science_functions"].as_array().unwrap() {
            println!("- {}", item.as_str().unwrap());
        }
        println!("\nCHEMISTRY FUNCTIONS");
        for item in report["chemistry_functions"].as_array().unwrap() {
            println!("- {}", item.as_str().unwrap());
        }
        println!(
            "CHEMISTRY_REGISTRY={} SHA256={}",
            report["chemistry_registry"]["id"].as_str().unwrap(),
            report["chemistry_registry"]["sha256"].as_str().unwrap()
        );
        println!("\nELECTRICAL FUNCTIONS");
        for item in report["electrical_functions"].as_array().unwrap() {
            println!("- {}", item.as_str().unwrap());
        }
        println!(
            "ELECTRICAL_REGISTRY={} SHA256={}",
            report["electrical_registry"]["id"].as_str().unwrap(),
            report["electrical_registry"]["sha256"].as_str().unwrap()
        );
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
