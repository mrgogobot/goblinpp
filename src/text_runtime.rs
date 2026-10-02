// This self-contained module is also embedded verbatim in generated native programs.
// Keep its public functions independent of the Goblin++ crate.
pub const MAX_TEXT_BYTES: usize = 1_048_576;
pub const MAX_TEXT_PARTS: usize = 100_000;
pub const SEMANTICS_POLICY: &str = "goblin.eager-text-and-protected-values.v1";

/// Capture source-literal placeholders exactly once. Resolver results are plain text,
/// never scanned again. Double braces escape placeholders; JSON/code braces are literal.
pub fn snapshot(
    template: &str,
    mut resolve: impl FnMut(&str, Option<&str>) -> Result<String, String>,
) -> Result<String, String> {
    let mut output = String::new();
    let mut cursor = 0;
    let mut escaped_openings = 0;
    while cursor < template.len() {
        let remaining = &template[cursor..];
        if remaining.starts_with("{{") {
            output.push('{');
            escaped_openings += 1;
            cursor += 2;
        } else if remaining.starts_with("}}") && escaped_openings > 0 {
            output.push('}');
            escaped_openings -= 1;
            cursor += 2;
        } else if let Some(opened) = remaining.strip_prefix('{') {
            if let Some(end) = remaining.find('}') {
                let field = &remaining[1..end];
                let (name, spec) = field
                    .split_once(':')
                    .map_or((field, None), |(name, spec)| (name, Some(spec)));
                if template_name(name) {
                    output.push_str(&resolve(name, spec)?);
                    cursor += end + 1;
                } else if name
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_alphabetic() || ch == '_')
                {
                    return Err(format!(
                        "Invalid text interpolation {{{field}}}. Use {{{{ and }}}} for literal placeholders."
                    ));
                } else {
                    output.push('{');
                    cursor += 1;
                }
            } else {
                if template_name(opened.split(':').next().unwrap_or("")) {
                    return Err(
                        "Unclosed text interpolation. Use {{ for a literal opening brace.".into(),
                    );
                }
                output.push('{');
                cursor += 1;
            }
        } else {
            let ch = remaining.chars().next().unwrap();
            output.push(ch);
            cursor += ch.len_utf8();
        }
        if output.len() > MAX_TEXT_BYTES {
            return Err(format!("Text result exceeds {MAX_TEXT_BYTES} UTF-8 bytes."));
        }
    }
    bounded(output)
}

fn template_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().enumerate().all(|(index, ch)| {
            ch == '_'
                || if index == 0 {
                    ch.is_alphabetic()
                } else {
                    ch.is_alphanumeric()
                }
        })
}

pub fn bounded(value: String) -> Result<String, String> {
    if value.len() > MAX_TEXT_BYTES {
        Err(format!("Text result exceeds {MAX_TEXT_BYTES} UTF-8 bytes."))
    } else {
        Ok(value)
    }
}

pub fn concat(left: &str, right: &str) -> Result<String, String> {
    let len = left
        .len()
        .checked_add(right.len())
        .ok_or("Text length overflow.")?;
    if len > MAX_TEXT_BYTES {
        return Err(format!("Text result exceeds {MAX_TEXT_BYTES} UTF-8 bytes."));
    }
    let mut result = String::with_capacity(len);
    result.push_str(left);
    result.push_str(right);
    Ok(result)
}

pub fn length(value: &str) -> usize {
    value.chars().count()
}

pub fn replace(value: &str, old: &str, new: &str) -> Result<String, String> {
    if old.is_empty() {
        return Err("str_replace() search text cannot be empty.".into());
    }
    let count = value.matches(old).count();
    let removed = count
        .checked_mul(old.len())
        .ok_or("Text length overflow.")?;
    let added = count
        .checked_mul(new.len())
        .ok_or("Text length overflow.")?;
    let len = value
        .len()
        .checked_sub(removed)
        .and_then(|n| n.checked_add(added))
        .ok_or("Text length overflow.")?;
    if len > MAX_TEXT_BYTES {
        return Err(format!("Text result exceeds {MAX_TEXT_BYTES} UTF-8 bytes."));
    }
    bounded(value.replace(old, new))
}

