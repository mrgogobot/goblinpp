// Shared verbatim by the interpreter and generated native programs.
// Values arrive in SI units; type/dimension checks belong to the caller.
pub fn sorted_numeric(values: &[f64]) -> Result<Vec<f64>, String> {
    finite_values("sort", values)?;
    let mut sorted = values.to_vec();
    // Stable numeric ordering: -0 and +0 compare equal, retaining source order.
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("validated finite values"));
    Ok(sorted)
}

fn finite_values(name: &str, values: &[f64]) -> Result<(), String> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(format!("{name}() requires finite numeric values."));
    }
    Ok(())
}

pub fn numeric_quantile(values: &[f64], probability: f64) -> Result<f64, String> {
    if values.is_empty() {
        return Err("quantile() requires a non-empty numeric array.".into());
    }
    if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
        return Err("quantile() requires a finite probability from 0 through 1.".into());
    }
    let sorted = sorted_numeric(values)?;
    // Hyndman-Fan type 7, zero-based h = (n - 1) * p; no boundary fuzz.
    let h = (sorted.len() - 1) as f64 * probability;
    let index = h.floor() as usize;
    let fraction = h - index as f64;
    let lower = sorted[index];
    if fraction == 0.0 || index + 1 == sorted.len() {
        return Ok(lower);
    }
    let upper = sorted[index + 1];
    if lower == upper {
        return Ok(lower);
    }
    // Avoid same-sign sum overflow and opposite-sign difference overflow.
    let result = if lower.is_sign_negative() == upper.is_sign_negative() {
        lower + (upper - lower) * fraction
    } else {
        lower * (1.0 - fraction) + upper * fraction
    };
    if !result.is_finite() {
        return Err("quantile() interpolation overflowed.".into());
    }
    Ok(result)
}

pub fn numeric_ecdf(values: &[f64], query: f64) -> Result<f64, String> {
    if values.is_empty() {
        return Err("ecdf() requires a non-empty numeric array.".into());
    }
    finite_values("ecdf", values)?;
    if !query.is_finite() {
        return Err("ecdf() requires a finite query.".into());
    }
    Ok(values.iter().filter(|value| **value <= query).count() as f64 / values.len() as f64)
}

fn standard_deviation(name: &str, values: &[f64]) -> Result<f64, String> {
    let sample = name == "std_sample";
    if sample && values.len() < 2 {
        return Err("std_sample() requires at least two observations.".into());
    }
    // Anchor before scaling to retain small spreads around large offsets.
    // If subtraction would overflow, normalize the original values instead.
    let origin = values[0];
    let shifted: Vec<_> = values.iter().map(|value| value - origin).collect();
    let (normalized, scale) = if shifted.iter().all(|value| value.is_finite()) {
        let scale = shifted.iter().fold(0.0_f64, |a, b| a.max(b.abs()));
        if scale == 0.0 {
            return Ok(0.0);
        }
        (
            shifted
                .iter()
                .map(|value| value / scale)
                .collect::<Vec<_>>(),
            scale,
        )
    } else {
        let scale = values.iter().fold(0.0_f64, |a, b| a.max(b.abs()));
        (
            values.iter().map(|value| value / scale).collect::<Vec<_>>(),
            scale,
        )
    };
    let center = numeric_reduction("mean", &normalized)?;
    let squares = compensated_sum(
        normalized.iter().map(|value| (value - center).powi(2)),
        name,
    )?;
    let divisor = (values.len() - usize::from(sample)) as f64;
    let result = (squares / divisor).sqrt() * scale;
    if !result.is_finite() {
        return Err(format!("{name}() result overflowed."));
    }
    Ok(result)
}

