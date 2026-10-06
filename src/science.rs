use crate::error::{GoblinError, Result};
use crate::evaluator::Value;
use crate::quantity::{DIMENSIONLESS, Dimension, LENGTH, MASS, Quantity, TIME, format_dimension};

pub const AU_METERS: f64 = 149_597_870_700.0;
pub const LIGHT_YEAR_METERS: f64 = 9_460_730_472_580_800.0;
pub const PARSEC_METERS: f64 = AU_METERS * 648_000.0 / std::f64::consts::PI;
pub const SPEED_OF_LIGHT_MPS: f64 = 299_792_458.0;

pub const VELOCITY: Dimension = [0, 1, -1, 0, 0, 0];
pub const ANGULAR_VELOCITY: Dimension = [0, 0, -1, 0, 0, 0];
pub const MOMENTUM: Dimension = [1, 1, -1, 0, 0, 0];

pub const FUNCTIONS: &[&str] = &[
    "sum",
    "mean",
    "sort",
    "median",
    "quantile",
    "std_population",
    "std_sample",
    "ecdf",
    "au2m",
    "m2au",
    "pc2m",
    "m2pc",
    "ly2m",
    "m2ly",
    "magnitude",
    "dot",
    "cross",
    "polar2cartesiand",
    "polar2cartesianr",
    "cartesian_radius",
    "cartesian_azimuthd",
    "cartesian_azimuthr",
    "cartesian_inclinationd",
    "cartesian_inclinationr",
    "spherical2cartesiand",
    "spherical2cartesianr",
    "velocity",
    "velocity_add_galilean",
    "velocity_add_relativistic_collinear",
    "angular_velocityd",
    "angular_velocityr",
    "tangential_velocity",
    "centripetal_acceleration",
    "angular_momentum",
    "angular_momentum_from_velocity",
];

pub fn is_function(name: &str) -> bool {
    FUNCTIONS.contains(&name)
}

pub fn is_distribution_function(name: &str) -> bool {
    matches!(
        name,
        "sort" | "median" | "quantile" | "std_population" | "std_sample" | "ecdf"
    )
}

pub fn frozen_statistics_policy_matches(receipt: &serde_json::Value) -> bool {
    // Historical freezes have no statistics pin: never invent one retroactively.
    receipt
        .get("statistics_policy")
        .is_none_or(|frozen| *frozen == statistics_policy())
}

pub fn statistics_policy() -> serde_json::Value {
    serde_json::json!({
        "schema": "goblin.statistics-policy.v1",
        "algorithm_sha256": crate::hashing::sha256_bytes(include_bytes!("statistics_runtime.rs")),
        "sort": "STABLE_ASCENDING_NUMERIC_COPY_SIGNED_ZERO_TIES_SOURCE_ORDER",
        "quantile": "HYNDMAN_FAN_TYPE_7_F64_NO_BOUNDARY_FUZZ_V1",
        "median": "QUANTILE_P_0.5",
        "standard_deviation": "ANCHORED_SCALED_COMPENSATED_TWO_PASS_V1",
        "std_population_divisor": "n",
        "std_sample_divisor": "n-1",
        "ecdf": "UNWEIGHTED_COUNT_LE_QUERY_DIV_N",
        "nonfinite_and_missing": "REFUSE_NO_IMPLICIT_ROW_DROPPING",
        "cross_platform_bitwise_guarantee": false
    })
}

pub fn arity(name: &str) -> Option<usize> {
    match name {
        "sum"
        | "mean"
        | "sort"
        | "median"
        | "std_population"
        | "std_sample"
        | "au2m"
        | "m2au"
        | "pc2m"
        | "m2pc"
        | "ly2m"
        | "m2ly"
        | "magnitude"
        | "cartesian_radius"
        | "cartesian_azimuthd"
        | "cartesian_azimuthr"
        | "cartesian_inclinationd"
        | "cartesian_inclinationr" => Some(1),
        "dot"
        | "quantile"
        | "ecdf"
        | "cross"
        | "polar2cartesiand"
        | "polar2cartesianr"
        | "velocity"
        | "velocity_add_galilean"
        | "velocity_add_relativistic_collinear"
        | "angular_velocityd"
        | "angular_velocityr"
        | "tangential_velocity"
        | "centripetal_acceleration"
        | "angular_momentum" => Some(2),
        "spherical2cartesiand" | "spherical2cartesianr" | "angular_momentum_from_velocity" => {
            Some(3)
        }
        _ => None,
    }
}

