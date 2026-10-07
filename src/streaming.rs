//! Incremental CSV/TSV reductions: bounded records, exact parsed-byte hashing.
use crate::error::{GoblinError, Result};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const MAX_FILE_BYTES: u64 = 1024 * 1024 * 1024;
pub const MAX_RECORD_BYTES: usize = 1024 * 1024;
pub const MAX_COLUMNS: usize = 1024;
pub fn is_function(name: &str) -> bool {
    matches!(name, "csv_scan_stats" | "tsv_scan_stats")
}
pub fn require_arity(name: &str, n: usize) -> Result<()> {
    if n == 2 {
        Ok(())
    } else {
        Err(GoblinError::data(format!(
            "{name} expects exactly 2 arguments: path, column."
        )))
    }
}

struct HashReader {
    file: File,
    digest: Sha256,
    count: u64,
}
impl Read for HashReader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let n = self.file.read(out)?;
        self.count += n as u64;
        if self.count > MAX_FILE_BYTES {
            return Err(std::io::Error::other("Streaming input exceeds 1 GiB."));
        }
        self.digest.update(&out[..n]);
        Ok(n)
    }
}
#[derive(Debug)]
pub struct Scan {
    pub path: PathBuf,
    pub sha256: String,
    pub byte_count: u64,
    pub delimiter: u8,
    pub stats: [f64; 5],
    pub access: Vec<String>,
    file: File,
}
impl Scan {
    pub fn open(path: &Path, delimiter: u8, column: &str) -> Result<Self> {
        if !matches!(delimiter, b',' | b'\t') {
            return Err(GoblinError::data("Invalid streaming delimiter."));
        }
        let meta = std::fs::symlink_metadata(path)?;
        if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_FILE_BYTES {
            return Err(GoblinError::data(
                "Streaming input must be an ordinary non-symlink file no larger than 1 GiB.",
            ));
        }
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(
            64 * 1024,
            HashReader {
                file,
                digest: Sha256::new(),
                count: 0,
            },
        );
        // One leading UTF-8 BOM is accepted and remains included in the hash.
        if reader.fill_buf()?.starts_with(&[0xef, 0xbb, 0xbf]) {
            reader.consume(3);
        }
        let header = record(&mut reader, delimiter)?
            .ok_or_else(|| GoblinError::data("Streaming table requires a header."))?;
        let mut seen = std::collections::HashSet::new();
        if header.iter().any(|h| h.is_empty() || !seen.insert(h)) {
            return Err(GoblinError::data(
                "Streaming headers must be nonempty and unique (case-sensitive).",
            ));
        }
        let selected = header.iter().position(|h| h == column).ok_or_else(|| {
            GoblinError::data(format!("Streaming input has no exact header {column:?}."))
        })?;
        let (mut count, mut sum, mut correction, mut min, mut max) =
            (0u64, 0.0f64, 0.0f64, f64::INFINITY, f64::NEG_INFINITY);
        while let Some(row) = record(&mut reader, delimiter)? {
            if row.len() != header.len() {
                return Err(GoblinError::data(format!(
                    "Streaming data row {} has the wrong column count; no rows are skipped.",
                    count + 1
                )));
            }
            let number = crate::text_runtime::parse_number(&row[selected]).map_err(|e| GoblinError::data(format!("Streaming data row {} is not a finite number: {e}; no null values are skipped.",count+1)))?;
            let next = sum + number;
            correction += if sum.abs() >= number.abs() {
                (sum - next) + number
            } else {
                (number - next) + sum
            };
            sum = next;
            if !sum.is_finite() || !correction.is_finite() {
                return Err(GoblinError::data(
                    "Streaming sum overflow; no non-finite statistics are emitted.",
                ));
            }
            min = min.min(number);
            max = max.max(number);
            count += 1;
        }
        if count == 0 {
            return Err(GoblinError::data(
                "Streaming numeric statistics require at least one data row.",
            ));
        }
        let total = sum + correction;
        if !total.is_finite() {
            return Err(GoblinError::data("Streaming sum overflow."));
        }
        let reader = reader.into_inner();
        let sha256 = format!("{:x}", reader.digest.finalize());
        let operation = if delimiter == b',' {
            "csv_scan_stats"
        } else {
            "tsv_scan_stats"
        };
        Ok(Self { path:path.to_path_buf(),sha256,byte_count:reader.count,delimiter,
            stats:[count as f64,total,total/count as f64,min,max],
            access:vec![serde_json::json!({"operation":operation,"column":column,"policy":"goblin.delimited-scan.v1"}).to_string()],file:reader.file })
    }
    /// Copy from the same opened file, with bounded memory and complete rehash.
    pub fn copy_evidence(&self, destination: &Path) -> Result<String> {
        let mut input = self.file.try_clone()?;
        input.seek(SeekFrom::Start(0))?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        let mut digest = Sha256::new();
        let mut count = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let n = input.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            count += n as u64;
            if count > MAX_FILE_BYTES {
                return Err(GoblinError::data(
                    "Streaming input grew beyond its limit during evidence copy.",
                ));
            }
            digest.update(&buffer[..n]);
            output.write_all(&buffer[..n])?;
        }
        output.sync_all()?;
        let hash = format!("{:x}", digest.finalize());
        if count != self.byte_count || hash != self.sha256 {
            return Err(GoblinError::data(
                "Streaming input changed after scanning; exact parsed-byte evidence cannot be preserved.",
            ));
        }
        Ok(hash)
    }
}

