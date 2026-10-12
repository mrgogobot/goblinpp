//! Bounded binary-table projection. Text preserves IDs; units are metadata,
//! not silently attached to parsed quantities. No hidden cuts or dropped rows.
use super::*;
use crate::evaluator::Value as RuntimeValue;
use serde_json::Value as Json;

pub const POLICY_ID: &str = "goblin.serial-fits-batches.v1";
pub const MAX_HDUS: usize = 128;
pub const MAX_HEADER_BYTES: usize = 1024 * 1024;
pub const MAX_ROW_BYTES: usize = 1024 * 1024;
pub const NULL_TEXT: &str = "\\N";

pub fn policy() -> Json {
    json!({"id":POLICY_ID,"table":"uncompressed-fixed-width-BINTABLE",
        "columns":"exact-case-sensitive-distinct-scalar-or-fixed-ASCII",
        "layout":"flat-row-major-text","order":"original-table-row-order",
        "integers":"unscaled-signed-integers-exact-decimal-text",
        "scaled_integers":"refuse-outside-safe-f64-range",
        "numbers":"scaled-shortest-roundtrip-finite-f64",
        "nulls":"NaN-TNULL-logical-null-as-marker-no-row-drops","null_marker":NULL_TEXT,
        "marker_collision":"refuse-literal-text-marker","text":"ASCII-without-NUL-right-space-trim",
        "units":"TUNIT-reported-not-converted","input":"private-disk-snapshot-before-parse",
        "maximum_hdus":MAX_HDUS,"maximum_header_bytes":MAX_HEADER_BYTES,
        "maximum_raw_row_bytes":MAX_ROW_BYTES,"read_buffer_bytes":crate::batches::BUFFER_BYTES,
        "batch_lifecycle_policy":crate::batches::POLICY_ID,"physical_blinding":false})
}
pub fn requires_policy(receipt: &Json) -> bool {
    let v = receipt["goblin_version"].as_str().unwrap_or("");
    !(v.starts_with("0.0.")
        || v.strip_prefix("0.1.0-alpha.")
            .and_then(|s| s.parse::<u32>().ok())
            .is_some_and(|n| n < 32))
}
pub fn frozen_policy_matches(receipt: &Json) -> bool {
    receipt
        .get("fits_batch_policy")
        .map_or(!requires_policy(receipt), |p| p == &policy())
}
pub fn program_uses(program: &crate::ast::Program) -> bool {
    fn visit(v: &Json) -> bool {
        v.as_array().is_some_and(|a| {
            (a.first().is_some_and(|x| x == "call")
                && a.get(1).is_some_and(|x| x == "fits_batch_open"))
                || a.iter().any(visit)
        })
    }
    visit(&program.canonical())
}