pub fn require_arity(name: &str, actual: usize) -> Result<()> {
    let expected =
        arity(name).ok_or_else(|| GoblinError::unknown(format!("Unknown function: {name}")))?;
    if actual != expected {
        return Err(GoblinError::parse(format!(
            "{name}() expects {expected} argument(s), got {actual}."
        )));
    }
    Ok(())
}

pub fn call(name: &str, values: Vec<Value>) -> Result<Value> {
    require_arity(name, values.len())?;
    match name {
        "sort" => {
            let Value::Array(items) = &values[0] else {
                return Err(GoblinError::new("G203", "sort() requires a numeric array."));
            };
            if items.is_empty() {
                return Ok(Value::Array(Vec::new()));
            }
            let quantities = vector(&values[0], name)?;
            let dimension = quantities[0].dimension;
            for value in &quantities {
                require_dimension(*value, dimension, name)?;
            }
            let numbers: Vec<_> = quantities.iter().map(|value| value.value_si).collect();
            array_value(
                crate::statistics_runtime::sorted_numeric(&numbers)
                    .map_err(GoblinError::numeric)?
                    .into_iter()
                    .map(|value| Quantity::new(value, dimension))
                    .collect::<Result<Vec<_>>>()?,
            )
        }
        "sum" | "mean" | "median" | "quantile" | "std_population" | "std_sample" | "ecdf" => {
            let quantities = vector(&values[0], name)?;
            let dimension = quantities[0].dimension;
            for value in &quantities {
                require_dimension(*value, dimension, name)?;
            }
            let numbers: Vec<_> = quantities.iter().map(|value| value.value_si).collect();
            let (result, result_dimension) = match name {
                "quantile" => {
                    let probability =
                        require_dimension(quantity(&values[1], name)?, DIMENSIONLESS, name)?;
                    (
                        crate::statistics_runtime::numeric_quantile(&numbers, probability.value_si),
                        dimension,
                    )
                }
                "ecdf" => {
                    let query = require_dimension(quantity(&values[1], name)?, dimension, name)?;
                    (
                        crate::statistics_runtime::numeric_ecdf(&numbers, query.value_si),
                        DIMENSIONLESS,
                    )
                }
                _ => (
                    crate::statistics_runtime::numeric_reduction(name, &numbers),
                    dimension,
                ),
            };
            let result = result.map_err(GoblinError::numeric)?;
            Quantity::new(result, result_dimension).map(Value::Quantity)
        }
        "au2m" => distance_from_numeric(name, quantity(&values[0], name)?, AU_METERS),
        "m2au" => distance_to_numeric(name, quantity(&values[0], name)?, AU_METERS),
        "pc2m" => distance_from_numeric(name, quantity(&values[0], name)?, PARSEC_METERS),
        "m2pc" => distance_to_numeric(name, quantity(&values[0], name)?, PARSEC_METERS),
        "ly2m" => distance_from_numeric(name, quantity(&values[0], name)?, LIGHT_YEAR_METERS),
        "m2ly" => distance_to_numeric(name, quantity(&values[0], name)?, LIGHT_YEAR_METERS),
        "magnitude" => magnitude_value(name, vector(&values[0], name)?),
        "dot" => dot_value(name, vector(&values[0], name)?, vector(&values[1], name)?),
        "cross" => array_value(cross_quantities(
            name,
            vector(&values[0], name)?,
            vector(&values[1], name)?,
        )?),
        "polar2cartesiand" | "polar2cartesianr" => {
            let radius = require_dimension(quantity(&values[0], name)?, LENGTH, name)?;
            require_nonnegative(radius, name, "radius")?;
            let angle = require_dimension(quantity(&values[1], name)?, DIMENSIONLESS, name)?;
            let radians = if name.ends_with('d') {
                angle.value_si.to_radians()
            } else {
                angle.value_si
            };
            array_value(vec![
                Quantity::new(radius.value_si * radians.cos(), LENGTH)?,
                Quantity::new(radius.value_si * radians.sin(), LENGTH)?,
            ])
        }
        "cartesian_radius" => {
            let coordinates = coordinate_vector(&values[0], name, &[2, 3])?;
            magnitude_value(name, coordinates)
        }
        "cartesian_azimuthd" | "cartesian_azimuthr" => {
            let coordinates = coordinate_vector(&values[0], name, &[2, 3])?;
            let x = coordinates[0].value_si;
            let y = coordinates[1].value_si;
            if x == 0.0 && y == 0.0 {
                return Err(GoblinError::numeric(format!(
                    "AZIMUTH DOMAIN ERROR\n\n{name}() requires a nonzero x or y coordinate."
                )));
            }
            let radians = y.atan2(x);
            scalar(if name.ends_with('d') {
                radians.to_degrees()
            } else {
                radians
            })
        }
        "cartesian_inclinationd" | "cartesian_inclinationr" => {
            let coordinates = coordinate_vector(&values[0], name, &[3])?;
            let radius = magnitude(&coordinates)?;
            if radius == 0.0 {
                return Err(GoblinError::numeric(format!(
                    "INCLINATION DOMAIN ERROR\n\n{name}() is undefined at the origin."
                )));
            }
            let radians = (coordinates[2].value_si / radius).clamp(-1.0, 1.0).acos();
            scalar(if name.ends_with('d') {
                radians.to_degrees()
            } else {
                radians
            })
        }
        "spherical2cartesiand" | "spherical2cartesianr" => {
            let radius = require_dimension(quantity(&values[0], name)?, LENGTH, name)?;
            require_nonnegative(radius, name, "radius")?;
            let inclination = require_dimension(quantity(&values[1], name)?, DIMENSIONLESS, name)?;
            let azimuth = require_dimension(quantity(&values[2], name)?, DIMENSIONLESS, name)?;
            let (inclination, azimuth, upper) = if name.ends_with('d') {
                (
                    inclination.value_si.to_radians(),
                    azimuth.value_si.to_radians(),
                    180.0,
                )
            } else {
                (inclination.value_si, azimuth.value_si, std::f64::consts::PI)
            };
            let raw_inclination = quantity(&values[1], name)?.value_si;
            if !(0.0..=upper).contains(&raw_inclination) {
                return Err(GoblinError::numeric(format!(
                    "SPHERICAL INCLINATION DOMAIN ERROR\n\n{name}() requires inclination from 0 through {upper}."
                )));
            }
            let radial_xy = radius.value_si * inclination.sin();
            array_value(vec![
                Quantity::new(radial_xy * azimuth.cos(), LENGTH)?,
                Quantity::new(radial_xy * azimuth.sin(), LENGTH)?,
                Quantity::new(radius.value_si * inclination.cos(), LENGTH)?,
            ])
        }
        "velocity" => {
            let elapsed = positive_time(quantity(&values[1], name)?, name)?;
            match &values[0] {
                Value::Quantity(displacement) => {
                    let displacement = require_dimension(*displacement, LENGTH, name)?;
                    displacement.checked_div(elapsed).map(Value::Quantity)
                }
                Value::Array(_) => {
                    let displacement = coordinate_vector(&values[0], name, &[])?;
                    array_value(
                        displacement
                            .into_iter()
                            .map(|component| component.checked_div(elapsed))
                            .collect::<Result<Vec<_>>>()?,
                    )
                }
                _ => Err(GoblinError::new(
                    "G203",
                    "velocity() requires a length quantity or length vector.",
                )),
            }
        }
        "velocity_add_galilean" => velocity_add_galilean(&values[0], &values[1]),
        "velocity_add_relativistic_collinear" => {
            let first = require_dimension(quantity(&values[0], name)?, VELOCITY, name)?;
            let second = require_dimension(quantity(&values[1], name)?, VELOCITY, name)?;
            if first.value_si.abs() > SPEED_OF_LIGHT_MPS
                || second.value_si.abs() > SPEED_OF_LIGHT_MPS
            {
                return Err(GoblinError::numeric(
                    "RELATIVISTIC VELOCITY DOMAIN ERROR\n\nInput speed magnitude cannot exceed c.",
                ));
            }
            let denominator = 1.0 + first.value_si * second.value_si / SPEED_OF_LIGHT_MPS.powi(2);
            if denominator == 0.0 {
                return Err(GoblinError::numeric(
                    "RELATIVISTIC VELOCITY DOMAIN ERROR\n\nThe collinear velocity-addition denominator is zero.",
                ));
            }
            Quantity::new((first.value_si + second.value_si) / denominator, VELOCITY)
                .map(Value::Quantity)
        }
        "angular_velocityd" | "angular_velocityr" => {
            let angle = require_dimension(quantity(&values[0], name)?, DIMENSIONLESS, name)?;
            let elapsed = positive_time(quantity(&values[1], name)?, name)?;
            let radians = if name.ends_with('d') {
                angle.value_si.to_radians()
            } else {
                angle.value_si
            };
            Quantity::new(radians / elapsed.value_si, ANGULAR_VELOCITY).map(Value::Quantity)
        }
        "tangential_velocity" => {
            let radius = require_dimension(quantity(&values[0], name)?, LENGTH, name)?;
            require_nonnegative(radius, name, "radius")?;
            let angular = require_dimension(quantity(&values[1], name)?, ANGULAR_VELOCITY, name)?;
            radius.checked_mul(angular).map(Value::Quantity)
        }
        "centripetal_acceleration" => {
            let radius = require_dimension(quantity(&values[0], name)?, LENGTH, name)?;
            require_nonnegative(radius, name, "radius")?;
            let angular = require_dimension(quantity(&values[1], name)?, ANGULAR_VELOCITY, name)?;
            radius.checked_mul(angular.powi(2)?).map(Value::Quantity)
        }
        "angular_momentum" => {
            let position = vector_with_dimension(&values[0], name, 3, LENGTH)?;
            let momentum = vector_with_dimension(&values[1], name, 3, MOMENTUM)?;
            array_value(cross_quantities(name, position, momentum)?)
        }
        "angular_momentum_from_velocity" => {
            let position = vector_with_dimension(&values[0], name, 3, LENGTH)?;
            let mass = require_dimension(quantity(&values[1], name)?, MASS, name)?;
            require_nonnegative(mass, name, "mass")?;
            let velocity = vector_with_dimension(&values[2], name, 3, VELOCITY)?;
            let momentum = velocity
                .into_iter()
                .map(|component| mass.checked_mul(component))
                .collect::<Result<Vec<_>>>()?;
            array_value(cross_quantities(name, position, momentum)?)
        }
        _ => Err(GoblinError::unknown(format!("Unknown function: {name}"))),
    }
}

