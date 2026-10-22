//! Exact text-key, bounded one-to-one joins. No numeric ID coercion or row guessing.
use crate::{
    delimited::{MAX_INPUT_BYTES, MAX_ROWS, Table},
    error::{GoblinError, Result},
    output::GeneratedOutput,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

pub const POLICY: &str = "goblin.table-join.exact-text.one-to-one.v1";
pub const MATCH_COLUMN: &str = "__join_matched";
pub fn supports_receipt(receipt: &Value) -> bool {
    let required = receipt["goblin_version"].as_str().is_some_and(|v| {
        !v.starts_with("0.0.")
            && v.strip_prefix("0.1.0-alpha.")
                .is_none_or(|n| n.parse::<u64>().is_ok_and(|n| n >= 38))
    });
    receipt
        .get("table_join_policy")
        .map_or(!required, |p| p == POLICY)
}
pub fn is_function(name: &str) -> bool {
    matches!(name, "csv_join" | "tsv_join")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub function: String,
    pub left_key: String,
    pub right_key: String,
    pub kind: String,
    pub left_prefix: String,
    pub right_prefix: String,
}
impl Rules {
    pub fn delimiter(&self) -> u8 {
        if self.function == "csv_join" {
            b','
        } else {
            b'\t'
        }
    }
    pub fn validate(&self, output: &str) -> Result<()> {
        crate::output::validate_name(output)?;
        let extension = if self.function == "csv_join" {
            "csv"
        } else {
            "tsv"
        };
        if !is_function(&self.function)
            || Path::new(output).extension().and_then(|v| v.to_str()) != Some(extension)
        {
            return Err(GoblinError::data(
                "Join function and output extension must agree: csv_join uses .csv, tsv_join uses .tsv.",
            ));
        }
        if !matches!(self.kind.as_str(), "inner" | "left") {
            return Err(GoblinError::data(
                "Join kind must explicitly be \"inner\" or \"left\".",
            ));
        }
        if self.left_key.is_empty() || self.right_key.is_empty() {
            return Err(GoblinError::data(
                "Join keys must name exact, nonempty headers.",
            ));
        }
        for prefix in [&self.left_prefix, &self.right_prefix] {
            if prefix.is_empty() || prefix.len() > 128 || prefix.chars().any(char::is_control) {
                return Err(GoblinError::data(
                    "Each join column prefix must be nonempty, at most 128 UTF-8 bytes, without control characters.",
                ));
            }
        }
        Ok(())
    }
}

fn key_index(table: &Table, key: &str, side: &str) -> Result<usize> {
    table.headers.iter().position(|h| h == key).ok_or_else(|| {
        GoblinError::data(format!(
            "Join {side} input has no exact key header {key:?}."
        ))
    })
}
fn index<'a>(table: &'a Table, column: usize, side: &str) -> Result<BTreeMap<&'a str, usize>> {
    let mut keys = BTreeMap::new();
    for (row, cells) in table.rows.iter().enumerate() {
        let key = cells[column].as_str();
        if key.is_empty() || key == "\\N" {
            return Err(GoblinError::data(format!(
                "Join {side} data row {} has a missing key (empty or \\N). No rows were silently skipped.",
                row + 1
            )));
        }
        if let Some(previous) = keys.insert(key, row) {
            return Err(GoblinError::data(format!(
                "Join {side} duplicate key {key:?} at data rows {} and {}. Policy is one-to-one; no many-to-many expansion is permitted.",
                previous + 1,
                row + 1
            )));
        }
    }
    Ok(keys)
}

/// Strict quoted CSV *and* TSV; never replaces tabs/newlines in scientific text.
fn write_row<'a>(
    bytes: &mut Vec<u8>,
    cells: impl Iterator<Item = &'a str>,
    delimiter: u8,
) -> Result<()> {
    for (index, cell) in cells.enumerate() {
        if index != 0 {
            bytes.push(delimiter);
        }
        let quoted = cell
            .bytes()
            .any(|b| matches!(b, b'"' | b'\n' | b'\r') || b == delimiter);
        let encoded_length = cell.len()
            + if quoted {
                2 + cell.bytes().filter(|b| *b == b'"').count()
            } else {
                0
            };
        if bytes.len().saturating_add(encoded_length).saturating_add(1) > MAX_INPUT_BYTES as usize {
            return Err(GoblinError::data(
                "Join output exceeds 16 MiB. No partial join is published.",
            ));
        }
        if quoted {
            bytes.push(b'"');
        }
        for byte in cell.bytes() {
            bytes.push(byte);
            if quoted && byte == b'"' {
                bytes.push(b'"');
            }
        }
        if quoted {
            bytes.push(b'"');
        }
    }
    bytes.push(b'\n');
    Ok(())
}

