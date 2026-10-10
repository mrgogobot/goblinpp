use crate::ast::{Expr, Program, Stmt};
use crate::constants::resolve;
use crate::error::{GoblinError, Result};
use crate::hashing::sha256_bytes;
use crate::inline_rust::{ExtractedSource, InlineRustBlock, extract};
use crate::lexer::{Token, TokenKind, lex};
use crate::quantity::is_unit;
use std::collections::HashSet;

pub const PARSER_POLICY: &str = "goblin.compound-units-loop-budget.v2";
pub const LEGACY_COMPOUND_PARSER_POLICY: &str = "goblin.compound-unit-literals.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxMode {
    Current,
    CompoundUnitsNoLoopBudget,
    LegacyQuantity,
}

/// Missing policy means the historical parser, never today's interpretation.
/// A current/future receipt cannot downgrade by removing its policy field.
pub fn evidence_mode(receipt: &serde_json::Value) -> Result<SyntaxMode> {
    let version = receipt["goblin_version"].as_str().unwrap_or("");
    let alpha = version
        .strip_prefix("0.1.0-alpha.")
        .and_then(|v| v.parse::<u32>().ok());
    match receipt.get("parser_policy") {
        Some(value) if value.as_str() == Some(PARSER_POLICY) => Ok(SyntaxMode::Current),
        Some(value)
            if value.as_str() == Some(LEGACY_COMPOUND_PARSER_POLICY)
                && alpha.is_some_and(|v| v <= 26) =>
        {
            Ok(SyntaxMode::CompoundUnitsNoLoopBudget)
        }
        Some(_) => Err(GoblinError::parse("Unsupported parser policy in evidence.")),
        None => {
            let historical = version.starts_with("0.0.")
                || version
                    .strip_prefix("0.1.0-alpha.")
                    .and_then(|v| v.parse::<u32>().ok())
                    .is_some_and(|v| v <= 24);
            if historical {
                Ok(SyntaxMode::LegacyQuantity)
            } else {
                Err(GoblinError::parse(
                    "Parser policy is required in alpha.25 and newer evidence.",
                ))
            }
        }
    }
}

pub fn reserved_function_name(name: &str) -> bool {
    matches!(
        name,
        "g_func"
            | "import"
            | "return"
            | "GO_PARANOID"
            | "GO_LOOP_BUDGET"
            | "seal"
            | "for"
            | "while"
            | "range"
            | "in"
            | "if"
            | "else"
            | "switch"
            | "case"
            | "default"
            | "and"
            | "or"
            | "not"
            | "break"
            | "continue"
            | "true"
            | "false"
            | "argc"
            | "argv"
            | "input"
            | "print"
            | "printf"
            | "len"
            | "append"
            | "to_text"
            | "parse_number"
            | "parse_integer"
            | "str_trim"
            | "str_contains"
            | "str_replace"
            | "str_split"
            | "str_join"
            | "abs"
            | "sqrt"
            | "min"
            | "max"
            | "floor"
            | "ceil"
            | "round"
            | "exp"
            | "ln"
            | "log10"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "atan2"
            | "sind"
            | "cosd"
            | "tand"
            | "sinr"
            | "cosr"
            | "tanr"
            | "asind"
            | "acosd"
            | "atand"
            | "asinr"
            | "acosr"
            | "atanr"
            | "atan2d"
            | "atan2r"
            | "deg2rad"
            | "rad2deg"
            | "hypot"
            | "fits_header"
            | "fits_axis"
            | "fits_count"
            | "fits_pixel"
            | "fits_mean"
            | "fits_hdu_count"
            | "fits_rows"
            | "fits_columns"
            | "fits_column"
            | "fits_column_valid_count"
            | "fits_column_mean"
            | "fits_column_min"
            | "fits_column_max"
            | "fits_select_stats"
            | "write_text"
            | "write_csv"
            | "write_tsv"
            | "write_json"
            | "plot_fits_histogram"
            | "plot_fits_scatter"
    ) || crate::delimited::is_function(name)
        || crate::batches::is_function(name)
        || (crate::science::is_function(name) && !crate::science::is_distribution_function(name))
        || crate::chemistry::is_function(name)
        || crate::electrical::is_function(name)
        || name.starts_with("__goblin_")
        || resolve(name).is_some()
        || is_unit(name)
}

/// Execution checks are intentionally separate from syntax/canonical parsing:
/// historical receipts must remain verifiable without executing their programs.
pub fn validate_execution(program: &Program) -> Result<()> {
    crate::resources::validate_program(program)?;
    validate_statements(&program.statements)
}

pub fn validate_statement(statement: &Stmt) -> Result<()> {
    validate_statements(std::slice::from_ref(statement))
}

