//! Bounded native data interchange with original-byte snapshots and replayable access evidence.
use crate::{
    error::{GoblinError, Result},
    evaluator::{DataImport, Value},
    hashing::{hash_canonical_json, sha256_bytes},
    quantity::Quantity,
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
#[path = "scientific_io/ecsv.rs"]
mod ecsv;
#[path = "scientific_io/npy.rs"]
pub mod npy;
#[path = "scientific_io/npz.rs"]
pub mod npz;
#[path = "scientific_io/sparse.rs"]
mod sparse;
pub const MAX_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_ITEMS: usize = 100_000;
pub const FUNCTIONS: &[&str] = &[
    "npy_read",
    "npy_text",
    "npy_shape",
    "npy_dtype",
    "npy_order",
    "npz_keys",
    "npz_read",
    "npz_text",
    "npz_shape",
    "npz_dtype",
    "npz_order",
    "scipy_sparse_shape",
    "scipy_sparse_format",
    "scipy_sparse_nnz",
    "scipy_sparse_dense",
    "scipy_sparse_row",
    "ecsv_rows",
    "ecsv_columns",
    "ecsv_headers",
    "ecsv_column",
    "ecsv_read",
    "ecsv_numbers",
    "ecsv_dtype",
    "ecsv_unit",
];
pub fn fail(message: impl Into<String>) -> GoblinError {
    GoblinError::data(message)
}
pub fn scalar(n: f64) -> Result<Value> {
    Quantity::scalar(n).map(Value::Quantity)
}
pub fn is_function(name: &str) -> bool {
    FUNCTIONS.contains(&name)
}
pub fn arity(name: &str) -> Option<usize> {
    if !is_function(name) {
        None
    } else {
        Some(
            if matches!(
                name,
                "npz_read"
                    | "npz_text"
                    | "npz_shape"
                    | "npz_dtype"
                    | "npz_order"
                    | "scipy_sparse_row"
                    | "ecsv_column"
                    | "ecsv_read"
                    | "ecsv_numbers"
                    | "ecsv_dtype"
                    | "ecsv_unit"
            ) {
                2
            } else {
                1
            },
        )
    }
}
pub fn require_arity(name: &str, n: usize) -> Result<()> {
    let expected = arity(name).ok_or_else(|| fail("Unknown scientific input function."))?;
    if expected != n {
        return Err(GoblinError::parse(format!(
            "{name}() expects {expected} arguments, got {n}."
        )));
    }
    Ok(())
}
pub fn policy() -> serde_json::Value {
    json!({"id":"goblin.scientific-input.v1","algorithm_sha256":hash_canonical_json(&json!([include_str!("scientific_io.rs"),include_str!("scientific_io/npy.rs"),include_str!("scientific_io/npz.rs"),include_str!("scientific_io/sparse.rs"),include_str!("scientific_io/ecsv.rs")])).expect("static source serialization"),"input_bytes":MAX_BYTES,"materialized_items":MAX_ITEMS,"cache_total_bytes":64*1024*1024,"cache_files":32,"access_records_per_file":64,"numpy_layout":"LOGICAL_C_ORDER_FLAT_COPY_WITH_EXPLICIT_SHAPE","integer_conversion":"EXACT_RANGE_NUMERIC_OR_EXPLICIT_TEXT","zip_crate":"8.6.0","deflate_backend":"flate2-1.1.10/miniz_oxide-0.9.1","npz":"STORED_OR_DEFLATE;128_ENTRIES;16_MIB_MEMBER;64_MIB_EXPANDED;NO_EXTRACTION;NO_PICKLE","sparse":"CSR_CSC_COO;NO_DUPLICATE_SUM;BOUNDED_EXPLICIT_DENSIFICATION","ecsv":"VERSION_1_SCALAR_FLOW_MAP_SUBSET;NO_MASKS_MIXINS_OR_YAML_CONSTRUCTORS;UNITS_APPLIED_ON_NUMERIC_READ","unit_registry_sha256":hash_canonical_json(&json!(crate::quantity::unit_snapshots())).expect("static unit serialization"),"authentication":"CHECKSUM_ONLY"})
}
pub fn requires_policy(receipt: &serde_json::Value) -> bool {
    receipt["goblin_version"]
        .as_str()
        .and_then(|v| v.strip_prefix("0.1.0-alpha."))
        .and_then(|v| v.parse::<u32>().ok())
        .is_some_and(|v| v >= 41)
}
fn format_for(name: &str) -> &'static str {
    if name.starts_with("npy_") {
        "NPY"
    } else if name.starts_with("ecsv_") {
        "ECSV"
    } else {
        "NPZ"
    }
}
#[derive(Debug)]
pub struct Snapshot {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub format: &'static str,
    pub access: Vec<String>,
}
#[derive(Debug, Default)]
pub struct Inputs {
    files: BTreeMap<PathBuf, Snapshot>,
}
impl Inputs {
    pub fn contains(&self, p: &Path) -> bool {
        self.files.contains_key(p)
    }
    pub fn call(&mut self, path: &Path, name: &str, selector: Option<Value>) -> Result<Value> {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(fail("Scientific input must be a regular nonsymlink file."));
        }
        let key = path.canonicalize()?;
        if !self.files.contains_key(&key) {
            if self.files.len() >= 32 {
                return Err(fail("Scientific input cache exceeds 32 files."));
            }
            if metadata.len() > MAX_BYTES as u64 {
                return Err(fail("Scientific input exceeds 16 MiB."));
            }
            let mut options = OpenOptions::new();
            options.read(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
            }
            let file = options.open(path)?;
            if !file.metadata()?.is_file() {
                return Err(fail("Scientific input changed to a nonregular file."));
            }
            let mut bytes = Vec::new();
            file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
            if bytes.len() > MAX_BYTES
                || self.files.values().map(|s| s.bytes.len()).sum::<usize>() + bytes.len()
                    > 64 * 1024 * 1024
            {
                return Err(fail("Scientific input exceeds the file/cache byte limit."));
            }
            self.files.insert(
                key.clone(),
                Snapshot {
                    path: key.clone(),
                    sha256: sha256_bytes(&bytes),
                    bytes,
                    format: format_for(name),
                    access: Vec::new(),
                },
            );
        }
        let source = self.files.get_mut(&key).unwrap();
        if source.format != format_for(name) {
            return Err(fail(
                "One captured path cannot be reinterpreted through another scientific container reader.",
            ));
        }
        if source.access.len() >= 64 {
            return Err(fail(
                "Scientific input access evidence exceeds 64 calls per file.",
            ));
        }
        let selector_json = selector.as_ref().map(Value::to_json);
        let outcome = perform(&source.bytes, name, selector.as_ref());
        let record = match &outcome {
            Ok((value, details)) => {
                json!({"operation":name,"selector":selector_json,"completed":true,"details":details,"result_sha256":hash_canonical_json(&value.to_json())?})
            }
            Err(error) => {
                json!({"operation":name,"selector":selector_json,"completed":false,"error_code":error.code,"error":error.message})
            }
        };
        source.access.push(record.to_string());
        outcome.map(|(value, _)| value)
    }
    pub fn imports(&self) -> Vec<DataImport> {
        self.files
            .values()
            .map(|s| DataImport {
                path: s.path.display().to_string(),
                sha256: s.sha256.clone(),
                byte_count: s.bytes.len() as u64,
                format: s.format.into(),
                access: s.access.clone(),
                evidence_path: None,
                evidence_sha256: None,
            })
            .collect()
    }
    pub fn copy(&self, hash: &str, destination: &Path) -> Option<Result<String>> {
        self.files.values().find(|s| s.sha256 == hash).map(|s| {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(destination)?;
            file.write_all(&s.bytes)?;
            file.sync_all()?;
            Ok(sha256_bytes(&s.bytes))
        })
    }
}
fn text_selector(selector: Option<&Value>) -> Result<&str> {
    selector
        .ok_or_else(|| fail("Missing scientific input selector."))?
        .text("Scientific input selector")
}
fn shape_value(shape: &[usize]) -> Result<Value> {
    shape
        .iter()
        .map(|n| scalar(*n as f64))
        .collect::<Result<Vec<_>>>()
        .map(Value::Array)
}
fn array_result(bytes: &[u8], name: &str) -> Result<(Value, serde_json::Value)> {
    let a = npy::Array::parse(bytes)?;
    let details = json!({"shape":a.shape,"dtype":a.dtype,"storage_order":if a.fortran{"F"}else{"C"},"returned_order":"LOGICAL_C_FLAT"});
    let op = name.split_once('_').unwrap().1;
    let v = match op {
        "read" | "text" => Value::Array(a.values(op == "text")?),
        "shape" => shape_value(&a.shape)?,
        "dtype" => Value::Text(a.dtype.clone()),
        "order" => Value::Text(if a.fortran { "F" } else { "C" }.into()),
        _ => return Err(fail("Unknown NPY operation.")),
    };
    Ok((v, details))
}
fn perform(
    bytes: &[u8],
    name: &str,
    selector: Option<&Value>,
) -> Result<(Value, serde_json::Value)> {
    if name.starts_with("npy_") {
        return array_result(bytes, name);
    }
    if name.starts_with("npz_") {
        let mut archive = npz::Archive::open(bytes)?;
        if name == "npz_keys" {
            return Ok((
                Value::Array(archive.keys.keys().cloned().map(Value::Text).collect()),
                json!({"selected_members_decoded":false}),
            ));
        }
        let key = text_selector(selector)?;
        let data = archive.member(key)?;
        let (value, mut details) = array_result(&data, name)?;
        details["key"] = json!(key);
        return Ok((value, details));
    }
    if name.starts_with("scipy_sparse_") {
        let m = sparse::Sparse::open(bytes)?;
        let details = json!({"shape":m.shape,"format":m.format,"stored_entries":m.entries.len(),"duplicate_policy":"REFUSE","dense_order":"LOGICAL_C_FLAT"});
        let value = match name {
            "scipy_sparse_shape" => shape_value(&m.shape)?,
            "scipy_sparse_format" => Value::Text(m.format.clone()),
            "scipy_sparse_nnz" => scalar(m.entries.len() as f64)?,
            "scipy_sparse_dense" => m.dense(None)?,
            "scipy_sparse_row" => {
                let q = selector
                    .ok_or_else(|| fail("Missing sparse row index."))?
                    .quantity("sparse row")?;
                if q.dimension != [0; 6]
                    || q.value_si < 0.0
                    || q.value_si.fract() != 0.0
                    || q.value_si > MAX_ITEMS as f64
                {
                    return Err(fail(
                        "Sparse row index must be a bounded nonnegative unitless integer.",
                    ));
                }
                m.dense(Some(q.value_si as usize))?
            }
            _ => return Err(fail("Unknown sparse input operation.")),
        };
        return Ok((value, details));
    }
    if name.starts_with("ecsv_") {
        let t = ecsv::Table::parse(bytes)?;
        let col = selector
            .map(|v| v.text("ECSV column").and_then(|name| t.index(name)))
            .transpose()?;
        let details = json!({"columns":t.columns.iter().map(|c|json!({"name":c.name,"datatype":c.dtype,"unit":c.unit})).collect::<Vec<_>>(),"rows":t.rows.len()});
        let value = match name {
            "ecsv_rows" => scalar(t.rows.len() as f64)?,
            "ecsv_columns" => scalar(t.columns.len() as f64)?,
            "ecsv_headers" => Value::Array(
                t.columns
                    .iter()
                    .map(|c| Value::Text(c.name.clone()))
                    .collect(),
            ),
            "ecsv_column" => Value::Array(
                t.rows
                    .iter()
                    .map(|row| Value::Text(row[col.unwrap()].clone()))
                    .collect(),
            ),
            "ecsv_dtype" => Value::Text(t.columns[col.unwrap()].dtype.clone()),
            "ecsv_unit" => Value::Text(t.columns[col.unwrap()].unit.clone()),
            "ecsv_read" | "ecsv_numbers" => t.read(col.unwrap(), name == "ecsv_numbers")?,
            _ => return Err(fail("Unknown ECSV operation.")),
        };
        return Ok((value, details));
    }
    Err(fail("Unknown scientific input function."))
}
/// Replay only the bounded documented reader calls from preserved bytes, not .gbl code.
pub fn verify_access(run: &Path, receipt: &serde_json::Value) -> bool {
    (|| -> Result<bool> {
        if receipt
            .get("scientific_input_policy")
            .is_some_and(|p| *p != policy())
            || needs_policy(run, receipt) && receipt.get("scientific_input_policy").is_none()
        {
            return Ok(false);
        }
        if let Some(imports) = receipt["data_imports"].as_array() {
            for source in imports {
                if !matches!(source["format"].as_str(), Some("NPY" | "NPZ" | "ECSV")) {
                    continue;
                }
                let relative = source["evidence_path"]
                    .as_str()
                    .ok_or_else(|| fail("Scientific source evidence missing."))?;
                let p = Path::new(relative);
                if p.is_absolute()
                    || p.components()
                        .any(|c| !matches!(c, std::path::Component::Normal(_)))
                {
                    return Ok(false);
                }
                let bytes = read_evidence(run, p)?;
                if source["sha256"] != sha256_bytes(&bytes)
                    || source["byte_count"] != bytes.len() as u64
                {
                    return Ok(false);
                }
                let access = source["access"]
                    .as_array()
                    .ok_or_else(|| fail("Scientific access list missing."))?;
                if access.is_empty() || access.len() > 64 {
                    return Ok(false);
                }
                for a in access {
                    let record: serde_json::Value = serde_json::from_str(
                        a.as_str()
                            .ok_or_else(|| fail("Malformed scientific access."))?,
                    )?;
                    let op = record["operation"]
                        .as_str()
                        .ok_or_else(|| fail("Scientific operation missing."))?;
                    if !is_function(op) || source["format"] != format_for(op) {
                        return Ok(false);
                    }
                    let selector = match &record["selector"] {
                        serde_json::Value::Null => None,
                        serde_json::Value::String(s) => Some(Value::Text(s.clone())),
                        q if q.is_object() => Some(Value::Quantity(Quantity::new(
                            q["value_si"]
                                .as_f64()
                                .ok_or_else(|| fail("Invalid replay index."))?,
                            serde_json::from_value(q["dimension"].clone())?,
                        )?)),
                        _ => return Ok(false),
                    };
                    if (arity(op) == Some(2)) != selector.is_some() {
                        return Ok(false);
                    }
                    match perform(&bytes, op, selector.as_ref()) {
                        Ok((v, details)) => {
                            if record["completed"] != true
                                || record["details"] != details
                                || record["result_sha256"] != hash_canonical_json(&v.to_json())?
                            {
                                return Ok(false);
                            }
                        }
                        Err(e) => {
                            if record["completed"] != false
                                || record["error_code"] != e.code
                                || record["error"] != e.message
                            {
                                return Ok(false);
                            }
                        }
                    }
                }
            }
        }
        Ok(true)
    })()
    .unwrap_or(false)
}

