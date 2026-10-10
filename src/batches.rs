//! Serial, disk-backed table batches. Handles own I/O state, never returned arrays.
use crate::error::{GoblinError, Result};
use crate::evaluator::{DataImport, Value};
use crate::hashing::sha256_file;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024 * 1024;
pub const MAX_BATCH_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_BATCH_CELLS: usize = 100_000;
pub const MAX_HANDLES: usize = 128;
pub const MAX_OPEN_READERS: usize = 4;
pub const MAX_OPEN_WRITERS: usize = 4;
pub const BUFFER_BYTES: usize = 64 * 1024;
pub const POLICY_ID: &str = "goblin.serial-table-batches.v1";

pub fn is_function(name: &str) -> bool {
    matches!(
        name,
        "csv_batch_open"
            | "tsv_batch_open"
            | "batch_next"
            | "batch_close"
            | "csv_stream_open"
            | "tsv_stream_open"
            | "stream_write"
            | "stream_close"
    )
}
pub fn require_arity(name: &str, count: usize) -> Result<()> {
    let expected = match name {
        "csv_batch_open" | "tsv_batch_open" => 5,
        "csv_stream_open" | "tsv_stream_open" => 3,
        "stream_write" => 2,
        _ => 1,
    };
    if count != expected {
        return Err(GoblinError::parse(format!(
            "{name} expects exactly {expected} argument(s)."
        )));
    }
    Ok(())
}
pub fn policy() -> Json {
    json!({"id":POLICY_ID,"execution":"serial","input":"private-disk-snapshot-before-parse",
        "layout":"flat-row-major-text","order":"original-logical-record-order",
        "buffer_bytes":BUFFER_BYTES,"maximum_file_bytes":MAX_FILE_BYTES,
        "maximum_record_bytes":crate::streaming::MAX_RECORD_BYTES,
        "maximum_columns":crate::streaming::MAX_COLUMNS,"maximum_batch_bytes":MAX_BATCH_BYTES,
        "maximum_batch_cells":MAX_BATCH_CELLS,"maximum_handles":MAX_HANDLES,
        "maximum_open_readers":MAX_OPEN_READERS,"maximum_open_writers":MAX_OPEN_WRITERS,
        "output":"incremental-complete-records","success":"all-readers-at-eof-and-all-writers-closed",
        "memory_scope":"I/O-owned-buffers-not-user-arrays-or-process-RSS"})
}
pub fn requires_policy(receipt: &Json) -> bool {
    let version = receipt["goblin_version"].as_str().unwrap_or("");
    !(version.starts_with("0.0.")
        || version
            .strip_prefix("0.1.0-alpha.")
            .and_then(|s| s.parse::<u32>().ok())
            .is_some_and(|n| n < 28))
}
pub fn frozen_policy_matches(receipt: &Json) -> bool {
    receipt
        .get("batch_policy")
        .map_or(!requires_policy(receipt), |p| p == &policy())
}
pub fn program_uses_batches(program: &crate::ast::Program) -> bool {
    fn visit(v: &Json) -> bool {
        v.as_array().is_some_and(|a| {
            (a.first().is_some_and(|x| x == "call")
                && a.get(1).and_then(Json::as_str).is_some_and(is_function))
                || a.iter().any(visit)
        })
    }
    visit(&program.canonical())
}

fn integer(value: &Value, context: &str, maximum: u64) -> Result<u64> {
    let q = value.quantity(context)?;
    if q.dimension != crate::quantity::DIMENSIONLESS
        || q.value_si.fract() != 0.0
        || q.value_si < 1.0
        || q.value_si > maximum as f64
    {
        return Err(GoblinError::data(format!(
            "{context} requires an integer from 1 to {maximum}."
        )));
    }
    Ok(q.value_si as u64)
}
fn headers(value: &Value, context: &str) -> Result<Vec<String>> {
    let Value::Array(values) = value else {
        return Err(GoblinError::data(format!(
            "{context} requires an array of column names."
        )));
    };
    if values.is_empty() || values.len() > crate::streaming::MAX_COLUMNS {
        return Err(GoblinError::data("Request 1 to 1024 distinct columns."));
    }
    let mut names = Vec::new();
    let mut bytes = 0;
    for value in values {
        let name = value.text(context)?;
        if name.is_empty() || name.contains('\0') || names.iter().any(|s| s == name) {
            return Err(GoblinError::data(
                "Columns must be nonempty, distinct text without NUL.",
            ));
        }
        bytes += name.len() * 2 + 3;
        if bytes > crate::streaming::MAX_RECORD_BYTES {
            return Err(GoblinError::data(
                "Column names exceed the bounded record budget.",
            ));
        }
        names.push(name.to_owned());
    }
    Ok(names)
}
fn row_cost(row: &[String]) -> usize {
    row.len() * std::mem::size_of::<Value>() + row.iter().map(|s| s.capacity()).sum::<usize>()
}