pub fn numeric_reduction(name: &str, values: &[f64]) -> Result<f64, String> {
    if values.is_empty() {
        return Err(format!("{name}() requires a non-empty numeric array."));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(format!("{name}() requires finite numeric values."));
    }
    match name {
        "median" => numeric_quantile(values, 0.5),
        "std_population" | "std_sample" => standard_deviation(name, values),
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
    use super::{numeric_ecdf, numeric_quantile, numeric_reduction, sorted_numeric};

    #[test]
    fn distribution_input_validation_and_small_cases() {
        assert!(sorted_numeric(&[]).unwrap().is_empty());
        for name in ["median", "std_population", "std_sample"] {
            assert!(numeric_reduction(name, &[]).is_err());
            for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                assert!(numeric_reduction(name, &[1.0, bad]).is_err());
                assert!(sorted_numeric(&[bad]).is_err());
                assert!(numeric_quantile(&[bad], 0.5).is_err());
                assert!(numeric_ecdf(&[bad], 1.0).is_err());
            }
        }
        for p in [-0.01, 1.01, f64::NAN, f64::INFINITY] {
            assert!(numeric_quantile(&[1.0], p).is_err());
        }
        assert!(numeric_quantile(&[], 0.5).is_err());
        assert!(numeric_ecdf(&[], 1.0).is_err());
        assert!(numeric_ecdf(&[1.0], f64::NAN).is_err());
        assert!(numeric_reduction("std_sample", &[1.0]).is_err());
        assert_eq!(numeric_reduction("std_population", &[1.0]).unwrap(), 0.0);
        assert_eq!(numeric_quantile(&[4.0, 1.0, 3.0, 2.0], 0.25).unwrap(), 1.75);
        assert_eq!(
            numeric_reduction("median", &[4.0, 1.0, 3.0, 2.0]).unwrap(),
            2.5
        );
        assert_eq!(numeric_ecdf(&[2.0, 1.0, 2.0, 4.0], 2.0).unwrap(), 0.75);
    }

    #[test]
    fn stable_sort_and_quantiles_preserve_endpoint_bits() {
        let values = [0.0, -0.0, 0.0, -0.0, -1.0, 1.0];
        let sorted = sorted_numeric(&values).unwrap();
        assert_eq!(
            sorted[1..5].iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            values[..4].iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        assert_eq!(
            numeric_quantile(&[-0.0, 0.0], 0.0).unwrap().to_bits(),
            (-0.0_f64).to_bits()
        );
        assert_eq!(
            numeric_quantile(&[-0.0, 0.0], 1.0).unwrap().to_bits(),
            0.0_f64.to_bits()
        );
        assert_eq!(
            numeric_quantile(&[-0.0, 0.0], 0.5).unwrap().to_bits(),
            (-0.0_f64).to_bits()
        );
        for p in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert_eq!(
                numeric_quantile(&[-0.0], p).unwrap().to_bits(),
                (-0.0_f64).to_bits()
            );
        }
        assert_eq!(numeric_ecdf(&[-0.0, 0.0], -0.0).unwrap(), 1.0);
    }

    #[test]
    fn deviations_and_quantiles_avoid_avoidable_overflow_and_underflow() {
        let max = f64::MAX;
        let tiny = f64::from_bits(1);
        assert_eq!(numeric_quantile(&[max, max], 0.5).unwrap(), max);
        assert_eq!(numeric_quantile(&[-max, max], 0.5).unwrap(), 0.0);
        // Two rounded weighted terms may differ from the analytic result by one ULP.
        let observed = numeric_quantile(&[-max, max], 0.75).unwrap();
        assert!(observed.to_bits().abs_diff((max / 2.0).to_bits()) <= 1);
        assert_eq!(numeric_quantile(&[tiny, tiny], 0.5).unwrap(), tiny);
        assert_eq!(
            numeric_reduction("std_population", &[-max, max]).unwrap(),
            max
        );
        assert!(numeric_reduction("std_sample", &[-max, max]).is_err());
        assert_eq!(
            numeric_reduction("std_population", &[max, max]).unwrap(),
            0.0
        );
        assert_eq!(
            numeric_reduction("std_population", &[0.0, tiny * 2.0]).unwrap(),
            tiny
        );
        assert_eq!(
            numeric_reduction("std_population", &[1e16, 1e16 + 2.0]).unwrap(),
            1.0
        );
        let data = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!((numeric_reduction("std_population", &data).unwrap() - 2.0).abs() < 1e-15);
        assert!(
            (numeric_reduction("std_sample", &data).unwrap() - (32.0_f64 / 7.0).sqrt()).abs()
                < 1e-15
        );
    }

    #[test]
    fn quantile_monotonicity_and_ecdf_boundaries_have_no_sampling() {
        let data: Vec<_> = (0..1000).rev().map(|i| (i % 53) as f64 - 26.0).collect();
        let mut last = f64::NEG_INFINITY;
        for step in 0..=1000 {
            let q = numeric_quantile(&data, step as f64 / 1000.0).unwrap();
            assert!(q >= last && (-26.0..=26.0).contains(&q));
            last = q;
        }
        assert_eq!(numeric_ecdf(&data, -27.0).unwrap(), 0.0);
        assert_eq!(numeric_ecdf(&data, 27.0).unwrap(), 1.0);
        for query in [-26.0, -1.0, 0.0, 1.0, 26.0] {
            let count = data.iter().filter(|value| **value <= query).count();
            assert_eq!(numeric_ecdf(&data, query).unwrap(), count as f64 / 1000.0);
        }
    }

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