pub fn require_writable_name(name: &str) -> Result<()> {
    if resolve(name).is_some() {
        return Err(GoblinError::parse(format!(
            "{name} is a registered constant and cannot be assigned or shadowed. Choose a different variable name."
        )));
    }
    Ok(())
}

fn value_only_builtin(name: &str) -> bool {
    // Effectful calls and user-defined functions may intentionally be statements.
    // All registered value-returning builtins must have their result consumed.
    reserved_function_name(name)
        && !matches!(
            name,
            "input"
                | "print"
                | "printf"
                | "write_text"
                | "write_csv"
                | "write_tsv"
                | "write_json"
                | "plot_fits_histogram"
                | "plot_fits_scatter"
                | "fits_export_csv"
                | "fits_export_tsv"
                | "batch_close"
                | "stream_write"
                | "stream_close"
        )
        && resolve(name).is_none()
        && !is_unit(name)
        && !name.starts_with("__goblin_")
        || matches!(
            name,
            "sum"
                | "mean"
                | "sort"
                | "median"
                | "quantile"
                | "std_population"
                | "std_sample"
                | "ecdf"
        )
        || crate::numeric_comparison::is_function(name)
        || crate::streaming::is_function(name)
        || crate::resampling::is_function(name)
        || crate::matrix::is_function(name)
        || (crate::random::is_function(name) && name != "rng_seed")
        || (crate::fits::is_selection_function(name)
            && !matches!(name, "fits_export_csv" | "fits_export_tsv"))
}

