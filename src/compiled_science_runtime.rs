// Included verbatim in generated Rust programs. `Value`, `Dim`, and `ZERO` are
// defined by the generated runtime before this file is inserted.
const SCIENCE_LENGTH: Dim = [0, 1, 0, 0, 0, 0];
const SCIENCE_MASS: Dim = [1, 0, 0, 0, 0, 0];
const SCIENCE_TIME: Dim = [0, 0, 1, 0, 0, 0];
const SCIENCE_VELOCITY: Dim = [0, 1, -1, 0, 0, 0];
const SCIENCE_ANGULAR_VELOCITY: Dim = [0, 0, -1, 0, 0, 0];
const SCIENCE_MOMENTUM: Dim = [1, 1, -1, 0, 0, 0];
const SCIENCE_AU_METERS: f64 = 149_597_870_700.0;
const SCIENCE_LIGHT_YEAR_METERS: f64 = 9_460_730_472_580_800.0;
const SCIENCE_PARSEC_METERS: f64 =
    SCIENCE_AU_METERS * 648_000.0 / std::f64::consts::PI;
const SCIENCE_C_MPS: f64 = 299_792_458.0;

fn goblin_science_call(name: &str, values: Vec<Value>) -> Result<Value, String> {
    let expected = match name {
        "sort" | "median" | "std_population" | "std_sample"
        | "sum" | "mean" | "au2m" | "m2au" | "pc2m" | "m2pc" | "ly2m" | "m2ly" | "magnitude"
        | "cartesian_radius" | "cartesian_azimuthd" | "cartesian_azimuthr"
        | "cartesian_inclinationd" | "cartesian_inclinationr" => 1,
        "quantile" | "ecdf" | "dot" | "cross" | "polar2cartesiand" | "polar2cartesianr" | "velocity"
        | "velocity_add_galilean" | "velocity_add_relativistic_collinear"
        | "angular_velocityd" | "angular_velocityr" | "tangential_velocity"
        | "centripetal_acceleration" | "angular_momentum" => 2,
        "spherical2cartesiand" | "spherical2cartesianr"
        | "angular_momentum_from_velocity" => 3,
        _ => return Err(format!("UNKNOWN SYMBOL: {name}")),
    };
    if values.len() != expected {
        return Err(format!(
            "{name}() expects {expected} argument(s), got {}.",
            values.len()
        ));
    }

    match name {
        "sort" => {
            let Value::Array(items) = &values[0] else { return Err("sort() requires a numeric array.".into()); };
            if items.is_empty() { return Ok(Value::Array(Vec::new())); }
            let quantities = science_vector(&values[0], name)?;
            let dimension = quantities[0].1;
            for value in &quantities { science_require_dim(*value, dimension, name)?; }
            let numbers: Vec<_> = quantities.iter().map(|value| value.0).collect();
            Ok(Value::Array(goblin_statistics::sorted_numeric(&numbers)?.into_iter()
                .map(|value| Value::q(value, dimension)).collect::<Result<Vec<_>, _>>()?))
        }
        "sum" | "mean" | "median" | "quantile" | "std_population" | "std_sample" | "ecdf" => {
            let quantities = science_vector(&values[0], name)?;
            let dimension = quantities[0].1;
            for value in &quantities {
                science_require_dim(*value, dimension, name)?;
            }
            let numbers: Vec<_> = quantities.iter().map(|value| value.0).collect();
            let (result, result_dimension) = match name {
                "quantile" => {
                    let p = science_require_dim(science_q(&values[1], name)?, ZERO, name)?;
                    (goblin_statistics::numeric_quantile(&numbers, p.0)?, dimension)
                }
                "ecdf" => {
                    let query = science_require_dim(science_q(&values[1], name)?, dimension, name)?;
                    (goblin_statistics::numeric_ecdf(&numbers, query.0)?, ZERO)
                }
                _ => (goblin_statistics::numeric_reduction(name, &numbers)?, dimension),
            };
            Value::q(result, result_dimension)
        }
        "au2m" => science_distance_from(&values[0], name, SCIENCE_AU_METERS),
        "m2au" => science_distance_to(&values[0], name, SCIENCE_AU_METERS),
        "pc2m" => science_distance_from(&values[0], name, SCIENCE_PARSEC_METERS),
        "m2pc" => science_distance_to(&values[0], name, SCIENCE_PARSEC_METERS),
        "ly2m" => science_distance_from(&values[0], name, SCIENCE_LIGHT_YEAR_METERS),
        "m2ly" => science_distance_to(&values[0], name, SCIENCE_LIGHT_YEAR_METERS),
        "magnitude" => {
            let vector = science_vector(&values[0], name)?;
            let dimension = vector[0].1;
            Value::q(science_magnitude(&vector)?, dimension)
        }
        "dot" => {
            let left = science_vector(&values[0], name)?;
            let right = science_vector(&values[1], name)?;
            if left.len() != right.len() {
                return Err(format!("{name}() requires vectors of equal length."));
            }
            let dimension = science_add_dim(left[0].1, right[0].1);
            if left
                .iter()
                .zip(&right)
                .any(|(a, b)| science_add_dim(a.1, b.1) != dimension)
            {
                return Err("Vector dot products must have compatible component dimensions.".into());
            }
            Value::q(
                left.iter().zip(&right).map(|(a, b)| a.0 * b.0).sum(),
                dimension,
            )
        }
        "cross" => science_cross_value(
            name,
            science_vector(&values[0], name)?,
            science_vector(&values[1], name)?,
        ),
        "polar2cartesiand" | "polar2cartesianr" => {
            let radius = science_require_dim(science_q(&values[0], name)?, SCIENCE_LENGTH, name)?;
            science_nonnegative(radius.0, name, "radius")?;
            let angle = science_require_dim(science_q(&values[1], name)?, ZERO, name)?.0;
            let radians = if name.ends_with('d') {
                angle.to_radians()
            } else {
                angle
            };
            science_array(vec![
                (radius.0 * radians.cos(), SCIENCE_LENGTH),
                (radius.0 * radians.sin(), SCIENCE_LENGTH),
            ])
        }
        "cartesian_radius" => {
            let coordinates = science_coordinates(&values[0], name, &[2, 3])?;
            Value::q(science_magnitude(&coordinates)?, SCIENCE_LENGTH)
        }
        "cartesian_azimuthd" | "cartesian_azimuthr" => {
            let coordinates = science_coordinates(&values[0], name, &[2, 3])?;
            if coordinates[0].0 == 0.0 && coordinates[1].0 == 0.0 {
                return Err(format!(
                    "AZIMUTH DOMAIN ERROR: {name}() requires a nonzero x or y coordinate."
                ));
            }
            let radians = coordinates[1].0.atan2(coordinates[0].0);
            Value::scalar(if name.ends_with('d') {
                radians.to_degrees()
            } else {
                radians
            })
        }
        "cartesian_inclinationd" | "cartesian_inclinationr" => {
            let coordinates = science_coordinates(&values[0], name, &[3])?;
            let radius = science_magnitude(&coordinates)?;
            if radius == 0.0 {
                return Err(format!(
                    "INCLINATION DOMAIN ERROR: {name}() is undefined at the origin."
                ));
            }
            let radians = (coordinates[2].0 / radius).clamp(-1.0, 1.0).acos();
            Value::scalar(if name.ends_with('d') {
                radians.to_degrees()
            } else {
                radians
            })
        }
        "spherical2cartesiand" | "spherical2cartesianr" => {
            let radius = science_require_dim(science_q(&values[0], name)?, SCIENCE_LENGTH, name)?;
            science_nonnegative(radius.0, name, "radius")?;
            let raw_inclination = science_require_dim(science_q(&values[1], name)?, ZERO, name)?.0;
            let raw_azimuth = science_require_dim(science_q(&values[2], name)?, ZERO, name)?.0;
            let (inclination, azimuth, upper) = if name.ends_with('d') {
                (raw_inclination.to_radians(), raw_azimuth.to_radians(), 180.0)
            } else {
                (raw_inclination, raw_azimuth, std::f64::consts::PI)
            };
            if !(0.0..=upper).contains(&raw_inclination) {
                return Err(format!(
                    "SPHERICAL INCLINATION DOMAIN ERROR: {name}() requires inclination from 0 through {upper}."
                ));
            }
            let radial_xy = radius.0 * inclination.sin();
            science_array(vec![
                (radial_xy * azimuth.cos(), SCIENCE_LENGTH),
                (radial_xy * azimuth.sin(), SCIENCE_LENGTH),
                (radius.0 * inclination.cos(), SCIENCE_LENGTH),
            ])
        }
        "velocity" => {
            let elapsed = science_positive_time(&values[1], name)?;
            match &values[0] {
                Value::Q(displacement, dimension) => {
                    science_require_dim((*displacement, *dimension), SCIENCE_LENGTH, name)?;
                    Value::q(*displacement / elapsed, SCIENCE_VELOCITY)
                }
                Value::Array(_) => {
                    let displacement = science_coordinates(&values[0], name, &[])?;
                    science_array(
                        displacement
                            .into_iter()
                            .map(|item| (item.0 / elapsed, SCIENCE_VELOCITY))
                            .collect(),
                    )
                }
                _ => Err("velocity() requires a length quantity or length vector.".into()),
            }
        }
        "velocity_add_galilean" => science_velocity_add_galilean(&values[0], &values[1]),
        "velocity_add_relativistic_collinear" => {
            let first = science_require_dim(science_q(&values[0], name)?, SCIENCE_VELOCITY, name)?.0;
            let second = science_require_dim(science_q(&values[1], name)?, SCIENCE_VELOCITY, name)?.0;
            if first.abs() > SCIENCE_C_MPS || second.abs() > SCIENCE_C_MPS {
                return Err("RELATIVISTIC VELOCITY DOMAIN ERROR: input speed magnitude cannot exceed c.".into());
            }
            let denominator = 1.0 + first * second / SCIENCE_C_MPS.powi(2);
            if denominator == 0.0 {
                return Err("RELATIVISTIC VELOCITY DOMAIN ERROR: denominator is zero.".into());
            }
            Value::q((first + second) / denominator, SCIENCE_VELOCITY)
        }
        "angular_velocityd" | "angular_velocityr" => {
            let angle = science_require_dim(science_q(&values[0], name)?, ZERO, name)?.0;
            let elapsed = science_positive_time(&values[1], name)?;
            let radians = if name.ends_with('d') {
                angle.to_radians()
            } else {
                angle
            };
            Value::q(radians / elapsed, SCIENCE_ANGULAR_VELOCITY)
        }
        "tangential_velocity" => {
            let radius = science_require_dim(science_q(&values[0], name)?, SCIENCE_LENGTH, name)?.0;
            science_nonnegative(radius, name, "radius")?;
            let angular = science_require_dim(
                science_q(&values[1], name)?,
                SCIENCE_ANGULAR_VELOCITY,
                name,
            )?
            .0;
            Value::q(radius * angular, SCIENCE_VELOCITY)
        }
        "centripetal_acceleration" => {
            let radius = science_require_dim(science_q(&values[0], name)?, SCIENCE_LENGTH, name)?.0;
            science_nonnegative(radius, name, "radius")?;
            let angular = science_require_dim(
                science_q(&values[1], name)?,
                SCIENCE_ANGULAR_VELOCITY,
                name,
            )?
            .0;
            Value::q(radius * angular.powi(2), [0, 1, -2, 0, 0, 0])
        }
        "angular_momentum" => {
            let position = science_vector_dim(&values[0], name, 3, SCIENCE_LENGTH)?;
            let momentum = science_vector_dim(&values[1], name, 3, SCIENCE_MOMENTUM)?;
            science_cross_value(name, position, momentum)
        }
        "angular_momentum_from_velocity" => {
            let position = science_vector_dim(&values[0], name, 3, SCIENCE_LENGTH)?;
            let mass = science_require_dim(science_q(&values[1], name)?, SCIENCE_MASS, name)?.0;
            science_nonnegative(mass, name, "mass")?;
            let velocity = science_vector_dim(&values[2], name, 3, SCIENCE_VELOCITY)?;
            let momentum = velocity
                .into_iter()
                .map(|item| (mass * item.0, SCIENCE_MOMENTUM))
                .collect();
            science_cross_value(name, position, momentum)
        }
        _ => Err(format!("UNKNOWN SYMBOL: {name}")),
    }
}