#[derive(Debug)]
struct Reader {
    source: PathBuf,
    snapshot: PathBuf,
    sha256: String,
    bytes: u64,
    limit: u64,
    delimiter: u8,
    columns: Vec<String>,
    indices: Vec<usize>,
    width: usize,
    reader: Option<BufReader<File>>,
    pending: Option<Vec<String>>,
    rows_per_batch: usize,
    batch_bytes: usize,
    rows: u64,
    batches: u64,
    peak: usize,
    eof: bool,
    failed: bool,
    closed: bool,
}
impl Reader {
    fn summary(&self, handle: &str) -> Json {
        json!({"handle":handle,"path":self.source.display().to_string(),"sha256":self.sha256,
            "byte_count":self.bytes,"file_limit_bytes":self.limit,"columns":self.columns,
            "delimiter":self.delimiter,"rows_per_batch":self.rows_per_batch,
            "batch_limit_bytes":self.batch_bytes,"rows":self.rows,"batches":self.batches,
            "peak_batch_bytes":self.peak,"eof":self.eof,"failed":self.failed,"closed":self.closed})
    }
    fn next(&mut self) -> Result<Vec<Value>> {
        if self.closed || self.failed {
            return Err(GoblinError::data("Batch reader is closed or failed."));
        }
        if self.eof {
            return Ok(Vec::new());
        }
        let mut cells = Vec::new();
        let mut cost = 0;
        let mut rows = 0;
        while rows < self.rows_per_batch {
            let selected = if let Some(row) = self.pending.take() {
                row
            } else {
                let Some(row) =
                    crate::streaming::record(self.reader.as_mut().unwrap(), self.delimiter)?
                else {
                    self.eof = true;
                    self.reader = None;
                    break;
                };
                if row.len() != self.width {
                    return Err(GoblinError::data(format!(
                        "Batch data row {} has the wrong column count; no rows are skipped.",
                        self.rows + rows as u64 + 1
                    )));
                }
                let mut row: Vec<Option<String>> = row.into_iter().map(Some).collect();
                self.indices
                    .iter()
                    .map(|&i| row[i].take().unwrap())
                    .collect::<Vec<_>>()
            };
            let row_bytes = row_cost(&selected);
            if row_bytes > self.batch_bytes {
                return Err(GoblinError::data(
                    "A selected row exceeds the declared decoded batch limit.",
                ));
            }
            if cost + row_bytes > self.batch_bytes {
                self.pending = Some(selected);
                break;
            }
            cost += row_bytes;
            cells.reserve_exact(selected.len());
            if cost
                + (cells.capacity() - cells.len() - selected.len()) * std::mem::size_of::<Value>()
                > self.batch_bytes
            {
                return Err(GoblinError::data(
                    "Batch allocator capacity exceeds decoded byte budget.",
                ));
            }
            cells.extend(selected.into_iter().map(Value::Text));
            rows += 1;
        }
        self.rows += rows as u64;
        if rows > 0 {
            self.batches += 1;
        }
        // Include the actual returned Vec allocation, not just payload lengths.
        let allocated = cells.capacity() * std::mem::size_of::<Value>()
            + cells
                .iter()
                .map(|v| {
                    if let Value::Text(s) = v {
                        s.capacity()
                    } else {
                        0
                    }
                })
                .sum::<usize>();
        self.peak = self.peak.max(allocated);
        Ok(cells)
    }
}

