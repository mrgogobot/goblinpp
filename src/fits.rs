use crate::error::{GoblinError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const CARD_BYTES: usize = 80;
const BLOCK_BYTES: usize = 2880;

#[derive(Debug, Clone)]
enum DataSource {
    File { file: Arc<File>, len: u64 },
    Memory(Arc<[u8]>),
}

impl DataSource {
    fn len(&self) -> u64 {
        match self {
            Self::File { len, .. } => *len,
            Self::Memory(bytes) => bytes.len() as u64,
        }
    }

    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>> {
        let end = offset
            .checked_add(length as u64)
            .ok_or_else(|| GoblinError::data("FITS read offset overflows."))?;
        if end > self.len() {
            return Err(GoblinError::data(format!(
                "FITS read at byte {offset} needs {length} bytes, but the file has {} bytes.",
                self.len()
            )));
        }
        let mut output = vec![0_u8; length];
        match self {
            Self::File { file, .. } => read_file_exact_at(file, &mut output, offset)?,
            Self::Memory(bytes) => {
                let start = usize::try_from(offset)
                    .map_err(|_| GoblinError::data("In-memory FITS offset overflows usize."))?;
                output.copy_from_slice(&bytes[start..start + length]);
            }
        }
        Ok(output)
    }

    fn sha256(&self) -> Result<String> {
        let mut digest = Sha256::new();
        let mut offset = 0_u64;
        while offset < self.len() {
            let length = usize::try_from((self.len() - offset).min(1024 * 1024)).unwrap();
            digest.update(self.read(offset, length)?);
            offset += length as u64;
        }
        Ok(format!("{:x}", digest.finalize()))
    }

    fn copy_to(&self, destination: &Path) -> Result<String> {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(|error| {
                GoblinError::data(format!(
                    "Unable to create FITS evidence {}: {error}",
                    destination.display()
                ))
            })?;
        let mut digest = Sha256::new();
        let mut offset = 0_u64;
        while offset < self.len() {
            let length = usize::try_from((self.len() - offset).min(1024 * 1024)).unwrap();
            let bytes = self.read(offset, length)?;
            output.write_all(&bytes)?;
            digest.update(&bytes);
            offset += length as u64;
        }
        output.flush()?;
        Ok(format!("{:x}", digest.finalize()))
    }
}

#[cfg(unix)]
fn read_file_exact_at(file: &File, buffer: &mut [u8], offset: u64) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.read_exact_at(buffer, offset)
}

