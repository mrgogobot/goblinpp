use crate::error::{GoblinError, Result};
use crate::evaluator::Value;
use crate::hashing::hash_canonical_json;
use crate::quantity::{
    CAPACITANCE, CHARGE, CONDUCTANCE, CURRENT, Dimension, FREQUENCY, INDUCTANCE, POWER, Quantity,
    RESISTANCE, TIME, VOLTAGE, format_dimension, unit,
};
use serde::Serialize;

pub const REGISTRY_ID: &str = "BIPM-SI-9-v3.02-Goblin-electrical-v1";
pub const REGISTRY_SOURCE: &str = "https://www.bipm.org/documents/20126/41483022/SI-Brochure-9.pdf";

pub const FUNCTIONS: &[&str] = &[
    "ee_voltage",
    "ee_current",
    "ee_resistance",
    "ee_power",
    "ee_power_i2r",
    "ee_power_v2r",
    "ee_energy",
    "ee_charge",
    "ee_series_resistance",
    "ee_parallel_resistance",
    "ee_rc_time_constant",
    "ee_capacitor_energy",
    "ee_inductor_energy",
    "ee_kcl_residual",
    "ee_kvl_residual",
    "ee_kcl_balanced",
    "ee_kvl_balanced",
    "ee_in_unit",
];

pub fn is_function(name: &str) -> bool {
    FUNCTIONS.contains(&name)
}

#[derive(Serialize)]
pub struct RegistrySnapshot {
    pub id: &'static str,
    pub source: &'static str,
    pub scope: &'static str,
    pub functions: &'static [&'static str],
}

pub fn registry_snapshot() -> RegistrySnapshot {
    RegistrySnapshot {
        id: REGISTRY_ID,
        source: REGISTRY_SOURCE,
        scope: "SI electrical dimensions, DC/passive-component helpers, explicit Kirchhoff residual checks; not an AC phasor or circuit-topology solver",
        functions: FUNCTIONS,
    }
}

pub fn registry_sha256() -> Result<String> {
    hash_canonical_json(&registry_snapshot())
}

