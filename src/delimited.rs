//! Bounded, explicit CSV/TSV input. No inferred types or silent missing-value policy.
use crate::error::{GoblinError, Result};
use crate::hashing::sha256_bytes;
use std::path::{Path, PathBuf};

pub const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_ROWS: usize = 100_000;
const MAX_CELLS: usize = 1_000_000;

pub fn is_function(name: &str) -> bool {
    matches!(
        name,
        "csv_rows"
            | "csv_columns"
            | "csv_headers"
            | "csv_column"
            | "csv_numbers"
            | "tsv_rows"
            | "tsv_columns"
            | "tsv_headers"
            | "tsv_column"
            | "tsv_numbers"
    )
}

#[derive(Debug)]
pub struct Table {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub delimiter: u8,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub access: Vec<String>,
}

impl Table {
    pub fn open(path: &Path, delimiter: u8) -> Result<Self> {
        let meta = std::fs::symlink_metadata(path)?;
        if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_INPUT_BYTES {
            return Err(GoblinError::data(
                "CSV/TSV input must be an ordinary non-symlink file no larger than 16 MiB.",
            ));
        }
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(MAX_INPUT_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_INPUT_BYTES {
            return Err(GoblinError::data("CSV/TSV input exceeds 16 MiB."));
        }
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| GoblinError::data("CSV/TSV input must be UTF-8."))?;
        let mut rows = parse(text.strip_prefix('\u{feff}').unwrap_or(text), delimiter)?;
        if rows.is_empty() {
            return Err(GoblinError::data("CSV/TSV requires a header row."));
        }
        let headers = rows.remove(0);
        let mut seen = std::collections::HashSet::new();
        if headers.iter().any(|h| h.is_empty() || !seen.insert(h)) {
            return Err(GoblinError::data(
                "CSV/TSV headers must be nonempty and unique (case-sensitive).",
            ));
        }
        if rows.iter().any(|r| r.len() != headers.len()) {
            return Err(GoblinError::data(
                "CSV/TSV rows must match the header's column count; ragged data is refused.",
            ));
        }
        Ok(Self {
            path: path.to_path_buf(),
            sha256: sha256_bytes(&bytes),
            bytes,
            delimiter,
            headers,
            rows,
            access: Vec::new(),
        })
    }
    pub fn column(&self, name: &str) -> Result<Vec<String>> {
        let index =
            self.headers.iter().position(|h| h == name).ok_or_else(|| {
                GoblinError::data(format!("CSV/TSV has no exact header {name:?}."))
            })?;
        Ok(self.rows.iter().map(|r| r[index].clone()).collect())
    }
}

pub fn parse(text: &str, delimiter: u8) -> Result<Vec<Vec<String>>> {
    if !matches!(delimiter, b',' | b'\t') || text.contains('\0') {
        return Err(GoblinError::data("Invalid delimiter or NUL in table."));
    }
    let bytes = text.as_bytes();
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cursor = 0;
    let mut cells = 0;
    while cursor < bytes.len() {
        let mut cell = Vec::new();
        if bytes[cursor] == b'"' {
            cursor += 1;
            loop {
                if cursor == bytes.len() {
                    return Err(GoblinError::data("Unclosed quoted CSV/TSV field."));
                }
                if bytes[cursor] == b'"' {
                    cursor += 1;
                    if cursor < bytes.len() && bytes[cursor] == b'"' {
                        cell.push(b'"');
                        cursor += 1;
                    } else {
                        break;
                    }
                } else {
                    cell.push(bytes[cursor]);
                    cursor += 1;
                }
            }
            if cursor < bytes.len()
                && !matches!(bytes[cursor], b'\r' | b'\n')
                && bytes[cursor] != delimiter
            {
                return Err(GoblinError::data(
                    "Unexpected characters after a quoted field.",
                ));
            }
        } else {
            while cursor < bytes.len()
                && !matches!(bytes[cursor], b'\r' | b'\n')
                && bytes[cursor] != delimiter
            {
                if bytes[cursor] == b'"' {
                    return Err(GoblinError::data(
                        "Quotes must enclose the entire CSV/TSV field.",
                    ));
                }
                cell.push(bytes[cursor]);
                cursor += 1;
            }
        }
        row.push(String::from_utf8(cell).map_err(|_| GoblinError::data("Invalid UTF-8 field."))?);
        cells += 1;
        if row.len() > 1024 || cells > MAX_CELLS {
            return Err(GoblinError::data(
                "CSV/TSV limit: 1024 columns and 1,000,000 cells.",
            ));
        }
        if cursor < bytes.len() && bytes[cursor] == delimiter {
            cursor += 1;
            if cursor == bytes.len() {
                row.push(String::new());
                cells += 1;
                if row.len() > 1024 || cells > MAX_CELLS {
                    return Err(GoblinError::data("CSV/TSV cell limit exceeded."));
                }
            } else {
                continue;
            }
        } else if cursor < bytes.len() {
            if bytes[cursor] == b'\r' {
                cursor += 1;
                if bytes.get(cursor) != Some(&b'\n') {
                    return Err(GoblinError::data(
                        "Use LF or CRLF line endings, not bare CR.",
                    ));
                }
            }
            cursor += 1;
        }
        rows.push(std::mem::take(&mut row));
        if rows.len() > MAX_ROWS + 1 {
            return Err(GoblinError::data("CSV/TSV row limit is 100,000 data rows."));
        }
    }
    Ok(rows)
}
