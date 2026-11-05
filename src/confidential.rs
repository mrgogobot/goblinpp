//! Explicit private scripts: ordinary values, no automatic scientific evidence.
use crate::ast::{Expr, Program, Stmt};
use crate::error::{GoblinError, Result};
use crate::evaluator::Value;
use std::path::{Path, PathBuf};

pub fn is_function(name: &str) -> bool {
    crate::ai::is_function(name)
        || crate::networking::is_function(name)
        || crate::recipient_crypto::is_function(name)
        || matches!(
            name,
            "input_secret"
                | "encrypt_text"
                | "encrypt_file"
                | "decrypt_file"
                | "decrypt_text"
                | "read_text"
                | "display_file"
        )
}
pub fn is_effect(name: &str) -> bool {
    crate::networking::is_effect(name)
        || matches!(
            name,
            "encrypt_text" | "encrypt_file" | "decrypt_file" | "display_file"
        )
}
pub fn private(program: &Program) -> bool {
    program
        .statements
        .iter()
        .any(|s| matches!(s, Stmt::Directive(n) if n == "GO_PRIVATE"))
}
/// Detect the explicit declaration even if the rest of a private source is malformed.
/// This is a fail-closed privacy routing hint, not the language parser.
pub fn declared(text: &str) -> bool {
    text.lines().any(|line| {
        line.split('#')
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches('\u{feff}')
            == "GO_PRIVATE"
    })
}
pub fn arity(name: &str, n: usize) -> Result<()> {
    if crate::ai::is_function(name) {
        return crate::ai::arity(name, n);
    }
    if crate::networking::is_function(name) {
        return crate::networking::arity(name, n);
    }
    if crate::recipient_crypto::is_function(name) {
        return crate::recipient_crypto::arity(name, n);
    }
    let valid = match name {
        "input_secret" => n <= 1,
        "encrypt_text" | "encrypt_file" | "decrypt_file" => n == 3,
        "decrypt_text" => n == 2,
        "read_text" | "display_file" => n == 1,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(GoblinError::new(
            "G902",
            format!("Invalid argument count for {name}()."),
        ))
    }
}
pub fn visit(
    program: &Program,
    mut on_call: impl FnMut(&str) -> Result<()>,
    mut on_stmt: impl FnMut(&Stmt) -> Result<()>,
) -> Result<()> {
    fn expr(e: &Expr, f: &mut impl FnMut(&str) -> Result<()>) -> Result<()> {
        match e {
            Expr::Call { name, args } => {
                f(name)?;
                for arg in args {
                    expr(arg, f)?;
                }
            }
            Expr::Array(items) => {
                for e in items {
                    expr(e, f)?;
                }
            }
            Expr::Unary { expr: e, .. } | Expr::Not(e) => expr(e, f)?,
            Expr::Binary { left, right, .. }
            | Expr::Compare { left, right, .. }
            | Expr::Logical { left, right, .. } => {
                expr(left, f)?;
                expr(right, f)?;
            }
            Expr::Index { array, index } => {
                expr(array, f)?;
                expr(index, f)?;
            }
            Expr::Slice { array, start, stop } => {
                expr(array, f)?;
                for e in [start, stop].into_iter().flatten() {
                    expr(e, f)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn stmts(
        ss: &[Stmt],
        f: &mut impl FnMut(&str) -> Result<()>,
        g: &mut impl FnMut(&Stmt) -> Result<()>,
    ) -> Result<()> {
        for s in ss {
            g(s)?;
            match s {
                Stmt::Assign { expr: e, .. } | Stmt::Expression(e) | Stmt::Return(e) => expr(e, f)?,
                Stmt::IndexAssign { index, expr: e, .. } => {
                    expr(index, f)?;
                    expr(e, f)?;
                }
                Stmt::Function { body, .. } => stmts(body, f, g)?,
                Stmt::For {
                    start,
                    stop,
                    step,
                    body,
                    ..
                } => {
                    for e in [start, stop, step] {
                        expr(e, f)?;
                    }
                    stmts(body, f, g)?;
                }
                Stmt::ForEach { iterable, body, .. } => {
                    expr(iterable, f)?;
                    stmts(body, f, g)?;
                }
                Stmt::While { condition, body } => {
                    expr(condition, f)?;
                    stmts(body, f, g)?;
                }
                Stmt::If {
                    branches,
                    else_body,
                } => {
                    for (e, b) in branches {
                        expr(e, f)?;
                        stmts(b, f, g)?;
                    }
                    if let Some(b) = else_body {
                        stmts(b, f, g)?;
                    }
                }
                Stmt::Switch {
                    selector,
                    cases,
                    default,
                } => {
                    expr(selector, f)?;
                    for (e, b) in cases {
                        expr(e, f)?;
                        stmts(b, f, g)?;
                    }
                    if let Some(b) = default {
                        stmts(b, f, g)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    stmts(&program.statements, &mut on_call, &mut on_stmt)
}
pub fn validate(program: &Program) -> Result<()> {
    let enabled = private(program);
    if enabled
        && (!matches!(program.statements.first(),Some(Stmt::Directive(n)) if n=="GO_PRIVATE")
            || program
                .statements
                .iter()
                .filter(|s| matches!(s,Stmt::Directive(n) if n=="GO_PRIVATE"))
                .count()
                != 1)
    {
        return Err(GoblinError::parse(
            "GO_PRIVATE must occur once at the start of the main source.",
        ));
    }
    visit(
        program,
        |name| {
            if is_function(name) && !enabled {
                return Err(GoblinError::new(
                    "G902",
                    "Confidential functions require GO_PRIVATE at the start of the main source; normal runs preserve plaintext evidence.",
                ));
            }
            if enabled && crate::evaluator::is_data_function(name) && !is_function(name) {
                return Err(GoblinError::new(
                    "G902",
                    format!(
                        "{name}() uses scientific evidence/output machinery and cannot be called in GO_PRIVATE. Use a separate audited science run."
                    ),
                ));
            }
            Ok(())
        },
        |stmt| {
            if enabled
                && matches!(
                    stmt,
                    Stmt::Seal(_) | Stmt::InlineRust { .. } | Stmt::NumericAudit(_)
                )
            {
                return Err(GoblinError::new(
                    "G902",
                    "GO_PRIVATE forbids seal, numerical audits and inline Rust; this execution produces no scientific receipt.",
                ));
            }
            if enabled && matches!(stmt,Stmt::Directive(n) if n!="GO_PRIVATE") {
                return Err(GoblinError::new(
                    "G902",
                    "GO_PRIVATE cannot be combined with GO_PARANOID or COMPLETELY_MAD.",
                ));
            }
            Ok(())
        },
    )
}
fn path(base: &Path, text: &str) -> PathBuf {
    let p = Path::new(text);
    if p.is_absolute() {
        p.to_owned()
    } else {
        base.join(p)
    }
}
pub fn call(base: &Path, name: &str, args: &[Value]) -> Result<Value> {
    arity(name, args.len())?;
    if crate::ai::is_function(name) {
        return crate::ai::code_call(base, name, args);
    }
    let texts = args
        .iter()
        .map(|v| v.text(name))
        .collect::<Result<Vec<_>>>()?;
    if crate::recipient_crypto::is_function(name) {
        let result = match name {
            "e2e_keygen" => {
                let public = crate::recipient_crypto::keygen(&path(base, texts[0]))?;
                if let Some(output) = texts.get(1) {
                    crate::recipient_crypto::save_new(&path(base, output), public.as_bytes())?;
                }
                Ok(public)
            }
            "e2e_public_key" => crate::recipient_crypto::public_key(&path(base, texts[0])),
            "e2e_encrypt" => crate::recipient_crypto::encrypt(texts[0], texts[1]),
            "e2e_decrypt" => crate::recipient_crypto::decrypt(texts[0], &path(base, texts[1])),
            _ => unreachable!(),
        };
        return result.map(Value::Text);
    }
    let input = path(base, texts[0]);
    match name {
        "encrypt_text" => {
            crate::crypto_cli::script_operation(
                None,
                Some(texts[0]),
                texts[1],
                Some(&path(base, texts[2])),
                true,
            )?;
            Ok(Value::Bool(true))
        }
        "encrypt_file" | "decrypt_file" => {
            crate::crypto_cli::script_operation(
                Some(&input),
                None,
                texts[1],
                Some(&path(base, texts[2])),
                name == "encrypt_file",
            )?;
            Ok(Value::Bool(true))
        }
        "decrypt_text" => Ok(Value::Text(
            crate::crypto_cli::script_operation(Some(&input), None, texts[1], None, false)?
                .unwrap(),
        )),
        "read_text" => Ok(Value::Text(crate::crypto_cli::script_read_text(&input)?)),
        "display_file" => {
            crate::crypto_cli::read_file(&input)?;
            Ok(Value::Bool(true))
        }
        _ => Err(GoblinError::new("G902", "Unknown confidential function.")),
    }
}