fn quantity(value: &Value, name: &str) -> Result<Quantity> {
    value.quantity(name)
}

fn scalar(value: f64) -> Result<Value> {
    Quantity::scalar(value).map(Value::Quantity)
}

fn array_value(values: Vec<Quantity>) -> Result<Value> {
    Ok(Value::Array(
        values.into_iter().map(Value::Quantity).collect(),
    ))
}

fn require_dimension(value: Quantity, expected: Dimension, name: &str) -> Result<Quantity> {
    if value.dimension != expected {
        return Err(GoblinError::dimension(format!(
            "{name}() requires {}, got {}.",
            format_dimension(expected),
            format_dimension(value.dimension)
        )));
    }
    Ok(value)
}

fn require_nonnegative(value: Quantity, name: &str, field: &str) -> Result<()> {
    if value.value_si < 0.0 {
        return Err(GoblinError::numeric(format!(
            "{name}() requires a non-negative {field}."
        )));
    }
    Ok(())
}

fn positive_time(value: Quantity, name: &str) -> Result<Quantity> {
    let value = require_dimension(value, TIME, name)?;
    if value.value_si <= 0.0 {
        return Err(GoblinError::numeric(format!(
            "{name}() requires elapsed time greater than zero."
        )));
    }
    Ok(value)
}

