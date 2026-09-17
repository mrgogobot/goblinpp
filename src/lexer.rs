use crate::error::{GoblinError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Newline,
    Number(String),
    Text(String),
    Ident(String),
    Operator(char),
    Compare(String),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub position: usize,
}

impl Token {
    pub fn describe(&self) -> String {
        match &self.kind {
            TokenKind::Newline => "newline".into(),
            TokenKind::Number(value) => value.clone(),
            TokenKind::Text(value) => format!("\"{value}\""),
            TokenKind::Ident(value) => value.clone(),
            TokenKind::Operator(value) => value.to_string(),
            TokenKind::Compare(value) => value.clone(),
            TokenKind::Eof => "end of file".into(),
        }
    }
}

pub fn lex(source: &str) -> Result<Vec<Token>> {
    if source.contains('\r') {
        return Err(GoblinError::lex(
            "Carriage return is not accepted; Goblin++ requires LF line endings.",
        ));
    }
    let normalized = normalize_display_syntax(source);
    let mut tokens = Vec::new();
    let mut cursor = 0;
    let bytes = normalized.as_bytes();
    while cursor < bytes.len() {
        let ch = normalized[cursor..]
            .chars()
            .next()
            .expect("cursor is a char boundary");
        let width = ch.len_utf8();
        match ch {
            ' ' | '\t' => cursor += width,
            '#' => {
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
            }
            '\n' => {
                let start = cursor;
                while cursor < bytes.len() && bytes[cursor] == b'\n' {
                    cursor += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    position: start,
                });
            }
            '0'..='9' | '.'
                if ch != '.'
                    || normalized[cursor + width..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit()) =>
            {
                let start = cursor;
                let mut saw_dot = ch == '.';
                cursor += width;
                while cursor < bytes.len() {
                    let next = normalized[cursor..].chars().next().unwrap();
                    if next.is_ascii_digit() {
                        cursor += 1;
                    } else if next == '.' && !saw_dot {
                        saw_dot = true;
                        cursor += 1;
                    } else {
                        break;
                    }
                }
                if cursor < bytes.len() && matches!(bytes[cursor], b'e' | b'E') {
                    let exponent_start = cursor;
                    cursor += 1;
                    if cursor < bytes.len() && matches!(bytes[cursor], b'+' | b'-') {
                        cursor += 1;
                    }
                    let digits = cursor;
                    while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                        cursor += 1;
                    }
                    if digits == cursor {
                        cursor = exponent_start;
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::Number(normalized[start..cursor].to_string()),
                    position: start,
                });
            }
            '"' => {
                let start = cursor;
                cursor += 1;
                let mut escaped = false;
                while cursor < bytes.len() {
                    let next = normalized[cursor..].chars().next().unwrap();
                    cursor += next.len_utf8();
                    if escaped {
                        escaped = false;
                    } else if next == '\\' {
                        escaped = true;
                    } else if next == '"' {
                        break;
                    }
                }
                if !normalized[start..cursor].ends_with('"') || cursor == start + 1 {
                    return Err(GoblinError::lex(format!(
                        "Unterminated string at position {start}."
                    )));
                }
                let raw = &normalized[start..cursor];
                let value: String = serde_json::from_str(raw).map_err(|error| {
                    GoblinError::lex(format!("Invalid string at position {start}: {error}"))
                })?;
                tokens.push(Token {
                    kind: TokenKind::Text(value),
                    position: start,
                });
            }
            '<' | '>' | '!' | '=' if normalized[cursor + width..].starts_with('=') => {
                tokens.push(Token {
                    kind: TokenKind::Compare(normalized[cursor..cursor + width + 1].into()),
                    position: cursor,
                });
                cursor += width + 1;
            }
            '<' | '>' => {
                tokens.push(Token {
                    kind: TokenKind::Compare(ch.to_string()),
                    position: cursor,
                });
                cursor += width;
            }
            '=' | '+' | '-' | '*' | '/' | '^' | '(' | ')' | '[' | ']' | ':' | ',' | '{' | '}' => {
                tokens.push(Token {
                    kind: TokenKind::Operator(ch),
                    position: cursor,
                });
                cursor += width;
            }
            _ if is_ident_start(ch) => {
                let start = cursor;
                cursor += width;
                while cursor < bytes.len() {
                    let next = normalized[cursor..].chars().next().unwrap();
                    if !is_ident_continue(next) {
                        break;
                    }
                    cursor += next.len_utf8();
                }
                tokens.push(Token {
                    kind: TokenKind::Ident(normalized[start..cursor].to_string()),
                    position: start,
                });
            }
            _ => {
                return Err(GoblinError::lex(format!(
                    "Unexpected character {ch:?} at position {cursor}."
                )));
            }
        }
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        position: normalized.len(),
    });
    Ok(tokens)
}

pub fn normalize_display_syntax(source: &str) -> String {
    let mut output = String::new();
    let mut in_string = false;
    let mut in_comment = false;
    let mut escaped = false;
    for ch in source.chars() {
        if in_comment {
            output.push(ch);
            if ch == '\n' {
                in_comment = false;
            }
        } else if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
        } else {
            match ch {
                '#' => {
                    in_comment = true;
                    output.push(ch);
                }
                '"' => {
                    in_string = true;
                    output.push(ch);
                }
                '²' => output.push_str("^2"),
                '³' => output.push_str("^3"),
                _ => output.push(ch),
            }
        }
    }
    output
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || "πħωΩΔΣλμσθ∇∂".contains(ch)
}

fn is_ident_continue(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_normalization_ignores_strings_and_comments() {
        assert_eq!(
            normalize_display_syntax("x = c² # ²\nprint(\"²\")\n"),
            "x = c^2 # ²\nprint(\"²\")\n"
        );
    }
}
