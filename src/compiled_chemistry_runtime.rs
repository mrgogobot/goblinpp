// Included verbatim in generated Rust programs. `Value`, `Dim`, and `ZERO` are
// defined by the generated runtime before this file is inserted.
const CHEM_REGISTRY_ID: &str = "IUPAC-2021-ABRIDGED-COMMON-v1";
const CHEM_MASS: Dim = [1, 0, 0, 0, 0, 0];
const CHEM_AMOUNT: Dim = [0, 0, 0, 0, 1, 0];
const CHEM_MASS_PER_AMOUNT: Dim = [1, 0, 0, 0, -1, 0];
const CHEM_VOLUME: Dim = [0, 3, 0, 0, 0, 0];
const CHEM_CONCENTRATION: Dim = [0, -3, 0, 0, 1, 0];

fn goblin_chemistry_call(name: &str, values: Vec<Value>) -> Result<Value, String> {
    let expected = match name {
        "chem_registry_version" => 0,
        "chem_atomic_number" | "chem_atomic_weight" | "chem_molar_mass" => 1,
        "chem_moles" | "chem_mass" | "chem_concentration" => 2,
        "chem_dilution" => 3,
        _ => return Err(format!("UNKNOWN SYMBOL: {name}")),
    };
    if values.len() != expected {
        return Err(format!(
            "{name}() expects {expected} argument(s), got {}.",
            values.len()
        ));
    }
    match name {
        "chem_registry_version" => Ok(Value::Text(CHEM_REGISTRY_ID.into())),
        "chem_atomic_number" => {
            let symbol = chemistry_text(&values[0], name)?;
            Value::scalar(chemistry_element(symbol)?.0 as f64)
        }
        "chem_atomic_weight" => {
            let symbol = chemistry_text(&values[0], name)?;
            Value::scalar(chemistry_element(symbol)?.1)
        }
        "chem_molar_mass" => {
            let formula = chemistry_text(&values[0], name)?;
            Value::q(chemistry_formula_mass(formula)? * 1e-3, CHEM_MASS_PER_AMOUNT)
        }
        "chem_moles" => {
            let mass = chemistry_require_dim(chemistry_q(&values[0], name)?, CHEM_MASS, name)?;
            let molar = chemistry_require_dim(
                chemistry_q(&values[1], name)?,
                CHEM_MASS_PER_AMOUNT,
                name,
            )?;
            chemistry_nonnegative(mass.0, name, "mass")?;
            chemistry_positive(molar.0, name, "molar mass")?;
            Value::q(mass.0 / molar.0, CHEM_AMOUNT)
        }
        "chem_mass" => {
            let amount = chemistry_require_dim(
                chemistry_q(&values[0], name)?,
                CHEM_AMOUNT,
                name,
            )?;
            let molar = chemistry_require_dim(
                chemistry_q(&values[1], name)?,
                CHEM_MASS_PER_AMOUNT,
                name,
            )?;
            chemistry_nonnegative(amount.0, name, "amount")?;
            chemistry_positive(molar.0, name, "molar mass")?;
            Value::q(amount.0 * molar.0, CHEM_MASS)
        }
        "chem_concentration" => {
            let amount = chemistry_require_dim(
                chemistry_q(&values[0], name)?,
                CHEM_AMOUNT,
                name,
            )?;
            let volume = chemistry_require_dim(
                chemistry_q(&values[1], name)?,
                CHEM_VOLUME,
                name,
            )?;
            chemistry_nonnegative(amount.0, name, "amount")?;
            chemistry_positive(volume.0, name, "volume")?;
            Value::q(amount.0 / volume.0, CHEM_CONCENTRATION)
        }
        "chem_dilution" => {
            let concentration = chemistry_require_dim(
                chemistry_q(&values[0], name)?,
                CHEM_CONCENTRATION,
                name,
            )?;
            let initial = chemistry_require_dim(
                chemistry_q(&values[1], name)?,
                CHEM_VOLUME,
                name,
            )?;
            let final_volume = chemistry_require_dim(
                chemistry_q(&values[2], name)?,
                CHEM_VOLUME,
                name,
            )?;
            chemistry_nonnegative(concentration.0, name, "concentration")?;
            chemistry_positive(initial.0, name, "initial volume")?;
            chemistry_positive(final_volume.0, name, "final volume")?;
            if final_volume.0 < initial.0 {
                return Err(
                    "chem_dilution() requires final volume greater than or equal to initial volume."
                        .into(),
                );
            }
            Value::q(
                concentration.0 * initial.0 / final_volume.0,
                CHEM_CONCENTRATION,
            )
        }
        _ => Err(format!("UNKNOWN SYMBOL: {name}")),
    }
}

fn chemistry_q(value: &Value, name: &str) -> Result<(f64, Dim), String> {
    match value {
        Value::Q(value, dimension) => Ok((*value, *dimension)),
        _ => Err(format!("{name}() requires numeric quantities.")),
    }
}

fn chemistry_text<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    match value {
        Value::Text(value) => Ok(value),
        _ => Err(format!("{name}() requires text.")),
    }
}

