//! Scientific comparison is separate from exact stored-evidence verification.
use crate::audit::verify_run;
use crate::error::{GoblinError, Result};
use crate::hashing::sha256_bytes;
use crate::quantity::{Dimension, Quantity, format_dimension};
use crate::runtime::read_receipt;
use serde_json::{Value, json};
use std::fs;
use std::path::{Component, Path};

fn read_verified_quantity(run: &Path, name: &str) -> Result<(Quantity, Value)> {
    let receipt = read_receipt(run)?;
    if !verify_run(run)?.verified || read_receipt(run)? != receipt {
        return Err(GoblinError::new(
            "G701",
            "Both runs must pass exact evidence verification before numeric comparison.",
        ));
    }
    if receipt["status"] != "PASS" {
        return Err(GoblinError::new(
            "G701",
            "Numeric comparison requires successful runs, not partial results from failed runs.",
        ));
    }
    let matches = receipt["sealed_artifacts"]
        .as_array()
        .ok_or_else(|| GoblinError::artifact("Missing sealed artifacts."))?
        .iter()
        .filter(|item| item["name"].as_str() == Some(name))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(GoblinError::artifact(format!(
            "Expected one sealed value named {name}, found {}.",
            matches.len()
        )));
    }
    let entry = matches[0];
    let path = entry["path"]
        .as_str()
        .ok_or_else(|| GoblinError::artifact("Missing sealed artifact path."))?;
    let components = Path::new(path).components().collect::<Vec<_>>();
    if !matches!(components.as_slice(), [Component::Normal(_)]) {
        return Err(GoblinError::artifact(
            "Numeric comparison refuses a non-local sealed artifact path.",
        ));
    }
    let path = run.join(path);
    if path.is_symlink() {
        return Err(GoblinError::artifact(
            "Numeric comparison refuses symbolic-link artifacts.",
        ));
    }
    let bytes = fs::read(path)?;
    if Some(sha256_bytes(&bytes).as_str()) != entry["sha256"].as_str() {
        return Err(GoblinError::new(
            "G701",
            "Sealed artifact changed after verification; comparison refused.",
        ));
    }
    let artifact: Value = serde_json::from_slice(&bytes)?;
    if artifact["schema"] != "goblin.sealed-artifact.v1" || artifact["name"] != name {
        return Err(GoblinError::artifact(
            "Sealed artifact identity does not match the verified receipt.",
        ));
    }
    let value = &artifact["value"];
    let number = value["value_si"].as_f64().ok_or_else(|| {
        GoblinError::artifact(
            "compare-value supports a sealed scalar quantity only; not text, Booleans or arrays.",
        )
    })?;
    let axes = value["dimension"]
        .as_array()
        .ok_or_else(|| GoblinError::dimension("Sealed quantity has no dimension vector."))?;
    if !matches!(axes.len(), 5 | 6) {
        return Err(GoblinError::dimension(
            "Sealed dimension must have five historical or six current axes.",
        ));
    }
    let mut dimension: Dimension = [0; 6];
    for (slot, axis) in dimension.iter_mut().zip(axes) {
        *slot = axis
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .ok_or_else(|| GoblinError::dimension("Invalid sealed dimension exponent."))?;
    }
    Ok((Quantity::new(number, dimension)?, receipt))
}

pub fn compare_sealed_quantities(
    left: impl AsRef<Path>,
    right: impl AsRef<Path>,
    name: &str,
    absolute_si: f64,
    relative: f64,
) -> Result<Value> {
    // Validate the user's policy before reading potentially large evidence.
    crate::numeric_comparison::is_close(0.0, 0.0, absolute_si, relative)
        .map_err(GoblinError::numeric)?;
    let (a, left_receipt) = read_verified_quantity(left.as_ref(), name)?;
    let (b, right_receipt) = read_verified_quantity(right.as_ref(), name)?;
    if a.dimension != b.dimension {
        return Err(GoblinError::dimension(
            "Numeric comparison requires matching sealed-value dimensions.",
        ));
    }
    let bits_equal = a.value_si.to_bits() == b.value_si.to_bits();
    let close = crate::numeric_comparison::is_close(a.value_si, b.value_si, absolute_si, relative)
        .map_err(GoblinError::numeric)?;
    let classification = if bits_equal {
        "BITWISE_IDENTICAL"
    } else if close {
        "WITHIN_DECLARED_TOLERANCE"
    } else {
        "OUTSIDE_DECLARED_TOLERANCE"
    };
    Ok(json!({
        "schema": "goblin.numeric-comparison.v1", "name": name,
        "evidence_verification": "EXACT_SHA256_PASS", "scope": "NAMED_SEALED_SCALAR_ONLY",
        "comparison_policy": crate::numeric_comparison::POLICY,
        "absolute_tolerance_si": absolute_si, "relative_tolerance": relative,
        "dimension": a.dimension, "unit_si": format_dimension(a.dimension),
        "left": {"value_si": a.value_si, "bits_hex": format!("{:016x}", a.value_si.to_bits()), "receipt_core_sha256": left_receipt["receipt_core_sha256"], "math_policy": left_receipt.get("math_policy"), "math_environment": left_receipt.get("math_environment")},
        "right": {"value_si": b.value_si, "bits_hex": format!("{:016x}", b.value_si.to_bits()), "receipt_core_sha256": right_receipt["receipt_core_sha256"], "math_policy": right_receipt.get("math_policy"), "math_environment": right_receipt.get("math_environment")},
        "numeric_equal": a.value_si == b.value_si,
        "bits_equal": bits_equal, "within_declared_tolerance": close, "classification": classification,
        "scientific_validity": "NOT_ESTABLISHED_BY_COMPARISON"
    }))
}
