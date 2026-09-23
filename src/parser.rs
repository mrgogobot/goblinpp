use crate::ast::{Expr, Program, Stmt};
use crate::constants::resolve;
use crate::error::{GoblinError, Result};
use crate::hashing::sha256_bytes;
use crate::inline_rust::{ExtractedSource, InlineRustBlock, extract};
use crate::lexer::{Token, TokenKind, lex};
use crate::quantity::is_unit;
use std::collections::HashSet;

pub fn reserved_function_name(name: &str) -> bool {
    matches!(
        name,
        "g_func"
            | "return"
            | "GO_PARANOID"
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
    ) || name.starts_with("__goblin_")
        || resolve(name).is_some()
        || is_unit(name)
}

#[derive(Debug, Clone)]
pub struct ParsedSource {
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
    let ExtractedSource {
        language_source,
        blocks,
    } = extract(source)?;
    let mut program = Parser::new(lex(&language_source)?).parse()?;
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
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            cursor: 0,
            in_function: false,
            loop_depth: 0,
        }
    }
    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
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
        if self.ident_is("g_func") {
            if in_block {
                return Err(GoblinError::parse("g_func declarations must be top-level."));
            }
            self.advance();
            let name = self.expect_ident()?;
            if reserved_function_name(&name) {
                return Err(GoblinError::parse(format!(
                    "{name:?} cannot name a g_func."
                )));
            }
            self.expect_operator('(')?;
            let mut params = Vec::new();
            if !self.operator_is(')') {
                loop {
                    let param = self.expect_ident()?;
                    if reserved_function_name(&param) || params.contains(&param) {
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
            if reserved_function_name(&variable) {
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
