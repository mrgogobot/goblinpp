//! CSR/CSC/COO admission and explicit bounded densification; never sum duplicates.
use super::{MAX_ITEMS, fail, npy, npz, scalar};
use crate::{error::Result, evaluator::Value};
use std::collections::BTreeSet;
pub struct Sparse {
    pub shape: [usize; 2],
    pub format: String,
    pub entries: Vec<(usize, usize, Value)>,
}
impl Sparse {
    pub fn open(bytes: &[u8]) -> Result<Self> {
        let mut z = npz::Archive::open(bytes)?;
        let format_bytes = z.member("format")?;
        let format_array = npy::Array::parse(&format_bytes)?;
        if !format_array.shape.is_empty() {
            return Err(fail("Sparse format must be a scalar string."));
        }
        let format = format_array.values(true)?;
        if format.len() != 1 {
            return Err(fail("Sparse format must be a scalar string."));
        }
        let format = format[0].text("sparse format")?.to_owned();
        if !matches!(format.as_str(), "csr" | "csc" | "coo") {
            return Err(fail(
                "Only SciPy CSR, CSC and COO NPZ are supported; BSR and DIA are refused.",
            ));
        }
        let required: BTreeSet<_> = if format == "coo" {
            ["format", "shape", "data", "row", "col"]
                .into_iter()
                .collect()
        } else {
            ["format", "shape", "data", "indices", "indptr"]
                .into_iter()
                .collect()
        };
        let mut keys = z.keys.keys().map(String::as_str).collect::<BTreeSet<_>>();
        keys.remove("_is_array");
        if keys != required {
            return Err(fail("Sparse NPZ has missing or unexpected members."));
        }
        if z.keys.contains_key("_is_array") {
            let marker = z.member("_is_array")?;
            let marker = npy::Array::parse(&marker)?;
            if !marker.shape.is_empty()
                || !matches!(marker.values(false)?.as_slice(), [Value::Bool(_)])
            {
                return Err(fail("Sparse _is_array marker must be a scalar boolean."));
            }
        }
        let shape = z.member("shape")?;
        let shape = npy::Array::parse(&shape)?.indices(2)?;
        if shape.len() != 2 || shape.iter().any(|n| *n > MAX_ITEMS) {
            return Err(fail("Sparse shape requires two axes, each at most 100000."));
        }
        let data = z.member("data")?;
        let array = npy::Array::parse(&data)?;
        if array.shape.len() != 1 {
            return Err(fail("Sparse data must be one-dimensional."));
        }
        let data = array.values(false)?;
        if data.iter().any(|v| !matches!(v, Value::Quantity(_))) {
            return Err(fail(
                "Sparse data must be real numeric values, not boolean or text.",
            ));
        }
        let nnz = data.len();
        let mut positions = Vec::new();
        if format == "coo" {
            let rows = z.member("row")?;
            let rows = npy::Array::parse(&rows)?.indices(MAX_ITEMS)?;
            let cols = z.member("col")?;
            let cols = npy::Array::parse(&cols)?.indices(MAX_ITEMS)?;
            if rows.len() != nnz || cols.len() != nnz {
                return Err(fail("Sparse COO row/column/data lengths differ."));
            }
            positions.extend(rows.into_iter().zip(cols));
        } else {
            let indices = z.member("indices")?;
            let indices = npy::Array::parse(&indices)?.indices(MAX_ITEMS)?;
            let pointers = z.member("indptr")?;
            let pointers = npy::Array::parse(&pointers)?.indices(MAX_ITEMS + 1)?;
            let major = if format == "csr" { shape[0] } else { shape[1] };
            if indices.len() != nnz
                || pointers.len() != major + 1
                || pointers.first() != Some(&0)
                || pointers.last() != Some(&nnz)
                || pointers.windows(2).any(|p| p[0] > p[1] || p[1] > nnz)
            {
                return Err(fail(
                    "Sparse compressed pointers/indices/data are inconsistent.",
                ));
            }
            for outer in 0..major {
                for &inner in &indices[pointers[outer]..pointers[outer + 1]] {
                    positions.push(if format == "csr" {
                        (outer, inner)
                    } else {
                        (inner, outer)
                    });
                }
            }
        }
        let mut seen = BTreeSet::new();
        for &(row, col) in &positions {
            if row >= shape[0] || col >= shape[1] || !seen.insert((row, col)) {
                return Err(fail(
                    "Sparse coordinate out of bounds or duplicate; canonicalize duplicates explicitly before export.",
                ));
            }
        }
        Ok(Self {
            shape: [shape[0], shape[1]],
            format,
            entries: positions
                .into_iter()
                .zip(data)
                .map(|((r, c), v)| (r, c, v))
                .collect(),
        })
    }
    pub fn dense(&self, row: Option<usize>) -> Result<Value> {
        if row.is_some_and(|r| r >= self.shape[0]) {
            return Err(fail("Sparse row index is outside its shape."));
        }
        let count = if row.is_some() {
            self.shape[1]
        } else {
            self.shape[0]
                .checked_mul(self.shape[1])
                .ok_or_else(|| fail("Dense sparse shape overflow."))?
        };
        if count > MAX_ITEMS {
            return Err(fail(
                "Explicit sparse densification exceeds 100000 values; use scipy_sparse_row.",
            ));
        }
        let mut out = vec![scalar(0.0)?; count];
        for (r, c, v) in &self.entries {
            if row == Some(*r) {
                out[*c] = v.clone();
            } else if row.is_none() {
                out[r * self.shape[1] + c] = v.clone();
            }
        }
        Ok(Value::Array(out))
    }
}