fn distance_from_numeric(name: &str, value: Quantity, factor: f64) -> Result<Value> {
    let value = require_dimension(value, DIMENSIONLESS, name)?;
    Quantity::new(value.value_si * factor, LENGTH).map(Value::Quantity)
}

fn distance_to_numeric(name: &str, value: Quantity, factor: f64) -> Result<Value> {
    let value = require_dimension(value, LENGTH, name)?;
    scalar(value.value_si / factor)
}

fn vector(value: &Value, name: &str) -> Result<Vec<Quantity>> {
    let Value::Array(items) = value else {
        return Err(GoblinError::new(
            "G203",
            format!("{name}() requires a numeric array."),
        ));
    };
    if items.is_empty() {
        return Err(GoblinError::new(
            "G203",
            format!("{name}() requires a non-empty vector."),
        ));
    }
    items
        .iter()
        .map(|item| item.quantity(name))
        .collect::<Result<Vec<_>>>()
}

fn vector_with_dimension(
    value: &Value,
    name: &str,
    length: usize,
    dimension: Dimension,
) -> Result<Vec<Quantity>> {
    let values = vector(value, name)?;
    if values.len() != length {
        return Err(GoblinError::new(
            "G203",
            format!("{name}() requires a {length}-component vector."),
        ));
    }
    for value in &values {
        require_dimension(*value, dimension, name)?;
    }
    Ok(values)
}

