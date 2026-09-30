// Included verbatim in generated Rust programs. `Value`, `Dim`, and `ZERO` are
// defined by the generated runtime before this file is inserted.
const EE_TIME: Dim = [0, 0, 1, 0, 0, 0];
const EE_CURRENT: Dim = [0, 0, 0, 0, 0, 1];
const EE_CHARGE: Dim = [0, 0, 1, 0, 0, 1];
const EE_VOLTAGE: Dim = [1, 2, -3, 0, 0, -1];
const EE_RESISTANCE: Dim = [1, 2, -3, 0, 0, -2];
const EE_CAPACITANCE: Dim = [-1, -2, 4, 0, 0, 2];
const EE_INDUCTANCE: Dim = [1, 2, -2, 0, 0, -2];
const EE_POWER: Dim = [1, 2, -3, 0, 0, 0];
const EE_ENERGY: Dim = [1, 2, -2, 0, 0, 0];
const EE_CONDUCTANCE: Dim = [-1, -2, 3, 0, 0, 2];
const EE_FREQUENCY: Dim = [0, 0, -1, 0, 0, 0];

fn goblin_electrical_call(name: &str, values: Vec<Value>) -> Result<Value, String> {
    let expected = match name {
        "ee_series_resistance" | "ee_parallel_resistance" | "ee_kcl_residual"
        | "ee_kvl_residual" => 1,
        "ee_voltage" | "ee_current" | "ee_resistance" | "ee_power" | "ee_power_i2r"
        | "ee_power_v2r" | "ee_energy" | "ee_charge" | "ee_rc_time_constant"
        | "ee_capacitor_energy" | "ee_inductor_energy" | "ee_kcl_balanced"
        | "ee_kvl_balanced" | "ee_in_unit" => 2,
        _ => return Err(format!("UNKNOWN SYMBOL: {name}")),
    };
    if values.len() != expected {
        return Err(format!(
            "{name}() expects {expected} argument(s), got {}.",
            values.len()
        ));
    }
    match name {
        "ee_voltage" => {
            let current = ee_require_dim(ee_q(&values[0], name)?, EE_CURRENT, name)?;
            let resistance = ee_nonnegative(ee_require_dim(ee_q(&values[1], name)?, EE_RESISTANCE, name)?, name, "resistance")?;
            Value::q(current.0 * resistance.0, EE_VOLTAGE)
        }
        "ee_current" => {
            let voltage = ee_require_dim(ee_q(&values[0], name)?, EE_VOLTAGE, name)?;
            let resistance = ee_positive(ee_require_dim(ee_q(&values[1], name)?, EE_RESISTANCE, name)?, name, "resistance")?;
            Value::q(voltage.0 / resistance.0, EE_CURRENT)
        }
        "ee_resistance" => {
            let voltage = ee_require_dim(ee_q(&values[0], name)?, EE_VOLTAGE, name)?;
            let current = ee_nonzero(ee_require_dim(ee_q(&values[1], name)?, EE_CURRENT, name)?, name, "current")?;
            let result = voltage.0 / current.0;
            if result < 0.0 { return Err(format!("{name}() requires non-negative resulting resistance.")); }
            Value::q(result, EE_RESISTANCE)
        }
        "ee_power" => {
            let voltage = ee_require_dim(ee_q(&values[0], name)?, EE_VOLTAGE, name)?;
            let current = ee_require_dim(ee_q(&values[1], name)?, EE_CURRENT, name)?;
            Value::q(voltage.0 * current.0, EE_POWER)
        }
        "ee_power_i2r" => {
            let current = ee_require_dim(ee_q(&values[0], name)?, EE_CURRENT, name)?;
            let resistance = ee_nonnegative(ee_require_dim(ee_q(&values[1], name)?, EE_RESISTANCE, name)?, name, "resistance")?;
            Value::q(current.0.powi(2) * resistance.0, EE_POWER)
        }
        "ee_power_v2r" => {
            let voltage = ee_require_dim(ee_q(&values[0], name)?, EE_VOLTAGE, name)?;
            let resistance = ee_positive(ee_require_dim(ee_q(&values[1], name)?, EE_RESISTANCE, name)?, name, "resistance")?;
            Value::q(voltage.0.powi(2) / resistance.0, EE_POWER)
        }
        "ee_energy" => {
            let power = ee_require_dim(ee_q(&values[0], name)?, EE_POWER, name)?;
            let time = ee_nonnegative(ee_require_dim(ee_q(&values[1], name)?, EE_TIME, name)?, name, "time")?;
            Value::q(power.0 * time.0, EE_ENERGY)
        }
        "ee_charge" => {
            let current = ee_require_dim(ee_q(&values[0], name)?, EE_CURRENT, name)?;
            let time = ee_nonnegative(ee_require_dim(ee_q(&values[1], name)?, EE_TIME, name)?, name, "time")?;
            Value::q(current.0 * time.0, EE_CHARGE)
        }
        "ee_series_resistance" => {
            let items = ee_array(&values[0], name)?;
            let mut total = 0.0;
            for item in items {
                total += ee_nonnegative(ee_require_dim(ee_q(item, name)?, EE_RESISTANCE, name)?, name, "resistance")?.0;
                Value::q(total, EE_RESISTANCE)?;
            }
            Value::q(total, EE_RESISTANCE)
        }
        "ee_parallel_resistance" => {
            let items = ee_array(&values[0], name)?;
            let mut resistances = Vec::with_capacity(items.len());
            for item in items {
                resistances.push(ee_positive(ee_require_dim(ee_q(item, name)?, EE_RESISTANCE, name)?, name, "resistance")?.0);
            }
            let minimum = resistances.iter().copied().fold(f64::INFINITY, f64::min);
            let scaled_sum = resistances.iter().map(|value| minimum / value).sum::<f64>();
            Value::q(minimum / scaled_sum, EE_RESISTANCE)
        }
        "ee_rc_time_constant" => {
            let resistance = ee_nonnegative(ee_require_dim(ee_q(&values[0], name)?, EE_RESISTANCE, name)?, name, "resistance")?;
            let capacitance = ee_nonnegative(ee_require_dim(ee_q(&values[1], name)?, EE_CAPACITANCE, name)?, name, "capacitance")?;
            Value::q(resistance.0 * capacitance.0, EE_TIME)
        }
        "ee_capacitor_energy" => {
            let capacitance = ee_nonnegative(ee_require_dim(ee_q(&values[0], name)?, EE_CAPACITANCE, name)?, name, "capacitance")?;
            let voltage = ee_require_dim(ee_q(&values[1], name)?, EE_VOLTAGE, name)?;
            Value::q(0.5 * capacitance.0 * voltage.0.powi(2), EE_ENERGY)
        }
        "ee_inductor_energy" => {
            let inductance = ee_nonnegative(ee_require_dim(ee_q(&values[0], name)?, EE_INDUCTANCE, name)?, name, "inductance")?;
            let current = ee_require_dim(ee_q(&values[1], name)?, EE_CURRENT, name)?;
            Value::q(0.5 * inductance.0 * current.0.powi(2), EE_ENERGY)
        }
        "ee_kcl_residual" => ee_residual(ee_array(&values[0], name)?, EE_CURRENT, name),
        "ee_kvl_residual" => ee_residual(ee_array(&values[0], name)?, EE_VOLTAGE, name),
        "ee_kcl_balanced" => ee_balanced(&values[0], &values[1], EE_CURRENT, name),
        "ee_kvl_balanced" => ee_balanced(&values[0], &values[1], EE_VOLTAGE, name),
        "ee_in_unit" => {
            let value = ee_q(&values[0], name)?;
            let unit = ee_text(&values[1], name)?;
            let (dimension, factor) = ee_unit(unit).ok_or_else(|| format!("{name}() does not recognize electrical unit {unit:?}."))?;
            if value.1 != dimension {
                return Err(format!("{name}() cannot express {} as {unit} ({}).", format_dim(value.1), format_dim(dimension)));
            }
            Value::scalar(value.0 / factor)
        }
        _ => Err(format!("UNKNOWN SYMBOL: {name}")),
    }
}