fn science_q(value: &Value, name: &str) -> Result<(f64, Dim), String> {
    match value {
        Value::Q(value, dimension) => Ok((*value, *dimension)),
        _ => Err(format!("{name}() requires numeric quantities.")),
    }
}

fn science_require_dim(
    value: (f64, Dim),
    expected: Dim,
    name: &str,
) -> Result<(f64, Dim), String> {
    if value.1 != expected {
        return Err(format!(
            "{name}() requires {}, got {}.",
            format_dim(expected),
            format_dim(value.1)
        ));
    }
    Ok(value)
}

fn science_vector(value: &Value, name: &str) -> Result<Vec<(f64, Dim)>, String> {
    let Value::Array(items) = value else {
        return Err(format!("{name}() requires a numeric array."));
    };
    if items.is_empty() {
        return Err(format!("{name}() requires a non-empty vector."));
    }
    items.iter().map(|item| science_q(item, name)).collect()
}

fn science_vector_dim(
    value: &Value,
    name: &str,
    length: usize,
    dimension: Dim,
) -> Result<Vec<(f64, Dim)>, String> {
    let vector = science_vector(value, name)?;
    if vector.len() != length {
        return Err(format!("{name}() requires a {length}-component vector."));
    }
    for item in &vector {
        science_require_dim(*item, dimension, name)?;
    }
    Ok(vector)
}