fn coordinate_vector(value: &Value, name: &str, accepted: &[usize]) -> Result<Vec<Quantity>> {
    let values = vector(value, name)?;
    if !accepted.is_empty() && !accepted.contains(&values.len()) {
        return Err(GoblinError::new(
            "G203",
            format!(
                "{name}() requires a {}-component Cartesian vector.",
                accepted
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join("-or-")
            ),
        ));
    }
    for value in &values {
        require_dimension(*value, LENGTH, name)?;
    }
    Ok(values)
}

fn magnitude(values: &[Quantity]) -> Result<f64> {
    let first_dimension = values[0].dimension;
    if values
        .iter()
        .any(|value| value.dimension != first_dimension)
    {
        return Err(GoblinError::dimension(
            "Vector components must have matching dimensions.",
        ));
    }
    let sum = values.iter().try_fold(0.0, |sum, value| {
        let next = sum + value.value_si.powi(2);
        if next.is_finite() {
            Ok(next)
        } else {
            Err(GoblinError::numeric("Vector magnitude overflowed."))
        }
    })?;
    Ok(sum.sqrt())
}

fn magnitude_value(_name: &str, values: Vec<Quantity>) -> Result<Value> {
    let dimension = values[0].dimension;
    let magnitude = magnitude(&values)?;
    Quantity::new(magnitude, dimension).map(Value::Quantity)
}

fn dot_value(name: &str, left: Vec<Quantity>, right: Vec<Quantity>) -> Result<Value> {
    if left.len() != right.len() {
        return Err(GoblinError::new(
            "G203",
            format!("{name}() requires vectors of equal length."),
        ));
    }
    let mut products = left
        .into_iter()
        .zip(right)
        .map(|(left, right)| left.checked_mul(right));
    let first = products
        .next()
        .ok_or_else(|| GoblinError::new("G203", "dot() requires non-empty vectors."))??;
    products
        .try_fold(first, |sum, product| sum.checked_add(product?))
        .map(Value::Quantity)
}

fn cross_quantities(
    name: &str,
    left: Vec<Quantity>,
    right: Vec<Quantity>,
) -> Result<Vec<Quantity>> {
    if left.len() != 3 || right.len() != 3 {
        return Err(GoblinError::new(
            "G203",
            format!("{name}() requires two three-component vectors."),
        ));
    }
    Ok(vec![
        left[1]
            .checked_mul(right[2])?
            .checked_sub(left[2].checked_mul(right[1])?)?,
        left[2]
            .checked_mul(right[0])?
            .checked_sub(left[0].checked_mul(right[2])?)?,
        left[0]
            .checked_mul(right[1])?
            .checked_sub(left[1].checked_mul(right[0])?)?,
    ])
}

fn velocity_add_galilean(left: &Value, right: &Value) -> Result<Value> {
    match (left, right) {
        (Value::Quantity(left), Value::Quantity(right)) => {
            let left = require_dimension(*left, VELOCITY, "velocity_add_galilean")?;
            let right = require_dimension(*right, VELOCITY, "velocity_add_galilean")?;
            left.checked_add(right).map(Value::Quantity)
        }
        (Value::Array(_), Value::Array(_)) => {
            let left = vector(left, "velocity_add_galilean")?;
            let right = vector(right, "velocity_add_galilean")?;
            if left.len() != right.len() {
                return Err(GoblinError::new(
                    "G203",
                    "velocity_add_galilean() requires vectors of equal length.",
                ));
            }
            array_value(
                left.into_iter()
                    .zip(right)
                    .map(|(left, right)| {
                        require_dimension(left, VELOCITY, "velocity_add_galilean")?.checked_add(
                            require_dimension(right, VELOCITY, "velocity_add_galilean")?,
                        )
                    })
                    .collect::<Result<Vec<_>>>()?,
            )
        }
        _ => Err(GoblinError::new(
            "G203",
            "velocity_add_galilean() requires two scalar velocities or two velocity vectors.",
        )),
    }
}
