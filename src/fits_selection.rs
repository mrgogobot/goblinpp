// Included as a child of fits.rs so the exporter shares its checked FITS layout
// and positional reader, but never converts unscaled integer IDs through f64.
use super::*;
use crate::output::{GeneratedOutput, MAX_OUTPUT_BYTES};
use std::collections::BTreeSet;

const FILTER_SCHEMA: &str = "goblin.fits-filter.v1";
const MAX_FILTER_NODES: usize = 256;
const MAX_FILTER_DEPTH: usize = 16;
const MAX_FILTER_BYTES: usize = 65_536;
const MAX_CHUNK_BYTES: usize = 8 * 1024 * 1024;
const NULL_TEXT: &str = "\\N";

pub fn is_selection_function(name: &str) -> bool {
    matches!(
        name,
        "fits_where"
            | "fits_all"
            | "fits_any"
            | "fits_export_csv"
            | "fits_export_tsv"
            | "fits_column_text"
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", deny_unknown_fields)]
pub enum FilterValue {
    Number(f64),
    Text(String),
    Boolean(bool),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompareOp {
    #[serde(rename = "==")]
    Eq,
    #[serde(rename = "!=")]
    Ne,
    #[serde(rename = "<")]
    Lt,
    #[serde(rename = "<=")]
    Le,
    #[serde(rename = ">")]
    Gt,
    #[serde(rename = ">=")]
    Ge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", deny_unknown_fields)]
pub enum FitsFilter {
    Compare {
        column: String,
        comparison: CompareOp,
        value: FilterValue,
    },
    IsNull {
        column: String,
    },
    IsValid {
        column: String,
    },
    All {
        filters: Vec<FitsFilter>,
    },
    Any {
        filters: Vec<FitsFilter>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FilterDocument {
    schema: String,
    filter: FitsFilter,
}

pub fn filter_where(column: &str, op: &str, value: Option<FilterValue>) -> Result<String> {
    let filter = match (op, value) {
        ("is_null", None) => FitsFilter::IsNull {
            column: column.into(),
        },
        ("is_valid", None) => FitsFilter::IsValid {
            column: column.into(),
        },
        (op, Some(value)) => {
            let comparison = match op {
                "==" => CompareOp::Eq,
                "!=" => CompareOp::Ne,
                "<" => CompareOp::Lt,
                "<=" => CompareOp::Le,
                ">" => CompareOp::Gt,
                ">=" => CompareOp::Ge,
                _ => {
                    return Err(GoblinError::data(
                        "fits_where() comparison must be ==, !=, <, <=, > or >=; null tests take no third argument.",
                    ));
                }
            };
            FitsFilter::Compare {
                column: column.into(),
                comparison,
                value,
            }
        }
        _ => {
            return Err(GoblinError::data(
                "fits_where() requires a value for comparisons, or two arguments for is_null/is_valid.",
            ));
        }
    };
    filter_text(filter)
}

pub fn filter_group(filters: &[String], all: bool) -> Result<String> {
    if filters.len() > MAX_FILTER_NODES {
        return Err(GoblinError::data("FITS filter group exceeds 256 nodes."));
    }
    let filters = filters
        .iter()
        .map(|s| filter_from_text(s))
        .collect::<Result<Vec<_>>>()?;
    filter_text(if all {
        FitsFilter::All { filters }
    } else {
        FitsFilter::Any { filters }
    })
}

fn filter_text(filter: FitsFilter) -> Result<String> {
    validate_filter(&filter)?;
    let text = serde_json::to_string(&FilterDocument {
        schema: FILTER_SCHEMA.into(),
        filter,
    })?;
    if text.len() > MAX_FILTER_BYTES {
        return Err(GoblinError::data("FITS filter exceeds 65536 bytes."));
    }
    Ok(text)
}

pub fn filter_from_text(text: &str) -> Result<FitsFilter> {
    if text.len() > MAX_FILTER_BYTES {
        return Err(GoblinError::data("FITS filter exceeds 65536 bytes."));
    }
    let document: FilterDocument = serde_json::from_str(text)
        .map_err(|e| GoblinError::data(format!("Invalid FITS filter document: {e}")))?;
    if document.schema != FILTER_SCHEMA {
        return Err(GoblinError::data("Unsupported FITS filter schema."));
    }
    validate_filter(&document.filter)?;
    Ok(document.filter)
}

fn validate_filter(filter: &FitsFilter) -> Result<()> {
    fn visit(filter: &FitsFilter, depth: usize, nodes: &mut usize) -> Result<()> {
        *nodes += 1;
        if depth > MAX_FILTER_DEPTH || *nodes > MAX_FILTER_NODES {
            return Err(GoblinError::data(
                "FITS filter exceeds 16 levels or 256 nodes.",
            ));
        }
        match filter {
            FitsFilter::All { filters } | FitsFilter::Any { filters } => {
                if filters.is_empty() {
                    return Err(GoblinError::data(
                        "fits_all()/fits_any() requires at least one filter; an empty group never silently selects all rows.",
                    ));
                }
                for filter in filters {
                    visit(filter, depth + 1, nodes)?;
                }
            }
            FitsFilter::Compare { column, value, .. } => {
                valid_column_name(column)?;
                if matches!(value, FilterValue::Number(n) if !n.is_finite()) {
                    return Err(GoblinError::data("FITS filter bounds must be finite."));
                }
            }
            FitsFilter::IsNull { column } | FitsFilter::IsValid { column } => {
                valid_column_name(column)?
            }
        }
        Ok(())
    }
    visit(filter, 1, &mut 0)
}

fn valid_column_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 1024 || name.chars().any(char::is_control) {
        return Err(GoblinError::data(
            "FITS filter column names must be nonempty text without control characters (at most 1024 bytes).",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum Cell {
    Integer(i64),
    Number(f64),
    Text(String),
    Boolean(bool),
    Null,
}

fn scalar_column<'a>(table: &'a FitsHdu, hdu: usize, name: &str) -> Result<&'a ColumnLayout> {
    let column = find_column(table, hdu, name)?;
    if !column.public.scale.is_finite() || !column.public.zero.is_finite() {
        return Err(GoblinError::data(
            "FITS subset column scaling must be finite.",
        ));
    }
    if !column.public.readable || (column.code != 'A' && column.public.repeat != 1) {
        return Err(GoblinError::data(format!(
            "FITS subset access requires supported scalar columns; {name} has TFORM={}.",
            column.public.format
        )));
    }
    Ok(column)
}

fn exact_cell(bytes: &[u8], column: &ColumnLayout, row: usize) -> Result<Cell> {
    if column.code == 'A' {
        if !bytes.is_ascii() || bytes.contains(&0) {
            return Err(GoblinError::data(format!(
                "FITS text column {} row {row} must be ASCII without NUL bytes.",
                column.public.name
            )));
        }
        return Ok(Cell::Text(
            String::from_utf8_lossy(bytes)
                .trim_end_matches(' ')
                .to_string(),
        ));
    }
    let integer = match column.code {
        'B' => Some(bytes[0] as i64),
        'I' => Some(i16::from_be_bytes(bytes.try_into().unwrap()) as i64),
        'J' => Some(i32::from_be_bytes(bytes.try_into().unwrap()) as i64),
        'K' => Some(i64::from_be_bytes(bytes.try_into().unwrap())),
        _ => None,
    };
    if let Some(value) = integer {
        if Some(value) == column.public.null {
            return Ok(Cell::Null);
        }
        if column.public.scale == 1.0 && column.public.zero == 0.0 {
            return Ok(Cell::Integer(value));
        }
        if value.unsigned_abs() > 9_007_199_254_740_991 {
            return Err(GoblinError::data(format!(
                "FITS scaled integer column {} row {row} exceeds safe f64 precision; scaled 64-bit ID export is not supported. No approximate ID is exported.",
                column.public.name
            )));
        }
    }
    match decode_table_value(bytes, column, row)? {
        ColumnValue::Number(value)
            if integer.is_some() && value.abs() > 9_007_199_254_740_991.0 =>
        {
            Err(GoblinError::data(
                "FITS scaled integer physical value exceeds safe f64 precision; exact scaled catalogue IDs are not supported.",
            ))
        }
        ColumnValue::Number(value) if value.is_finite() => Ok(Cell::Number(value)),
        ColumnValue::Number(value) if value.is_nan() => Ok(Cell::Null),
        ColumnValue::Number(_) => Err(GoblinError::data("FITS subset cell is not finite.")),
        ColumnValue::Boolean(value) => Ok(Cell::Boolean(value)),
        ColumnValue::Null => Ok(Cell::Null),
        ColumnValue::Text(_) => unreachable!(),
    }
}

fn row_cell(row_data: &[u8], column: &ColumnLayout, row: usize) -> Result<Cell> {
    exact_cell(
        &row_data[column.public.byte_offset..column.public.byte_offset + column.public.byte_width],
        column,
        row,
    )
}

enum BoundFilter<'a> {
    Compare(&'a ColumnLayout, CompareOp, Cell),
    IsNull(&'a ColumnLayout),
    IsValid(&'a ColumnLayout),
    All(Vec<BoundFilter<'a>>),
    Any(Vec<BoundFilter<'a>>),
}

fn bind_filter<'a>(
    filter: &FitsFilter,
    table: &'a FitsHdu,
    hdu: usize,
    columns: &mut BTreeSet<String>,
) -> Result<BoundFilter<'a>> {
    Ok(match filter {
        FitsFilter::All { filters } => BoundFilter::All(
            filters
                .iter()
                .map(|f| bind_filter(f, table, hdu, columns))
                .collect::<Result<_>>()?,
        ),
        FitsFilter::Any { filters } => BoundFilter::Any(
            filters
                .iter()
                .map(|f| bind_filter(f, table, hdu, columns))
                .collect::<Result<_>>()?,
        ),
        FitsFilter::Compare {
            column,
            comparison,
            value,
        } => {
            let layout = scalar_column(table, hdu, column)?;
            columns.insert(layout.public.name.clone());
            let bound = match layout.code {
                'A' => match value {
                    FilterValue::Text(s) => Cell::Text(s.clone()),
                    _ => {
                        return Err(GoblinError::data(
                            "FITS text comparisons require text bounds.",
                        ));
                    }
                },
                'L' => match value {
                    FilterValue::Boolean(b)
                        if matches!(comparison, CompareOp::Eq | CompareOp::Ne) =>
                    {
                        Cell::Boolean(*b)
                    }
                    _ => {
                        return Err(GoblinError::data(
                            "FITS logical comparisons require true/false and ==/!=.",
                        ));
                    }
                },
                'B' | 'I' | 'J' | 'K'
                    if layout.public.scale == 1.0 && layout.public.zero == 0.0 =>
                {
                    let integer = match value {
                        FilterValue::Text(s) if !s.is_empty() && s.trim() == s => {
                            s.parse::<i64>().map_err(|_| {
                                GoblinError::data(
                                    "Exact FITS integer bounds require signed 64-bit decimal text.",
                                )
                            })?
                        }
                        FilterValue::Number(n)
                            if n.is_finite()
                                && n.fract() == 0.0
                                && n.abs() <= 9_007_199_254_740_991.0 =>
                        {
                            *n as i64
                        }
                        _ => {
                            return Err(GoblinError::data(
                                "FITS integer comparisons require a safe integral number or exact decimal text; quote large catalogue IDs.",
                            ));
                        }
                    };
                    Cell::Integer(integer)
                }
                _ => match value {
                    FilterValue::Number(n) if n.is_finite() => Cell::Number(*n),
                    _ => {
                        return Err(GoblinError::data(
                            "FITS floating/scaled comparisons require finite numeric bounds, not text.",
                        ));
                    }
                },
            };
            BoundFilter::Compare(layout, *comparison, bound)
        }
        FitsFilter::IsNull { column } | FitsFilter::IsValid { column } => {
            let layout = scalar_column(table, hdu, column)?;
            columns.insert(layout.public.name.clone());
            if matches!(filter, FitsFilter::IsNull { .. }) {
                BoundFilter::IsNull(layout)
            } else {
                BoundFilter::IsValid(layout)
            }
        }
    })
}

fn ordered<T: PartialOrd>(a: &T, b: &T, op: CompareOp) -> bool {
    match op {
        CompareOp::Eq => a == b,
        CompareOp::Ne => a != b,
        CompareOp::Lt => a < b,
        CompareOp::Le => a <= b,
        CompareOp::Gt => a > b,
        CompareOp::Ge => a >= b,
    }
}

impl BoundFilter<'_> {
    // Every leaf is evaluated: null accounting and malformed-cell failures do
    // not depend on AND/OR short-circuit order. Null comparisons, including !=,
    // are false; use an explicit is_null branch to include null rows.
    fn evaluate(&self, data: &[u8], row: usize) -> Result<(bool, bool)> {
        match self {
            Self::All(filters) | Self::Any(filters) => {
                let all = matches!(self, Self::All(_));
                let mut selected = all;
                let mut null = false;
                for filter in filters {
                    let (matches, missing) = filter.evaluate(data, row)?;
                    selected = if all {
                        selected && matches
                    } else {
                        selected || matches
                    };
                    null |= missing;
                }
                Ok((selected, null))
            }
            Self::IsNull(column) | Self::IsValid(column) => {
                let null = matches!(row_cell(data, column, row)?, Cell::Null);
                Ok((
                    if matches!(self, Self::IsNull(_)) {
                        null
                    } else {
                        !null
                    },
                    null,
                ))
            }
            Self::Compare(column, op, bound) => {
                let cell = row_cell(data, column, row)?;
                let selected = match (&cell, bound) {
                    (Cell::Null, _) => return Ok((false, true)),
                    (Cell::Integer(a), Cell::Integer(b)) => ordered(a, b, *op),
                    (Cell::Number(a), Cell::Number(b)) => ordered(a, b, *op),
                    (Cell::Text(a), Cell::Text(b)) => ordered(a, b, *op),
                    (Cell::Boolean(a), Cell::Boolean(b)) => ordered(a, b, *op),
                    _ => {
                        return Err(GoblinError::data(
                            "FITS comparison cell/bound type mismatch.",
                        ));
                    }
                };
                Ok((selected, false))
            }
        }
    }
}

fn cell_text(cell: Cell) -> String {
    match cell {
        Cell::Integer(n) => n.to_string(),
        Cell::Number(n) => crate::text_runtime::format_number(n),
        Cell::Text(s) => s,
        Cell::Boolean(b) => b.to_string(),
        Cell::Null => NULL_TEXT.into(),
    }
}

fn write_row(bytes: &mut Vec<u8>, cells: &[String], delimiter: u8) -> Result<()> {
    // Unlike the older general-purpose TSV writer, never silently strip text.
    if delimiter == b'\t' && cells.iter().any(|s| s.contains(['\t', '\r', '\n'])) {
        return Err(GoblinError::data(
            "FITS TSV export refuses tabs/newlines in text cells; use CSV for lossless quoting.",
        ));
    }
    let row = crate::output::delimited_bytes(&[cells.to_vec()], delimiter);
    if row.len() > MAX_OUTPUT_BYTES.saturating_sub(bytes.len()) {
        return Err(GoblinError::artifact(
            "FITS subset export exceeds the 64 MiB output limit; no truncated output is created.",
        ));
    }
    bytes.extend(row);
    Ok(())
}

impl FitsFile {
    pub fn column_text(&self, hdu: usize, name: &str, row: usize) -> Result<String> {
        let table = self.binary_table(hdu)?;
        if row >= table.rows.unwrap_or(0) {
            return Err(GoblinError::data("FITS row index is outside the table."));
        }
        let column = scalar_column(table, hdu, name)?;
        if column.public.byte_width > crate::text_runtime::MAX_TEXT_BYTES {
            return Err(GoblinError::data(
                "fits_column_text() cell exceeds the 1 MiB text limit; use a subset export.",
            ));
        }
        let raw = self
            .source
            .read(cell_start(table, column, row)?, column.public.byte_width)?;
        let cell = exact_cell(&raw, column, row)?;
        if matches!(cell, Cell::Null) {
            return Err(GoblinError::data("fits_column_text() cell is null/NaN."));
        }
        crate::text_runtime::bounded(cell_text(cell)).map_err(GoblinError::data)
    }

    pub fn export_subset(
        &self,
        name: &str,
        hdu: usize,
        names: &[String],
        filter: &FitsFilter,
        tsv: bool,
    ) -> Result<(GeneratedOutput, usize)> {
        validate_filter(filter)?;
        let extension = if tsv { "tsv" } else { "csv" };
        if crate::output::extension(name)? != extension {
            return Err(GoblinError::artifact(format!(
                "FITS subset output must use .{extension}."
            )));
        }
        if names.is_empty() || names.len() > 1024 {
            return Err(GoblinError::data(
                "FITS export requires 1..1024 projected column names.",
            ));
        }
        let table = self.binary_table(hdu)?;
        let columns = names
            .iter()
            .map(|s| scalar_column(table, hdu, s))
            .collect::<Result<Vec<_>>>()?;
        let headers = columns
            .iter()
            .map(|c| c.public.name.clone())
            .collect::<Vec<_>>();
        if headers
            .iter()
            .map(|s| s.to_ascii_uppercase())
            .collect::<BTreeSet<_>>()
            .len()
            != headers.len()
        {
            return Err(GoblinError::data(
                "FITS export projection contains duplicate columns.",
            ));
        }
        if headers.iter().any(|s| s == NULL_TEXT) {
            return Err(GoblinError::data(
                "FITS column name collides with the reserved null marker.",
            ));
        }
        let mut predicate_columns = BTreeSet::new();
        let bound = bind_filter(filter, table, hdu, &mut predicate_columns)?;
        let predicate_layouts = predicate_columns
            .iter()
            .map(|s| find_column(table, hdu, s).map(|c| c.public.clone()))
            .collect::<Result<Vec<_>>>()?;
        let rows = table.rows.unwrap_or(0);
        let row_bytes = table.row_bytes.unwrap_or(0);
        if row_bytes == 0 || row_bytes > MAX_CHUNK_BYTES {
            return Err(GoblinError::data(
                "FITS subset rows must occupy 1 byte to 8 MiB.",
            ));
        }
        let mut bytes = Vec::new();
        let delimiter = if tsv { b'\t' } else { b',' };
        write_row(&mut bytes, &headers, delimiter)?;
        let mut first = 0;
        let mut selected = 0;
        let mut predicate_null_rows = 0;
        let mut null_counts = vec![0_usize; columns.len()];
        while first < rows {
            let count = (rows - first).min(MAX_CHUNK_BYTES / row_bytes);
            let offset = first
                .checked_mul(row_bytes)
                .and_then(|n| table.data_offset.checked_add(n as u64))
                .ok_or_else(|| GoblinError::data("FITS subset chunk offset overflows."))?;
            let chunk = self.source.read(offset, count * row_bytes)?;
            for local in 0..count {
                let row = first + local;
                let data = &chunk[local * row_bytes..(local + 1) * row_bytes];
                let (matches, null) = bound.evaluate(data, row)?;
                predicate_null_rows += usize::from(null);
                if !matches {
                    continue;
                }
                let cells = columns.iter().enumerate().map(|(index, column)| {
                    let cell = row_cell(data, column, row)?;
                    if matches!(cell, Cell::Null) { null_counts[index] += 1; }
                    if matches!(&cell, Cell::Text(s) if s == NULL_TEXT) { return Err(GoblinError::data("FITS text value collides with reserved null marker \\N; no ambiguous subset is exported.")); }
                    Ok(cell_text(cell))
                }).collect::<Result<Vec<_>>>()?;
                write_row(&mut bytes, &cells, delimiter)?;
                selected += 1;
            }
            first += count;
        }
        let metadata = json!({
            "schema": "goblin.fits-subset.v1", "input_sha256": self.sha256,
            "hdu": hdu, "input_rows": rows, "selected_rows": selected, "rejected_rows": rows - selected,
            "row_order": "original", "projection": columns.iter().map(|c| &c.public).collect::<Vec<_>>(),
            "filter_schema": FILTER_SCHEMA, "filter": filter, "predicate_columns": predicate_layouts,
            "predicate_null_rows": predicate_null_rows, "output_null_counts": null_counts,
            "null_encoding": NULL_TEXT, "null_comparisons": "false_including_not_equal",
            "integer_serialization": "unscaled_signed_i64_decimal_exact", "float_serialization": "shortest_round_trip",
            "units": "reported_not_converted", "chunk_bytes_limit": MAX_CHUNK_BYTES,
            "output_bytes_limit": MAX_OUTPUT_BYTES, "physical_blinding": false,
        });
        let artifact = GeneratedOutput::new(
            name,
            if tsv {
                "text/tab-separated-values; charset=utf-8"
            } else {
                "text/csv; charset=utf-8"
            },
            if tsv {
                "fits_export_tsv"
            } else {
                "fits_export_csv"
            },
            bytes,
            metadata,
        )?;
        Ok((artifact, selected))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_limit_refuses_whole_row_without_truncation() {
        let mut bytes = vec![b'x'; MAX_OUTPUT_BYTES - 1];
        assert!(write_row(&mut bytes, &["ab".into()], b',').is_err());
        assert_eq!(bytes.len(), MAX_OUTPUT_BYTES - 1);
        assert!(bytes.iter().all(|b| *b == b'x'));
    }
}