fn science_coordinates(
    value: &Value,
    name: &str,
    accepted: &[usize],
) -> Result<Vec<(f64, Dim)>, String> {
    let vector = science_vector(value, name)?;
    if !accepted.is_empty() && !accepted.contains(&vector.len()) {
        return Err(format!("{name}() received the wrong Cartesian vector length."));
    }
    for item in &vector {
        science_require_dim(*item, SCIENCE_LENGTH, name)?;
    }
    Ok(vector)
}

fn science_array(values: Vec<(f64, Dim)>) -> Result<Value, String> {
    values
        .into_iter()
        .map(|item| Value::q(item.0, item.1))
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
}

fn science_magnitude(values: &[(f64, Dim)]) -> Result<f64, String> {
    let dimension = values[0].1;
    if values.iter().any(|item| item.1 != dimension) {
        return Err("Vector components must have matching dimensions.".into());
    }
    let sum: f64 = values.iter().map(|item| item.0.powi(2)).sum();
    if !sum.is_finite() {
        return Err("Vector magnitude overflowed.".into());
    }
    Ok(sum.sqrt())
}

fn science_cross_value(
    name: &str,
    left: Vec<(f64, Dim)>,
    right: Vec<(f64, Dim)>,
) -> Result<Value, String> {
    if left.len() != 3 || right.len() != 3 {
        return Err(format!("{name}() requires two three-component vectors."));
    }
    let dimension = science_add_dim(left[0].1, right[0].1);
    if left
        .iter()
        .zip(&right)
        .any(|(a, b)| science_add_dim(a.1, b.1) != dimension)
    {
        return Err("Vector cross products require compatible component dimensions.".into());
    }
    science_array(vec![
        (left[1].0 * right[2].0 - left[2].0 * right[1].0, dimension),
        (left[2].0 * right[0].0 - left[0].0 * right[2].0, dimension),
        (left[0].0 * right[1].0 - left[1].0 * right[0].0, dimension),
    ])
}

