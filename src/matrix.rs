//! Bounded, dimension-aware flat row-major matrix operations.
//! No implicit covariance repair, jitter, mixed-unit coercion, or RNG reseeding.
use crate::error::{GoblinError, Result};
use crate::evaluator::Value;
use crate::quantity::{DIMENSIONLESS, Dimension, Quantity};
use crate::random::Randomness;

pub const MAX_ENTRIES: usize = 100_000;
pub const MAX_COVARIANCE_DIMENSION: usize = 128;
pub const MAX_MULTIPLY_WORK: usize = 50_000_000;

pub fn is_function(name: &str) -> bool {
    matches!(
        name,
        "matrix_transpose"
            | "matrix_multiply"
            | "vector_norm"
            | "vector_unit"
            | "covariance"
            | "cholesky"
            | "mvnormal"
    )
}

pub fn require_arity(name: &str, count: usize) -> Result<()> {
    let expected = match name {
        "matrix_transpose" | "cholesky" => 3,
        "matrix_multiply" => 6,
        "vector_norm" | "vector_unit" => 1,
        "covariance" => 4,
        "mvnormal" => 5,
        _ => return Err(error(format!("Unknown matrix function {name:?}."))),
    };
    if count == expected {
        Ok(())
    } else {
        Err(error(format!(
            "{name}() requires exactly {expected} arguments; received {count}. See MATRICES_COVARIANCE.md."
        )))
    }
}

fn error(message: impl Into<String>) -> GoblinError {
    GoblinError::numeric(message)
}

fn number(value: &Value, context: &str) -> Result<f64> {
    let quantity = value.quantity(context)?;
    if quantity.dimension != DIMENSIONLESS {
        return Err(GoblinError::dimension(format!(
            "{context} must be dimensionless."
        )));
    }
    finite(quantity.value_si, context)
}

fn positive_integer(value: &Value, maximum: usize, context: &str) -> Result<usize> {
    let n = number(value, context)?;
    if n < 1.0 || n > maximum as f64 || n.fract() != 0.0 {
        Err(error(format!(
            "{context} must be a positive integer no greater than {maximum}."
        )))
    } else {
        Ok(n as usize)
    }
}

fn finite(value: f64, context: &str) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(error(format!(
            "{context} produced or received a non-finite value; execution refused."
        )))
    }
}

fn entries(rows: usize, columns: usize, context: &str) -> Result<usize> {
    rows.checked_mul(columns)
        .filter(|n| *n <= MAX_ENTRIES)
        .ok_or_else(|| {
            error(format!(
                "{context} exceeds the {MAX_ENTRIES}-entry matrix limit."
            ))
        })
}

fn numeric_array(
    value: &Value,
    length: Option<usize>,
    context: &str,
) -> Result<(Vec<f64>, Dimension)> {
    let Value::Array(values) = value else {
        return Err(error(format!("{context} requires a flat numeric array.")));
    };
    if values.is_empty() || values.len() > MAX_ENTRIES || length.is_some_and(|n| n != values.len())
    {
        return Err(error(format!(
            "{context} requires a non-empty flat array with the declared shape and no more than {MAX_ENTRIES} entries."
        )));
    }
    let dimension = values[0].quantity(context)?.dimension;
    let mut numeric = Vec::with_capacity(values.len());
    for value in values {
        let quantity = value.quantity(context)?;
        if quantity.dimension != dimension {
            return Err(GoblinError::dimension(format!(
                "{context} requires homogeneous numeric dimensions. Mixed-unit matrices are not supported; explicitly standardize first."
            )));
        }
        numeric.push(finite(quantity.value_si, context)?);
    }
    Ok((numeric, dimension))
}

fn array(values: Vec<f64>, dimension: Dimension) -> Value {
    Value::Array(
        values
            .into_iter()
            .map(|value_si| {
                Value::Quantity(Quantity {
                    value_si,
                    dimension,
                })
            })
            .collect(),
    )
}

fn add_dimensions(left: Dimension, right: Dimension) -> Result<Dimension> {
    let mut result = DIMENSIONLESS;
    for index in 0..result.len() {
        result[index] = left[index]
            .checked_add(right[index])
            .ok_or_else(|| GoblinError::dimension("Matrix dimension exponent overflow."))?;
    }
    Ok(result)
}