fn validate_statements(statements: &[Stmt]) -> Result<()> {
    for statement in statements {
        match statement {
            Stmt::Assign { name, .. } | Stmt::IndexAssign { name, .. } => {
                require_writable_name(name)?
            }
            Stmt::Function { name, params, body } => {
                // Keep historical canonical parsing possible, but prevent a new
                // builtin from silently overriding a user function at execution.
                if crate::numeric_comparison::is_function(name)
                    || crate::streaming::is_function(name)
                    || crate::resampling::is_function(name)
                    || crate::matrix::is_function(name)
                    || crate::random::is_function(name)
                    || crate::fits::is_selection_function(name)
                    || crate::science::is_distribution_function(name)
                {
                    return Err(GoblinError::parse(format!(
                        "{name} is a registered builtin function and cannot be redefined. Rename the user function before executing under this runtime."
                    )));
                }
                require_writable_name(name)?;
                for param in params {
                    if crate::science::is_distribution_function(param)
                        || crate::streaming::is_function(param)
                        || crate::resampling::is_function(param)
                        || crate::matrix::is_function(param)
                        || crate::random::is_function(param)
                    {
                        return Err(GoblinError::parse(format!(
                            "{param} is a registered builtin function and cannot name a g_func parameter."
                        )));
                    }
                    require_writable_name(param)?;
                }
                validate_statements(body)?;
            }
            Stmt::For { variable, body, .. } | Stmt::ForEach { variable, body, .. } => {
                require_writable_name(variable)?;
                validate_statements(body)?;
            }
            Stmt::While { body, .. } => validate_statements(body)?,
            Stmt::If {
                branches,
                else_body,
            } => {
                for (_, body) in branches {
                    validate_statements(body)?;
                }
                if let Some(body) = else_body {
                    validate_statements(body)?;
                }
            }
            Stmt::Switch { cases, default, .. } => {
                for (_, body) in cases {
                    validate_statements(body)?;
                }
                if let Some(body) = default {
                    validate_statements(body)?;
                }
            }
            Stmt::Expression(Expr::Call { name, .. }) if value_only_builtin(name) => {
                return Err(GoblinError::parse(format!(
                    "Discarded result of {name}(). Assign, return, print, or otherwise use its value. {}",
                    if name == "append" {
                        "append() returns an independent copy; use a = append(a, value)."
                    } else {
                        "This builtin does not mutate its arguments."
                    }
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ParsedSource {
    pub syntax_mode: SyntaxMode,
    pub program: Program,
    pub inline_rust: Vec<InlineRustBlock>,
    pub language_source: String,
}

impl ParsedSource {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        crate::hashing::canonical_json_bytes(&self.program.canonical())
    }

    pub fn canonical_sha256(&self) -> Result<String> {
        Ok(sha256_bytes(&self.canonical_bytes()?))
    }
    pub fn paranoid(&self) -> bool {
        self.program
            .statements
            .iter()
            .any(|stmt| matches!(stmt, Stmt::Directive(name) if name == "GO_PARANOID"))
    }
    pub fn require_executable_program(&self) -> Result<()> {
        if self.program.statements.is_empty() {
            Err(GoblinError::parse(
                "EMPTY PROGRAM\n\nGoblin++ refuses to certify an empty or comment-only source file as a successful run.",
            ))
        } else {
            Ok(())
        }
    }
}

pub fn parse_source(source: &str) -> Result<ParsedSource> {
    parse_source_with_mode(source, SyntaxMode::Current)
}

pub fn parse_source_with_mode(source: &str, mode: SyntaxMode) -> Result<ParsedSource> {
    let ExtractedSource {
        language_source,
        blocks,
    } = extract(source)?;
    let mut program = Parser::new(lex(&language_source)?, mode).parse()?;
    for statement in &mut program.statements {
        if let Stmt::Expression(Expr::Name {
            source,
            canonical_id: None,
        }) = statement
            && let Some(index) = source
                .strip_prefix("__goblin_inline_rust_")
                .and_then(|value| value.parse::<usize>().ok())
        {
            let block = blocks
                .get(index)
                .ok_or_else(|| GoblinError::parse("Invalid inline Rust placeholder."))?;
            *statement = Stmt::InlineRust {
                index,
                sha256: block.sha256.clone(),
            };
        }
    }
    Ok(ParsedSource {
        syntax_mode: mode,
        program,
        inline_rust: blocks,
        language_source,
    })
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    in_function: bool,
    loop_depth: usize,
    mode: SyntaxMode,
}

impl Parser {
    fn new(tokens: Vec<Token>, mode: SyntaxMode) -> Self {
        Self {
            tokens,
            cursor: 0,
            in_function: false,
            loop_depth: 0,
            mode,
        }
    }
    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }
    fn name_reserved(&self, name: &str) -> bool {
        if name == "GO_LOOP_BUDGET" {
            self.mode == SyntaxMode::Current
        } else {
            reserved_function_name(name)
        }
    }
    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        self.cursor += 1;
        token
    }

    fn parse(mut self) -> Result<Program> {
        let statements = self.statements(false)?;
        let mut names = HashSet::new();
        for statement in &statements {
            if let Stmt::Function { name, .. } = statement
                && !names.insert(name)
            {
                return Err(GoblinError::parse(format!("Duplicate g_func {name:?}.")));
            }
        }
        if statements
            .iter()
            .filter(|s| matches!(s, Stmt::LoopBudget(_)))
            .count()
            > 1
        {
            return Err(GoblinError::parse(
                "GO_LOOP_BUDGET may be declared only once in the main source.",
            ));
        }
        Ok(Program { statements })
    }

    fn statements(&mut self, in_block: bool) -> Result<Vec<Stmt>> {
        let mut statements = Vec::new();
        self.skip_newlines();
        loop {
            if self.operator_is('}') {
                if !in_block {
                    return Err(GoblinError::parse("Unexpected closing brace."));
                }
                self.advance();
                return Ok(statements);
            }
            if matches!(self.current().kind, TokenKind::Eof) {
                if in_block {
                    return Err(GoblinError::parse("Unclosed statement block."));
                }
                return Ok(statements);
            }
            statements.push(self.statement(in_block)?);
            if !(matches!(self.current().kind, TokenKind::Newline | TokenKind::Eof)
                || (in_block && self.operator_is('}')))
            {
                return Err(GoblinError::parse(format!(
                    "Expected end of statement, got {:?} at {}.",
                    self.current().describe(),
                    self.current().position
                )));
            }
            self.skip_newlines();
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current().kind, TokenKind::Newline) {
            self.cursor += 1;
        }
    }

    fn statement(&mut self, in_block: bool) -> Result<Stmt> {
        if self.mode == SyntaxMode::Current && self.ident_is("GO_LOOP_BUDGET") {
            if in_block {
                return Err(GoblinError::parse(
                    "GO_LOOP_BUDGET must be top-level in the main source, not inside a block or function.",
                ));
            }
            self.advance();
            let token = self.advance();
            let TokenKind::Number(number) = token.kind else {
                return Err(GoblinError::parse(
                    "GO_LOOP_BUDGET requires one integer literal.",
                ));
            };
            let number = number.parse::<u64>().map_err(|_| {
                GoblinError::parse("GO_LOOP_BUDGET requires an unsigned decimal integer literal.")
            })?;
            if !(1..=crate::resources::MAX_LOOP_BUDGET).contains(&number) {
                return Err(GoblinError::parse(
                    "GO_LOOP_BUDGET must be an integer from 1 to 50000000.",
                ));
            }
            return Ok(Stmt::LoopBudget(number));
        }
        if self.ident_is("import") {
            if in_block {
                return Err(GoblinError::parse("import declarations must be top-level."));
            }
            self.advance();
            let TokenKind::Text(path) = self.advance().kind else {
                return Err(GoblinError::parse(
                    "import requires a quoted local .gbl path.",
                ));
            };
            return Ok(Stmt::Import(path));
        }
        if self.ident_is("g_func") {
            if in_block {
                return Err(GoblinError::parse("g_func declarations must be top-level."));
            }
            self.advance();
            let name = self.expect_ident()?;
            if self.name_reserved(&name) {
                return Err(GoblinError::parse(format!(
                    "{name:?} cannot name a g_func."
                )));
            }
            self.expect_operator('(')?;
            let mut params = Vec::new();
            if !self.operator_is(')') {
                loop {
                    let param = self.expect_ident()?;
                    if self.name_reserved(&param) || params.contains(&param) {
                        return Err(GoblinError::parse(format!(
                            "Invalid or duplicate g_func parameter {param:?}."
                        )));
                    }
                    params.push(param);
                    if params.len() > 32 {
                        return Err(GoblinError::parse("g_func accepts at most 32 parameters."));
                    }
                    if !self.accept_operator(',') {
                        break;
                    }
                }
            }
            self.expect_operator(')')?;
            self.expect_operator('{')?;
            self.in_function = true;
            let body = self.statements(true)?;
            self.in_function = false;
            return Ok(Stmt::Function { name, params, body });
        }
        if self.ident_is("return") {
            if !self.in_function {
                return Err(GoblinError::parse("return is only valid inside g_func."));
            }
            self.advance();
            return Ok(Stmt::Return(self.expression()?));
        }
        if self.ident_is("break") || self.ident_is("continue") {
            if self.loop_depth == 0 {
                return Err(GoblinError::parse(
                    "break and continue are only valid inside a loop.",
                ));
            }
            let is_break = self.ident_is("break");
            self.advance();
            return Ok(if is_break {
                Stmt::Break
            } else {
                Stmt::Continue
            });
        }
        if self.ident_is("for") {
            self.advance();
            let variable = self.expect_ident()?;
            if self.name_reserved(&variable) {
                return Err(GoblinError::parse(format!(
                    "{variable:?} cannot be a loop variable."
                )));
            }
            self.expect_keyword("in")?;
            if self.ident_is("range")
                && self
                    .tokens
                    .get(self.cursor + 1)
                    .is_some_and(|token| matches!(token.kind, TokenKind::Operator('(')))
            {
                self.advance();
                self.expect_operator('(')?;
                let first = self.expression()?;
                let (start, stop, step) = if self.accept_operator(',') {
                    let second = self.expression()?;
                    let step = if self.accept_operator(',') {
                        self.expression()?
                    } else {
                        Expr::Number(1.0)
                    };
                    (first, second, step)
                } else {
                    (Expr::Number(0.0), first, Expr::Number(1.0))
                };
                self.expect_operator(')')?;
                self.expect_operator('{')?;
                self.loop_depth += 1;
                let body = self.statements(true);
                self.loop_depth -= 1;
                return Ok(Stmt::For {
                    variable,
                    start,
                    stop,
                    step,
                    body: body?,
                });
            } else {
                let iterable = self.expression()?;
                self.expect_operator('{')?;
                self.loop_depth += 1;
                let body = self.statements(true);
                self.loop_depth -= 1;
                return Ok(Stmt::ForEach {
                    variable,
                    iterable,
                    body: body?,
                });
            }
        }
        if self.ident_is("while") {
            self.advance();
            let condition = self.expression()?;
            self.expect_operator('{')?;
            self.loop_depth += 1;
            let body = self.statements(true);
            self.loop_depth -= 1;
            return Ok(Stmt::While {
                condition,
                body: body?,
            });
        }
        if self.ident_is("if") {
            self.advance();
            let condition = self.expression()?;
            self.expect_operator('{')?;
            let mut branches = vec![(condition, self.statements(true)?)];
            let mut else_body = None;
            loop {
                let checkpoint = self.cursor;
                self.skip_newlines();
                if !self.ident_is("else") {
                    self.cursor = checkpoint;
                    break;
                }
                self.advance();
                if self.ident_is("if") {
                    self.advance();
                    let condition = self.expression()?;
                    self.expect_operator('{')?;
                    branches.push((condition, self.statements(true)?));
                } else {
                    self.expect_operator('{')?;
                    else_body = Some(self.statements(true)?);
                    break;
                }
            }
            return Ok(Stmt::If {
                branches,
                else_body,
            });
        }
        if self.ident_is("switch") {
            self.advance();
            let selector = self.expression()?;
            self.expect_operator('{')?;
            let mut cases = Vec::new();
            let mut default = None;
            loop {
                self.skip_newlines();
                if self.accept_operator('}') {
                    break;
                }
                if matches!(self.current().kind, TokenKind::Eof) {
                    return Err(GoblinError::parse("Unclosed switch block."));
                }
                if self.ident_is("case") {
                    if default.is_some() {
                        return Err(GoblinError::parse("A switch case cannot follow default."));
                    }
                    self.advance();
                    let label = self.expression()?;
                    self.expect_operator('{')?;
                    cases.push((label, self.statements(true)?));
                } else if self.ident_is("default") {
                    if default.is_some() {
                        return Err(GoblinError::parse("A switch may have only one default."));
                    }
                    self.advance();
                    self.expect_operator('{')?;
                    default = Some(self.statements(true)?);
                } else {
                    return Err(GoblinError::parse("Expected case or default in switch."));
                }
            }
            if cases.is_empty() {
                return Err(GoblinError::parse("A switch requires at least one case."));
            }
            return Ok(Stmt::Switch {
                selector,
                cases,
                default,
            });
        }
        if self.ident_is("GO_PARANOID") {
            if in_block {
                return Err(GoblinError::parse(
                    "GO_PARANOID must be a top-level directive.",
                ));
            }
            self.advance();
            if self.accept_operator('(') {
                self.expect_operator(')')?;
            }
            return Ok(Stmt::Directive("GO_PARANOID".into()));
        }
        if self.ident_is("seal") {
            if self.in_function {
                return Err(GoblinError::parse(
                    "seal must be outside g_func; seal the returned value in the caller.",
                ));
            }
            self.advance();
            return Ok(Stmt::Seal(self.expect_ident()?));
        }
        if in_block
            && matches!(&self.current().kind, TokenKind::Ident(name) if name.starts_with("__goblin_inline_rust_"))
        {
            return Err(GoblinError::parse(
                "Inline Rust blocks are not supported inside control-flow blocks; keep them top-level and explicitly authorized.",
            ));
        }
        if let TokenKind::Ident(name) = &self.current().kind
            && self
                .tokens
                .get(self.cursor + 1)
                .is_some_and(|token| matches!(token.kind, TokenKind::Operator('=')))
        {
            let name = name.clone();
            if name == "argc" {
                return Err(GoblinError::parse(
                    "argc is a read-only program argument count.",
                ));
            }
            self.cursor += 2;
            return Ok(Stmt::Assign {
                name,
                expr: self.expression()?,
            });
        }
        if let TokenKind::Ident(name) = &self.current().kind
            && self
                .tokens
                .get(self.cursor + 1)
                .is_some_and(|token| matches!(token.kind, TokenKind::Operator('[')))
        {
            let name = name.clone();
            let checkpoint = self.cursor;
            self.cursor += 2;
            if !self.operator_is(':') && !self.operator_is(']') {
                let index = self.expression()?;
                if self.accept_operator(']') && self.accept_operator('=') {
                    if name == "argc" || resolve(&name).is_some() {
                        return Err(GoblinError::parse(format!("{name:?} is read-only.")));
                    }
                    return Ok(Stmt::IndexAssign {
                        name,
                        index,
                        expr: self.expression()?,
                    });
                }
            }
            self.cursor = checkpoint;
        }
        Ok(Stmt::Expression(self.expression()?))
    }

    fn expression(&mut self) -> Result<Expr> {
        self.logical_or()
    }

    fn logical_or(&mut self) -> Result<Expr> {
        let mut node = self.logical_and()?;
        while self.ident_is("or") {
            self.advance();
            node = Expr::Logical {
                op: "or".into(),
                left: Box::new(node),
                right: Box::new(self.logical_and()?),
            };
        }
        Ok(node)
    }

    fn logical_and(&mut self) -> Result<Expr> {
        let mut node = self.logical_not()?;
        while self.ident_is("and") {
            self.advance();
            node = Expr::Logical {
                op: "and".into(),
                left: Box::new(node),
                right: Box::new(self.logical_not()?),
            };
        }
        Ok(node)
    }

    fn logical_not(&mut self) -> Result<Expr> {
        if self.ident_is("not") {
            self.advance();
            Ok(Expr::Not(Box::new(self.logical_not()?)))
        } else {
            self.comparison()
        }
    }

    fn comparison(&mut self) -> Result<Expr> {
        let left = self.additive()?;
        if let TokenKind::Compare(op) = &self.current().kind {
            let op = op.clone();
            self.advance();
            Ok(Expr::Compare {
                op,
                left: Box::new(left),
                right: Box::new(self.additive()?),
            })
        } else {
            Ok(left)
        }
    }

    fn additive(&mut self) -> Result<Expr> {
        let mut node = self.multiplicative()?;
        while self.operator_is('+') || self.operator_is('-') {
            let op = self.expect_any_operator()?;
            node = Expr::Binary {
                op,
                left: Box::new(node),
                right: Box::new(self.multiplicative()?),
            };
        }
        Ok(node)
    }

    fn multiplicative(&mut self) -> Result<Expr> {
        let mut node = self.power()?;
        loop {
            if self.operator_is('*') || self.operator_is('/') || self.operator_is('%') {
                let op = self.expect_any_operator()?;
                node = Expr::Binary {
                    op,
                    left: Box::new(node),
                    right: Box::new(self.power()?),
                };
            } else if matches!(
                self.current().kind,
                TokenKind::Number(_) | TokenKind::Operator('(')
            ) || matches!(&self.current().kind, TokenKind::Ident(name) if name != "and" && name != "or")
            {
                node = Expr::Binary {
                    op: '*',
                    left: Box::new(node),
                    right: Box::new(self.power()?),
                };
            } else {
                break;
            }
        }
        Ok(node)
    }

    fn power(&mut self) -> Result<Expr> {
        let node = self.unary()?;
        if self.accept_operator('^') {
            Ok(Expr::Binary {
                op: '^',
                left: Box::new(node),
                right: Box::new(self.power()?),
            })
        } else {
            Ok(node)
        }
    }

    fn unary(&mut self) -> Result<Expr> {
        if self.operator_is('+') || self.operator_is('-') {
            let op = self.expect_any_operator()?;
            Ok(Expr::Unary {
                op,
                expr: Box::new(self.unary()?),
            })
        } else {
            self.postfix()
        }
    }

    fn postfix(&mut self) -> Result<Expr> {
        let mut node = self.primary()?;
        while self.accept_operator('[') {
            let start = if self.operator_is(':') || self.operator_is(']') {
                None
            } else {
                Some(Box::new(self.expression()?))
            };
            if self.accept_operator(':') {
                let stop = if self.operator_is(']') {
                    None
                } else {
                    Some(Box::new(self.expression()?))
                };
                self.expect_operator(']')?;
                node = Expr::Slice {
                    array: Box::new(node),
                    start,
                    stop,
                };
            } else {
                let index =
                    start.ok_or_else(|| GoblinError::parse("Array index cannot be empty."))?;
                self.expect_operator(']')?;
                node = Expr::Index {
                    array: Box::new(node),
                    index,
                };
            }
        }
        Ok(node)
    }

    fn primary(&mut self) -> Result<Expr> {
        match self.advance() {
            Token {
                kind: TokenKind::Operator('['),
                ..
            } => {
                let mut items = Vec::new();
                if !self.operator_is(']') {
                    items.push(self.expression()?);
                    while self.accept_operator(',') {
                        items.push(self.expression()?);
                    }
                }
                self.expect_operator(']')?;
                Ok(Expr::Array(items))
            }
            Token {
                kind: TokenKind::Number(raw),
                ..
            } => {
                let value = raw.parse::<f64>().map_err(|error| {
                    GoblinError::parse(format!("Invalid number {raw}: {error}"))
                })?;
                if self.mode != SyntaxMode::LegacyQuantity && self.unit_start() {
                    let units = self.unit_expression(0, self.cursor)?;
                    // Retain the exact historical AST/hash for a simple unit.
                    if let Expr::QuantityLiteral { unit, .. } = units {
                        return Ok(Expr::QuantityLiteral { value, unit });
                    }
                    return Ok(Expr::Binary {
                        op: '*',
                        left: Box::new(Expr::Number(value)),
                        right: Box::new(units),
                    });
                }
                if let TokenKind::Ident(name) = &self.current().kind
                    && is_unit(name)
                {
                    let unit = name.clone();
                    self.cursor += 1;
                    return Ok(Expr::QuantityLiteral { value, unit });
                }
                Ok(Expr::Number(value))
            }
            Token {
                kind: TokenKind::Text(value),
                ..
            } => Ok(Expr::Text(value)),
            Token {
                kind: TokenKind::Ident(name),
                ..
            } => {
                if name == "true" || name == "false" {
                    return Ok(Expr::Bool(name == "true"));
                }
                if self.accept_operator('(') {
                    let mut args = Vec::new();
                    if !self.operator_is(')') {
                        args.push(self.expression()?);
                        while self.accept_operator(',') {
                            args.push(self.expression()?);
                        }
                    }
                    self.expect_operator(')')?;
                    Ok(Expr::Call { name, args })
                } else {
                    let canonical_id = resolve(&name).map(|constant| constant.id.to_string());
                    Ok(Expr::Name {
                        source: name,
                        canonical_id,
                    })
                }
            }
            Token {
                kind: TokenKind::Operator('('),
                ..
            } => {
                let expression = self.expression()?;
                self.expect_operator(')')?;
                Ok(expression)
            }
            token => Err(GoblinError::parse(format!(
                "Unexpected token {:?} at {}.",
                token.describe(),
                token.position
            ))),
        }
    }

    fn unit_start(&self) -> bool {
        match &self.current().kind {
            TokenKind::Ident(name) => is_unit(name),
            TokenKind::Number(raw) if raw == "1" => {
                self.tokens
                    .get(self.cursor + 1)
                    .is_some_and(|t| matches!(t.kind, TokenKind::Operator('/')))
                    && self.tokens.get(self.cursor + 2).is_some_and(|t| {
                        matches!(&t.kind, TokenKind::Ident(name) if is_unit(name))
                            || matches!(t.kind, TokenKind::Operator('('))
                    })
            }
            // Numeric implicit multiplication by an ordinary parenthesized
            // expression stays ordinary, even when a variable is unit-spelled.
            // Grouped units are accepted after a suffix's explicit * or /.
            _ => false,
        }
    }

    /// A suffix contains only registered units and signed integer powers.
    /// Arithmetic operands (numbers, variables, calls) remain in the ordinary
    /// expression parser. Parenthesize denominators with several factors.
    fn unit_expression(&mut self, depth: usize, start: usize) -> Result<Expr> {
        if depth > 32 {
            return Err(GoblinError::parse(
                "Compound units may nest at most 32 groups.",
            ));
        }
        let mut node = self.unit_power(depth, start)?;
        let mut divided = false;
        loop {
            if !(self.operator_is('*') || self.operator_is('/')) {
                break;
            }
            let checkpoint = self.cursor;
            let op = self.expect_any_operator()?;
            if !self.unit_factor_start() {
                self.cursor = checkpoint;
                break;
            }
            if divided {
                return Err(GoblinError::parse(
                    "AMBIGUOUS COMPOUND UNIT\nUse parentheses for multiple denominator factors, e.g. 3 kg/(m*s^2), or parenthesize the completed quantity before further unit arithmetic.",
                ));
            }
            divided = op == '/';
            node = Expr::Binary {
                op,
                left: Box::new(node),
                right: Box::new(self.unit_power(depth, start)?),
            };
            Self::validate_units(&node)?;
        }
        Ok(node)
    }

    fn unit_power(&mut self, depth: usize, start: usize) -> Result<Expr> {
        if self.cursor.saturating_sub(start) >= 256 {
            return Err(GoblinError::parse(
                "Compound-unit suffixes may contain at most 256 tokens.",
            ));
        }
        let node = match self.advance().kind {
            TokenKind::Ident(name) if is_unit(&name) => Expr::QuantityLiteral {
                value: 1.0,
                unit: name,
            },
            TokenKind::Number(raw) if raw == "1" => Expr::Number(1.0),
            TokenKind::Operator('(') => {
                let units = self.unit_expression(depth + 1, start)?;
                self.expect_operator(')')?;
                units
            }
            _ => {
                return Err(GoblinError::parse(
                    "Expected a registered unit in compound-unit literal.",
                ));
            }
        };
        if self.cursor.saturating_sub(start) > 256 {
            return Err(GoblinError::parse(
                "Compound-unit suffixes may contain at most 256 tokens.",
            ));
        }
        if !self.accept_operator('^') {
            return Ok(node);
        }
        let sign = if self.accept_operator('-') {
            -1
        } else {
            self.accept_operator('+');
            1
        };
        let raw = match self.advance().kind {
            TokenKind::Number(raw) if raw.bytes().all(|b| b.is_ascii_digit()) => raw,
            _ => {
                return Err(GoblinError::parse(
                    "UNIT EXPONENT MUST BE AN INTEGER\nUse a signed integer between -32 and 32, e.g. m^2 or s^-2. Parenthesize a whole quantity before applying an arithmetic power.",
                ));
            }
        };
        let exponent = raw
            .parse::<i32>()
            .ok()
            .and_then(|v| v.checked_mul(sign))
            .filter(|v| (-32..=32).contains(v))
            .ok_or_else(|| GoblinError::parse("Unit exponents must be between -32 and 32."))?;
        if self.cursor.saturating_sub(start) > 256 {
            return Err(GoblinError::parse(
                "Compound-unit suffixes may contain at most 256 tokens.",
            ));
        }
        if self.operator_is('^') {
            return Err(GoblinError::parse(
                "Chained unit powers are ambiguous. Use a single signed integer exponent.",
            ));
        }
        let powered = Expr::Binary {
            op: '^',
            left: Box::new(node),
            right: Box::new(Expr::Number(f64::from(exponent))),
        };
        Self::validate_units(&powered)?;
        Ok(powered)
    }

    fn unit_factor_start(&self) -> bool {
        if self.unit_start() {
            return true;
        }
        let mut cursor = self.cursor;
        while self
            .tokens
            .get(cursor)
            .is_some_and(|t| matches!(t.kind, TokenKind::Operator('(')))
        {
            cursor += 1;
            if cursor - self.cursor > 32 {
                return true;
            }
        }
        cursor > self.cursor
            && self.tokens.get(cursor).is_some_and(|t| {
                matches!(&t.kind, TokenKind::Ident(name) if is_unit(name))
                    || (matches!(&t.kind, TokenKind::Number(raw) if raw == "1")
                        && self.tokens.get(cursor + 1).is_some_and(|next| {
                            matches!(next.kind, TokenKind::Operator('/' | '*' | ')'))
                        }))
            })
    }

    fn validate_units(node: &Expr) -> Result<crate::quantity::Quantity> {
        use crate::quantity::Quantity;
        let value = match node {
            Expr::QuantityLiteral { unit, .. } => Quantity::from_unit(1.0, unit)?,
            Expr::Number(_) => Quantity::scalar(1.0)?,
            Expr::Binary {
                op: '^',
                left,
                right,
            } => {
                let base = Self::validate_units(left)?;
                let Expr::Number(exponent) = **right else {
                    unreachable!("unit exponent")
                };
                let power = exponent as i32;
                if base.dimension.iter().any(|d| {
                    d.checked_mul(power)
                        .is_none_or(|v| !(-256..=256).contains(&v))
                }) {
                    return Err(GoblinError::parse(
                        "Combined unit dimension exponents must be between -256 and 256.",
                    ));
                }
                base.powi(power)?
            }
            Expr::Binary { op, left, right } => {
                let a = Self::validate_units(left)?;
                let b = Self::validate_units(right)?;
                if a.dimension.iter().zip(b.dimension).any(|(x, y)| {
                    let combined = if *op == '*' {
                        x.checked_add(y)
                    } else {
                        x.checked_sub(y)
                    };
                    combined.is_none_or(|v| !(-256..=256).contains(&v))
                }) {
                    return Err(GoblinError::parse(
                        "Combined unit dimension exponents must be between -256 and 256.",
                    ));
                }
                if *op == '*' {
                    a.checked_mul(b)?
                } else {
                    a.checked_div(b)?
                }
            }
            _ => unreachable!("unit-only expression"),
        };
        if value.value_si == 0.0 {
            return Err(GoblinError::numeric(
                "Compound unit scale underflowed to zero; simplify the unit expression.",
            ));
        }
        Ok(value)
    }

    fn ident_is(&self, expected: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Ident(value) if value == expected)
    }
    fn operator_is(&self, expected: char) -> bool {
        matches!(self.current().kind, TokenKind::Operator(value) if value == expected)
    }
    fn accept_operator(&mut self, expected: char) -> bool {
        if self.operator_is(expected) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }
    fn expect_operator(&mut self, expected: char) -> Result<()> {
        if self.accept_operator(expected) {
            Ok(())
        } else {
            Err(GoblinError::parse(format!(
                "Expected {expected:?}, got {:?} at {}.",
                self.current().describe(),
                self.current().position
            )))
        }
    }
    fn expect_any_operator(&mut self) -> Result<char> {
        match self.advance().kind {
            TokenKind::Operator(value) => Ok(value),
            _ => Err(GoblinError::parse("Expected operator.")),
        }
    }
    fn expect_ident(&mut self) -> Result<String> {
        match self.advance() {
            Token {
                kind: TokenKind::Ident(value),
                ..
            } => Ok(value),
            token => Err(GoblinError::parse(format!(
                "Expected identifier, got {:?} at {}.",
                token.describe(),
                token.position
            ))),
        }
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<()> {
        if self.ident_is(keyword) {
            self.advance();
            Ok(())
        } else {
            Err(GoblinError::parse(format!(
                "Expected {keyword:?} at {}.",
                self.current().position
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notation_forms_are_canonical() {
        let explicit = parse_source("mass = 1 kg\nenergy = mass * c^2\n").unwrap();
        let unicode = parse_source("mass = 1 kg\nenergy = mass c²\n").unwrap();
        assert_eq!(
            explicit.canonical_sha256().unwrap(),
            unicode.canonical_sha256().unwrap()
        );
    }

    #[test]
    fn pi_aliases_are_canonical() {
        assert_eq!(
            parse_source("x = pi\n")
                .unwrap()
                .canonical_sha256()
                .unwrap(),
            parse_source("x = π\n").unwrap().canonical_sha256().unwrap()
        );
    }
}