fn byte(reader: &mut impl Read, size: &mut usize) -> Result<Option<u8>> {
    let mut b = [0];
    if reader.read(&mut b)? == 0 {
        return Ok(None);
    }
    *size += 1;
    if *size > MAX_RECORD_BYTES {
        return Err(GoblinError::data("Streaming record exceeds 1 MiB."));
    }
    if b[0] == 0 {
        return Err(GoblinError::data(
            "NUL is not permitted in streaming tables.",
        ));
    }
    Ok(Some(b[0]))
}
fn field(row: &mut Vec<String>, bytes: &mut Vec<u8>) -> Result<()> {
    row.push(
        String::from_utf8(std::mem::take(bytes))
            .map_err(|_| GoblinError::data("Streaming input must be UTF-8."))?,
    );
    if row.len() > MAX_COLUMNS {
        return Err(GoblinError::data("Streaming table exceeds 1024 columns."));
    }
    Ok(())
}
fn record(reader: &mut impl Read, delimiter: u8) -> Result<Option<Vec<String>>> {
    let mut row = Vec::new();
    let mut cell = Vec::new();
    let mut size = 0;
    // state: 0 field beginning, 1 unquoted, 2 quoted, 3 after closing quote.
    let mut state = 0;
    loop {
        let Some(b) = byte(reader, &mut size)? else {
            if state == 2 {
                return Err(GoblinError::data("Unclosed quoted streaming field."));
            }
            if size == 0 {
                return Ok(None);
            }
            field(&mut row, &mut cell)?;
            return Ok(Some(row));
        };
        if state == 2 {
            if b == b'"' {
                state = 3;
            } else {
                cell.push(b);
            }
            continue;
        }
        if state == 3 && b == b'"' {
            cell.push(b);
            state = 2;
            continue;
        }
        if b == delimiter {
            field(&mut row, &mut cell)?;
            state = 0;
            continue;
        }
        if matches!(b, b'\r' | b'\n') {
            if b == b'\r' && byte(reader, &mut size)? != Some(b'\n') {
                return Err(GoblinError::data("Use LF or CRLF, not bare CR."));
            }
            field(&mut row, &mut cell)?;
            return Ok(Some(row));
        }
        if state == 3 {
            return Err(GoblinError::data(
                "Unexpected characters after a quoted streaming field.",
            ));
        }
        if b == b'"' {
            if state != 0 {
                return Err(GoblinError::data(
                    "Quotes must enclose an entire streaming field.",
                ));
            }
            state = 2;
        } else {
            state = 1;
            cell.push(b);
        }
    }
}