fn square_root_dimension(dimension: Dimension) -> Result<Dimension> {
    let mut result = DIMENSIONLESS;
    for (index, exponent) in dimension.into_iter().enumerate() {
        if exponent % 2 != 0 {
            return Err(GoblinError::dimension(
                "Cholesky entries require dimensions with even exponents so their square roots are representable.",
            ));
        }
        result[index] = exponent / 2;
    }
    Ok(result)
}

// Neumaier accumulation: preserve cancellation without reassociation or FMA.
fn compensated_sum(values: impl Iterator<Item = f64>, context: &str) -> Result<f64> {
    let mut total = 0.0_f64;
    let mut correction = 0.0_f64;
    for value in values {
        finite(value, context)?;
        let next = finite(total + value, context)?;
        correction = finite(
            correction
                + if total.abs() >= value.abs() {
                    (total - next) + value
                } else {
                    (value - next) + total
                },
            context,
        )?;
        total = next;
    }
    finite(total + correction, context)
}

fn transpose(args: &[Value]) -> Result<Value> {
    let rows = positive_integer(&args[1], MAX_ENTRIES, "matrix_transpose rows")?;
    let columns = positive_integer(&args[2], MAX_ENTRIES, "matrix_transpose columns")?;
    let length = entries(rows, columns, "matrix_transpose")?;
    let (values, dimension) = numeric_array(&args[0], Some(length), "matrix_transpose")?;
    let mut result = vec![0.0; length];
    for row in 0..rows {
        for column in 0..columns {
            result[column * rows + row] = values[row * columns + column];
        }
    }
    Ok(array(result, dimension))
}

fn multiply(args: &[Value]) -> Result<Value> {
    let left_rows = positive_integer(&args[1], MAX_ENTRIES, "matrix_multiply left rows")?;
    let left_columns = positive_integer(&args[2], MAX_ENTRIES, "matrix_multiply left columns")?;
    let right_rows = positive_integer(&args[4], MAX_ENTRIES, "matrix_multiply right rows")?;
    let right_columns = positive_integer(&args[5], MAX_ENTRIES, "matrix_multiply right columns")?;
    if left_columns != right_rows {
        return Err(error("matrix_multiply inner dimensions must agree."));
    }
    let left_length = entries(left_rows, left_columns, "matrix_multiply left input")?;
    let right_length = entries(right_rows, right_columns, "matrix_multiply right input")?;
    let result_length = entries(left_rows, right_columns, "matrix_multiply output")?;
    let work = result_length
        .checked_mul(left_columns)
        .filter(|n| *n <= MAX_MULTIPLY_WORK);
    if work.is_none() {
        return Err(error(format!(
            "matrix_multiply exceeds the {MAX_MULTIPLY_WORK}-scalar-product work limit; split the operation explicitly."
        )));
    }
    let (left, left_dimension) =
        numeric_array(&args[0], Some(left_length), "matrix_multiply left input")?;
    let (right, right_dimension) =
        numeric_array(&args[3], Some(right_length), "matrix_multiply right input")?;
    let dimension = add_dimensions(left_dimension, right_dimension)?;
    let mut result = Vec::with_capacity(result_length);
    for row in 0..left_rows {
        for column in 0..right_columns {
            result.push(compensated_sum(
                (0..left_columns).map(|inner| {
                    left[row * left_columns + inner] * right[inner * right_columns + column]
                }),
                "matrix_multiply",
            )?);
        }
    }
    Ok(array(result, dimension))
}

fn vector_unit(value: &Value) -> Result<Value> {
    let (values, _) = numeric_array(value, None, "vector_unit")?;
    let scale = values
        .iter()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    if scale == 0.0 {
        return Err(error("vector_unit cannot normalize the zero vector."));
    }
    let normalized: Vec<_> = values.iter().map(|value| value / scale).collect();
    let norm = compensated_sum(normalized.iter().map(|value| value * value), "vector_unit")?.sqrt();
    // Divide in scaled space, never constructing the potentially overflowing norm.
    let result = normalized.into_iter().map(|value| value / norm).collect();
    Ok(array(result, DIMENSIONLESS))
}