#[derive(Debug)]
pub(crate) struct Source {
    file: FitsFile,
    hdu: usize,
    columns: Vec<ColumnLayout>,
    next_row: usize,
    buffer: Vec<u8>,
    first_buffer_row: usize,
    buffer_rows: usize,
}
impl Source {
    pub(crate) fn open(path: &Path, hdu: usize, names: &[String]) -> Result<Self> {
        let file = FitsFile::inspect_batch(path)?;
        let table = file.binary_table(hdu)?;
        if table.bitpix != 8
            || table.axes.len() != 2
            || optional_integer(&table.header, "GCOUNT", 1)? != 1
        {
            return Err(GoblinError::data(
                "FITS batch BINTABLE requires BITPIX=8, NAXIS=2 and GCOUNT=1.",
            ));
        }
        let width = table.row_bytes.unwrap_or(0);
        if width == 0 || width > MAX_ROW_BYTES {
            return Err(GoblinError::data(
                "FITS batch raw row width must be 1 to 1 MiB bytes.",
            ));
        }
        let columns = names.iter().map(|name| {
            let mut found = table.columns.iter().filter(|c| c.public.name == *name);
            let column = found.next().ok_or_else(|| GoblinError::data(format!(
                "FITS batch HDU {hdu} has no exact case-sensitive column {name:?}.")))?;
            if found.next().is_some() {
                return Err(GoblinError::data(format!("Ambiguous duplicate FITS column {name:?}.")));
            }
            if !column.public.readable || (column.code != 'A' && column.public.repeat != 1)
                || !column.public.scale.is_finite() || !column.public.zero.is_finite()
            {
                return Err(GoblinError::data(format!("FITS batch column {name:?} requires a supported scalar or fixed ASCII field with finite scaling; TFORM={:?}.", column.public.format)));
            }
            Ok(column.clone())
        }).collect::<Result<Vec<_>>>()?;
        Ok(Self {
            file,
            hdu,
            columns,
            next_row: 0,
            buffer: Vec::new(),
            first_buffer_row: 0,
            buffer_rows: 0,
        })
    }
    pub(crate) fn metadata(&self) -> Json {
        let table = &self.file.hdus[self.hdu];
        json!({"policy":POLICY_ID,"hdu":self.hdu,"extension_name":table.extension_name,
            "table_rows":table.rows.unwrap(),"raw_row_bytes":table.row_bytes.unwrap(),
            "data_offset":table.data_offset,"columns":self.columns.iter().map(|c| &c.public).collect::<Vec<_>>(),
            "null_marker":NULL_TEXT,"units":"reported-not-converted"})
    }
    pub(crate) fn next(&mut self) -> Result<Option<Vec<String>>> {
        let table = &self.file.hdus[self.hdu];
        let total = table.rows.unwrap();
        if self.next_row == total {
            self.buffer.clear();
            self.buffer.shrink_to_fit();
            return Ok(None);
        }
        let width = table.row_bytes.unwrap();
        if self.next_row >= self.first_buffer_row + self.buffer_rows {
            self.first_buffer_row = self.next_row;
            self.buffer_rows =
                (total - self.next_row).min((crate::batches::BUFFER_BYTES / width).max(1));
            self.buffer = self.file.source.read(
                table.data_offset + (self.next_row * width) as u64,
                self.buffer_rows * width,
            )?;
        }
        let start = (self.next_row - self.first_buffer_row) * width;
        let raw = &self.buffer[start..start + width];
        let values = self
            .columns
            .iter()
            .map(|c| {
                selection::batch_cell_text(raw, c, self.next_row).map_err(|e| {
                    GoblinError::data(format!(
                        "FITS batch HDU {} column {:?} row {}: {}",
                        self.hdu, c.public.name, self.next_row, e
                    ))
                })
            })
            .collect::<Result<Vec<_>>>()?;
        self.next_row += 1;
        Ok(Some(values))
    }
}

/// Metadata/lifecycle binding to the preserved input, not a replay of science.
pub fn validate_snapshots(run: &Path, receipt: &Json) -> bool {
    let Some(readers) = receipt["batches"]["readers"].as_array() else {
        return false;
    };
    for r in readers.iter().filter(|r| r.get("fits").is_some()) {
        let Some(imports) = receipt["data_imports"].as_array() else {
            return false;
        };
        let Some(d) = imports
            .iter()
            .find(|d| d["sha256"] == r["sha256"] && d["path"] == r["path"])
        else {
            return false;
        };
        let Some(path) = d["evidence_path"].as_str() else {
            return false;
        };
        // Do not follow a resealed path outside the run, or symlink evidence.
        let relative = Path::new(path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return false;
        }
        let target = run.join(relative);
        let Ok(root) = run.canonicalize() else {
            return false;
        };
        if !target.canonicalize().is_ok_and(|p| p.starts_with(&root))
            || !fs::symlink_metadata(&target)
                .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
        {
            return false;
        }
        let Some(hdu) = r["fits"]["hdu"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
        else {
            return false;
        };
        let Some(names) = r["columns"].as_array().and_then(|a| {
            a.iter()
                .map(|v| v.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        }) else {
            return false;
        };
        // Failed open/metadata validation has no bound schema and remains a failure.
        if r["fits"].get("columns").is_none() {
            if receipt["status"] == "PASS" || r["failed"] != true {
                return false;
            }
            continue;
        }
        let Ok(source) = Source::open(&target, hdu, &names) else {
            return false;
        };
        if source.metadata() != r["fits"] {
            return false;
        }
        let Some(rows) = r["rows"].as_u64() else {
            return false;
        };
        let total = r["fits"]["table_rows"].as_u64().unwrap();
        if rows > total || (r["eof"] == true && rows != total) {
            return false;
        }
    }
    true
}

pub(crate) fn hdu_argument(value: &RuntimeValue) -> Result<usize> {
    let q = value.quantity("fits_batch_open HDU")?;
    if q.dimension != crate::quantity::DIMENSIONLESS
        || !q.value_si.is_finite()
        || q.value_si.fract() != 0.0
        || !(0.0..MAX_HDUS as f64).contains(&q.value_si)
    {
        return Err(GoblinError::data(
            "fits_batch_open HDU must be a zero-based integer from 0 to 127.",
        ));
    }
    Ok(q.value_si as usize)
}
