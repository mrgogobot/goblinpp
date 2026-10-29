//! Explicit ECSV 1.0 scalar subset. No YAML constructors, tags, aliases or code.
use super::{MAX_BYTES, MAX_ITEMS, fail};
use crate::{error::Result, evaluator::Value, quantity::Quantity};
use std::collections::BTreeMap;
#[derive(Debug)]
pub struct Column {
    pub name: String,
    pub dtype: String,
    pub unit: String,
}
pub struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
}
fn unquote(s: &str) -> Result<String> {
    let s = s.trim();
    if s.starts_with('"') {
        return serde_json::from_str::<String>(s)
            .map_err(|_| fail("Invalid double-quoted ECSV header scalar."));
    }
    if s.starts_with('\'') {
        if !s.ends_with('\'') || s.len() < 2 {
            return Err(fail("Invalid single-quoted ECSV scalar."));
        }
        let inner = &s[1..s.len() - 1];
        let mut out = String::new();
        let mut chars = inner.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\'' && chars.next() != Some('\'') {
                return Err(fail("Invalid YAML quote escape."));
            }
            out.push(c);
        }
        return Ok(out);
    }
    if s.is_empty()
        || s.starts_with(['!', '&', '*', '[', '{', '|', '>'])
        || s.contains(['#', '\'', '"', '\n', '\r'])
    {
        return Err(fail(
            "Unsupported ECSV header scalar; tags, aliases and nested metadata are refused.",
        ));
    }
    Ok(s.to_owned())
}
fn mapping(s: &str) -> Result<BTreeMap<String, String>> {
    let inner = s
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .ok_or_else(|| fail("ECSV columns require single-line scalar flow mappings."))?;
    let (mut pieces, mut start, mut quote, mut escaped) = (Vec::new(), 0, None, false);
    for (i, c) in inner.char_indices() {
        if let Some(q) = quote {
            if q == '"' && c == '\\' && !escaped {
                escaped = true;
                continue;
            }
            if c == q && !escaped {
                quote = None;
            }
            escaped = false;
        } else if matches!(c, '\'' | '"') {
            quote = Some(c);
        } else if c == ',' {
            pieces.push(&inner[start..i]);
            start = i + 1;
        }
    }
    if quote.is_some() {
        return Err(fail("Unclosed ECSV header quote."));
    }
    pieces.push(&inner[start..]);
    let mut out = BTreeMap::new();
    for field in pieces {
        let (key, value) = field
            .split_once(':')
            .ok_or_else(|| fail("Malformed ECSV column mapping."))?;
        let key = unquote(key)?;
        let value = unquote(value)?;
        if !matches!(
            key.as_str(),
            "name" | "datatype" | "unit" | "description" | "format"
        ) || out.insert(key, value).is_some()
        {
            return Err(fail(
                "ECSV column has duplicate/unsupported metadata (including subtype/mixins).",
            ));
        }
    }
    Ok(out)
}
impl Table {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_BYTES {
            return Err(fail("ECSV input exceeds 16 MiB."));
        }
        let text = std::str::from_utf8(bytes).map_err(|_| fail("ECSV must be UTF-8."))?;
        let mut lines = text.split_inclusive('\n');
        if lines.next().map(str::trim_end) != Some("# %ECSV 1.0") {
            return Err(fail(
                "ECSV must begin with # %ECSV 1.0 (no inferred format).",
            ));
        }
        let (mut columns, mut delimiter, mut datatype, mut schema, mut boundary) =
            (Vec::new(), None, false, false, false);
        let mut meta = false;
        let mut offset = text.find('\n').map_or(text.len(), |i| i + 1);
        let mut header_bytes = offset;
        for line in lines {
            if !line.starts_with('#') {
                break;
            }
            offset += line.len();
            header_bytes += line.len();
            if header_bytes > 65536 {
                return Err(fail("ECSV header exceeds 64 KiB."));
            }
            let s = line[1..].trim();
            if s.is_empty() {
                continue;
            }
            if s == "---" && !boundary {
                boundary = true;
                continue;
            }
            if !boundary {
                return Err(fail("ECSV YAML document marker --- is required."));
            }
            if s == "datatype:" && !datatype {
                datatype = true;
                continue;
            }
            if let Some(row) = s.strip_prefix("- ") {
                if !datatype {
                    return Err(fail("ECSV datatype must precede its column mappings."));
                }
                let mut fields = mapping(row)?;
                let name = fields
                    .remove("name")
                    .ok_or_else(|| fail("ECSV column name missing."))?;
                let dtype = fields
                    .remove("datatype")
                    .ok_or_else(|| fail("ECSV column datatype missing."))?;
                if !matches!(
                    dtype.as_str(),
                    "bool"
                        | "string"
                        | "int8"
                        | "int16"
                        | "int32"
                        | "int64"
                        | "uint8"
                        | "uint16"
                        | "uint32"
                        | "uint64"
                        | "float32"
                        | "float64"
                ) {
                    return Err(fail(format!("Unsupported ECSV scalar datatype {dtype:?}.")));
                }
                if name.is_empty()
                    || columns.iter().any(|c: &Column| c.name == name)
                    || columns.len() >= 1024
                {
                    return Err(fail(
                        "ECSV column names must be nonempty/unique; maximum 1024 columns.",
                    ));
                }
                columns.push(Column {
                    name,
                    dtype,
                    unit: fields.remove("unit").unwrap_or_default(),
                });
                continue;
            }
            if let Some(v) = s.strip_prefix("delimiter:") {
                if delimiter.is_some() {
                    return Err(fail("Duplicate ECSV delimiter."));
                }
                delimiter = Some(match unquote(v)?.as_str() {
                    "," => b',',
                    " " => b' ',
                    _ => return Err(fail("ECSV delimiter must be comma or space.")),
                });
                continue;
            }
            if let Some(v) = s.strip_prefix("schema:") {
                if schema || unquote(v)? != "astropy-2.0" {
                    return Err(fail("Unsupported or duplicate ECSV schema."));
                }
                schema = true;
                continue;
            }
            if s == "meta: {}" && !meta {
                meta = true;
                continue;
            }
            return Err(fail(
                "Unsupported ECSV YAML metadata/mixin. This stage accepts scalar column flow mappings, not arbitrary YAML.",
            ));
        }
        if columns.is_empty() || !boundary {
            return Err(fail("ECSV needs a datatype list and document marker."));
        }
        let mut rows = crate::delimited::parse_ecsv(&text[offset..], delimiter.unwrap_or(b' '))?;
        if rows.is_empty()
            || rows.remove(0) != columns.iter().map(|c| c.name.clone()).collect::<Vec<_>>()
        {
            return Err(fail(
                "ECSV data header differs from the declared column names/order.",
            ));
        }
        if rows.len() > MAX_ITEMS
            || rows
                .iter()
                .any(|r| r.len() != columns.len() || r.iter().any(String::is_empty))
        {
            return Err(fail(
                "ECSV row width, empty/masked cells or row limit refused. No missing values are silently filled or dropped.",
            ));
        }
        Ok(Self { columns, rows })
    }
    pub fn index(&self, name: &str) -> Result<usize> {
        self.columns
            .iter()
            .position(|c| c.name == name)
            .ok_or_else(|| fail(format!("ECSV has no exact column {name:?}.")))
    }
    pub fn read(&self, col: usize, numbers_only: bool) -> Result<Value> {
        let column = &self.columns[col];
        if numbers_only && matches!(column.dtype.as_str(), "string" | "bool") {
            return Err(fail(
                "ecsv_numbers requires an explicitly numeric datatype.",
            ));
        }
        let factor = if matches!(column.dtype.as_str(), "string" | "bool") {
            if !column.unit.is_empty() {
                return Err(fail("ECSV units on nonnumeric columns are unsupported."));
            }
            Quantity::scalar(1.0)?
        } else {
            unit_factor(&column.unit)?
        };
        let values = self.rows.iter().enumerate().map(|(row, cells)| {
            let cell = &cells[col];
            if column.dtype == "string" {
                return crate::text_runtime::bounded(cell.clone()).map(Value::Text).map_err(fail);
            }
            if column.dtype == "bool" {
                return match cell.as_str() {
                    "True" => Ok(Value::Bool(true)),
                    "False" => Ok(Value::Bool(false)),
                    _ => Err(fail(format!("Invalid ECSV boolean at data row {}.", row + 1))),
                };
            }
            let value = if column.dtype.starts_with("float") {
                let number = crate::text_runtime::parse_number(cell)
                    .map_err(|e| fail(format!("ECSV data row {}: {e}", row + 1)))?;
                if column.dtype == "float32" {
                    let single = number as f32;
                    if !single.is_finite() || single == 0.0 && number != 0.0 {
                        return Err(fail(format!("ECSV float32 overflow/underflow at data row {}.", row + 1)));
                    }
                    f64::from(single)
                } else { number }
            } else {
                let unsigned = column.dtype.starts_with('u');
                let bits = column.dtype.trim_start_matches("uint").trim_start_matches("int").parse::<u32>().unwrap();
                let integer = cell.parse::<i128>()
                    .map_err(|_| fail(format!("Invalid ECSV integer at data row {}.", row + 1)))?;
                let (lo, hi) = if unsigned { (0, (1i128 << bits) - 1) }
                    else { (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1) };
                if integer < lo || integer > hi {
                    return Err(fail(format!("ECSV integer outside declared dtype at data row {}.", row + 1)));
                }
                if !(-9_007_199_254_740_991..=9_007_199_254_740_991).contains(&integer) {
                    return Err(fail("ECSV integer exceeds exact numeric range; use ecsv_column for exact catalogue-ID text."));
                }
                integer as f64
            };
            let result = Quantity::scalar(value)?.checked_mul(factor)?;
            if result.value_si == 0.0 && value != 0.0 {
                return Err(fail("ECSV unit conversion underflowed; no zero was substituted."));
            }
            Ok(Value::Quantity(result))
        }).collect::<Result<Vec<_>>>()?;
        Ok(Value::Array(values))
    }
}

