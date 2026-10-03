// Included verbatim in generated Rust programs. `Value`, `Dim`, and `ZERO` are
// defined by the generated runtime before this file is inserted.
fn goblin_math_call(name: &str, values: Vec<Value>) -> Result<Value, String> {
    warn_legacy_angle(name);
    let minimum = if matches!(name, "min" | "max") { 2 } else { 0 };
    if minimum != 0 && values.len() < minimum {
        return Err(format!(
            "{name}() expects at least {minimum} arguments, got {}.",
            values.len()
        ));
    }
    let expected = if name == "is_close" {
        4
    } else if matches!(name, "atan2" | "atan2d" | "atan2r" | "hypot" | "same_bits") {
        2
    } else if minimum == 0 {
        1
    } else {
        values.len()
    };
    if values.len() != expected {
        return Err(format!(
            "{name}() expects {expected} argument(s), got {}.",
            values.len()
        ));
    }

    let quantities = values
        .into_iter()
        .map(as_q)
        .collect::<Result<Vec<_>, _>>()?;
    let (first, first_dim) = quantities[0];
    let require_dimensionless = |value: f64, dimension: Dim| -> Result<f64, String> {
        if dimension != ZERO {
            return Err(format!("{name}() requires a dimensionless input."));
        }
        Ok(value)
    };
    let require_matching_dimensions = || -> Result<(), String> {
        if quantities.iter().any(|(_, dim)| *dim != first_dim) {
            return Err(format!("{name}() requires matching dimensions."));
        }
        Ok(())
    };

    match name {
        "is_close" | "same_bits" => {
            if first_dim != quantities[1].1 {
                return Err(format!("{name}() requires matching operand dimensions."));
            }
            if name == "same_bits" {
                return Ok(Value::Bool(first.to_bits() == quantities[1].0.to_bits()));
            }
            if quantities[2].1 != first_dim || quantities[3].1 != ZERO {
                return Err("is_close() requires an absolute tolerance with the operands' dimensions and a dimensionless relative tolerance.".into());
            }
            goblin_numeric_comparison::is_close(first, quantities[1].0, quantities[2].0, quantities[3].0).map(Value::Bool)
        }
        "abs" => Value::q(first.abs(), first_dim),
        "sqrt" => {
            if first < 0.0 {
                return Err("SQUARE ROOT DOMAIN ERROR: sqrt() requires a non-negative value.".into());
            }
            if first_dim.iter().any(|power| power % 2 != 0) {
                return Err("SQUARE ROOT DIMENSION ERROR: sqrt() requires even unit exponents.".into());
            }
            Value::q(first.sqrt(), first_dim.map(|power| power / 2))
        }
        "min" | "max" => {
            require_matching_dimensions()?;
            let selected = quantities[1..]
                .iter()
                .fold(first, |current, (value, _)| {
                    if name == "min" {
                        current.min(*value)
                    } else {
                        current.max(*value)
                    }
                });
            Value::q(selected, first_dim)
        }
        "floor" => Value::scalar(require_dimensionless(first, first_dim)?.floor()),
        "ceil" => Value::scalar(require_dimensionless(first, first_dim)?.ceil()),
        "round" => Value::scalar(require_dimensionless(first, first_dim)?.round()),
        "exp" => Value::scalar(require_dimensionless(first, first_dim)?.exp()),
        "ln" | "log10" => {
            let value = require_dimensionless(first, first_dim)?;
            if value <= 0.0 {
                return Err(format!(
                    "LOGARITHM DOMAIN ERROR: {name}() requires a value greater than zero."
                ));
            }
            Value::scalar(if name == "ln" { value.ln() } else { value.log10() })
        }
        "sin" | "sinr" => Value::scalar(require_dimensionless(first, first_dim)?.sin()),
        "cos" | "cosr" => Value::scalar(require_dimensionless(first, first_dim)?.cos()),
        "tan" | "tanr" => Value::scalar(require_dimensionless(first, first_dim)?.tan()),
        "sind" => Value::scalar(require_dimensionless(first, first_dim)?.to_radians().sin()),
        "cosd" => Value::scalar(require_dimensionless(first, first_dim)?.to_radians().cos()),
        "tand" => Value::scalar(require_dimensionless(first, first_dim)?.to_radians().tan()),
        "asin" | "acos" | "asinr" | "acosr" | "asind" | "acosd" => {
            let value = require_dimensionless(first, first_dim)?;
            if !(-1.0..=1.0).contains(&value) {
                return Err(format!(
                    "INVERSE TRIGONOMETRIC DOMAIN ERROR: {name}() requires a value from -1 through 1."
                ));
            }
            let radians = if matches!(name, "asin" | "asinr" | "asind") {
                value.asin()
            } else {
                value.acos()
            };
            Value::scalar(if matches!(name, "asind" | "acosd") {
                radians.to_degrees()
            } else {
                radians
            })
        }
        "atan" | "atanr" => Value::scalar(require_dimensionless(first, first_dim)?.atan()),
        "atand" => Value::scalar(require_dimensionless(first, first_dim)?.atan().to_degrees()),
        "atan2" | "atan2r" | "atan2d" => {
            require_matching_dimensions()?;
            let x = quantities[1].0;
            if first == 0.0 && x == 0.0 {
                return Err("ATAN2 DOMAIN ERROR: atan2(0, 0) has no defined direction.".into());
            }
            let radians = first.atan2(x);
            Value::scalar(if name == "atan2d" {
                radians.to_degrees()
            } else {
                radians
            })
        }
        "deg2rad" => Value::scalar(require_dimensionless(first, first_dim)?.to_radians()),
        "rad2deg" => Value::scalar(require_dimensionless(first, first_dim)?.to_degrees()),
        "hypot" => {
            require_matching_dimensions()?;
            Value::q(first.hypot(quantities[1].0), first_dim)
        }
        _ => Err(format!("UNKNOWN SYMBOL: {name}")),
    }
}

fn warn_legacy_angle(name: &str) {
    let Some((radians, degrees)) = (match name {
        "sin" => Some(("sinr", "sind")),
        "cos" => Some(("cosr", "cosd")),
        "tan" => Some(("tanr", "tand")),
        "asin" => Some(("asinr", "asind")),
        "acos" => Some(("acosr", "acosd")),
        "atan" => Some(("atanr", "atand")),
        "atan2" => Some(("atan2r", "atan2d")),
        _ => None,
    }) else {
        return;
    };
    static WARNED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::OnceLock::new();
    let warned = WARNED.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()));
    if let Ok(mut warned) = warned.lock()
        && warned.insert(name.to_string())
    {
        eprintln!(
            "GOBLIN WARNING G302\n\nAMBIGUOUS ANGLE FUNCTION\n\n{name}() currently means radians for compatibility.\n\nUse:\n    {radians}()    # radians\n    {degrees}()    # degrees\n"
        );
    }
}
