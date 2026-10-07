//! Explicit-stream IID resampling and Box-Muller normal draws.
//! These helpers consume the existing, auditable alpha.26 PCG mappings.
use crate::error::{GoblinError, Result};
use crate::evaluator::Value;
use crate::quantity::{DIMENSIONLESS, Dimension, Quantity};
use crate::random::{self, Randomness};

pub const MAX_ITEMS: usize = 100_000;
pub const MAX_BOOTSTRAP_SELECTIONS: usize = 1_000_000;
pub const FUNCTIONS: &[&str] = &[
    "resample",
    "bootstrap_mean",
    "bootstrap_median",
    "rng_normal",
    "rng_normal_array",
];

pub fn is_function(name: &str) -> bool {
    FUNCTIONS.contains(&name)
}

pub fn require_arity(name: &str, count: usize) -> Result<()> {
    let expected = match name {
        "resample" | "bootstrap_mean" | "bootstrap_median" => 3,
        "rng_normal" => 1,
        "rng_normal_array" => 2,
        _ => return Err(error(format!("Unknown resampling function {name}."))),
    };
    if expected == count {
        Ok(())
    } else {
        Err(error(format!(
            "{name}() requires {expected} arguments, including an explicit seeded stream."
        )))
    }
}

fn error(message: impl Into<String>) -> GoblinError {
    GoblinError::new("G204", message)
}

fn count(value: &Value, context: &str) -> Result<usize> {
    if let Value::Quantity(q) = value
        && q.dimension == DIMENSIONLESS
        && q.value_si.is_finite()
        && q.value_si > 0.0
        && q.value_si <= MAX_ITEMS as f64
        && q.value_si.fract() == 0.0
    {
        return Ok(q.value_si as usize);
    }
    Err(error(format!(
        "{context} requires a positive dimensionless integer no greater than {MAX_ITEMS}."
    )))
}

fn numeric_array(value: &Value, context: &str) -> Result<(Vec<f64>, Dimension)> {
    let Value::Array(items) = value else {
        return Err(error(format!(
            "{context} requires a non-empty numeric array."
        )));
    };
    if items.is_empty() || items.len() > MAX_ITEMS {
        return Err(error(format!(
            "{context} requires 1 through {MAX_ITEMS} numeric items."
        )));
    }
    let Value::Quantity(first) = &items[0] else {
        return Err(error(format!("{context} requires numeric quantities.")));
    };
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let Value::Quantity(q) = item else {
            return Err(error(format!("{context} requires numeric quantities.")));
        };
        if q.dimension != first.dimension || !q.value_si.is_finite() {
            return Err(error(format!(
                "{context} requires finite values with one shared dimension; no implicit dropping or coercion."
            )));
        }
        values.push(q.value_si);
    }
    Ok((values, first.dimension))
}

fn numeric_result(value: f64, dimension: Dimension) -> Result<Value> {
    Quantity::new(value, dimension).map(Value::Quantity)
}

/// One dimensionless N(0,1) draw using exactly two alpha.26 uniform operations.
/// Callers performing a larger transaction must clone and commit RNG state.
pub fn standard_normal(rng: &mut Randomness, stream: u64) -> Result<f64> {
    let first = rng.draw_uniform(stream)?;
    let second = rng.draw_uniform(stream)?;
    // first in [0,1), so 1-first is in (0,1], including the safe zero-radius endpoint.
    let normal = (-2.0 * (1.0 - first).ln()).sqrt() * (2.0 * std::f64::consts::PI * second).cos();
    if !normal.is_finite() {
        return Err(error(
            "Box-Muller normal transformation produced a non-finite result.",
        ));
    }
    Ok(normal)
}