pub fn arity(name: &str) -> Option<usize> {
    match name {
        "ee_series_resistance"
        | "ee_parallel_resistance"
        | "ee_kcl_residual"
        | "ee_kvl_residual" => Some(1),
        "ee_voltage"
        | "ee_current"
        | "ee_resistance"
        | "ee_power"
        | "ee_power_i2r"
        | "ee_power_v2r"
        | "ee_energy"
        | "ee_charge"
        | "ee_rc_time_constant"
        | "ee_capacitor_energy"
        | "ee_inductor_energy"
        | "ee_kcl_balanced"
        | "ee_kvl_balanced"
        | "ee_in_unit" => Some(2),
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
        "ee_voltage" => {
            let current = quantity(&values[0], CURRENT, name)?;
            let resistance =
                nonnegative(quantity(&values[1], RESISTANCE, name)?, name, "resistance")?;
            current.checked_mul(resistance).map(Value::Quantity)
        }
        "ee_current" => {
            let voltage = quantity(&values[0], VOLTAGE, name)?;
            let resistance = positive(quantity(&values[1], RESISTANCE, name)?, name, "resistance")?;
            voltage.checked_div(resistance).map(Value::Quantity)
        }
        "ee_resistance" => {
            let voltage = quantity(&values[0], VOLTAGE, name)?;
            let current = nonzero(quantity(&values[1], CURRENT, name)?, name, "current")?;
            let result = voltage.checked_div(current)?;
            nonnegative(result, name, "resulting resistance").map(Value::Quantity)
        }
        "ee_power" => quantity(&values[0], VOLTAGE, name)?
            .checked_mul(quantity(&values[1], CURRENT, name)?)
            .map(Value::Quantity),
        "ee_power_i2r" => {
            let current = quantity(&values[0], CURRENT, name)?;
            let resistance =
                nonnegative(quantity(&values[1], RESISTANCE, name)?, name, "resistance")?;
            current
                .powi(2)?
                .checked_mul(resistance)
                .map(Value::Quantity)
        }
        "ee_power_v2r" => {
            let voltage = quantity(&values[0], VOLTAGE, name)?;
            let resistance = positive(quantity(&values[1], RESISTANCE, name)?, name, "resistance")?;
            voltage
                .powi(2)?
                .checked_div(resistance)
                .map(Value::Quantity)
        }
        "ee_energy" => {
            let power = quantity(&values[0], POWER, name)?;
            let time = nonnegative(quantity(&values[1], TIME, name)?, name, "time")?;
            power.checked_mul(time).map(Value::Quantity)
        }
        "ee_charge" => {
            let current = quantity(&values[0], CURRENT, name)?;
            let time = nonnegative(quantity(&values[1], TIME, name)?, name, "time")?;
            current.checked_mul(time).map(Value::Quantity)
        }
        "ee_series_resistance" => resistance_sum(array(&values[0], name)?, name),
        "ee_parallel_resistance" => parallel_resistance(array(&values[0], name)?, name),
        "ee_rc_time_constant" => {
            let resistance =
                nonnegative(quantity(&values[0], RESISTANCE, name)?, name, "resistance")?;
            let capacitance = nonnegative(
                quantity(&values[1], CAPACITANCE, name)?,
                name,
                "capacitance",
            )?;
            resistance.checked_mul(capacitance).map(Value::Quantity)
        }
        "ee_capacitor_energy" => {
            let capacitance = nonnegative(
                quantity(&values[0], CAPACITANCE, name)?,
                name,
                "capacitance",
            )?;
            let voltage = quantity(&values[1], VOLTAGE, name)?;
            Quantity::scalar(0.5)?
                .checked_mul(capacitance)?
                .checked_mul(voltage.powi(2)?)
                .map(Value::Quantity)
        }
        "ee_inductor_energy" => {
            let inductance =
                nonnegative(quantity(&values[0], INDUCTANCE, name)?, name, "inductance")?;
            let current = quantity(&values[1], CURRENT, name)?;
            Quantity::scalar(0.5)?
                .checked_mul(inductance)?
                .checked_mul(current.powi(2)?)
                .map(Value::Quantity)
        }
        "ee_kcl_residual" => residual(array(&values[0], name)?, CURRENT, name),
        "ee_kvl_residual" => residual(array(&values[0], name)?, VOLTAGE, name),
        "ee_kcl_balanced" => balanced(&values[0], &values[1], CURRENT, name),
        "ee_kvl_balanced" => balanced(&values[0], &values[1], VOLTAGE, name),
        "ee_in_unit" => {
            let value = values[0].quantity(name)?;
            let spelling = values[1].text(name)?;
            let selected = unit(spelling).ok_or_else(|| {
                GoblinError::data(format!("{name}() does not recognize unit {spelling:?}."))
            })?;
            if !matches!(
                selected.dimension,
                CURRENT
                    | CHARGE
                    | VOLTAGE
                    | RESISTANCE
                    | CAPACITANCE
                    | INDUCTANCE
                    | POWER
                    | CONDUCTANCE
                    | FREQUENCY
            ) {
                return Err(GoblinError::dimension(format!(
                    "{name}() accepts electrical units only; {spelling} is {}.",
                    format_dimension(selected.dimension)
                )));
            }
            if value.dimension != selected.dimension {
                return Err(GoblinError::dimension(format!(
                    "{name}() cannot express {} as {spelling} ({}).",
                    format_dimension(value.dimension),
                    format_dimension(selected.dimension)
                )));
            }
            Quantity::scalar(value.value_si / selected.factor).map(Value::Quantity)
        }
        _ => Err(GoblinError::unknown(format!("Unknown function: {name}"))),
    }
}

fn quantity(value: &Value, expected: Dimension, name: &str) -> Result<Quantity> {
    let value = value.quantity(name)?;
    if value.dimension != expected {
        return Err(GoblinError::dimension(format!(
            "{name}() requires {}, got {}.",
            format_dimension(expected),
            format_dimension(value.dimension)
        )));
    }
    Ok(value)
}

fn array<'a>(value: &'a Value, name: &str) -> Result<&'a [Value]> {
    match value {
        Value::Array(items) if !items.is_empty() => Ok(items),
        Value::Array(_) => Err(GoblinError::data(format!(
            "{name}() requires a non-empty array."
        ))),
        _ => Err(GoblinError::data(format!("{name}() requires an array."))),
    }
}

fn nonnegative(value: Quantity, name: &str, field: &str) -> Result<Quantity> {
    if value.value_si < 0.0 {
        return Err(GoblinError::numeric(format!(
            "{name}() requires non-negative {field}."
        )));
    }
    Ok(value)
}

fn positive(value: Quantity, name: &str, field: &str) -> Result<Quantity> {
    if value.value_si <= 0.0 {
        return Err(GoblinError::numeric(format!(
            "{name}() requires {field} greater than zero."
        )));
    }
    Ok(value)
}

