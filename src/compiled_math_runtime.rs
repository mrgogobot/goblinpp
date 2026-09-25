// Included verbatim in generated Rust programs. `Value`, `Dim`, and `ZERO` are
// defined by the generated runtime before this file is inserted.
fn goblin_math_call(name: &str, values: Vec<Value>) -> Result<Value, String> {
    let minimum = if matches!(name, "min" | "max") { 2 } else { 0 };
    if minimum != 0 && values.len() < minimum {
        return Err(format!(
            "{name}() expects at least {minimum} arguments, got {}.",
            values.len()
        ));
    }
    let expected = if matches!(name, "atan2" | "hypot") {
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
        "sin" => Value::scalar(require_dimensionless(first, first_dim)?.sin()),
        "cos" => Value::scalar(require_dimensionless(first, first_dim)?.cos()),
        "tan" => Value::scalar(require_dimensionless(first, first_dim)?.tan()),
        "asin" | "acos" => {
            let value = require_dimensionless(first, first_dim)?;
            if !(-1.0..=1.0).contains(&value) {
                return Err(format!(
                    "INVERSE TRIGONOMETRIC DOMAIN ERROR: {name}() requires a value from -1 through 1."
                ));
            }
            Value::scalar(if name == "asin" { value.asin() } else { value.acos() })
        }
        "atan" => Value::scalar(require_dimensionless(first, first_dim)?.atan()),
        "atan2" => {
            require_matching_dimensions()?;
            let x = quantities[1].0;
            if first == 0.0 && x == 0.0 {
                return Err("ATAN2 DOMAIN ERROR: atan2(0, 0) has no defined direction.".into());
            }
            Value::scalar(first.atan2(x))
        }
        "hypot" => {
            require_matching_dimensions()?;
            Value::q(first.hypot(quantities[1].0), first_dim)
        }
        _ => Err(format!("UNKNOWN SYMBOL: {name}")),
    }
}