/// Validate first, then consume a clone; all refusals preserve RNG evidence/state.
pub fn call(name: &str, args: &[Value], rng: &mut Randomness) -> Result<Value> {
    require_arity(name, args.len())?;
    let stream = random::stream_id(args.last().expect("checked nonzero arity"))?;
    rng.validate_stream(stream)?;
    let output_count = match name {
        "rng_normal" => 1,
        "rng_normal_array" => count(&args[0], name)?,
        _ => count(&args[1], name)?,
    };
    let input = if matches!(name, "resample" | "bootstrap_mean" | "bootstrap_median") {
        Some(numeric_array(&args[0], name)?)
    } else {
        None
    };
    if matches!(name, "bootstrap_mean" | "bootstrap_median")
        && input
            .as_ref()
            .expect("validated input")
            .0
            .len()
            .checked_mul(output_count)
            .is_none_or(|n| n > MAX_BOOTSTRAP_SELECTIONS)
    {
        return Err(error(format!(
            "{name}() exceeds the {MAX_BOOTSTRAP_SELECTIONS}-selection bootstrap budget."
        )));
    }
    let mut candidate = rng.clone();
    let result = match name {
        "rng_normal" => numeric_result(standard_normal(&mut candidate, stream)?, DIMENSIONLESS)?,
        "rng_normal_array" => {
            let mut values = Vec::with_capacity(output_count);
            for _ in 0..output_count {
                values.push(numeric_result(
                    standard_normal(&mut candidate, stream)?,
                    DIMENSIONLESS,
                )?);
            }
            Value::Array(values)
        }
        "resample" => {
            let (values, dimension) = input.expect("validated input");
            let mut sample = Vec::with_capacity(output_count);
            for _ in 0..output_count {
                let index = candidate.draw_index(values.len(), stream)?;
                sample.push(numeric_result(values[index], dimension)?);
            }
            Value::Array(sample)
        }
        "bootstrap_mean" | "bootstrap_median" => {
            let (values, dimension) = input.expect("validated input");
            let reduction = if name == "bootstrap_mean" {
                "mean"
            } else {
                "median"
            };
            let mut distribution = Vec::with_capacity(output_count);
            let mut sample = vec![0.0; values.len()];
            for _ in 0..output_count {
                for value in &mut sample {
                    *value = values[candidate.draw_index(values.len(), stream)?];
                }
                let statistic = crate::statistics_runtime::numeric_reduction(reduction, &sample)
                    .map_err(error)?;
                distribution.push(numeric_result(statistic, dimension)?);
            }
            Value::Array(distribution)
        }
        _ => unreachable!("arity rejects unknown function"),
    };
    *rng = candidate;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(n: f64) -> Value {
        Value::Quantity(Quantity::scalar(n).unwrap())
    }
    fn array(ns: &[f64]) -> Value {
        Value::Array(ns.iter().copied().map(scalar).collect())
    }
    fn seeded() -> Randomness {
        let mut rng = Randomness::default();
        rng.call("rng_seed", &[scalar(42.0), scalar(7.0)]).unwrap();
        rng
    }
    fn values(v: Value) -> Vec<f64> {
        let Value::Array(items) = v else {
            panic!("not an array")
        };
        items
            .iter()
            .map(|v| v.quantity("test").unwrap().value_si)
            .collect()
    }

    #[test]
    fn resampling_matches_independent_index_draws_and_is_a_copy() {
        let mut rng = seeded();
        let mut reference = seeded();
        let source = array(&[10.0, 20.0, 30.0]);
        let sample = values(
            call(
                "resample",
                &[source.clone(), scalar(12.0), scalar(7.0)],
                &mut rng,
            )
            .unwrap(),
        );
        let expected: Vec<_> = (0..12)
            .map(|_| {
                let index = reference
                    .call("rng_integer", &[scalar(0.0), scalar(3.0), scalar(7.0)])
                    .unwrap()
                    .quantity("index")
                    .unwrap()
                    .value_si as usize;
                [10.0, 20.0, 30.0][index]
            })
            .collect();
        assert_eq!(sample, expected);
        assert_eq!(source, array(&[10.0, 20.0, 30.0]));
        assert_eq!(rng.evidence(), reference.evidence());
        crate::random::verify_evidence(&serde_json::to_value(rng.evidence()).unwrap()).unwrap();
    }

    #[test]
    fn bootstrap_replicates_match_hand_reductions_and_preserve_units() {
        for function in ["bootstrap_mean", "bootstrap_median"] {
            let mut rng = seeded();
            let mut reference = seeded();
            let source = Value::Array(
                [1.0, 3.0, 8.0]
                    .iter()
                    .map(|n| Value::Quantity(Quantity::from_unit(*n, "m").unwrap()))
                    .collect(),
            );
            let Value::Array(replicates) =
                call(function, &[source, scalar(8.0), scalar(7.0)], &mut rng).unwrap()
            else {
                panic!()
            };
            for replicate in replicates {
                let mut sample: Vec<_> = (0..3)
                    .map(|_| [1.0, 3.0, 8.0][reference.draw_index(3, 7).unwrap()])
                    .collect();
                sample.sort_by(f64::total_cmp);
                let expected = if function == "bootstrap_median" {
                    sample[1]
                } else {
                    sample.iter().sum::<f64>() / 3.0
                };
                let q = replicate.quantity("replicate").unwrap();
                assert_eq!(q.dimension, crate::quantity::LENGTH);
                assert!((q.value_si - expected).abs() < 1e-14);
            }
            assert_eq!(rng.evidence(), reference.evidence());
            let singleton = values(
                call(
                    function,
                    &[array(&[9.0]), scalar(3.0), scalar(7.0)],
                    &mut rng,
                )
                .unwrap(),
            );
            assert_eq!(singleton, vec![9.0; 3]);
        }
    }

    #[test]
    fn normal_matches_documented_transform_and_consumption() {
        let mut rng = seeded();
        let mut reference = seeded();
        for _ in 0..10 {
            let u = reference.draw_uniform(7).unwrap();
            let v = reference.draw_uniform(7).unwrap();
            let expected = (-2.0 * (1.0 - u).ln()).sqrt() * (2.0 * std::f64::consts::PI * v).cos();
            let q = call("rng_normal", &[scalar(7.0)], &mut rng)
                .unwrap()
                .quantity("normal")
                .unwrap();
            assert_eq!(q.dimension, DIMENSIONLESS);
            assert_eq!(q.value_si.to_bits(), expected.to_bits());
        }
        assert_eq!(rng.evidence(), reference.evidence());
        let mut array_rng = seeded();
        let mut scalar_rng = seeded();
        let array = values(
            call(
                "rng_normal_array",
                &[scalar(16.0), scalar(7.0)],
                &mut array_rng,
            )
            .unwrap(),
        );
        let scalars: Vec<_> = (0..16)
            .map(|_| {
                call("rng_normal", &[scalar(7.0)], &mut scalar_rng)
                    .unwrap()
                    .quantity("normal")
                    .unwrap()
                    .value_si
            })
            .collect();
        assert_eq!(array, scalars);
        assert_eq!(array_rng.evidence(), scalar_rng.evidence());
    }

    #[test]
    fn first_normal_is_anchored_to_published_pcg_words() {
        let mut rng = Randomness::default();
        rng.call("rng_seed", &[scalar(42.0), scalar(54.0)]).unwrap();
        // Published PCG C demo reference words, independent of returned helpers.
        let first_numerator = ((0xa15c02b7_u64 >> 5) << 26) | (0x7b47f409_u64 >> 6);
        let second_numerator = ((0xba1d3330_u64 >> 5) << 26) | (0x83d2f293_u64 >> 6);
        let u = first_numerator as f64 / 9_007_199_254_740_992.0;
        let v = second_numerator as f64 / 9_007_199_254_740_992.0;
        let expected = (-2.0 * (1.0 - u).ln()).sqrt() * (2.0 * std::f64::consts::PI * v).cos();
        let normal = call("rng_normal", &[scalar(54.0)], &mut rng)
            .unwrap()
            .quantity("normal")
            .unwrap();
        assert_eq!(normal.value_si.to_bits(), expected.to_bits());
        let evidence = rng.evidence();
        assert_eq!(evidence.streams[0].operations, 2);
        assert_eq!(evidence.streams[0].raw_words, 4);
        assert_eq!(evidence.streams[0].segments[0].operation, "rng_uniform");
        assert_eq!(evidence.streams[0].segments[0].count, 2);
    }

    #[test]
    fn refuses_bad_inputs_before_rng_side_effects() {
        let mut rng = seeded();
        let before = rng.evidence();
        let mixed = Value::Array(vec![
            scalar(1.0),
            Value::Quantity(Quantity::from_unit(2.0, "m").unwrap()),
        ]);
        let nonfinite = Value::Array(vec![Value::Quantity(Quantity {
            value_si: f64::NAN,
            dimension: DIMENSIONLESS,
        })]);
        let cases = vec![
            ("resample", vec![array(&[]), scalar(1.0), scalar(7.0)]),
            ("resample", vec![mixed, scalar(1.0), scalar(7.0)]),
            ("resample", vec![nonfinite, scalar(1.0), scalar(7.0)]),
            (
                "resample",
                vec![
                    Value::Array(vec![Value::Text("x".into())]),
                    scalar(1.0),
                    scalar(7.0),
                ],
            ),
            ("resample", vec![array(&[1.0]), scalar(0.0), scalar(7.0)]),
            ("resample", vec![array(&[1.0]), scalar(1.5), scalar(7.0)]),
            (
                "resample",
                vec![array(&[1.0]), scalar(100001.0), scalar(7.0)],
            ),
            (
                "resample",
                vec![array(&[1.0]), scalar(1.0), Value::Text("07".into())],
            ),
            ("rng_normal", vec![scalar(8.0)]),
            ("rng_normal", vec![]),
            ("rng_normal_array", vec![scalar(1.0)]),
            ("rng_normal_array", vec![scalar(-1.0), scalar(7.0)]),
            (
                "bootstrap_mean",
                vec![array(&vec![1.0; 1001]), scalar(1000.0), scalar(7.0)],
            ),
        ];
        for (function, args) in cases {
            assert!(call(function, &args, &mut rng).is_err(), "{function}");
            assert_eq!(rng.evidence(), before);
        }
    }

    #[test]
    fn helper_budget_failure_rolls_back_prior_successful_draws() {
        let mut rng = seeded();
        // Consume all but one operation, then request a normal needing two.
        for _ in 0..random::MAX_OPERATIONS - 1 {
            rng.draw_uniform(7).unwrap();
        }
        let before = rng.evidence();
        assert!(call("rng_normal", &[scalar(7.0)], &mut rng).is_err());
        assert_eq!(rng.evidence(), before);
    }

    #[test]
    fn fixed_seed_normal_population_has_expected_smoke_test_moments() {
        let mut rng = seeded();
        let normals = values(
            call(
                "rng_normal_array",
                &[scalar(20000.0), scalar(7.0)],
                &mut rng,
            )
            .unwrap(),
        );
        let mean = normals.iter().sum::<f64>() / normals.len() as f64;
        let second = normals.iter().map(|x| x * x).sum::<f64>() / normals.len() as f64;
        assert!(mean.abs() < 0.035, "fixed-seed mean {mean}");
        assert!(
            (second - 1.0).abs() < 0.04,
            "fixed-seed second moment {second}"
        );
        // This finite deterministic smoke check is not a proof of RNG quality.
    }
}
