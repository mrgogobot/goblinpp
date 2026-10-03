//! Shared finite-f64 comparison implementation, also embedded in native code.
pub const POLICY: &str = "goblin.symmetric-max-absolute-relative.v1";

pub fn is_function(name: &str) -> bool {
    matches!(name, "is_close" | "same_bits")
}

/// Inclusive symmetric max(abs_tol, rel_tol * max(abs(a), abs(b))) rule.
/// A scaled relative check avoids overflow for opposite-sign extreme operands.
/// Validation occurs even for identical inputs; no implicit default tolerance.
pub fn is_close(a: f64, b: f64, absolute: f64, relative: f64) -> Result<bool, String> {
    if ![a, b, absolute, relative]
        .iter()
        .all(|value| value.is_finite())
    {
        return Err("Numeric comparison requires finite values and finite tolerances.".into());
    }
    if absolute < 0.0 || relative < 0.0 {
        return Err("Numeric comparison tolerances must be non-negative.".into());
    }
    let difference = (a - b).abs();
    if difference <= absolute {
        return Ok(true);
    }
    if relative == 0.0 {
        return Ok(false);
    }
    let scale = a.abs().max(b.abs());
    let relative_difference = if difference.is_finite() {
        difference / scale
    } else {
        (a / scale - b / scale).abs()
    };
    Ok(relative_difference <= relative)
}