fn vector_norm(value: &Value) -> Result<Value> {
    let (values, dimension) = numeric_array(value, None, "vector_norm")?;
    let scale = values
        .iter()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    let norm = if scale == 0.0 {
        0.0
    } else {
        compensated_sum(
            values.iter().map(|value| (value / scale).powi(2)),
            "vector_norm",
        )?
        .sqrt()
            * scale
    };
    Ok(Value::Quantity(Quantity::new(norm, dimension)?))
}

fn balanced_product(mut values: [f64; 3], context: &str) -> Result<f64> {
    // Multiply the smallest and largest magnitudes first so a representable
    // result is not needlessly lost to a tiny or overflowing intermediate.
    values.sort_by(|left, right| left.abs().total_cmp(&right.abs()));
    let intermediate = finite(values[0] * values[2], context)?;
    finite(intermediate * values[1], context)
}

fn covariance(args: &[Value]) -> Result<Value> {
    let observations = positive_integer(&args[1], MAX_ENTRIES, "covariance observations")?;
    let variables = positive_integer(&args[2], MAX_COVARIANCE_DIMENSION, "covariance variables")?;
    let length = entries(observations, variables, "covariance input")?;
    let ddof = number(&args[3], "covariance ddof")?;
    if ddof != 0.0 && ddof != 1.0 {
        return Err(error(
            "covariance ddof must be explicitly 0 (population) or 1 (sample).",
        ));
    }
    let ddof = ddof as usize;
    if observations <= ddof {
        return Err(error("covariance requires observations greater than ddof."));
    }
    let (values, dimension) = numeric_array(&args[0], Some(length), "covariance")?;
    let result_dimension = add_dimensions(dimension, dimension)?;
    let mut centered = vec![0.0; length];
    let mut scales = vec![0.0; variables];
    for variable in 0..variables {
        let origin = values[variable];
        // Anchor differences to retain small spreads on a large common offset.
        // When opposite extremes overflow subtraction, scale originals instead.
        let shifted: Vec<_> = (0..observations)
            .map(|row| values[row * variables + variable] - origin)
            .collect();
        let source: Vec<_> = if shifted.iter().all(|value| value.is_finite()) {
            shifted
        } else {
            (0..observations)
                .map(|row| values[row * variables + variable])
                .collect()
        };
        let scale = source
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        scales[variable] = scale;
        if scale == 0.0 {
            continue;
        }
        let center = compensated_sum(
            source.iter().map(|value| value / scale),
            "covariance center",
        )? / observations as f64;
        for row in 0..observations {
            centered[row * variables + variable] = source[row] / scale - center;
        }
    }
    let mut result = vec![0.0; variables * variables];
    for row in 0..variables {
        for column in 0..=row {
            let normalized = compensated_sum(
                (0..observations).map(|observation| {
                    centered[observation * variables + row]
                        * centered[observation * variables + column]
                }),
                "covariance",
            )? / (observations - ddof) as f64;
            let value = balanced_product(
                [normalized, scales[row], scales[column]],
                "covariance rescaling",
            )?;
            result[row * variables + column] = value;
            result[column * variables + row] = value;
        }
    }
    Ok(array(result, result_dimension))
}

fn symmetry_tolerance(value: &Value) -> Result<f64> {
    let tolerance = number(value, "Cholesky symmetry tolerance")?;
    if tolerance < 0.0 {
        return Err(error(
            "Cholesky symmetry tolerance must be finite and nonnegative.",
        ));
    }
    Ok(tolerance)
}