fn ee_q(value: &Value, name: &str) -> Result<(f64, Dim), String> {
    match value {
        Value::Q(value, dimension) => Ok((*value, *dimension)),
        _ => Err(format!("{name}() requires numeric quantities.")),
    }
}

fn ee_text<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    match value {
        Value::Text(value) => Ok(value),
        _ => Err(format!("{name}() requires text.")),
    }
}

fn ee_array<'a>(value: &'a Value, name: &str) -> Result<&'a [Value], String> {
    match value {
        Value::Array(items) if !items.is_empty() => Ok(items),
        Value::Array(_) => Err(format!("{name}() requires a non-empty array.")),
        _ => Err(format!("{name}() requires an array.")),
    }
}

fn ee_require_dim(value: (f64, Dim), expected: Dim, name: &str) -> Result<(f64, Dim), String> {
    if value.1 != expected {
        return Err(format!("{name}() requires {}, got {}.", format_dim(expected), format_dim(value.1)));
    }
    Ok(value)
}

fn ee_nonnegative(value: (f64, Dim), name: &str, field: &str) -> Result<(f64, Dim), String> {
    if value.0 < 0.0 { return Err(format!("{name}() requires non-negative {field}.")); }
    Ok(value)
}

fn ee_positive(value: (f64, Dim), name: &str, field: &str) -> Result<(f64, Dim), String> {
    if value.0 <= 0.0 { return Err(format!("{name}() requires {field} greater than zero.")); }
    Ok(value)
}

