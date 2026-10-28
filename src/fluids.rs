use crate::error::{GoblinError, Result};
use crate::evaluator::Value;
pub use crate::fluid_runtime::FUNCTIONS;
use crate::quantity::Quantity;

pub fn is_function(name: &str) -> bool {
    FUNCTIONS.contains(&name)
}
pub fn require_arity(name: &str, count: usize) -> Result<()> {
    let expected = crate::fluid_runtime::arity(name)
        .ok_or_else(|| GoblinError::unknown(format!("Unknown fluid function: {name}")))?;
    if count != expected {
        return Err(GoblinError::parse(format!(
            "{name}() expects {expected} arguments, got {count}."
        )));
    }
    Ok(())
}
pub fn call(name: &str, values: &[Value]) -> Result<Value> {
    require_arity(name, values.len())?;
    let quantities = values
        .iter()
        .map(|v| match v {
            Value::Quantity(q) => Ok((q.value_si, q.dimension)),
            _ => Err(GoblinError::new(
                "G203",
                format!("{name}() requires scalar numeric quantities."),
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let (value, dim) = crate::fluid_runtime::call(name, &quantities).map_err(|e| {
        if e.dimension {
            GoblinError::dimension(e.message)
        } else {
            GoblinError::numeric(e.message)
        }
    })?;
    Quantity::new(value, dim).map(Value::Quantity)
}