#[cfg(windows)]
fn read_file_exact_at(file: &File, buffer: &mut [u8], offset: u64) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    let mut total = 0_usize;
    while total < buffer.len() {
        let read = file.seek_read(&mut buffer[total..], offset + total as u64)?;
        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "short positional FITS read",
            ));
        }
        total += read;
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn read_file_exact_at(file: &File, buffer: &mut [u8], offset: u64) -> std::io::Result<()> {
    use std::io::{Read, Seek, SeekFrom};
    let mut local = file.try_clone()?;
    local.seek(SeekFrom::Start(offset))?;
    local.read_exact(buffer)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HeaderValue {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Text(String),
}

impl HeaderValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Integer(value) => Some(*value as f64),
            Self::Float(value) => Some(*value),
            _ => None,
        }
    }

    pub fn render(&self) -> String {
        match self {
            Self::Boolean(value) => {
                if *value {
                    "T".into()
                } else {
                    "F".into()
                }
            }
            Self::Integer(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::Text(value) => value.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeaderCard {
    pub keyword: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<HeaderValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HduKind {
    Primary,
    Image,
    BinaryTable,
    AsciiTable,
    Unknown,
}

impl HduKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Primary => "PRIMARY",
            Self::Image => "IMAGE",
            Self::BinaryTable => "BINTABLE",
            Self::AsciiTable => "TABLE",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FitsColumn {
    pub index: usize,
    pub name: String,
    pub format: String,
    pub element_type: String,
    pub repeat: usize,
    pub byte_offset: usize,
    pub byte_width: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub scale: f64,
    pub zero: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub null: Option<i64>,
    pub readable: bool,
}

#[derive(Debug, Clone)]
struct ColumnLayout {
    public: FitsColumn,
    code: char,
    element_width: usize,
}

#[derive(Debug, Clone)]
pub struct FitsHdu {
    pub index: usize,
    pub kind: HduKind,
    pub extension_type: Option<String>,
    pub extension_name: Option<String>,
    pub cards: Vec<HeaderCard>,
    pub header: BTreeMap<String, HeaderValue>,
    pub data_offset: u64,
    pub data_length: u64,
    pub bitpix: i32,
    pub axes: Vec<usize>,
    pub rows: Option<usize>,
    pub row_bytes: Option<usize>,
    columns: Vec<ColumnLayout>,
}

impl FitsHdu {
    pub fn columns(&self) -> Vec<FitsColumn> {
        self.columns
            .iter()
            .map(|column| column.public.clone())
            .collect()
    }

    pub fn is_compressed_image(&self) -> bool {
        self.kind == HduKind::BinaryTable
            && matches!(self.header.get("ZIMAGE"), Some(HeaderValue::Boolean(true)))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnValue {
    Number(f64),
    Text(String),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnStats {
    pub valid_count: usize,
    pub mean: f64,
    pub minimum: f64,
    pub maximum: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectedColumnStats {
    pub selected_rows: usize,
    pub used_rows: usize,
    pub weight_sum: f64,
    pub mean: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericSample {
    pub values: Vec<f64>,
    pub population_rows: usize,
    pub examined_rows: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairedNumericSample {
    pub points: Vec<(f64, f64)>,
    pub population_rows: usize,
    pub examined_rows: usize,
}

#[derive(Debug, Clone)]
pub struct FitsFile {
    pub path: PathBuf,
    pub byte_count: u64,
    pub sha256: String,
    pub hdus: Vec<FitsHdu>,
    source: DataSource,
}

impl FitsFile {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_internal(path.as_ref(), true)
    }

    pub fn inspect(path: impl AsRef<Path>, compute_hash: bool) -> Result<Self> {
        Self::open_internal(path.as_ref(), compute_hash)
    }

    fn open_internal(path: &Path, compute_hash: bool) -> Result<Self> {
        let metadata = fs::symlink_metadata(path).map_err(|error| {
            GoblinError::data(format!(
                "Unable to inspect FITS input {}: {error}",
                path.display()
            ))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(GoblinError::data(format!(
                "Refusing symbolic-link FITS input: {}",
                path.display()
            )));
        }
        if !metadata.is_file() {
            return Err(GoblinError::data(format!(
                "FITS input is not a regular file: {}",
                path.display()
            )));
        }
        let file = File::open(path).map_err(|error| {
            GoblinError::data(format!(
                "Unable to open FITS input {}: {error}",
                path.display()
            ))
        })?;
        let len = file.metadata()?.len();
        Self::from_source(
            path.to_path_buf(),
            DataSource::File {
                file: Arc::new(file),
                len,
            },
            compute_hash,
        )
    }

    pub fn from_bytes(path: PathBuf, bytes: Vec<u8>) -> Result<Self> {
        Self::from_source(path, DataSource::Memory(Arc::from(bytes)), true)
    }

    fn from_source(path: PathBuf, source: DataSource, compute_hash: bool) -> Result<Self> {
        if source.len() < BLOCK_BYTES as u64 {
            return Err(GoblinError::data(
                "FITS input is shorter than one 2880-byte header block.",
            ));
        }
        let hdus = parse_hdus(&source)?;
        let sha256 = if compute_hash {
            source.sha256()?
        } else {
            String::new()
        };
        Ok(Self {
            path,
            byte_count: source.len(),
            sha256,
            hdus,
            source,
        })
    }

    pub fn copy_evidence(&self, destination: impl AsRef<Path>) -> Result<String> {
        self.source.copy_to(destination.as_ref())
    }

    pub fn hdu_count(&self) -> usize {
        self.hdus.len()
    }

    pub fn hdu(&self, index: usize) -> Result<&FitsHdu> {
        self.hdus.get(index).ok_or_else(|| {
            GoblinError::data(format!(
                "FITS HDU index {index} is outside 0..{}.",
                self.hdus.len()
            ))
        })
    }

    pub fn header_value(&self, hdu: usize, key: &str) -> Result<HeaderValue> {
        self.hdu(hdu)?
            .header
            .get(&key.to_ascii_uppercase())
            .cloned()
            .ok_or_else(|| GoblinError::data(format!("FITS HDU {hdu} header has no {key} card.")))
    }

    pub fn axis(&self, hdu: usize, one_based: usize) -> Result<usize> {
        let selected = self.image_hdu(hdu, true)?;
        if one_based == 0 || one_based > selected.axes.len() {
            return Err(GoblinError::data(format!(
                "FITS HDU {hdu} axis {one_based} is outside 1..={}.",
                selected.axes.len()
            )));
        }
        Ok(selected.axes[one_based - 1])
    }

    pub fn pixel_count(&self, hdu: usize) -> Result<usize> {
        product_axes(&self.image_hdu(hdu, false)?.axes)
    }

    pub fn pixel(&self, hdu: usize, zero_based: usize) -> Result<f64> {
        let selected = self.image_hdu(hdu, false)?;
        let count = product_axes(&selected.axes)?;
        if zero_based >= count {
            return Err(GoblinError::data(format!(
                "FITS HDU {hdu} pixel index {zero_based} is outside 0..{count}."
            )));
        }
        read_image_value(&self.source, selected, zero_based)
    }

    pub fn mean(&self, hdu: usize) -> Result<f64> {
        let count = self.pixel_count(hdu)?;
        let mut sum = 0.0;
        let mut correction = 0.0;
        let mut valid = 0_usize;
        for index in 0..count {
            let value = self.pixel(hdu, index)?;
            if value.is_nan() {
                continue;
            }
            kahan_add(&mut sum, &mut correction, value);
            valid += 1;
        }
        if valid == 0 {
            return Err(GoblinError::data(format!(
                "FITS HDU {hdu} image has no finite, non-BLANK values."
            )));
        }
        Ok(sum / valid as f64)
    }

    pub fn row_count(&self, hdu: usize) -> Result<usize> {
        self.binary_table(hdu)?
            .rows
            .ok_or_else(|| GoblinError::data(format!("FITS HDU {hdu} has no table row count.")))
    }

    pub fn column_count(&self, hdu: usize) -> Result<usize> {
        Ok(self.binary_table(hdu)?.columns.len())
    }

    pub fn column_value(
        &self,
        hdu: usize,
        name: &str,
        row: usize,
        element: Option<usize>,
    ) -> Result<ColumnValue> {
        let table = self.binary_table(hdu)?;
        let rows = table.rows.unwrap_or(0);
        if row >= rows {
            return Err(GoblinError::data(format!(
                "FITS HDU {hdu} row {row} is outside 0..{rows}."
            )));
        }
        let column = find_column(table, hdu, name)?;
        if !column.public.readable {
            return Err(unsupported_column(hdu, column));
        }
        if column.code == 'A' {
            if element.is_some_and(|value| value != 0) {
                return Err(GoblinError::data(format!(
                    "FITS text column {} is one value per row; element must be 0.",
                    column.public.name
                )));
            }
            let start = cell_start(table, column, row)?;
            let raw = self.source.read(start, column.public.byte_width)?;
            if !raw.is_ascii() {
                return Err(GoblinError::data(format!(
                    "FITS text column {} contains non-ASCII bytes at row {row}.",
                    column.public.name
                )));
            }
            return Ok(ColumnValue::Text(
                String::from_utf8_lossy(&raw).trim_end().to_string(),
            ));
        }
        if column.public.repeat != 1 && element.is_none() {
            return Err(GoblinError::data(format!(
                "FITS column {} contains {} elements per row; supply an explicit element index.",
                column.public.name, column.public.repeat
            )));
        }
        let element = element.unwrap_or(0);
        if element >= column.public.repeat {
            return Err(GoblinError::data(format!(
                "FITS column {} element {element} is outside 0..{}.",
                column.public.name, column.public.repeat
            )));
        }
        read_table_value(&self.source, table, column, row, element)
    }

    pub fn column_stats(&self, hdu: usize, name: &str) -> Result<ColumnStats> {
        let table = self.binary_table(hdu)?;
        let column = find_column(table, hdu, name)?;
        if !column.public.readable || !matches!(column.code, 'B' | 'I' | 'J' | 'K' | 'E' | 'D') {
            return Err(GoblinError::data(format!(
                "FITS column {} with TFORM={} is not a supported real numeric column.",
                column.public.name, column.public.format
            )));
        }
        let mut sum = 0.0;
        let mut correction = 0.0;
        let mut valid = 0_usize;
        let mut minimum = f64::INFINITY;
        let mut maximum = f64::NEG_INFINITY;
        let rows = table.rows.unwrap_or(0);
        let row_bytes = table.row_bytes.unwrap_or(0);
        let rows_per_chunk = (8 * 1024 * 1024 / row_bytes.max(1)).max(1);
        let mut first_row = 0_usize;
        while first_row < rows {
            let chunk_rows = (rows - first_row).min(rows_per_chunk);
            let chunk_offset = table
                .data_offset
                .checked_add((first_row * row_bytes) as u64)
                .ok_or_else(|| GoblinError::data("FITS table chunk offset overflows."))?;
            let chunk = self.source.read(
                chunk_offset,
                chunk_rows
                    .checked_mul(row_bytes)
                    .ok_or_else(|| GoblinError::data("FITS table chunk size overflows."))?,
            )?;
            for local_row in 0..chunk_rows {
                let row = first_row + local_row;
                for element in 0..column.public.repeat {
                    let start = local_row * row_bytes
                        + column.public.byte_offset
                        + element * column.element_width;
                    let end = start + column.element_width;
                    match decode_table_value(&chunk[start..end], column, row)? {
                        ColumnValue::Number(value) => {
                            kahan_add(&mut sum, &mut correction, value);
                            minimum = minimum.min(value);
                            maximum = maximum.max(value);
                            valid += 1;
                        }
                        ColumnValue::Null => {}
                        _ => unreachable!("numeric formats only"),
                    }
                }
            }
            first_row += chunk_rows;
        }
        if valid == 0 {
            return Err(GoblinError::data(format!(
                "FITS HDU {hdu} column {} has no finite, non-null numeric values.",
                column.public.name
            )));
        }
        Ok(ColumnStats {
            valid_count: valid,
            mean: sum / valid as f64,
            minimum,
            maximum,
        })
    }

    pub fn selected_column_stats(
        &self,
        hdu: usize,
        selection_name: &str,
        lower: f64,
        upper: f64,
        value_name: &str,
        weight_name: Option<&str>,
    ) -> Result<SelectedColumnStats> {
        if !lower.is_finite() || !upper.is_finite() || lower >= upper {
            return Err(GoblinError::data(
                "FITS selection requires finite bounds with lower < upper.",
            ));
        }
        let table = self.binary_table(hdu)?;
        let selection = numeric_scalar_column(table, hdu, selection_name)?;
        let value = numeric_scalar_column(table, hdu, value_name)?;
        let weight = weight_name
            .map(|name| numeric_scalar_column(table, hdu, name))
            .transpose()?;
        let rows = table.rows.unwrap_or(0);
        let row_bytes = table.row_bytes.unwrap_or(0);
        const MAX_CHUNK_BYTES: usize = 8 * 1024 * 1024;
        if row_bytes == 0 || row_bytes > MAX_CHUNK_BYTES {
            return Err(GoblinError::data(
                "FITS selected statistics require a nonempty table row of at most 8 MiB.",
            ));
        }
        let rows_per_chunk = (MAX_CHUNK_BYTES / row_bytes).max(1);
        let mut selected_rows = 0_usize;
        let mut used_rows = 0_usize;
        let mut weight_sum = 0.0;
        let mut weight_correction = 0.0;
        let mut weighted_sum = 0.0;
        let mut weighted_correction = 0.0;
        let mut first_row = 0_usize;
        while first_row < rows {
            let chunk_rows = (rows - first_row).min(rows_per_chunk);
            let chunk_offset = first_row
                .checked_mul(row_bytes)
                .and_then(|offset| table.data_offset.checked_add(offset as u64))
                .ok_or_else(|| GoblinError::data("FITS table chunk offset overflows."))?;
            let chunk = self.source.read(
                chunk_offset,
                chunk_rows
                    .checked_mul(row_bytes)
                    .ok_or_else(|| GoblinError::data("FITS table chunk size overflows."))?,
            )?;
            for local_row in 0..chunk_rows {
                let row = first_row + local_row;
                let row_data = &chunk[local_row * row_bytes..(local_row + 1) * row_bytes];
                let selected = numeric_row_value(row_data, selection, row)?;
                let Some(selected) = selected else { continue };
                if selected < lower || selected >= upper {
                    continue;
                }
                selected_rows += 1;
                let row_weight = match weight {
                    Some(column) => numeric_row_value(row_data, column, row)?,
                    None => Some(1.0),
                };
                if let Some(row_weight) = row_weight
                    && row_weight <= 0.0
                {
                    return Err(GoblinError::data(format!(
                        "FITS HDU {hdu} selected row {row} has a non-positive weight."
                    )));
                }
                let row_value = numeric_row_value(row_data, value, row)?;
                let (Some(row_weight), Some(row_value)) = (row_weight, row_value) else {
                    continue;
                };
                let contribution = row_weight * row_value;
                if !contribution.is_finite() {
                    return Err(GoblinError::data(format!(
                        "FITS HDU {hdu} selected row {row} weighted value overflows."
                    )));
                }
                kahan_add(&mut weight_sum, &mut weight_correction, row_weight);
                kahan_add(&mut weighted_sum, &mut weighted_correction, contribution);
                if !weight_sum.is_finite() || !weighted_sum.is_finite() {
                    return Err(GoblinError::data(
                        "FITS selected statistics accumulated a non-finite sum.",
                    ));
                }
                used_rows += 1;
            }
            first_row += chunk_rows;
        }
        if used_rows == 0 {
            return Err(GoblinError::data(format!(
                "FITS HDU {hdu} selection has no rows with valid values and weights."
            )));
        }
        let mean = weighted_sum / weight_sum;
        if !mean.is_finite() {
            return Err(GoblinError::data(
                "FITS selected statistics produced a non-finite mean.",
            ));
        }
        Ok(SelectedColumnStats {
            selected_rows,
            used_rows,
            weight_sum,
            mean,
        })
    }

    pub fn numeric_column_sample(
        &self,
        hdu: usize,
        name: &str,
        requested_points: usize,
    ) -> Result<NumericSample> {
        let table = self.binary_table(hdu)?;
        let column = numeric_scalar_column(table, hdu, name)?;
        let rows = table.rows.unwrap_or(0);
        let indexes = even_row_indexes(rows, requested_points);
        let mut values = Vec::with_capacity(indexes.len());
        for row in &indexes {
            if let ColumnValue::Number(value) =
                read_table_value(&self.source, table, column, *row, 0)?
            {
                values.push(value);
            }
        }
        Ok(NumericSample {
            values,
            population_rows: rows,
            examined_rows: indexes.len(),
        })
    }

    pub fn paired_numeric_column_sample(
        &self,
        hdu: usize,
        x_name: &str,
        y_name: &str,
        requested_points: usize,
    ) -> Result<PairedNumericSample> {
        let table = self.binary_table(hdu)?;
        let x_column = numeric_scalar_column(table, hdu, x_name)?;
        let y_column = numeric_scalar_column(table, hdu, y_name)?;
        let rows = table.rows.unwrap_or(0);
        let indexes = even_row_indexes(rows, requested_points);
        let mut points = Vec::with_capacity(indexes.len());
        for row in &indexes {
            let x = read_table_value(&self.source, table, x_column, *row, 0)?;
            let y = read_table_value(&self.source, table, y_column, *row, 0)?;
            if let (ColumnValue::Number(x), ColumnValue::Number(y)) = (x, y) {
                points.push((x, y));
            }
        }
        Ok(PairedNumericSample {
            points,
            population_rows: rows,
            examined_rows: indexes.len(),
        })
    }

    pub fn inspection(&self) -> Value {
        let hdus = self
            .hdus
            .iter()
            .map(|hdu| {
                let header = hdu
                    .cards
                    .iter()
                    .filter(|card| card.keyword != "END")
                    .map(|card| {
                        json!({
                            "keyword": card.keyword,
                            "value": card.value,
                            "rendered_value": card.value.as_ref().map(HeaderValue::render),
                            "comment": card.comment,
                        })
                    })
                    .collect::<Vec<_>>();
                json!({
                    "index": hdu.index,
                    "type": hdu.kind.label(),
                    "extension_type": hdu.extension_type,
                    "extension_name": hdu.extension_name,
                    "bitpix": hdu.bitpix,
                    "axes": hdu.axes,
                    "data_offset": hdu.data_offset,
                    "data_bytes": hdu.data_length,
                    "rows": hdu.rows,
                    "row_bytes": hdu.row_bytes,
                    "compressed_image": hdu.is_compressed_image(),
                    "header": header,
                    "columns": hdu.columns(),
                })
            })
            .collect::<Vec<_>>();
        let hashed = !self.sha256.is_empty();
        json!({
            "schema": "goblin.fits-info.v1",
            "file": self.path,
            "sha256": if hashed { Some(self.sha256.clone()) } else { None },
            "byte_count": self.byte_count,
            "hdu_count": self.hdus.len(),
            "hdus": hdus,
            "authority": if hashed { "INSPECTION_ONLY_NOT_RUN_EVIDENCE" } else { "QUICK_INSPECTION_UNHASHED_NOT_EVIDENCE" },
        })
    }

    fn image_hdu(&self, hdu: usize, require_axis: bool) -> Result<&FitsHdu> {
        let selected = self.hdu(hdu)?;
        if !matches!(selected.kind, HduKind::Primary | HduKind::Image) {
            return Err(GoblinError::data(format!(
                "FITS HDU {hdu} is {}, not an image HDU.",
                selected.kind.label()
            )));
        }
        if require_axis && selected.axes.is_empty() {
            return Err(GoblinError::data(format!(
                "FITS HDU {hdu} has NAXIS=0 and therefore no image axes."
            )));
        }
        Ok(selected)
    }

    fn binary_table(&self, hdu: usize) -> Result<&FitsHdu> {
        let selected = self.hdu(hdu)?;
        if selected.kind != HduKind::BinaryTable {
            return Err(GoblinError::data(format!(
                "FITS HDU {hdu} is {}, not a binary table.",
                selected.kind.label()
            )));
        }
        if selected.is_compressed_image() {
            return Err(GoblinError::data(format!(
                "FITS HDU {hdu} is a tile-compressed image table; decompression is not implemented."
            )));
        }
        Ok(selected)
    }
}

fn parse_hdus(source: &DataSource) -> Result<Vec<FitsHdu>> {
    let mut hdus = Vec::new();
    let mut offset = 0_u64;
    while offset < source.len() {
        if source.len() - offset < BLOCK_BYTES as u64 {
            return Err(GoblinError::data(format!(
                "FITS HDU {} header is shorter than one 2880-byte block.",
                hdus.len()
            )));
        }
        let (header, cards, data_offset) = parse_header(source, offset)?;
        let index = hdus.len();
        let (kind, extension_type) = classify_hdu(index, &header)?;
        let bitpix_raw = integer(&header, "BITPIX")?;
        let bitpix = i32::try_from(bitpix_raw)
            .map_err(|_| GoblinError::data(format!("Invalid FITS BITPIX={bitpix_raw}.")))?;
        if !matches!(bitpix, 8 | 16 | 32 | 64 | -32 | -64) {
            return Err(GoblinError::data(format!(
                "Unsupported FITS BITPIX={bitpix} in HDU {index}."
            )));
        }
        let naxis = integer(&header, "NAXIS")?;
        if !(0..=999).contains(&naxis) {
            return Err(GoblinError::data(format!(
                "Invalid FITS NAXIS={naxis} in HDU {index}."
            )));
        }
        let mut axes = Vec::with_capacity(naxis as usize);
        for axis in 1..=naxis {
            let length = integer(&header, &format!("NAXIS{axis}"))?;
            if length < 0 {
                return Err(GoblinError::data(format!(
                    "Negative FITS NAXIS{axis} in HDU {index}."
                )));
            }
            axes.push(length as usize);
        }
        let pcount = optional_integer(&header, "PCOUNT", 0)?;
        let gcount = optional_integer(&header, "GCOUNT", 1)?;
        if pcount < 0 || gcount < 1 {
            return Err(GoblinError::data(format!(
                "Invalid FITS PCOUNT/GCOUNT in HDU {index}."
            )));
        }
        if matches!(header.get("GROUPS"), Some(HeaderValue::Boolean(true))) {
            return Err(GoblinError::data(
                "FITS random groups are not supported by the native importer.",
            ));
        }
        let elements = if axes.is_empty() {
            pcount as u64
        } else {
            (product_axes(&axes)? as u64)
                .checked_add(pcount as u64)
                .ok_or_else(|| GoblinError::data("FITS HDU size overflows this platform."))?
        };
        let data_length = elements
            .checked_mul(gcount as u64)
            .and_then(|count| count.checked_mul(bitpix.unsigned_abs() as u64 / 8))
            .ok_or_else(|| GoblinError::data("FITS HDU byte size overflows this platform."))?;
        let padded_length = data_length.div_ceil(BLOCK_BYTES as u64) * BLOCK_BYTES as u64;
        let next_offset = data_offset
            .checked_add(padded_length)
            .ok_or_else(|| GoblinError::data("FITS HDU offset overflows this platform."))?;
        if next_offset > source.len() {
            return Err(GoblinError::data(format!(
                "FITS HDU {index} data is truncated: need {next_offset} bytes, found {}.",
                source.len()
            )));
        }
        let extension_name = text(&header, "EXTNAME");
        let (rows, row_bytes, columns) = if kind == HduKind::BinaryTable {
            let row_bytes = axis_value(&axes, 0, index, "NAXIS1")?;
            let rows = axis_value(&axes, 1, index, "NAXIS2")?;
            let columns = parse_binary_columns(&header, row_bytes, index)?;
            let table_bytes = row_bytes
                .checked_mul(rows)
                .ok_or_else(|| GoblinError::data("FITS table size overflows this platform."))?;
            if table_bytes as u64 > data_length {
                return Err(GoblinError::data(format!(
                    "FITS HDU {index} row area exceeds its declared data size."
                )));
            }
            (Some(rows), Some(row_bytes), columns)
        } else if kind == HduKind::AsciiTable {
            (axes.get(1).copied(), axes.first().copied(), Vec::new())
        } else {
            (None, None, Vec::new())
        };
        hdus.push(FitsHdu {
            index,
            kind,
            extension_type,
            extension_name,
            cards,
            header,
            data_offset,
            data_length,
            bitpix,
            axes,
            rows,
            row_bytes,
            columns,
        });
        if next_offset == offset {
            return Err(GoblinError::data("FITS parser made no forward progress."));
        }
        offset = next_offset;
    }
    if hdus.is_empty() {
        return Err(GoblinError::data("FITS file contains no HDUs."));
    }
    Ok(hdus)
}

fn classify_hdu(
    index: usize,
    header: &BTreeMap<String, HeaderValue>,
) -> Result<(HduKind, Option<String>)> {
    if index == 0 {
        if !matches!(header.get("SIMPLE"), Some(HeaderValue::Boolean(true))) {
            return Err(GoblinError::data(
                "Primary HDU does not declare SIMPLE = T.",
            ));
        }
        return Ok((HduKind::Primary, None));
    }
    let extension = text(header, "XTENSION").ok_or_else(|| {
        GoblinError::data(format!("FITS extension HDU {index} has no XTENSION card."))
    })?;
    let kind = match extension.trim().to_ascii_uppercase().as_str() {
        "IMAGE" => HduKind::Image,
        "BINTABLE" => HduKind::BinaryTable,
        "TABLE" => HduKind::AsciiTable,
        _ => HduKind::Unknown,
    };
    Ok((kind, Some(extension)))
}

fn parse_binary_columns(
    header: &BTreeMap<String, HeaderValue>,
    row_bytes: usize,
    hdu: usize,
) -> Result<Vec<ColumnLayout>> {
    let count = optional_integer(header, "TFIELDS", 0)?;
    if !(0..=999).contains(&count) {
        return Err(GoblinError::data(format!(
            "Invalid FITS TFIELDS={count} in HDU {hdu}."
        )));
    }
    let mut columns = Vec::with_capacity(count as usize);
    let mut byte_offset = 0_usize;
    for one_based in 1..=count as usize {
        let format = text(header, &format!("TFORM{one_based}")).ok_or_else(|| {
            GoblinError::data(format!(
                "FITS HDU {hdu} column {one_based} has no TFORM{one_based} card."
            ))
        })?;
        let parsed = parse_binary_format(&format, hdu, one_based)?;
        let name = text(header, &format!("TTYPE{one_based}"))
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("COL{one_based}"));
        let unit = text(header, &format!("TUNIT{one_based}"));
        let scale = numeric(header, &format!("TSCAL{one_based}"), 1.0)?;
        let zero = numeric(header, &format!("TZERO{one_based}"), 0.0)?;
        let null = match header.get(&format!("TNULL{one_based}")) {
            Some(HeaderValue::Integer(value)) => Some(*value),
            Some(_) => {
                return Err(GoblinError::data(format!(
                    "FITS HDU {hdu} TNULL{one_based} must be an integer."
                )));
            }
            None => None,
        };
        let end = byte_offset
            .checked_add(parsed.byte_width)
            .ok_or_else(|| GoblinError::data("FITS column layout overflows this platform."))?;
        if end > row_bytes {
            return Err(GoblinError::data(format!(
                "FITS HDU {hdu} columns exceed NAXIS1={row_bytes} bytes."
            )));
        }
        columns.push(ColumnLayout {
            public: FitsColumn {
                index: one_based,
                name,
                format: format.trim().to_string(),
                element_type: parsed.element_type,
                repeat: parsed.repeat,
                byte_offset,
                byte_width: parsed.byte_width,
                unit,
                scale,
                zero,
                null,
                readable: parsed.readable,
            },
            code: parsed.code,
            element_width: parsed.element_width,
        });
        byte_offset = end;
    }
    Ok(columns)
}

struct ParsedFormat {
    repeat: usize,
    code: char,
    element_width: usize,
    byte_width: usize,
    element_type: String,
    readable: bool,
}

fn parse_binary_format(format: &str, hdu: usize, column: usize) -> Result<ParsedFormat> {
    let normalized = format.trim().to_ascii_uppercase();
    let digit_count = normalized.bytes().take_while(u8::is_ascii_digit).count();
    let repeat = if digit_count == 0 {
        1
    } else {
        normalized[..digit_count].parse::<usize>().map_err(|_| {
            GoblinError::data(format!(
                "Invalid FITS TFORM{column}={format:?} in HDU {hdu}."
            ))
        })?
    };
    if repeat == 0 {
        return Err(GoblinError::data(format!(
            "FITS TFORM{column} in HDU {hdu} has zero repeat."
        )));
    }
    let code = normalized[digit_count..]
        .chars()
        .next()
        .ok_or_else(|| GoblinError::data(format!("Empty FITS TFORM{column} in HDU {hdu}.")))?;
    let (element_width, byte_width, element_type, readable) = match code {
        'L' => (1, repeat, "logical", true),
        'X' => (0, repeat.div_ceil(8), "bit", false),
        'B' => (1, repeat, "unsigned-byte", true),
        'I' => (2, checked_width(repeat, 2)?, "i16", true),
        'J' => (4, checked_width(repeat, 4)?, "i32", true),
        'K' => (8, checked_width(repeat, 8)?, "i64", true),
        'A' => (1, repeat, "ascii", true),
        'E' => (4, checked_width(repeat, 4)?, "f32", true),
        'D' => (8, checked_width(repeat, 8)?, "f64", true),
        'C' => (8, checked_width(repeat, 8)?, "complex-f32", false),
        'M' => (16, checked_width(repeat, 16)?, "complex-f64", false),
        'P' => (8, checked_width(repeat, 8)?, "variable-array-32", false),
        'Q' => (16, checked_width(repeat, 16)?, "variable-array-64", false),
        _ => {
            return Err(GoblinError::data(format!(
                "Unsupported FITS TFORM{column}={format:?} in HDU {hdu}."
            )));
        }
    };
    Ok(ParsedFormat {
        repeat,
        code,
        element_width,
        byte_width,
        element_type: element_type.into(),
        readable,
    })
}

fn read_image_value(source: &DataSource, hdu: &FitsHdu, index: usize) -> Result<f64> {
    let width = hdu.bitpix.unsigned_abs() as usize / 8;
    let start = hdu
        .data_offset
        .checked_add((index * width) as u64)
        .ok_or_else(|| GoblinError::data("FITS image offset overflows."))?;
    let bytes = source.read(start, width)?;
    let raw_integer = read_integer(&bytes, hdu.bitpix);
    let blank = hdu.header.get("BLANK").and_then(|value| match value {
        HeaderValue::Integer(value) => Some(*value),
        _ => None,
    });
    if raw_integer.is_some_and(|value| Some(value) == blank) {
        return Ok(f64::NAN);
    }
    let raw = match hdu.bitpix {
        -32 => f32::from_be_bytes(bytes[..4].try_into().unwrap()) as f64,
        -64 => f64::from_be_bytes(bytes[..8].try_into().unwrap()),
        _ => raw_integer.unwrap() as f64,
    };
    let scale = numeric(&hdu.header, "BSCALE", 1.0)?;
    let zero = numeric(&hdu.header, "BZERO", 0.0)?;
    scale_value(raw, scale, zero, "FITS image scaling")
}

fn read_table_value(
    source: &DataSource,
    table: &FitsHdu,
    column: &ColumnLayout,
    row: usize,
    element: usize,
) -> Result<ColumnValue> {
    let start = cell_start(table, column, row)?
        .checked_add((element * column.element_width) as u64)
        .ok_or_else(|| GoblinError::data("FITS table element offset overflows."))?;
    let bytes = source.read(start, column.element_width)?;
    decode_table_value(&bytes, column, row)
}

fn decode_table_value(bytes: &[u8], column: &ColumnLayout, row: usize) -> Result<ColumnValue> {
    if column.code == 'L' {
        return match bytes[0] {
            b'T' => Ok(ColumnValue::Boolean(true)),
            b'F' => Ok(ColumnValue::Boolean(false)),
            0 | b' ' | b'?' => Ok(ColumnValue::Null),
            value => Err(GoblinError::data(format!(
                "Invalid FITS logical byte 0x{value:02x} in column {} row {row}.",
                column.public.name
            ))),
        };
    }
    let raw_integer = match column.code {
        'B' => Some(bytes[0] as i64),
        'I' => Some(i16::from_be_bytes(bytes[..2].try_into().unwrap()) as i64),
        'J' => Some(i32::from_be_bytes(bytes[..4].try_into().unwrap()) as i64),
        'K' => Some(i64::from_be_bytes(bytes[..8].try_into().unwrap())),
        _ => None,
    };
    if raw_integer.is_some_and(|value| Some(value) == column.public.null) {
        return Ok(ColumnValue::Null);
    }
    let raw = match column.code {
        'E' => f32::from_be_bytes(bytes[..4].try_into().unwrap()) as f64,
        'D' => f64::from_be_bytes(bytes[..8].try_into().unwrap()),
        _ => raw_integer.unwrap() as f64,
    };
    if raw.is_nan() {
        return Ok(ColumnValue::Null);
    }
    Ok(ColumnValue::Number(scale_value(
        raw,
        column.public.scale,
        column.public.zero,
        &format!("FITS column {} scaling", column.public.name),
    )?))
}

fn cell_start(table: &FitsHdu, column: &ColumnLayout, row: usize) -> Result<u64> {
    let row_offset = row
        .checked_mul(table.row_bytes.unwrap_or(0))
        .ok_or_else(|| GoblinError::data("FITS table row offset overflows."))?;
    table
        .data_offset
        .checked_add(row_offset as u64)
        .and_then(|value| value.checked_add(column.public.byte_offset as u64))
        .ok_or_else(|| GoblinError::data("FITS table cell offset overflows."))
}

fn find_column<'a>(table: &'a FitsHdu, hdu: usize, name: &str) -> Result<&'a ColumnLayout> {
    let matches = table
        .columns
        .iter()
        .filter(|column| column.public.name.eq_ignore_ascii_case(name))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [column] => Ok(column),
        [] => Err(GoblinError::data(format!(
            "FITS HDU {hdu} binary table has no {name} column."
        ))),
        _ => Err(GoblinError::data(format!(
            "FITS HDU {hdu} has multiple columns named {name}; name-based access is ambiguous."
        ))),
    }
}

fn numeric_scalar_column<'a>(
    table: &'a FitsHdu,
    hdu: usize,
    name: &str,
) -> Result<&'a ColumnLayout> {
    let column = find_column(table, hdu, name)?;
    if !column.public.readable
        || column.public.repeat != 1
        || !matches!(column.code, 'B' | 'I' | 'J' | 'K' | 'E' | 'D')
    {
        return Err(GoblinError::data(format!(
            "FITS analysis requires scalar real numeric columns; HDU {hdu} column {} has TFORM={}.",
            column.public.name, column.public.format
        )));
    }
    Ok(column)
}

fn numeric_row_value(row_data: &[u8], column: &ColumnLayout, row: usize) -> Result<Option<f64>> {
    let start = column.public.byte_offset;
    let end = start + column.element_width;
    match decode_table_value(&row_data[start..end], column, row)? {
        ColumnValue::Number(value) => Ok(Some(value)),
        ColumnValue::Null => Ok(None),
        _ => unreachable!("numeric scalar columns only"),
    }
}

fn even_row_indexes(rows: usize, requested_points: usize) -> Vec<usize> {
    let count = rows.min(requested_points);
    match count {
        0 => Vec::new(),
        1 => vec![0],
        _ => (0..count)
            .map(|index| ((index as u128 * (rows - 1) as u128) / (count - 1) as u128) as usize)
            .collect(),
    }
}

fn unsupported_column(hdu: usize, column: &ColumnLayout) -> GoblinError {
    GoblinError::data(format!(
        "FITS HDU {hdu} column {} uses TFORM={}, which is discoverable but not readable in this release.",
        column.public.name, column.public.format
    ))
}

fn parse_header(
    source: &DataSource,
    header_offset: u64,
) -> Result<(BTreeMap<String, HeaderValue>, Vec<HeaderCard>, u64)> {
    let mut header = BTreeMap::new();
    let mut cards = Vec::new();
    let mut block_offset = header_offset;
    loop {
        if block_offset
            .checked_add(BLOCK_BYTES as u64)
            .is_none_or(|end| end > source.len())
        {
            return Err(GoblinError::data(format!(
                "FITS header beginning at byte {header_offset} has no END card before end of file."
            )));
        }
        let block = source.read(block_offset, BLOCK_BYTES)?;
        for (card_index, card_bytes) in block.as_chunks::<CARD_BYTES>().0.iter().enumerate() {
            let card_offset = block_offset + (card_index * CARD_BYTES) as u64;
            if !card_bytes.is_ascii() {
                return Err(GoblinError::data(format!(
                    "Non-ASCII FITS header card at byte {card_offset}."
                )));
            }
            let card = std::str::from_utf8(card_bytes)
                .map_err(|_| GoblinError::data("Invalid FITS header encoding."))?;
            let keyword = card[..8].trim().to_string();
            if keyword == "END" {
                cards.push(HeaderCard {
                    keyword,
                    value: None,
                    comment: None,
                });
                return Ok((header, cards, block_offset + BLOCK_BYTES as u64));
            }
            if keyword.is_empty() {
                continue;
            }
            let (value, comment) = if card[8..].starts_with("= ") {
                let (raw_value, comment) = split_value_comment(&card[10..]);
                (parse_value(raw_value.trim()), comment)
            } else {
                let comment = card[8..].trim();
                (
                    None,
                    if comment.is_empty() {
                        None
                    } else {
                        Some(comment.to_string())
                    },
                )
            };
            if let Some(value) = value.clone() {
                header.insert(keyword.clone(), value);
            }
            cards.push(HeaderCard {
                keyword,
                value,
                comment,
            });
        }
        block_offset = block_offset
            .checked_add(BLOCK_BYTES as u64)
            .ok_or_else(|| GoblinError::data("FITS header offset overflows this platform."))?;
    }
}

fn split_value_comment(field: &str) -> (&str, Option<String>) {
    let bytes = field.as_bytes();
    let mut quoted = false;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\'' {
            if quoted && index + 1 < bytes.len() && bytes[index + 1] == b'\'' {
                index += 2;
                continue;
            }
            quoted = !quoted;
        } else if bytes[index] == b'/' && !quoted {
            let comment = field[index + 1..].trim();
            return (
                &field[..index],
                if comment.is_empty() {
                    None
                } else {
                    Some(comment.to_string())
                },
            );
        }
        index += 1;
    }
    (field, None)
}

fn parse_value(value: &str) -> Option<HeaderValue> {
    if value == "T" {
        return Some(HeaderValue::Boolean(true));
    }
    if value == "F" {
        return Some(HeaderValue::Boolean(false));
    }
    if let Some(stripped) = value.strip_prefix('\'') {
        let mut output = String::new();
        let mut chars = stripped.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\'' {
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    output.push('\'');
                    continue;
                }
                return Some(HeaderValue::Text(output.trim_end().to_string()));
            }
            output.push(ch);
        }
        return Some(HeaderValue::Text(value.to_string()));
    }
    if let Ok(integer) = value.parse::<i64>() {
        return Some(HeaderValue::Integer(integer));
    }
    if let Ok(float) = value.replace(['D', 'd'], "E").parse::<f64>() {
        return Some(HeaderValue::Float(float));
    }
    if value.is_empty() {
        None
    } else {
        Some(HeaderValue::Text(value.to_string()))
    }
}

fn integer(header: &BTreeMap<String, HeaderValue>, key: &str) -> Result<i64> {
    match header.get(key) {
        Some(HeaderValue::Integer(value)) => Ok(*value),
        _ => Err(GoblinError::data(format!(
            "FITS header requires integer {key}."
        ))),
    }
}

fn optional_integer(
    header: &BTreeMap<String, HeaderValue>,
    key: &str,
    default: i64,
) -> Result<i64> {
    match header.get(key) {
        Some(HeaderValue::Integer(value)) => Ok(*value),
        Some(_) => Err(GoblinError::data(format!(
            "FITS header requires integer {key}."
        ))),
        None => Ok(default),
    }
}

fn numeric(header: &BTreeMap<String, HeaderValue>, key: &str, default: f64) -> Result<f64> {
    match header.get(key) {
        Some(value) => value
            .as_f64()
            .ok_or_else(|| GoblinError::data(format!("FITS header requires numeric {key}."))),
        None => Ok(default),
    }
}

fn text(header: &BTreeMap<String, HeaderValue>, key: &str) -> Option<String> {
    match header.get(key) {
        Some(HeaderValue::Text(value)) => Some(value.clone()),
        _ => None,
    }
}

fn product_axes(axes: &[usize]) -> Result<usize> {
    if axes.is_empty() {
        return Ok(0);
    }
    axes.iter().try_fold(1_usize, |total, value| {
        total
            .checked_mul(*value)
            .ok_or_else(|| GoblinError::data("FITS axis product overflows this platform."))
    })
}

fn axis_value(axes: &[usize], index: usize, hdu: usize, keyword: &str) -> Result<usize> {
    axes.get(index).copied().ok_or_else(|| {
        GoblinError::data(format!("FITS binary table HDU {hdu} requires {keyword}."))
    })
}

fn checked_width(repeat: usize, width: usize) -> Result<usize> {
    repeat
        .checked_mul(width)
        .ok_or_else(|| GoblinError::data("FITS TFORM byte width overflows this platform."))
}

fn read_integer(bytes: &[u8], bitpix: i32) -> Option<i64> {
    match bitpix {
        8 => Some(bytes[0] as i64),
        16 => Some(i16::from_be_bytes(bytes[..2].try_into().unwrap()) as i64),
        32 => Some(i32::from_be_bytes(bytes[..4].try_into().unwrap()) as i64),
        64 => Some(i64::from_be_bytes(bytes[..8].try_into().unwrap())),
        _ => None,
    }
}

fn scale_value(raw: f64, scale: f64, zero: f64, context: &str) -> Result<f64> {
    let physical = raw * scale + zero;
    if !physical.is_finite() && !physical.is_nan() {
        return Err(GoblinError::data(format!(
            "{context} produced an infinite value."
        )));
    }
    Ok(physical)
}

fn kahan_add(sum: &mut f64, correction: &mut f64, value: f64) {
    let adjusted = value - *correction;
    let next = *sum + adjusted;
    *correction = (next - *sum) - adjusted;
    *sum = next;
}

#[cfg(test)]
pub(crate) fn fixture_i16(values: &[i16], axes: &[usize], extra: &[(&str, &str)]) -> Vec<u8> {
    let mut cards = vec![
        card("SIMPLE", "T"),
        card("BITPIX", "16"),
        card("NAXIS", &axes.len().to_string()),
    ];
    for (index, axis) in axes.iter().enumerate() {
        cards.push(card(&format!("NAXIS{}", index + 1), &axis.to_string()));
    }
    for (key, value) in extra {
        cards.push(card(key, value));
    }
    let mut bytes = finish_header(cards);
    for value in values {
        bytes.extend(value.to_be_bytes());
    }
    bytes.resize(bytes.len().div_ceil(BLOCK_BYTES) * BLOCK_BYTES, 0);
    bytes
}

#[cfg(test)]
pub(crate) fn fixture_image_and_table() -> Vec<u8> {
    let mut bytes = fixture_i16(&[1, 2, 3, 4], &[2, 2], &[("ORIGIN", "'Goblin++'")]);
    let cards = vec![
        card("XTENSION", "'BINTABLE'"),
        card("BITPIX", "8"),
        card("NAXIS", "2"),
        card("NAXIS1", "20"),
        card("NAXIS2", "3"),
        card("PCOUNT", "0"),
        card("GCOUNT", "1"),
        card("TFIELDS", "3"),
        card("EXTNAME", "'CATALOG'"),
        card("TTYPE1", "'OBJECT'"),
        card("TFORM1", "'8A'"),
        card("TTYPE2", "'Z'"),
        card("TFORM2", "'1D'"),
        card("TTYPE3", "'QUALITY'"),
        card("TFORM3", "'1J'"),
        card("TNULL3", "-999"),
    ];
    bytes.extend(finish_header(cards));
    for (name, redshift, quality) in [
        ("GALAXY", 0.125_f64, 3_i32),
        ("QSO", 2.25_f64, 7_i32),
        ("STAR", f64::NAN, -999_i32),
    ] {
        let mut field = name.as_bytes().to_vec();
        field.resize(8, b' ');
        bytes.extend(field);
        bytes.extend(redshift.to_be_bytes());
        bytes.extend(quality.to_be_bytes());
    }
    bytes.resize(bytes.len().div_ceil(BLOCK_BYTES) * BLOCK_BYTES, 0);
    bytes
}

#[cfg(test)]
fn card(key: &str, value: &str) -> Vec<u8> {
    let mut card = format!("{key:<8}= {value:>20}").into_bytes();
    card.resize(CARD_BYTES, b' ');
    card
}

#[cfg(test)]
fn finish_header(mut cards: Vec<Vec<u8>>) -> Vec<u8> {
    let mut end = format!("{:<8}", "END").into_bytes();
    end.resize(CARD_BYTES, b' ');
    cards.push(end);
    let mut bytes = cards.into_iter().flatten().collect::<Vec<_>>();
    bytes.resize(bytes.len().div_ceil(BLOCK_BYTES) * BLOCK_BYTES, b' ');
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_primary_array_without_c_library() {
        let bytes = fixture_i16(&[1, 2, 3, 4], &[2, 2], &[("BSCALE", "2"), ("BZERO", "1")]);
        let fits = FitsFile::from_bytes(PathBuf::from("fixture.fits"), bytes).unwrap();
        assert_eq!(fits.axis(0, 1).unwrap(), 2);
        assert_eq!(fits.pixel(0, 2).unwrap(), 7.0);
        assert_eq!(fits.mean(0).unwrap(), 6.0);
    }

    #[test]
    fn discovers_hdus_and_reads_binary_table() {
        let fits =
            FitsFile::from_bytes(PathBuf::from("catalog.fits"), fixture_image_and_table()).unwrap();
        assert_eq!(fits.hdu_count(), 2);
        assert_eq!(
            fits.hdu(1).unwrap().extension_name.as_deref(),
            Some("CATALOG")
        );
        assert_eq!(fits.row_count(1).unwrap(), 3);
        assert_eq!(fits.column_count(1).unwrap(), 3);
        assert_eq!(
            fits.column_value(1, "OBJECT", 0, None).unwrap(),
            ColumnValue::Text("GALAXY".into())
        );
        assert_eq!(
            fits.column_value(1, "QUALITY", 2, None).unwrap(),
            ColumnValue::Null
        );
        assert_eq!(fits.column_stats(1, "Z").unwrap().valid_count, 2);
        assert_eq!(fits.column_stats(1, "Z").unwrap().mean, 1.1875);
    }

    #[test]
    fn naxis_zero_can_lead_to_an_extension() {
        let mut bytes = fixture_i16(&[], &[], &[]);
        let extension = fixture_image_and_table();
        bytes.extend_from_slice(&extension[BLOCK_BYTES * 2..]);
        let fits = FitsFile::from_bytes(PathBuf::from("empty-primary.fits"), bytes).unwrap();
        assert_eq!(fits.hdu_count(), 2);
        assert_eq!(fits.pixel_count(0).unwrap(), 0);
    }

    #[test]
    fn random_groups_are_explicitly_refused() {
        let bytes = fixture_i16(&[1], &[1], &[("GROUPS", "T"), ("GCOUNT", "2")]);
        let error = FitsFile::from_bytes(PathBuf::from("groups.fits"), bytes).unwrap_err();
        assert!(error.message.contains("random groups"));
    }

    #[test]
    fn reports_header_keys_and_column_types() {
        let fits =
            FitsFile::from_bytes(PathBuf::from("catalog.fits"), fixture_image_and_table()).unwrap();
        let report = fits.inspection();
        assert_eq!(report["hdu_count"], 2);
        assert_eq!(report["hdus"][1]["type"], "BINTABLE");
        assert_eq!(report["hdus"][1]["columns"][1]["name"], "Z");
        assert_eq!(report["hdus"][1]["columns"][1]["element_type"], "f64");
    }
}