#[derive(Debug)]
struct Writer {
    name: String,
    path: PathBuf,
    file: Option<File>,
    delimiter: u8,
    columns: Vec<String>,
    limit: u64,
    bytes: u64,
    rows: u64,
    closed: bool,
    failed: bool,
    digest: Sha256,
}
impl Writer {
    fn summary(&self, handle: &str) -> Json {
        json!({"handle":handle,"name":self.name,"delimiter":self.delimiter,"columns":self.columns,
            "file_limit_bytes":self.limit,"byte_count":self.bytes,"rows":self.rows,
            "closed":self.closed,"failed":self.failed,"sha256":format!("{:x}",self.digest.clone().finalize())})
    }
    fn write_record(&mut self, row: &[String]) -> Result<()> {
        if row.iter().any(|s| s.contains('\0')) {
            return Err(GoblinError::data("NUL in stream output."));
        }
        // Both formats use lossless quoting, including embedded tabs/newlines in TSV.
        let mut bytes = Vec::new();
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                bytes.push(self.delimiter);
            }
            let quoted = cell
                .bytes()
                .any(|b| b == self.delimiter || matches!(b, b'"' | b'\r' | b'\n'));
            if quoted {
                bytes.push(b'"');
            }
            for byte in cell.bytes() {
                if quoted && byte == b'"' {
                    bytes.push(b'"');
                }
                bytes.push(byte);
            }
            if quoted {
                bytes.push(b'"');
            }
        }
        bytes.push(b'\n');
        if bytes.len() > crate::streaming::MAX_RECORD_BYTES
            || self.bytes + bytes.len() as u64 > self.limit
        {
            return Err(GoblinError::artifact(
                "Stream record/file exceeds its explicit output limit; partial output is incomplete.",
            ));
        }
        self.file.as_mut().unwrap().write_all(&bytes)?;
        self.digest.update(&bytes);
        self.bytes += bytes.len() as u64;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Batches {
    directory: Option<PathBuf>,
    readers: Vec<Reader>,
    writers: Vec<Writer>,
}
impl Drop for Batches {
    fn drop(&mut self) {
        // Only the exclusively-created directory owned by this instance is removed.
        self.readers.clear();
        self.writers.clear();
        if let Some(path) = self.directory.take() {
            let _ = fs::remove_dir_all(path);
        }
    }
}
impl Batches {
    pub fn staging_directory(&self) -> Option<&Path> {
        self.directory.as_deref()
    }
    fn directory(&mut self) -> Result<PathBuf> {
        if let Some(path) = &self.directory {
            return Ok(path.clone());
        }
        static NEXT: AtomicU64 = AtomicU64::new(0);
        for _ in 0..16 {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| GoblinError::data(e.to_string()))?
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "goblin-batches-{}-{stamp}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => {
                    self.directory = Some(path.clone());
                    return Ok(path);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Err(GoblinError::data(
            "Unable to create exclusive batch staging directory.",
        ))
    }
    fn reader_index(&self, arg: &Value) -> Result<usize> {
        let s = arg.text("batch reader handle")?;
        let index = s
            .strip_prefix("@goblin-reader-")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&i| i < self.readers.len())
            .ok_or_else(|| {
                GoblinError::data("Unknown batch reader handle; handles are run-local.")
            })?;
        if s != format!("@goblin-reader-{index}") {
            return Err(GoblinError::data("Invalid reader handle."));
        }
        Ok(index)
    }
    fn writer_index(&self, arg: &Value) -> Result<usize> {
        let s = arg.text("stream writer handle")?;
        let index = s
            .strip_prefix("@goblin-writer-")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&i| i < self.writers.len())
            .ok_or_else(|| {
                GoblinError::data("Unknown stream writer handle; handles are run-local.")
            })?;
        if s != format!("@goblin-writer-{index}") {
            return Err(GoblinError::data("Invalid writer handle."));
        }
        Ok(index)
    }
    pub fn has_name(&self, name: &str) -> bool {
        self.writers
            .iter()
            .any(|w| w.name == name || format!("{}.partial", w.name) == name)
    }
    pub fn has_source(&self, path: &Path) -> bool {
        self.readers.iter().any(|r| r.source == path)
    }
    pub fn call(&mut self, base: &Path, name: &str, args: &[Value]) -> Result<Value> {
        require_arity(name, args.len())?;
        match name {
            "csv_batch_open" | "tsv_batch_open" => {
                if self.readers.len() + self.writers.len() >= MAX_HANDLES
                    || self.readers.iter().filter(|r| !r.eof && !r.closed).count()
                        >= MAX_OPEN_READERS
                {
                    return Err(GoblinError::data(
                        "Batch handle/open-reader limit exceeded.",
                    ));
                }
                let columns = headers(&args[1], name)?;
                let row_limit =
                    integer(&args[2], name, (MAX_BATCH_CELLS / columns.len()) as u64)? as usize;
                let batch_bytes = integer(&args[3], name, MAX_BATCH_BYTES as u64)? as usize;
                let limit = integer(&args[4], name, MAX_FILE_BYTES)?;
                let requested = Path::new(args[0].text(name)?);
                let path = if requested.is_absolute() {
                    requested.to_owned()
                } else {
                    base.join(requested)
                };
                let meta = fs::symlink_metadata(&path)?;
                if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > limit {
                    return Err(GoblinError::data(
                        "Batch input must be an ordinary non-symlink file within its declared byte budget.",
                    ));
                }
                let path = path.canonicalize()?;
                if self.has_source(&path) {
                    return Err(GoblinError::data(
                        "Open each batch input once per run; no silent re-read or delimiter change.",
                    ));
                }
                let mut input = File::open(&path)?;
                if !input.metadata()?.is_file() || input.metadata()?.len() > limit {
                    return Err(GoblinError::data("Batch input changed while opening."));
                }
                let snapshot = self
                    .directory()?
                    .join(format!("input-{}", self.readers.len()));
                let mut saved = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&snapshot)?;
                let mut digest = Sha256::new();
                let mut count = 0;
                let mut buffer = [0u8; BUFFER_BYTES];
                loop {
                    let n = input.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    count += n as u64;
                    if count > limit {
                        return Err(GoblinError::data(
                            "Batch input grew beyond its declared byte budget.",
                        ));
                    }
                    saved.write_all(&buffer[..n])?;
                    digest.update(&buffer[..n]);
                }
                saved.sync_all()?;
                let hash = format!("{:x}", digest.finalize());
                if input.metadata()?.len() != count || sha256_file(&path)? != hash {
                    return Err(GoblinError::data(
                        "Batch input changed during snapshot; processing refused.",
                    ));
                }
                let delimiter = if name.starts_with("csv_") {
                    b','
                } else {
                    b'\t'
                };
                let mut reader = BufReader::with_capacity(BUFFER_BYTES, File::open(&snapshot)?);
                use std::io::BufRead;
                if reader.fill_buf()?.starts_with(&[0xef, 0xbb, 0xbf]) {
                    reader.consume(3);
                }
                self.readers.push(Reader {
                    source: path,
                    snapshot,
                    sha256: hash,
                    bytes: count,
                    limit,
                    delimiter,
                    columns,
                    indices: Vec::new(),
                    width: 0,
                    reader: Some(reader),
                    pending: None,
                    rows_per_batch: row_limit,
                    batch_bytes,
                    rows: 0,
                    batches: 0,
                    peak: 0,
                    eof: false,
                    failed: false,
                    closed: false,
                });
                let index = self.readers.len() - 1;
                let r = &mut self.readers[index];
                let result = (|| {
                    let names = crate::streaming::record(r.reader.as_mut().unwrap(), delimiter)?
                        .ok_or_else(|| GoblinError::data("Batch input requires a header."))?;
                    let mut seen = std::collections::HashSet::new();
                    if names.iter().any(|s| s.is_empty() || !seen.insert(s)) {
                        return Err(GoblinError::data(
                            "Batch headers must be nonempty and unique.",
                        ));
                    }
                    r.indices = r
                        .columns
                        .iter()
                        .map(|s| {
                            names.iter().position(|h| h == s).ok_or_else(|| {
                                GoblinError::data(format!("Batch input has no exact header {s:?}."))
                            })
                        })
                        .collect::<Result<_>>()?;
                    r.width = names.len();
                    Ok(Value::Text(format!("@goblin-reader-{index}")))
                })();
                if result.is_err() {
                    r.failed = true;
                    r.reader = None;
                }
                result
            }
            "batch_next" => {
                let index = self.reader_index(&args[0])?;
                let r = &mut self.readers[index];
                let result = r.next().map(Value::Array);
                if result.is_err() {
                    r.failed = true;
                    r.reader = None;
                }
                result
            }
            "batch_close" => {
                let index = self.reader_index(&args[0])?;
                let r = &mut self.readers[index];
                if !r.eof || r.failed || r.closed {
                    return Err(GoblinError::data(
                        "Reader close requires observed EOF and an unclosed, successful reader.",
                    ));
                }
                r.closed = true;
                Ok(Value::Bool(true))
            }
            "csv_stream_open" | "tsv_stream_open" => {
                if self.readers.len() + self.writers.len() >= MAX_HANDLES
                    || self.writers.iter().filter(|w| !w.closed).count() >= MAX_OPEN_WRITERS
                {
                    return Err(GoblinError::artifact(
                        "Batch handle/open-writer limit exceeded.",
                    ));
                }
                let name_text = args[0].text(name)?;
                let ext = crate::output::extension(name_text)?;
                let delimiter = if name.starts_with("csv_") {
                    b','
                } else {
                    b'\t'
                };
                if ext != if delimiter == b',' { "csv" } else { "tsv" } {
                    return Err(GoblinError::artifact(
                        "Stream output requires the matching .csv or .tsv extension.",
                    ));
                }
                if self.has_name(name_text) {
                    return Err(GoblinError::artifact(
                        "Output name is already used; overwrites refused.",
                    ));
                }
                let columns = headers(&args[1], name)?;
                let limit = integer(&args[2], name, MAX_FILE_BYTES)?;
                let path = self
                    .directory()?
                    .join(format!("output-{}", self.writers.len()));
                let file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)?;
                let mut writer = Writer {
                    name: name_text.into(),
                    path,
                    file: Some(file),
                    delimiter,
                    columns: columns.clone(),
                    limit,
                    bytes: 0,
                    rows: 0,
                    closed: false,
                    failed: false,
                    digest: Sha256::new(),
                };
                let written = writer.write_record(&columns);
                if written.is_err() {
                    writer.failed = true;
                }
                self.writers.push(writer);
                written?;
                Ok(Value::Text(format!(
                    "@goblin-writer-{}",
                    self.writers.len() - 1
                )))
            }
            "stream_write" => {
                let index = self.writer_index(&args[0])?;
                let w = &mut self.writers[index];
                if w.closed || w.failed {
                    return Err(GoblinError::artifact("Stream writer is closed or failed."));
                }
                let Value::Array(cells) = &args[1] else {
                    return Err(GoblinError::artifact(
                        "stream_write requires a flat array of complete rows.",
                    ));
                };
                let result = (|| {
                    if cells.len() > MAX_BATCH_CELLS || !cells.len().is_multiple_of(w.columns.len())
                    {
                        return Err(GoblinError::artifact(
                            "Stream cells must form complete rows within the 100000-cell bound.",
                        ));
                    }
                    let mut cost = 0;
                    for cell in cells {
                        cost += std::mem::size_of::<Value>()
                            + match cell {
                                Value::Text(s) => s.len(),
                                Value::Array(_) => {
                                    return Err(GoblinError::artifact(
                                        "Nested stream output arrays refuse.",
                                    ));
                                }
                                _ => 64,
                            };
                        if cost > MAX_BATCH_BYTES {
                            return Err(GoblinError::artifact(
                                "Stream cells exceed the 8 MiB decoded write limit.",
                            ));
                        }
                    }
                    for row in cells.chunks(w.columns.len()) {
                        let texts = row.iter().map(Value::render).collect::<Vec<_>>();
                        w.write_record(&texts)?;
                        w.rows += 1;
                    }
                    Ok(Value::Quantity(crate::quantity::Quantity::scalar(
                        (cells.len() / w.columns.len()) as f64,
                    )?))
                })();
                if result.is_err() {
                    w.failed = true;
                }
                result
            }
            "stream_close" => {
                let index = self.writer_index(&args[0])?;
                let w = &mut self.writers[index];
                if w.closed || w.failed {
                    return Err(GoblinError::artifact("Writer is already closed or failed."));
                }
                w.file.as_mut().unwrap().sync_all()?;
                w.file = None;
                w.closed = true;
                Ok(Value::Bool(true))
            }
            _ => unreachable!(),
        }
    }
    pub fn finish(&self) -> Result<()> {
        if self.readers.iter().any(|r| !r.eof || !r.closed || r.failed)
            || self.writers.iter().any(|w| !w.closed || w.failed)
        {
            return Err(GoblinError::data(
                "Incomplete batch lifecycle: read through EOF, close every reader and close every writer before the run can pass.",
            ));
        }
        Ok(())
    }
    pub fn evidence(&self) -> Json {
        json!({"readers":self.readers.iter().enumerate().map(|(i,r)|r.summary(&format!("@goblin-reader-{i}"))).collect::<Vec<_>>(),
            "writers":self.writers.iter().enumerate().map(|(i,w)|w.summary(&format!("@goblin-writer-{i}"))).collect::<Vec<_>>()})
    }
    pub fn imports(&self) -> Vec<DataImport> {
        self.readers.iter().enumerate().map(|(i,r)|DataImport {path:r.source.display().to_string(),
            sha256:r.sha256.clone(),byte_count:r.bytes,format:if r.delimiter==b',' {"CSV-UTF8"} else {"TSV-UTF8"}.into(),
            access:vec![json!({"operation":"batch_read","policy":POLICY_ID,"summary":r.summary(&format!("@goblin-reader-{i}"))}).to_string()],
            evidence_path:None,evidence_sha256:None}).collect()
    }
    pub fn copy_input(&self, hash: &str, destination: &Path) -> Option<Result<String>> {
        self.readers.iter().find(|r| r.sha256 == hash).map(|r| {
            copy_exclusive(&r.snapshot, destination).and_then(|_| {
                let observed = sha256_file(destination)?;
                if observed != r.sha256 {
                    return Err(GoblinError::data(
                        "Batch snapshot changed before preservation.",
                    ));
                }
                Ok(observed)
            })
        })
    }
    pub fn descriptors(&self, success: bool) -> Result<Vec<Json>> {
        self.writers.iter().enumerate().map(|(i,w)|{
            let complete=success && w.closed && !w.failed;
            let name=if complete {w.name.clone()} else {format!("{}.partial",w.name)};
            // Hash on-disk bytes, including any partial OS write after an I/O error.
            let actual_bytes=fs::metadata(&w.path)?.len();
            Ok(json!({"name":name,"sha256":sha256_file(&w.path)?,"byte_count":actual_bytes,
                "media_type":if w.delimiter==b',' {"text/csv; charset=utf-8"} else {"text/tab-separated-values; charset=utf-8"},
                "producer":"stream_write","metadata":{"policy":POLICY_ID,"complete":complete,"requested_name":w.name,
                    "summary":w.summary(&format!("@goblin-writer-{i}"))}}))
        }).collect()
    }
    pub fn publish(&self, directory: &Path, success: bool) -> Result<Vec<Json>> {
        if self.writers.is_empty() {
            return Ok(Vec::new());
        }
        fs::create_dir_all(directory)?;
        let descriptors = self.descriptors(success)?;
        for (w, d) in self.writers.iter().zip(&descriptors) {
            if let Some(file) = &w.file {
                file.sync_all()?;
            }
            copy_exclusive(&w.path, &directory.join(d["name"].as_str().unwrap()))?;
        }
        Ok(descriptors)
    }
}
fn copy_exclusive(source: &Path, destination: &Path) -> Result<()> {
    let mut input = File::open(source)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    std::io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    Ok(())
}