/// Parse a restricted Astropy-style unit expression using only registered units.
/// No Goblin source evaluation, constants, functions, YAML tags or constructors.
fn unit_factor(text: &str) -> Result<Quantity> {
    if text.trim().is_empty() {
        return Quantity::scalar(1.0);
    }
    struct Units<'a> {
        s: &'a str,
        at: usize,
        tokens: usize,
    }
    impl Units<'_> {
        fn ws(&mut self) {
            while self.s[self.at..].starts_with(char::is_whitespace) {
                self.at += self.s[self.at..].chars().next().unwrap().len_utf8();
            }
        }
        fn take(&mut self, s: &str) -> bool {
            self.ws();
            if self.s[self.at..].starts_with(s) {
                self.at += s.len();
                true
            } else {
                false
            }
        }
        fn expr(&mut self, depth: usize) -> Result<Quantity> {
            if depth > 16 {
                return Err(fail("ECSV unit nesting exceeds 16."));
            }
            let mut out = self.atom(depth)?;
            loop {
                self.ws();
                if self.at == self.s.len() || self.s[self.at..].starts_with(')') {
                    break;
                }
                let divide = self.take("/");
                if !divide {
                    self.take("*");
                }
                let next = self.atom(depth)?;
                out = if divide {
                    out.checked_div(next)?
                } else {
                    out.checked_mul(next)?
                };
            }
            Ok(out)
        }
        fn atom(&mut self, depth: usize) -> Result<Quantity> {
            self.tokens += 1;
            if self.tokens > 128 {
                return Err(fail("ECSV unit expression exceeds 128 atoms."));
            }
            self.ws();
            let mut q = if self.take("(") {
                let q = self.expr(depth + 1)?;
                if !self.take(")") {
                    return Err(fail("Unclosed ECSV unit group."));
                }
                q
            } else if self.take("1") {
                Quantity::scalar(1.0)?
            } else {
                let start = self.at;
                while let Some(c) = self.s[self.at..].chars().next() {
                    if c.is_alphabetic() || c == '_' {
                        self.at += c.len_utf8();
                    } else {
                        break;
                    }
                }
                if self.at == start {
                    return Err(fail("Unsupported ECSV unit expression."));
                }
                Quantity::from_unit(1.0, &self.s[start..self.at]).map_err(|_| {
                    fail(format!(
                        "Unsupported ECSV unit {:?}; no units were discarded.",
                        &self.s[start..self.at]
                    ))
                })?
            };
            // m2 and s-2 are Astropy spellings; ^2 and **2 are also admitted.
            let adjacent = self.at;
            let explicit = self.take("**") || self.take("^");
            if !explicit && self.at != adjacent {
                return Ok(q);
            }
            let start = self.at;
            if self.s[self.at..].starts_with(['+', '-']) {
                self.at += 1;
            }
            while self
                .s
                .as_bytes()
                .get(self.at)
                .is_some_and(u8::is_ascii_digit)
            {
                self.at += 1;
            }
            if self.at != start || explicit {
                let exponent = self.s[start..self.at]
                    .parse::<i32>()
                    .ok()
                    .filter(|n| (-32..=32).contains(n))
                    .ok_or_else(|| fail("ECSV unit powers must be integers in -32..32."))?;
                q = q.powi(exponent)?;
            }
            Ok(q)
        }
    }
    if text.len() > 1024 {
        return Err(fail("ECSV unit string exceeds 1024 bytes."));
    }
    let mut p = Units {
        s: text,
        at: 0,
        tokens: 0,
    };
    let q = p.expr(0)?;
    p.ws();
    if p.at != text.len() || !q.value_si.is_finite() || q.value_si == 0.0 {
        return Err(fail("Invalid or unrepresentable ECSV unit factor."));
    }
    Ok(q)
}