pub fn join(output: &str, left: &Table, right: &Table, rules: &Rules) -> Result<GeneratedOutput> {
    rules.validate(output)?;
    if left.delimiter != rules.delimiter() || right.delimiter != rules.delimiter() {
        return Err(GoblinError::data(
            "Both join inputs must use the function's explicit delimiter.",
        ));
    }
    let left_column = key_index(left, &rules.left_key, "left")?;
    let right_column = key_index(right, &rules.right_key, "right")?;
    let left_keys = index(left, left_column, "left")?;
    let right_keys = index(right, right_column, "right")?;
    let mut headers = vec![MATCH_COLUMN.to_string()];
    headers.extend(
        left.headers
            .iter()
            .map(|h| format!("{}{h}", rules.left_prefix)),
    );
    headers.extend(
        right
            .headers
            .iter()
            .map(|h| format!("{}{h}", rules.right_prefix)),
    );
    let mut seen = std::collections::BTreeSet::new();
    if headers.len() > 1024 || headers.iter().any(|h| !seen.insert(h)) {
        return Err(GoblinError::data(
            "Join output headers collide or exceed 1024 columns. Choose distinct, non-colliding prefixes; columns are never silently renamed or discarded.",
        ));
    }
    let matched = left_keys
        .keys()
        .filter(|key| right_keys.contains_key(**key))
        .count();
    let output_rows = if rules.kind == "left" {
        left.rows.len()
    } else {
        matched
    };
    if output_rows > MAX_ROWS || (output_rows + 1).saturating_mul(headers.len()) > 1_000_000 {
        return Err(GoblinError::data(
            "Join output exceeds 100,000 data rows or 1,000,000 cells including headers. No partial join is published.",
        ));
    }
    let mut bytes = Vec::new();
    write_row(
        &mut bytes,
        headers.iter().map(String::as_str),
        rules.delimiter(),
    )?;
    for cells in &left.rows {
        let right_row = right_keys
            .get(cells[left_column].as_str())
            .map(|row| &right.rows[*row]);
        if right_row.is_none() && rules.kind == "inner" {
            continue;
        }
        let right_cells =
            (0..right.headers.len()).map(|col| right_row.map_or("", |r| r[col].as_str()));
        write_row(
            &mut bytes,
            std::iter::once(if right_row.is_some() { "true" } else { "false" })
                .chain(cells.iter().map(String::as_str))
                .chain(right_cells),
            rules.delimiter(),
        )?;
    }
    GeneratedOutput::new(
        output,
        if rules.delimiter() == b',' {
            "text/csv; charset=utf-8"
        } else {
            "text/tab-separated-values; charset=utf-8"
        },
        "table_join",
        bytes,
        json!({
            "table_join_policy":POLICY, "rules":rules,
            "key_identity":"exact UTF-8 text; no numeric conversion, trimming, case folding or Unicode normalization",
            "cardinality":"one_to_one", "duplicate_keys":"error_on_either_input_including_unmatched_rows",
            "missing_keys":"error_on_empty_or_backslash_N", "order":"left_input_order",
            "unmatched_right_cells":"empty; consult __join_matched to distinguish absent rows",
            "left":{"sha256":left.sha256,"byte_count":left.bytes.len(),"rows":left.rows.len()},
            "right":{"sha256":right.sha256,"byte_count":right.bytes.len(),"rows":right.rows.len()},
            "matched_rows":matched,"unmatched_left_rows":left.rows.len()-matched,
            "unmatched_right_rows":right.rows.len()-matched,"output_rows":output_rows,
            "dropped_left_rows":if rules.kind=="inner" {left.rows.len()-matched} else {0},
            "output_headers":headers,
        }),
    )
}

pub fn access(function: &str, role: &str, output: &str) -> String {
    json!({"operation":function,"table_join_policy":POLICY,"role":role,"output":output}).to_string()
}

