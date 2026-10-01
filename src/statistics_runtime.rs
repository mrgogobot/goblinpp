// Shared verbatim by the interpreter and generated native programs.
// Values arrive in SI units; type/dimension checks belong to the caller.
pub fn numeric_reduction(name: &str, values: &[f64]) -> Result<f64, String> {
    if values.is_empty() {
        return Err(format!("{name}() requires a non-empty numeric array."));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(format!("{name}() requires finite numeric values."));
    }
    match name {
        "sum" => compensated_sum(values.iter().copied(), name),
        "mean" => {
            // Scaling before accumulation avoids sum overflow without dividing
            // tiny inputs by the count first (which could erase subnormals).
            let scale = values.iter().fold(0.0_f64, |a, b| a.max(b.abs()));
            if scale == 0.0 {
                return Ok(0.0);
            }
            let total = compensated_sum(values.iter().map(|value| value / scale), name)?;
            let result = (total / values.len() as f64) * scale;
            if !result.is_finite() {
                return Err(format!("{name}() overflowed."));
            }
            Ok(result)
        }
        _ => Err(format!("Unknown numeric reduction: {name}")),
    }
}

fn compensated_sum(values: impl Iterator<Item = f64>, name: &str) -> Result<f64, String> {
    let mut total = 0.0_f64;
    let mut correction = 0.0_f64;
    // Neumaier compensation recovers small terms lost to cancellation.
    for value in values {
        let next = total + value;
        if !next.is_finite() {
            return Err(format!("{name}() accumulation overflowed."));
        }
        correction += if total.abs() >= value.abs() {
            (total - next) + value
        } else {
            (value - next) + total
        };
        if !correction.is_finite() {
            return Err(format!("{name}() compensation overflowed."));
        }
        total = next;
    }
    let result = total + correction;
    if !result.is_finite() {
        return Err(format!("{name}() result overflowed."));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::numeric_reduction;

    #[test]
    fn nonfinite_inputs_and_empty_arrays_are_refused() {
        for name in ["sum", "mean"] {
            assert!(numeric_reduction(name, &[]).is_err());
            for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                assert!(numeric_reduction(name, &[1.0, value]).is_err());
            }
        }
    }

    #[test]
    fn compensation_and_scaling_cover_cancellation_and_extremes() {
        assert_eq!(numeric_reduction("sum", &[1e16, 1.0, -1e16]).unwrap(), 1.0);
        assert_eq!(
            numeric_reduction("mean", &[f64::MAX, f64::MAX]).unwrap(),
            f64::MAX
        );
        let tiny = f64::from_bits(1);
        assert_eq!(numeric_reduction("mean", &[tiny, tiny]).unwrap(), tiny);
        assert_eq!(
            numeric_reduction("mean", &[f64::MAX, -f64::MAX]).unwrap(),
            0.0
        );
        assert!(numeric_reduction("sum", &[f64::MAX, f64::MAX]).is_err());
        assert!(numeric_reduction("sum", &[f64::MAX, f64::MAX, -f64::MAX]).is_err());
    }
}