fn ee_nonzero(value: (f64, Dim), name: &str, field: &str) -> Result<(f64, Dim), String> {
    if value.0 == 0.0 { return Err(format!("{name}() requires nonzero {field}.")); }
    Ok(value)
}

fn ee_residual(items: &[Value], expected: Dim, name: &str) -> Result<Value, String> {
    let mut total = 0.0;
    for item in items {
        total += ee_require_dim(ee_q(item, name)?, expected, name)?.0;
        Value::q(total, expected)?;
    }
    Value::q(total, expected)
}

fn ee_balanced(values: &Value, tolerance: &Value, expected: Dim, name: &str) -> Result<Value, String> {
    let tolerance = ee_nonnegative(ee_require_dim(ee_q(tolerance, name)?, expected, name)?, name, "tolerance")?;
    let Value::Q(residual, _) = ee_residual(ee_array(values, name)?, expected, name)? else { unreachable!() };
    Ok(Value::Bool(residual.abs() <= tolerance.0))
}

fn ee_unit(name: &str) -> Option<(Dim, f64)> {
    let value = match name {
        "A" => (EE_CURRENT, 1.0), "mA" => (EE_CURRENT, 1e-3), "uA" | "µA" => (EE_CURRENT, 1e-6),
        "C" => (EE_CHARGE, 1.0),
        "V" => (EE_VOLTAGE, 1.0), "mV" => (EE_VOLTAGE, 1e-3), "kV" => (EE_VOLTAGE, 1e3),
        "ohm" | "Ω" => (EE_RESISTANCE, 1.0), "kohm" | "kΩ" => (EE_RESISTANCE, 1e3), "Mohm" | "MΩ" => (EE_RESISTANCE, 1e6),
        "F" => (EE_CAPACITANCE, 1.0), "mF" => (EE_CAPACITANCE, 1e-3), "uF" | "µF" => (EE_CAPACITANCE, 1e-6), "nF" => (EE_CAPACITANCE, 1e-9), "pF" => (EE_CAPACITANCE, 1e-12),
        "H" => (EE_INDUCTANCE, 1.0), "mH" => (EE_INDUCTANCE, 1e-3), "uH" | "µH" => (EE_INDUCTANCE, 1e-6),
        "W" => (EE_POWER, 1.0), "mW" => (EE_POWER, 1e-3), "kW" => (EE_POWER, 1e3),
        "S" => (EE_CONDUCTANCE, 1.0), "mS" => (EE_CONDUCTANCE, 1e-3), "uS" | "µS" => (EE_CONDUCTANCE, 1e-6),
        "Hz" => (EE_FREQUENCY, 1.0), "kHz" => (EE_FREQUENCY, 1e3), "MHz" => (EE_FREQUENCY, 1e6),
        _ => return None,
    };
    Some(value)
}