fn nonzero(value: Quantity, name: &str, field: &str) -> Result<Quantity> {
    if value.value_si == 0.0 {
        return Err(GoblinError::numeric(format!(
            "{name}() requires nonzero {field}."
        )));
    }
    Ok(value)
}

fn resistance_sum(items: &[Value], name: &str) -> Result<Value> {
    let mut total = Quantity::new(0.0, RESISTANCE)?;
    for item in items {
        total = total.checked_add(nonnegative(
            quantity(item, RESISTANCE, name)?,
            name,
            "resistance",
        )?)?;
    }
    Ok(Value::Quantity(total))
}

fn parallel_resistance(items: &[Value], name: &str) -> Result<Value> {
    let mut resistances = Vec::with_capacity(items.len());
    for item in items {
        let resistance = positive(quantity(item, RESISTANCE, name)?, name, "resistance")?;
        resistances.push(resistance.value_si);
    }
    // Scaling avoids overflowing 1/R for small but finite resistances.
    let minimum = resistances.iter().copied().fold(f64::INFINITY, f64::min);
    let scaled_sum = resistances.iter().map(|value| minimum / value).sum::<f64>();
    Quantity::new(minimum / scaled_sum, RESISTANCE).map(Value::Quantity)
}

fn residual(items: &[Value], expected: Dimension, name: &str) -> Result<Value> {
    let mut total = Quantity::new(0.0, expected)?;
    for item in items {
        total = total.checked_add(quantity(item, expected, name)?)?;
    }
    Ok(Value::Quantity(total))
}

fn balanced(values: &Value, tolerance: &Value, expected: Dimension, name: &str) -> Result<Value> {
    let items = array(values, name)?;
    let tolerance = nonnegative(quantity(tolerance, expected, name)?, name, "tolerance")?;
    let Value::Quantity(residual) = residual(items, expected, name)? else {
        unreachable!()
    };
    Ok(Value::Bool(residual.value_si.abs() <= tolerance.value_si))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ohms_law_and_kirchhoff_are_dimension_checked() {
        let voltage = call(
            "ee_voltage",
            vec![
                Value::Quantity(Quantity::from_unit(2.0, "A").unwrap()),
                Value::Quantity(Quantity::from_unit(3.0, "ohm").unwrap()),
            ],
        )
        .unwrap();
        assert_eq!(
            voltage,
            Value::Quantity(Quantity::from_unit(6.0, "V").unwrap())
        );

        let balanced = call(
            "ee_kcl_balanced",
            vec![
                Value::Array(vec![
                    Value::Quantity(Quantity::from_unit(3.0, "mA").unwrap()),
                    Value::Quantity(Quantity::from_unit(-3.001, "mA").unwrap()),
                ]),
                Value::Quantity(Quantity::from_unit(0.01, "mA").unwrap()),
            ],
        )
        .unwrap();
        assert_eq!(balanced, Value::Bool(true));
    }

    #[test]
    fn conversion_and_passive_domains_are_strict() {
        let converted = call(
            "ee_in_unit",
            vec![
                Value::Quantity(Quantity::from_unit(4_700.0, "ohm").unwrap()),
                Value::Text("kΩ".into()),
            ],
        )
        .unwrap();
        assert_eq!(converted, Value::Quantity(Quantity::scalar(4.7).unwrap()));
        assert!(
            call(
                "ee_parallel_resistance",
                vec![Value::Array(vec![Value::Quantity(
                    Quantity::from_unit(0.0, "ohm").unwrap()
                )])]
            )
            .is_err()
        );
    }

    #[test]
    fn declared_result_dimensions_are_not_accidental() {
        assert_eq!(
            quantity(
                &call(
                    "ee_charge",
                    vec![
                        Value::Quantity(Quantity::from_unit(2.0, "A").unwrap()),
                        Value::Quantity(Quantity::from_unit(3.0, "s").unwrap()),
                    ],
                )
                .unwrap(),
                CHARGE,
                "test",
            )
            .unwrap()
            .value_si,
            6.0
        );
        assert_eq!(
            quantity(
                &call(
                    "ee_capacitor_energy",
                    vec![
                        Value::Quantity(Quantity::from_unit(2.0, "F").unwrap()),
                        Value::Quantity(Quantity::from_unit(3.0, "V").unwrap()),
                    ],
                )
                .unwrap(),
                crate::quantity::ENERGY,
                "test",
            )
            .unwrap()
            .value_si,
            9.0
        );
    }
}
