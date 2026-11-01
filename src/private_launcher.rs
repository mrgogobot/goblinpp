//! Private execution never enters runtime::run_file or the scientific custody path.
use crate::error::{GoblinError, Result};
use crate::evaluator::Evaluation;
use crate::interaction::InputPolicy;
use crate::parser::ParsedSource;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

pub fn failure() -> GoblinError {
    GoblinError::new(
        "G902",
        "Private execution failed. Secret-bearing values and paths are omitted. Check argument types, passphrase, input integrity, output existence, TLS certificates, explicit endpoint permission, timeout and documented limits. Previously completed file operations remain completed.",
    )
}
pub fn load(source: &Path) -> Result<ParsedSource> {
    if source.is_symlink() {
        return Err(failure());
    }
    let bytes = fs::read(source).map_err(|_| failure())?;
    let text = std::str::from_utf8(&bytes).map_err(|_| failure())?;
    let parsed = crate::modules::resolve(source, text)
        .map_err(|_| failure())?
        .parsed;
    parsed.require_executable_program().map_err(|_| failure())?;
    crate::parser::validate_execution(&parsed.program)?;
    if !crate::confidential::private(&parsed.program) {
        return Err(GoblinError::new(
            "G902",
            "A private script must start with GO_PRIVATE.",
        ));
    }
    if fs::symlink_metadata(crate::custody::freeze_path(source)).is_ok() {
        return Err(GoblinError::new(
            "G902",
            "GO_PRIVATE does not execute frozen scientific sources. Use an explicit separate private script; the original freeze is unchanged.",
        ));
    }
    let mut validation = Evaluation::new(".");
    validation.register_functions(&parsed.program)?;
    Ok(parsed)
}
pub fn run(source: &Path, compile: bool, program_args: Vec<String>) -> Result<i32> {
    let parsed = load(source)?;
    if compile {
        let temporary = tempfile::tempdir().map_err(|_| failure())?;
        let output = temporary.path().join("private-program");
        crate::compiler::compile(&parsed, &output, &[]).map_err(|_| failure())?;
        let base = source
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .canonicalize()
            .map_err(|_| failure())?;
        let result = Command::new(output)
            .args(program_args)
            .env("GOBLIN_NATIVE_DATA_BASE", base)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|_| failure())?;
        return Ok(if result.success() { 0 } else { 1 });
    }
    let mut evaluation = Evaluation::new(
        source
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    );
    let mut argv = vec![source.to_string_lossy().into_owned()];
    argv.extend(program_args);
    evaluation
        .configure_interaction(argv, InputPolicy::Interactive)
        .map_err(|_| failure())?;
    println!(
        "GOBLIN++ GO_PRIVATE MODE\nEXECUTION_PROFILE=GO_PRIVATE\nCONFIDENTIAL_OPERATION=NO_RUN_RECEIPT"
    );
    let evaluation = evaluation
        .eval_program(&parsed.program)
        .map_err(|_| failure())?;
    for line in evaluation.stdout {
        println!("{line}");
    }
    println!("RUN_STATUS=PASS");
    Ok(0)
}