pub fn split(value: &str, separator: &str) -> Result<Vec<String>, String> {
    if separator.is_empty() {
        return Err("str_split() separator cannot be empty.".into());
    }
    let mut parts = Vec::new();
    for part in value.split(separator) {
        if parts.len() >= MAX_TEXT_PARTS {
            return Err(format!("str_split() exceeds {MAX_TEXT_PARTS} parts."));
        }
        parts.push(bounded(part.to_string())?);
    }
    Ok(parts)
}

pub fn join(separator: &str, parts: &[String]) -> Result<String, String> {
    if parts.len() > MAX_TEXT_PARTS {
        return Err(format!("str_join() exceeds {MAX_TEXT_PARTS} parts."));
    }
    let mut len = 0usize;
    for part in parts {
        len = len.checked_add(part.len()).ok_or("Text length overflow.")?;
    }
    let separators = parts
        .len()
        .saturating_sub(1)
        .checked_mul(separator.len())
        .ok_or("Text length overflow.")?;
    len = len.checked_add(separators).ok_or("Text length overflow.")?;
    if len > MAX_TEXT_BYTES {
        return Err(format!("Text result exceeds {MAX_TEXT_BYTES} UTF-8 bytes."));
    }
    Ok(parts.join(separator))
}

pub fn parse_number(value: &str) -> Result<f64, String> {
    let value = value.trim();
    if value.len() > 256 || !decimal_syntax(value) {
        return Err("parse_number() requires a finite, unitless decimal number.".into());
    }
    if !value.contains(['.', 'e', 'E']) {
        const MAX_EXACT_INTEGER: i128 = 9_007_199_254_740_991;
        let integer: i128 = value
            .parse()
            .map_err(|_| "parse_number() integer is outside the exact range.".to_string())?;
        if !(-MAX_EXACT_INTEGER..=MAX_EXACT_INTEGER).contains(&integer) {
            return Err("parse_number() integer is outside the exact range.".into());
        }
    }
    let number: f64 = value
        .parse()
        .map_err(|_| "parse_number() requires a finite, unitless decimal number.")?;
    if !number.is_finite() {
        return Err("parse_number() refuses NaN, infinity, and overflow.".into());
    }
    if number == 0.0
        && value
            .split(['e', 'E'])
            .next()
            .unwrap_or("")
            .bytes()
            .any(|digit| (b'1'..=b'9').contains(&digit))
    {
        return Err("parse_number() refuses a nonzero value that underflows to zero.".into());
    }
    Ok(number)
}

pub fn parse_integer(value: &str) -> Result<f64, String> {
    const MAX_EXACT_INTEGER: i128 = 9_007_199_254_740_991;
    let value = value.trim();
    let bytes = value.as_bytes();
    let digits = bytes
        .strip_prefix(b"+")
        .or_else(|| bytes.strip_prefix(b"-"))
        .unwrap_or(bytes);
    if value.len() > 32 || digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return Err("parse_integer() requires signed or unsigned decimal integer text.".into());
    }
    let integer: i128 = value
        .parse()
        .map_err(|_| "parse_integer() integer is outside the exact range.".to_string())?;
    if !(-MAX_EXACT_INTEGER..=MAX_EXACT_INTEGER).contains(&integer) {
        return Err("parse_integer() integer is outside the exact range.".into());
    }
    Ok(integer as f64)
}

fn decimal_syntax(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut pos = 0;
    if bytes.get(pos).is_some_and(|c| *c == b'+' || *c == b'-') {
        pos += 1;
    }
    let mut digits = 0;
    while bytes.get(pos).is_some_and(u8::is_ascii_digit) {
        pos += 1;
        digits += 1;
    }
    if bytes.get(pos) == Some(&b'.') {
        pos += 1;
        while bytes.get(pos).is_some_and(u8::is_ascii_digit) {
            pos += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return false;
    }
    if bytes.get(pos).is_some_and(|c| *c == b'e' || *c == b'E') {
        pos += 1;
        if bytes.get(pos).is_some_and(|c| *c == b'+' || *c == b'-') {
            pos += 1;
        }
        let start = pos;
        while bytes.get(pos).is_some_and(u8::is_ascii_digit) {
            pos += 1;
        }
        if pos == start {
            return false;
        }
    }
    pos == bytes.len()
}
