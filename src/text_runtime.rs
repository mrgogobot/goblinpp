// This self-contained module is also embedded verbatim in generated native programs.
// Keep its public functions independent of the Goblin++ crate.
pub const MAX_TEXT_BYTES: usize = 1_048_576;
pub const MAX_TEXT_PARTS: usize = 100_000;
pub const SEMANTICS_POLICY: &str = "goblin.eager-text-and-roundtrip-numbers.v2";

/// Shortest round-trip decimal text for finite f64 values, including signed zero.
/// Scientific notation also avoids parse_number's exact-integer input guard for
/// large floating-point values. Explicit presentation precision is separate.
pub fn format_number(value: f64) -> String {
    if value != 0.0 && (value.abs() >= 1e15 || value.abs() < 1e-4) {
        let raw = format!("{value:e}");
        if let Some((mantissa, exponent)) = raw.split_once('e') {
            let exponent: i32 = exponent.parse().expect("Rust f64 exponent");
            return format!("{mantissa}e{exponent:+}");
        }
        raw
    } else {
        value.to_string()
    }
}

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

/// Additive helpers; existing interpolation and comparison policies are unchanged.
pub fn toolkit_arity(name: &str) -> Option<usize> {
    match name {
        "str_upper" | "str_lower" | "str_bytes_len" | "sizeof" | "str_to_hex" | "str_from_hex"
        | "str_to_octal" | "str_from_octal" | "str_to_binary" | "str_from_binary" => Some(1),
        "str_compare" | "to_base" | "from_base" => Some(2),
        _ => None,
    }
}

pub fn change_case(value: &str, upper: bool) -> Result<String, String> {
    // Preflight expansion before allocation, then retain whole-string contextual
    // Unicode rules (notably Greek final sigma). No locale-specific tailoring.
    let mut size = 0usize;
    for ch in value.chars() {
        let mapped = if upper {
            ch.to_uppercase().to_string()
        } else {
            ch.to_lowercase().to_string()
        };
        size += mapped.len();
        if size > MAX_TEXT_BYTES {
            return Err(format!("Text result exceeds {MAX_TEXT_BYTES} UTF-8 bytes."));
        }
    }
    bounded(if upper {
        value.to_uppercase()
    } else {
        value.to_lowercase()
    })
}

/// A locale-independent total ordering, with no normalization or case folding.
pub fn compare(left: &str, right: &str) -> f64 {
    match left.cmp(right) {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Equal => 0.0,
        std::cmp::Ordering::Greater => 1.0,
    }
}

fn byte_codec(name: &str) -> Result<(u32, usize, bool), String> {
    Ok(match name {
        "str_to_hex" => (16, 2, true),
        "str_from_hex" => (16, 2, false),
        "str_to_octal" => (8, 3, true),
        "str_from_octal" => (8, 3, false),
        "str_to_binary" => (2, 8, true),
        "str_from_binary" => (2, 8, false),
        _ => return Err(format!("Unknown text encoding: {name}")),
    })
}

/// Fixed-width UTF-8 byte groups; not encryption or integer formatting.
pub fn byte_encoding(name: &str, value: &str) -> Result<String, String> {
    let (base, width, encode) = byte_codec(name)?;
    if value.len() > MAX_TEXT_BYTES {
        return Err(format!(
            "{name}() input exceeds {MAX_TEXT_BYTES} UTF-8 bytes."
        ));
    }
    if encode {
        let size = value
            .len()
            .checked_mul(width)
            .filter(|n| *n <= MAX_TEXT_BYTES)
            .ok_or_else(|| format!("{name}() result exceeds {MAX_TEXT_BYTES} UTF-8 bytes."))?;
        let mut output = String::with_capacity(size);
        for b in value.bytes() {
            // Byte groups are small and always fit the prechecked output size.
            output.push_str(&match base {
                16 => format!("{b:02x}"),
                8 => format!("{b:03o}"),
                _ => format!("{b:08b}"),
            });
        }
        Ok(output)
    } else {
        if !value.is_ascii() || !value.len().is_multiple_of(width) {
            return Err(format!(
                "{name}() requires complete {width}-digit ASCII byte groups; no prefixes or separators."
            ));
        }
        let mut bytes = Vec::with_capacity(value.len() / width);
        for group in value.as_bytes().chunks(width) {
            let digits = std::str::from_utf8(group).expect("checked ASCII");
            if !digits.bytes().all(|b| (b as char).is_digit(base)) {
                return Err(format!(
                    "{name}() invalid base-{base} byte group: {digits:?}."
                ));
            }
            bytes.push(
                u8::from_str_radix(digits, base)
                    .map_err(|_| format!("{name}() byte group is outside 0..255: {digits:?}."))?,
            );
        }
        String::from_utf8(bytes).map_err(|_| format!("{name}() decoded bytes are not valid UTF-8."))
    }
}

pub fn checked_base(base: f64) -> Result<u32, String> {
    match base {
        2.0 => Ok(2),
        8.0 => Ok(8),
        10.0 => Ok(10),
        16.0 => Ok(16),
        _ => Err("Base must be exactly 2, 8, 10 or 16.".into()),
    }
}

pub fn to_base(value: f64, base: f64) -> Result<String, String> {
    let radix = checked_base(base)?;
    if !value.is_finite() || value.fract() != 0.0 || value.abs() > 9_007_199_254_740_991.0 {
        return Err("to_base() requires an integer in the exact range ±(2^53-1).".into());
    }
    let magnitude = value.abs() as u64;
    let digits = match radix {
        2 => format!("{magnitude:b}"),
        8 => format!("{magnitude:o}"),
        16 => format!("{magnitude:x}"),
        _ => magnitude.to_string(),
    };
    Ok(if value < 0.0 {
        format!("-{digits}")
    } else {
        digits
    })
}

pub fn from_base(value: &str, base: f64) -> Result<f64, String> {
    let radix = checked_base(base)?;
    let value = value.trim();
    let (negative, digits) = if let Some(v) = value.strip_prefix('-') {
        (true, v)
    } else {
        (false, value.strip_prefix('+').unwrap_or(value))
    };
    if digits.is_empty()
        || value.len() > 64
        || !digits
            .bytes()
            .all(|b| b.is_ascii() && (b as char).is_digit(radix))
    {
        return Err(
            "from_base() requires signed integer digits; no prefixes, separators or fractions."
                .into(),
        );
    }
    let magnitude = u64::from_str_radix(digits, radix)
        .ok()
        .filter(|v| *v <= 9_007_199_254_740_991)
        .ok_or("from_base() integer is outside the exact range ±(2^53-1).")?;
    Ok(if negative {
        -(magnitude as f64)
    } else {
        magnitude as f64
    })
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