fn factor(values: &[f64], n: usize, tolerance: f64) -> Result<Vec<f64>> {
    for row in 0..n {
        for column in 0..row {
            let lower = values[row * n + column];
            let upper = values[column * n + row];
            let scale = lower.abs().max(upper.abs());
            let difference = if scale == 0.0 {
                0.0
            } else {
                (lower / scale - upper / scale).abs()
            };
            if difference > tolerance {
                return Err(error(format!(
                    "Cholesky matrix is not symmetric within the declared relative tolerance at [{row}, {column}]."
                )));
            }
        }
    }
    let scale = values
        .iter()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    if scale == 0.0 {
        return Err(error(
            "Cholesky requires a positive-definite matrix, not a zero or singular matrix.",
        ));
    }
    let mut lower = vec![0.0; values.len()];
    for row in 0..n {
        for column in 0..=row {
            let product = compensated_sum(
                (0..column).map(|inner| lower[row * n + inner] * lower[column * n + inner]),
                "Cholesky product",
            )?;
            let scaled_entry = values[row * n + column] / scale;
            let pivot = finite(scaled_entry - product, "Cholesky pivot")?;
            lower[row * n + column] = if row == column {
                // A roundoff-sized positive residual must not make an exactly
                // singular covariance appear usable for sampling. This is a
                // conservative refusal criterion, not a pivot modification.
                let roundoff_bound =
                    16.0 * f64::EPSILON * n as f64 * scaled_entry.abs().max(product.abs());
                if pivot <= roundoff_bound {
                    return Err(error(format!(
                        "Cholesky requires a numerically positive-definite matrix; pivot {row} is non-positive or indistinguishable from roundoff. No jitter or repair is performed."
                    )));
                }
                pivot.sqrt()
            } else {
                finite(pivot / lower[column * n + column], "Cholesky division")?
            };
        }
    }
    let root_scale = scale.sqrt();
    for value in &mut lower {
        *value = finite(*value * root_scale, "Cholesky rescaling")?;
    }
    Ok(lower)
}

fn cholesky(args: &[Value]) -> Result<Value> {
    let n = positive_integer(&args[1], MAX_COVARIANCE_DIMENSION, "cholesky dimension")?;
    let length = entries(n, n, "cholesky")?;
    let tolerance = symmetry_tolerance(&args[2])?;
    let (values, dimension) = numeric_array(&args[0], Some(length), "cholesky")?;
    let root_dimension = square_root_dimension(dimension)?;
    Ok(array(factor(&values, n, tolerance)?, root_dimension))
}

fn mvnormal(args: &[Value], rng: &mut Randomness) -> Result<Value> {
    let n = positive_integer(&args[2], MAX_COVARIANCE_DIMENSION, "mvnormal dimension")?;
    let length = entries(n, n, "mvnormal covariance")?;
    let tolerance = symmetry_tolerance(&args[3])?;
    let stream = crate::random::stream_id(&args[4])?;
    let (mean, dimension) = numeric_array(&args[0], Some(n), "mvnormal mean")?;
    let (covariance, covariance_dimension) =
        numeric_array(&args[1], Some(length), "mvnormal covariance")?;
    if covariance_dimension != add_dimensions(dimension, dimension)? {
        return Err(GoblinError::dimension(
            "mvnormal covariance dimensions must be the square of the homogeneous mean dimensions.",
        ));
    }
    let lower = factor(&covariance, n, tolerance)?;
    rng.validate_stream(stream)?;
    // Validate every input before drawing, and commit only a complete finite draw.
    let mut transaction = rng.clone();
    let normal = (0..n)
        .map(|_| crate::resampling::standard_normal(&mut transaction, stream))
        .collect::<Result<Vec<_>>>()?;
    let mut result = Vec::with_capacity(n);
    for row in 0..n {
        let offset = compensated_sum(
            (0..=row).map(|column| lower[row * n + column] * normal[column]),
            "mvnormal transform",
        )?;
        result.push(finite(mean[row] + offset, "mvnormal result")?);
    }
    *rng = transaction;
    Ok(array(result, dimension))
}

pub fn call(name: &str, args: &[Value], rng: &mut Randomness) -> Result<Value> {
    require_arity(name, args.len())?;
    match name {
        "matrix_transpose" => transpose(args),
        "matrix_multiply" => multiply(args),
        "vector_norm" => vector_norm(&args[0]),
        "vector_unit" => vector_unit(&args[0]),
        "covariance" => covariance(args),
        "cholesky" => cholesky(args),
        "mvnormal" => mvnormal(args, rng),
        _ => Err(error(format!("Unknown matrix function {name:?}."))),
    }
}
