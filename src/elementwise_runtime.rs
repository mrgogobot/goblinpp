//! Shared, allocation-bounded lane mapping; also embedded in standalone Rust.
//! Scalars repeat only when an actual array operand is present. No reshaping,
//! truncation, nested arrays, filtering, reduction or implicit matrix product.

pub const MAX_ITEMS: usize = 100_000;

pub enum Operand<'a, T> {
    Scalar(&'a T),
    Array(&'a [T]),
}

pub fn is_function(name: &str) -> bool {
    matches!(
        name,
        "abs"
            | "sqrt"
            | "min"
            | "max"
            | "floor"
            | "ceil"
            | "round"
            | "exp"
            | "ln"
            | "log10"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "atan2"
            | "sind"
            | "cosd"
            | "tand"
            | "sinr"
            | "cosr"
            | "tanr"
            | "asind"
            | "acosd"
            | "atand"
            | "asinr"
            | "acosr"
            | "atanr"
            | "atan2d"
            | "atan2r"
            | "deg2rad"
            | "rad2deg"
            | "hypot"
    )
}

// Shape is checked before any scalar computation. Type/unit validation is also
// done for every input, even with empty arrays. Domain checks run per actual lane.
pub fn map<T: Clone, E>(
    operation: &str,
    inputs: &[Operand<'_, T>],
    numeric: impl Fn(&T) -> bool,
    compatible: impl Fn(&T, &T) -> bool,
    mut scalar: impl FnMut(Vec<T>) -> Result<T, E>,
    failure: impl Fn(String) -> E,
    annotate: impl Fn(E, usize) -> E,
) -> Result<Vec<T>, E> {
    let mut length = None;
    for (argument, input) in inputs.iter().enumerate() {
        if let Operand::Array(items) = input {
            if items.len() > MAX_ITEMS {
                return Err(failure(format!(
                    "{operation}: argument {} exceeds the {MAX_ITEMS}-item array limit.",
                    argument + 1
                )));
            }
            if let Some(expected) = length {
                if items.len() != expected {
                    return Err(failure(format!(
                        "{operation}: array length mismatch: expected {expected}, argument {} has {}. No truncation or length-one array broadcasting is allowed.",
                        argument + 1,
                        items.len()
                    )));
                }
            } else {
                length = Some(items.len());
            }
        }
    }
    let length = length.ok_or_else(|| failure(format!("{operation}: array operand required.")))?;
    for (argument, input) in inputs.iter().enumerate() {
        match input {
            Operand::Scalar(value) if !numeric(value) => {
                return Err(failure(format!(
                    "{operation}: argument {} must be a finite numeric quantity, not text, Boolean or a nested array.",
                    argument + 1
                )));
            }
            Operand::Array(items) => {
                for (index, value) in items.iter().enumerate() {
                    if !numeric(value) {
                        return Err(failure(format!(
                            "{operation}: argument {}, element index {index} (zero-based) must be a finite numeric quantity.",
                            argument + 1
                        )));
                    }
                    if !compatible(&items[0], value) {
                        return Err(failure(format!(
                            "{operation}: argument {}, element index {index} (zero-based) has incompatible dimensions within the input array.",
                            argument + 1
                        )));
                    }
                }
            }
            _ => {}
        }
    }
    let mut result = Vec::with_capacity(length);
    for index in 0..length {
        let args = inputs
            .iter()
            .map(|input| match input {
                Operand::Scalar(value) => (*value).clone(),
                Operand::Array(items) => items[index].clone(),
            })
            .collect();
        let value = scalar(args).map_err(|error| annotate(error, index))?;
        if !numeric(&value) {
            return Err(failure(format!(
                "{operation}: element index {index} (zero-based) produced a non-finite or non-numeric result."
            )));
        }
        if result
            .first()
            .is_some_and(|first| !compatible(first, &value))
        {
            return Err(failure(format!(
                "{operation}: element index {index} (zero-based) produces a different result dimension; homogeneous arrays cannot store varying unit exponents."
            )));
        }
        result.push(value);
    }
    Ok(result)
}