fn chemistry_require_dim(
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

fn chemistry_nonnegative(value: f64, name: &str, field: &str) -> Result<(), String> {
    if value < 0.0 {
        return Err(format!("{name}() requires non-negative {field}."));
    }
    Ok(())
}

fn chemistry_positive(value: f64, name: &str, field: &str) -> Result<(), String> {
    if value <= 0.0 {
        return Err(format!("{name}() requires {field} greater than zero."));
    }
    Ok(())
}

fn chemistry_element(symbol: &str) -> Result<(u8, f64), String> {
    let value = match symbol {
        "H" => (1, 1.0080), "He" => (2, 4.0026), "Li" => (3, 6.94),
        "Be" => (4, 9.0122), "B" => (5, 10.81), "C" => (6, 12.011),
        "N" => (7, 14.007), "O" => (8, 15.999), "F" => (9, 18.998),
        "Ne" => (10, 20.180), "Na" => (11, 22.990), "Mg" => (12, 24.305),
        "Al" => (13, 26.982), "Si" => (14, 28.085), "P" => (15, 30.974),
        "S" => (16, 32.06), "Cl" => (17, 35.45), "Ar" => (18, 39.95),
        "K" => (19, 39.098), "Ca" => (20, 40.078), "Sc" => (21, 44.956),
        "Ti" => (22, 47.867), "V" => (23, 50.942), "Cr" => (24, 51.996),
        "Mn" => (25, 54.938), "Fe" => (26, 55.845), "Co" => (27, 58.933),
        "Ni" => (28, 58.693), "Cu" => (29, 63.546), "Zn" => (30, 65.38),
        "Se" => (34, 78.971), "Br" => (35, 79.904), "Mo" => (42, 95.95),
        "Ag" => (47, 107.8682), "Cd" => (48, 112.414), "I" => (53, 126.90447),
        "Xe" => (54, 131.293), "Ba" => (56, 137.327), "Pt" => (78, 195.084),
        "Au" => (79, 196.96657), "Hg" => (80, 200.592), "Pb" => (82, 207.2),
        "U" => (92, 238.02891),
        _ => return Err(format!(
            "CHEMISTRY ELEMENT NOT AVAILABLE: {symbol:?} is not an exact element symbol in {CHEM_REGISTRY_ID}."
        )),
    };
    Ok(value)
}

fn chemistry_formula_mass(formula: &str) -> Result<f64, String> {
    if formula.is_empty() || formula.len() > 1_024 {
        return Err("UNSUPPORTED CHEMICAL FORMULA: formula must contain 1 through 1024 bytes.".into());
    }
    let chars = formula.chars().collect::<Vec<_>>();
    let mut cursor = 0usize;
    let value = chemistry_formula_sequence(&chars, &mut cursor, None, 0)?;
    if cursor != chars.len() || !value.is_finite() || value <= 0.0 {
        return Err("UNSUPPORTED CHEMICAL FORMULA: invalid trailing content or mass.".into());
    }
    Ok(value)
}

fn chemistry_formula_sequence(
    chars: &[char],
    cursor: &mut usize,
    stop: Option<char>,
    depth: usize,
) -> Result<f64, String> {
    if depth > 8 {
        return Err("UNSUPPORTED CHEMICAL FORMULA: nesting exceeds eight groups.".into());
    }
    let start = *cursor;
    let mut total = 0.0;
    while *cursor < chars.len() {
        let current = chars[*cursor];
        if Some(current) == stop {
            break;
        }
        let mass = if current == '(' {
            *cursor += 1;
            let group = chemistry_formula_sequence(chars, cursor, Some(')'), depth + 1)?;
            if chars.get(*cursor) != Some(&')') {
                return Err("UNSUPPORTED CHEMICAL FORMULA: missing closing parenthesis.".into());
            }
            *cursor += 1;
            group
        } else if current.is_ascii_uppercase() {
            let mut symbol = String::from(current);
            *cursor += 1;
            if chars.get(*cursor).is_some_and(|next| next.is_ascii_lowercase()) {
                symbol.push(chars[*cursor]);
                *cursor += 1;
            }
            chemistry_element(&symbol)?.1
        } else {
            return Err(format!(
                "UNSUPPORTED CHEMICAL FORMULA: character {current:?} at position {}.",
                *cursor + 1
            ));
        };
        let count_start = *cursor;
        while chars.get(*cursor).is_some_and(|ch| ch.is_ascii_digit()) {
            *cursor += 1;
        }
        let count = if count_start == *cursor {
            1u32
        } else {
            let text = chars[count_start..*cursor].iter().collect::<String>();
            text.parse::<u32>()
                .map_err(|_| "UNSUPPORTED CHEMICAL FORMULA: count is too large.".to_string())?
        };
        if count == 0 || count > 1_000_000 {
            return Err("UNSUPPORTED CHEMICAL FORMULA: count outside 1 through 1,000,000.".into());
        }
        total += mass * count as f64;
        if !total.is_finite() {
            return Err("UNSUPPORTED CHEMICAL FORMULA: molar mass overflowed.".into());
        }
    }
    if *cursor == start {
        return Err("UNSUPPORTED CHEMICAL FORMULA: empty group.".into());
    }
    Ok(total)
}