fn science_velocity_add_galilean(left: &Value, right: &Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Q(..), Value::Q(..)) => {
            let left = science_require_dim(
                science_q(left, "velocity_add_galilean")?,
                SCIENCE_VELOCITY,
                "velocity_add_galilean",
            )?;
            let right = science_require_dim(
                science_q(right, "velocity_add_galilean")?,
                SCIENCE_VELOCITY,
                "velocity_add_galilean",
            )?;
            Value::q(left.0 + right.0, SCIENCE_VELOCITY)
        }
        (Value::Array(_), Value::Array(_)) => {
            let left = science_vector(left, "velocity_add_galilean")?;
            let right = science_vector(right, "velocity_add_galilean")?;
            if left.len() != right.len() {
                return Err("velocity_add_galilean() requires vectors of equal length.".into());
            }
            let mut result = Vec::with_capacity(left.len());
            for (left, right) in left.into_iter().zip(right) {
                science_require_dim(left, SCIENCE_VELOCITY, "velocity_add_galilean")?;
                science_require_dim(right, SCIENCE_VELOCITY, "velocity_add_galilean")?;
                result.push((left.0 + right.0, SCIENCE_VELOCITY));
            }
            science_array(result)
        }
        _ => Err(
            "velocity_add_galilean() requires two scalar velocities or two velocity vectors."
                .into(),
        ),
    }
}

fn science_distance_from(value: &Value, name: &str, factor: f64) -> Result<Value, String> {
    let value = science_require_dim(science_q(value, name)?, ZERO, name)?.0;
    Value::q(value * factor, SCIENCE_LENGTH)
}

fn science_distance_to(value: &Value, name: &str, factor: f64) -> Result<Value, String> {
    let value = science_require_dim(science_q(value, name)?, SCIENCE_LENGTH, name)?.0;
    Value::scalar(value / factor)
}

fn science_positive_time(value: &Value, name: &str) -> Result<f64, String> {
    let value = science_require_dim(science_q(value, name)?, SCIENCE_TIME, name)?.0;
    if value <= 0.0 {
        return Err(format!(
            "{name}() requires elapsed time greater than zero."
        ));
    }
    Ok(value)
}

fn science_nonnegative(value: f64, name: &str, field: &str) -> Result<(), String> {
    if value < 0.0 {
        return Err(format!("{name}() requires a non-negative {field}."));
    }
    Ok(())
}

fn science_add_dim(left: Dim, right: Dim) -> Dim {
    std::array::from_fn(|index| left[index] + right[index])
}