/// Late audit/ledger failures must not leave streamed outputs labelled complete.
pub fn mark_incomplete(run: &Path, receipt: &mut Json) -> Result<()> {
    if let Some(artifacts) = receipt["generated_artifacts"].as_array_mut() {
        for artifact in artifacts {
            if artifact["producer"] != "stream_write" || artifact["metadata"]["complete"] != true {
                continue;
            }
            let name = artifact["metadata"]["requested_name"]
                .as_str()
                .ok_or_else(|| GoblinError::artifact("Missing stream name."))?
                .to_owned();
            crate::output::validate_name(&name)?;
            let partial = format!("{name}.partial");
            let original = run.join("outputs").join(&name);
            // Hard-link creation refuses collisions, unlike overwrite-capable rename.
            fs::hard_link(&original, run.join("outputs").join(&partial))?;
            fs::remove_file(original)?;
            artifact["name"] = json!(partial);
            artifact["path"] = json!(format!("outputs/{partial}"));
            artifact["metadata"]["complete"] = json!(false);
        }
    }
    Ok(())
}

/// Structural/relational integrity checks, not a replay of scientific arithmetic.
pub fn validate_evidence(receipt: &Json) -> bool {
    fn hash(v: &Json) -> bool {
        v.as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    }
    fn names(v: &Json) -> bool {
        v.as_array().is_some_and(|a| {
            !a.is_empty() && a.len() <= crate::streaming::MAX_COLUMNS && {
                let mut seen = std::collections::HashSet::new();
                a.iter().all(|v| {
                    v.as_str()
                        .is_some_and(|s| !s.is_empty() && !s.contains('\0') && seen.insert(s))
                })
            }
        })
    }
    let Some(readers) = receipt["batches"]["readers"].as_array() else {
        return false;
    };
    let Some(writers) = receipt["batches"]["writers"].as_array() else {
        return false;
    };
    let Some(imports) = receipt["data_imports"].as_array() else {
        return false;
    };
    let Some(artifacts) = receipt["generated_artifacts"].as_array() else {
        return false;
    };
    if readers.len() + writers.len() > MAX_HANDLES {
        return false;
    }
    let pass = receipt["status"] == "PASS";
    for (i, r) in readers.iter().enumerate() {
        if r["handle"] != format!("@goblin-reader-{i}")
            || !names(&r["columns"])
            || !hash(&r["sha256"])
            || !matches!(r["delimiter"].as_u64(), Some(9 | 44))
        {
            return false;
        }
        let (
            Some(bytes),
            Some(limit),
            Some(rows_limit),
            Some(batch_limit),
            Some(rows),
            Some(batches),
            Some(peak),
        ) = (
            r["byte_count"].as_u64(),
            r["file_limit_bytes"].as_u64(),
            r["rows_per_batch"].as_u64(),
            r["batch_limit_bytes"].as_u64(),
            r["rows"].as_u64(),
            r["batches"].as_u64(),
            r["peak_batch_bytes"].as_u64(),
        )
        else {
            return false;
        };
        let width = r["columns"].as_array().unwrap().len() as u64;
        if limit == 0
            || limit > MAX_FILE_BYTES
            || bytes > limit
            || rows_limit == 0
            || rows_limit > MAX_BATCH_CELLS as u64 / width
            || batch_limit == 0
            || batch_limit > MAX_BATCH_BYTES as u64
            || peak > batch_limit
            || batches > rows
            || rows > batches.saturating_mul(rows_limit)
        {
            return false;
        }
        if ["eof", "failed", "closed"]
            .iter()
            .any(|k| r[*k].as_bool().is_none())
            || (r["closed"] == true && (r["eof"] != true || r["failed"] == true))
            || (pass && (r["closed"] != true || r["eof"] != true || r["failed"] != false))
        {
            return false;
        }
        let access = json!({"operation":"batch_read","policy":POLICY_ID,"summary":r}).to_string();
        if !imports.iter().any(|d| {
            d["path"] == r["path"]
                && d["sha256"] == r["sha256"]
                && d["byte_count"] == r["byte_count"]
                && d["access"]
                    .as_array()
                    .is_some_and(|a| a.contains(&json!(access)))
        }) {
            return false;
        }
    }
    if artifacts
        .iter()
        .filter(|a| a["producer"] == "stream_write")
        .count()
        != writers.len()
    {
        return false;
    }
    for (i, w) in writers.iter().enumerate() {
        if w["handle"] != format!("@goblin-writer-{i}")
            || !names(&w["columns"])
            || !hash(&w["sha256"])
            || !matches!(w["delimiter"].as_u64(), Some(9 | 44))
        {
            return false;
        }
        let (Some(bytes), Some(limit), Some(rows), Some(name)) = (
            w["byte_count"].as_u64(),
            w["file_limit_bytes"].as_u64(),
            w["rows"].as_u64(),
            w["name"].as_str(),
        ) else {
            return false;
        };
        if crate::output::validate_name(name).is_err()
            || limit == 0
            || limit > MAX_FILE_BYTES
            || bytes > limit
            || rows > bytes
            || w["closed"].as_bool().is_none()
            || w["failed"].as_bool().is_none()
        {
            return false;
        }
        let Some(a) = artifacts
            .iter()
            .find(|a| a["producer"] == "stream_write" && a["metadata"]["requested_name"] == name)
        else {
            return false;
        };
        let complete = pass && w["closed"] == true && w["failed"] == false;
        let expected = if complete {
            name.to_owned()
        } else {
            format!("{name}.partial")
        };
        if a["metadata"]["summary"] != *w
            || a["metadata"]["policy"] != POLICY_ID
            || a["metadata"]["complete"] != complete
            || a["name"] != expected
            || a["path"] != format!("outputs/{expected}")
            || !a["byte_count"]
                .as_u64()
                .is_some_and(|n| n >= bytes && n <= limit)
            || (complete && (a["byte_count"] != bytes || a["sha256"] != w["sha256"]))
        {
            return false;
        }
        if pass && !complete {
            return false;
        }
    }
    true
}