/// Rebuild joins from preserved inputs, without executing a source program or
/// accessing live catalogues. Checksums detect inconsistency, not authorship.
pub fn verify(run: &Path, receipt: &Value) -> Result<()> {
    if !supports_receipt(receipt) {
        return Err(GoblinError::data(
            "Missing or unsupported table join policy.",
        ));
    }
    // Early receipts predate generated files and tabular imports. Their absent
    // lists do not imply a broken join; malformed present lists still refuse.
    let list = |name: &str| -> Result<&[Value]> {
        match receipt.get(name) {
            None => Ok(&[]),
            Some(value) => value
                .as_array()
                .map(Vec::as_slice)
                .ok_or_else(|| GoblinError::data(format!("Invalid {name} evidence list."))),
        }
    };
    let artifacts = list("generated_artifacts")?;
    let imports = list("data_imports")?;
    for artifact in artifacts.iter().filter(|a| a["producer"] == "table_join") {
        let output = artifact["name"]
            .as_str()
            .ok_or_else(|| GoblinError::data("Missing join output name."))?;
        let metadata = &artifact["metadata"];
        let rules: Rules = serde_json::from_value(metadata["rules"].clone())?;
        rules.validate(output)?;
        let load = |role: &str| -> Result<Table> {
            let expected_access = access(&rules.function, role, output);
            let candidates: Vec<_> = imports
                .iter()
                .filter(|i| {
                    i["access"]
                        .as_array()
                        .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(&expected_access)))
                })
                .collect();
            if candidates.len() != 1 {
                return Err(GoblinError::data(
                    "Join requires one preserved input per role.",
                ));
            }
            let import = candidates[0];
            let relative = import["evidence_path"]
                .as_str()
                .ok_or_else(|| GoblinError::data("Missing join input snapshot."))?;
            let path = Path::new(relative);
            if path.components().count() != 2
                || path.parent() != Some(Path::new("imports"))
                || !matches!(path.file_name(), Some(name) if !name.is_empty())
            {
                return Err(GoblinError::data(
                    "Join input evidence must be directly below imports/.",
                ));
            }
            let directory = run.join("imports");
            if std::fs::symlink_metadata(&directory)?
                .file_type()
                .is_symlink()
            {
                return Err(GoblinError::data(
                    "Refusing symbolic-link evidence directory.",
                ));
            }
            let table = Table::open(&run.join(path), rules.delimiter())?;
            let expected_format = if rules.delimiter() == b',' {
                "CSV-UTF8"
            } else {
                "TSV-UTF8"
            };
            if table.sha256 != metadata[role]["sha256"]
                || import["sha256"] != table.sha256
                || import["evidence_sha256"] != table.sha256
                || import["format"] != expected_format
                || import["byte_count"] != table.bytes.len() as u64
            {
                return Err(GoblinError::data(
                    "Join input identity or delimiter evidence differs.",
                ));
            }
            Ok(table)
        };
        let left = load("left")?;
        let right = load("right")?;
        let rebuilt = join(output, &left, &right, &rules)?;
        let descriptor = crate::output::artifact_descriptor(&rebuilt);
        for field in [
            "name",
            "sha256",
            "byte_count",
            "media_type",
            "producer",
            "metadata",
        ] {
            if artifact[field] != descriptor[field] {
                return Err(GoblinError::data(format!(
                    "Reconstructed join {field} differs."
                )));
            }
        }
        if artifact["path"] != format!("outputs/{output}") {
            return Err(GoblinError::data("Invalid join output evidence path."));
        }
        let output_directory = run.join("outputs");
        if std::fs::symlink_metadata(&output_directory)?
            .file_type()
            .is_symlink()
        {
            return Err(GoblinError::data(
                "Refusing symbolic-link output directory.",
            ));
        }
        let actual = run.join("outputs").join(output);
        let meta = std::fs::symlink_metadata(&actual)?;
        if !meta.is_file()
            || meta.file_type().is_symlink()
            || meta.len() != rebuilt.bytes.len() as u64
            || std::fs::read(actual)? != rebuilt.bytes
        {
            return Err(GoblinError::data(
                "Join bytes differ from reconstructed output.",
            ));
        }
    }
    // A successful run cannot discard its join descriptor while retaining the
    // input access trail. Failed attempts may legitimately have no output.
    if receipt["status"] == "PASS" {
        for import in imports {
            if let Some(accesses) = import["access"].as_array() {
                for entry in accesses {
                    let Some(text) = entry.as_str() else {
                        continue;
                    };
                    let Ok(entry) = serde_json::from_str::<Value>(text) else {
                        continue;
                    };
                    if is_function(entry["operation"].as_str().unwrap_or(""))
                        && (entry["table_join_policy"] != POLICY
                            || !artifacts.iter().any(|a| {
                                a["producer"] == "table_join" && a["name"] == entry["output"]
                            }))
                    {
                        return Err(GoblinError::data(
                            "Completed join evidence is missing or unsupported.",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