pub fn program_uses(program: &crate::ast::Program) -> bool {
    let user_functions = program
        .statements
        .iter()
        .filter_map(|s| {
            if let crate::ast::Stmt::Function { name, .. } = s {
                Some(name.as_str())
            } else {
                None
            }
        })
        .collect::<std::collections::BTreeSet<_>>();
    fn visit(v: &serde_json::Value, users: &std::collections::BTreeSet<&str>) -> bool {
        v.as_array().is_some_and(|a| {
            a.first().is_some_and(|s| s == "call")
                && a.get(1)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|n| is_function(n) && !users.contains(n))
                || a.iter().any(|v| visit(v, users))
        })
    }
    visit(&program.canonical(), &user_functions)
}
pub fn needs_policy(run: &Path, receipt: &serde_json::Value) -> bool {
    if requires_policy(receipt)
        || receipt.get("scientific_input_policy").is_some()
        || receipt["data_imports"].as_array().is_some_and(|a| {
            a.iter()
                .any(|s| matches!(s["format"].as_str(), Some("NPY" | "NPZ" | "ECSV")))
        })
    {
        return true;
    }
    // Preserve historical unknown-symbol failures and user-defined names. A
    // purported successful reader program cannot hide its policy by version downgrade.
    if receipt["status"] != "PASS" {
        return false;
    }
    let source_uses = (|| -> Result<bool> {
        let path = receipt["source"]["path"]
            .as_str()
            .ok_or_else(|| fail("Missing source."))?;
        if Path::new(path).components().count() != 1 {
            return Ok(false);
        }
        let text = String::from_utf8(read_evidence(run, Path::new(path))?)
            .map_err(|_| fail("Preserved source is not UTF-8."))?;
        let mode = crate::parser::evidence_mode(receipt)?;
        let parsed = if let Some(entries) = receipt.get("module_imports") {
            crate::modules::verify_preserved_with_mode(run, &text, entries, mode)?
        } else {
            crate::parser::parse_source_with_mode(&text, mode)?
        };
        Ok(program_uses(&parsed.program))
    })();
    source_uses.unwrap_or(false)
}

/// Bound reads and reject every symlink component; no archive member is extracted.
fn read_evidence(root: &Path, relative: &Path) -> Result<Vec<u8>> {
    let mut path = root.to_path_buf();
    for part in relative.components() {
        if !matches!(part, std::path::Component::Normal(_)) {
            return Err(fail("Unsafe scientific evidence path."));
        }
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(fail("Scientific evidence cannot traverse symlinks."));
        }
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES as u64 {
        return Err(fail("Scientific evidence must be a bounded regular file."));
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err(fail("Scientific evidence grew beyond its byte limit."));
    }
    Ok(bytes)
}
