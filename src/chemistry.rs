use crate::error::{GoblinError, Result};
use crate::evaluator::Value;
use crate::hashing::hash_canonical_json;
use crate::quantity::{AMOUNT, Dimension, MASS, Quantity, format_dimension};
use serde::Serialize;

pub const REGISTRY_ID: &str = "IUPAC-2021-ABRIDGED-COMMON-v1";
pub const REGISTRY_SOURCE: &str = "https://iupac.org/what-we-do/periodic-table-of-elements/";
pub const MASS_PER_AMOUNT: Dimension = [1, 0, 0, 0, -1, 0];
pub const VOLUME: Dimension = [0, 3, 0, 0, 0, 0];
pub const CONCENTRATION: Dimension = [0, -3, 0, 0, 1, 0];

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Element {
    pub atomic_number: u8,
    pub symbol: &'static str,
    pub name: &'static str,
    pub abridged_standard_atomic_weight: f64,
}

// Deliberately scoped to common laboratory and biological formulae. Values are
// the abridged standard atomic weights presented by IUPAC's 2021 table. This is
// not an isotope registry and must not be used when sample isotopic composition
// is material to the result.
pub const ELEMENTS: &[Element] = &[
    Element {
        atomic_number: 1,
        symbol: "H",
        name: "hydrogen",
        abridged_standard_atomic_weight: 1.0080,
    },
    Element {
        atomic_number: 2,
        symbol: "He",
        name: "helium",
        abridged_standard_atomic_weight: 4.0026,
    },
    Element {
        atomic_number: 3,
        symbol: "Li",
        name: "lithium",
        abridged_standard_atomic_weight: 6.94,
    },
    Element {
        atomic_number: 4,
        symbol: "Be",
        name: "beryllium",
        abridged_standard_atomic_weight: 9.0122,
    },
    Element {
        atomic_number: 5,
        symbol: "B",
        name: "boron",
        abridged_standard_atomic_weight: 10.81,
    },
    Element {
        atomic_number: 6,
        symbol: "C",
        name: "carbon",
        abridged_standard_atomic_weight: 12.011,
    },
    Element {
        atomic_number: 7,
        symbol: "N",
        name: "nitrogen",
        abridged_standard_atomic_weight: 14.007,
    },
    Element {
        atomic_number: 8,
        symbol: "O",
        name: "oxygen",
        abridged_standard_atomic_weight: 15.999,
    },
    Element {
        atomic_number: 9,
        symbol: "F",
        name: "fluorine",
        abridged_standard_atomic_weight: 18.998,
    },
    Element {
        atomic_number: 10,
        symbol: "Ne",
        name: "neon",
        abridged_standard_atomic_weight: 20.180,
    },
    Element {
        atomic_number: 11,
        symbol: "Na",
        name: "sodium",
        abridged_standard_atomic_weight: 22.990,
    },
    Element {
        atomic_number: 12,
        symbol: "Mg",
        name: "magnesium",
        abridged_standard_atomic_weight: 24.305,
    },
    Element {
        atomic_number: 13,
        symbol: "Al",
        name: "aluminium",
        abridged_standard_atomic_weight: 26.982,
    },
    Element {
        atomic_number: 14,
        symbol: "Si",
        name: "silicon",
        abridged_standard_atomic_weight: 28.085,
    },
    Element {
        atomic_number: 15,
        symbol: "P",
        name: "phosphorus",
        abridged_standard_atomic_weight: 30.974,
    },
    Element {
        atomic_number: 16,
        symbol: "S",
        name: "sulfur",
        abridged_standard_atomic_weight: 32.06,
    },
    Element {
        atomic_number: 17,
        symbol: "Cl",
        name: "chlorine",
        abridged_standard_atomic_weight: 35.45,
    },
    Element {
        atomic_number: 18,
        symbol: "Ar",
        name: "argon",
        abridged_standard_atomic_weight: 39.95,
    },
    Element {
        atomic_number: 19,
        symbol: "K",
        name: "potassium",
        abridged_standard_atomic_weight: 39.098,
    },
    Element {
        atomic_number: 20,
        symbol: "Ca",
        name: "calcium",
        abridged_standard_atomic_weight: 40.078,
    },
    Element {
        atomic_number: 21,
        symbol: "Sc",
        name: "scandium",
        abridged_standard_atomic_weight: 44.956,
    },
    Element {
        atomic_number: 22,
        symbol: "Ti",
        name: "titanium",
        abridged_standard_atomic_weight: 47.867,
    },
    Element {
        atomic_number: 23,
        symbol: "V",
        name: "vanadium",
        abridged_standard_atomic_weight: 50.942,
    },
    Element {
        atomic_number: 24,
        symbol: "Cr",
        name: "chromium",
        abridged_standard_atomic_weight: 51.996,
    },
    Element {
        atomic_number: 25,
        symbol: "Mn",
        name: "manganese",
        abridged_standard_atomic_weight: 54.938,
    },
    Element {
        atomic_number: 26,
        symbol: "Fe",
        name: "iron",
        abridged_standard_atomic_weight: 55.845,
    },
    Element {
        atomic_number: 27,
        symbol: "Co",
        name: "cobalt",
        abridged_standard_atomic_weight: 58.933,
    },
    Element {
        atomic_number: 28,
        symbol: "Ni",
        name: "nickel",
        abridged_standard_atomic_weight: 58.693,
    },
    Element {
        atomic_number: 29,
        symbol: "Cu",
        name: "copper",
        abridged_standard_atomic_weight: 63.546,
    },
    Element {
        atomic_number: 30,
        symbol: "Zn",
        name: "zinc",
        abridged_standard_atomic_weight: 65.38,
    },
    Element {
        atomic_number: 34,
        symbol: "Se",
        name: "selenium",
        abridged_standard_atomic_weight: 78.971,
    },
    Element {
        atomic_number: 35,
        symbol: "Br",
        name: "bromine",
        abridged_standard_atomic_weight: 79.904,
    },
    Element {
        atomic_number: 42,
        symbol: "Mo",
        name: "molybdenum",
        abridged_standard_atomic_weight: 95.95,
    },
    Element {
        atomic_number: 47,
        symbol: "Ag",
        name: "silver",
        abridged_standard_atomic_weight: 107.8682,
    },
    Element {
        atomic_number: 48,
        symbol: "Cd",
        name: "cadmium",
        abridged_standard_atomic_weight: 112.414,
    },
    Element {
        atomic_number: 53,
        symbol: "I",
        name: "iodine",
        abridged_standard_atomic_weight: 126.90447,
    },
    Element {
        atomic_number: 54,
        symbol: "Xe",
        name: "xenon",
        abridged_standard_atomic_weight: 131.293,
    },
    Element {
        atomic_number: 56,
        symbol: "Ba",
        name: "barium",
        abridged_standard_atomic_weight: 137.327,
    },
    Element {
        atomic_number: 78,
        symbol: "Pt",
        name: "platinum",
        abridged_standard_atomic_weight: 195.084,
    },
    Element {
        atomic_number: 79,
        symbol: "Au",
        name: "gold",
        abridged_standard_atomic_weight: 196.96657,
    },
    Element {
        atomic_number: 80,
        symbol: "Hg",
        name: "mercury",
        abridged_standard_atomic_weight: 200.592,
    },
    Element {
        atomic_number: 82,
        symbol: "Pb",
        name: "lead",
        abridged_standard_atomic_weight: 207.2,
    },
    Element {
        atomic_number: 92,
        symbol: "U",
        name: "uranium",
        abridged_standard_atomic_weight: 238.02891,
    },
];

pub const FUNCTIONS: &[&str] = &[
    "chem_registry_version",
    "chem_atomic_number",
    "chem_atomic_weight",
    "chem_molar_mass",
    "chem_moles",
    "chem_mass",
    "chem_concentration",
    "chem_dilution",
];

pub fn is_function(name: &str) -> bool {
    FUNCTIONS.contains(&name)
}

pub fn arity(name: &str) -> Option<usize> {
    match name {
        "chem_registry_version" => Some(0),
        "chem_atomic_number" | "chem_atomic_weight" | "chem_molar_mass" => Some(1),
        "chem_moles" | "chem_mass" | "chem_concentration" => Some(2),
        "chem_dilution" => Some(3),
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
        "chem_registry_version" => Ok(Value::Text(REGISTRY_ID.into())),
        "chem_atomic_number" => {
            let element = element(values[0].text(name)?)?;
            Quantity::scalar(element.atomic_number as f64).map(Value::Quantity)
        }
        "chem_atomic_weight" => {
            let element = element(values[0].text(name)?)?;
            Quantity::scalar(element.abridged_standard_atomic_weight).map(Value::Quantity)
        }
        "chem_molar_mass" => {
            let grams_per_mol = molar_mass_g_per_mol(values[0].text(name)?)?;
            Quantity::new(grams_per_mol * 1e-3, MASS_PER_AMOUNT).map(Value::Quantity)
        }
        "chem_moles" => {
            let mass = require_dimension(values[0].quantity(name)?, MASS, name)?;
            let molar_mass = require_dimension(values[1].quantity(name)?, MASS_PER_AMOUNT, name)?;
            require_nonnegative(mass, name, "mass")?;
            require_positive(molar_mass, name, "molar mass")?;
            mass.checked_div(molar_mass).map(Value::Quantity)
        }
        "chem_mass" => {
            let amount = require_dimension(values[0].quantity(name)?, AMOUNT, name)?;
            let molar_mass = require_dimension(values[1].quantity(name)?, MASS_PER_AMOUNT, name)?;
            require_nonnegative(amount, name, "amount")?;
            require_positive(molar_mass, name, "molar mass")?;
            amount.checked_mul(molar_mass).map(Value::Quantity)
        }
        "chem_concentration" => {
            let amount = require_dimension(values[0].quantity(name)?, AMOUNT, name)?;
            let volume = require_dimension(values[1].quantity(name)?, VOLUME, name)?;
            require_nonnegative(amount, name, "amount")?;
            require_positive(volume, name, "volume")?;
            amount.checked_div(volume).map(Value::Quantity)
        }
        "chem_dilution" => {
            let concentration = require_dimension(values[0].quantity(name)?, CONCENTRATION, name)?;
            let initial = require_dimension(values[1].quantity(name)?, VOLUME, name)?;
            let final_volume = require_dimension(values[2].quantity(name)?, VOLUME, name)?;
            require_nonnegative(concentration, name, "concentration")?;
            require_positive(initial, name, "initial volume")?;
            require_positive(final_volume, name, "final volume")?;
            if final_volume.value_si < initial.value_si {
                return Err(GoblinError::numeric(
                    "chem_dilution() requires final volume greater than or equal to initial volume.",
                ));
            }
            concentration
                .checked_mul(initial)?
                .checked_div(final_volume)
                .map(Value::Quantity)
        }
        _ => Err(GoblinError::unknown(format!("Unknown function: {name}"))),
    }
}

pub fn element(symbol: &str) -> Result<&'static Element> {
    ELEMENTS.iter().find(|item| item.symbol == symbol).ok_or_else(|| {
        GoblinError::data(format!(
            "CHEMISTRY ELEMENT NOT AVAILABLE\n\n{symbol:?} is not an exact element symbol in {REGISTRY_ID}.\n\nThe alpha registry is intentionally scoped; element symbols are case-sensitive."
        ))
    })
}

pub fn molar_mass_g_per_mol(formula: &str) -> Result<f64> {
    if formula.is_empty() {
        return Err(formula_error("Chemical formula cannot be empty."));
    }
    if formula.len() > 1_024 {
        return Err(formula_error("Chemical formula exceeds 1024 bytes."));
    }
    let chars = formula.chars().collect::<Vec<_>>();
    let mut parser = FormulaParser {
        chars: &chars,
        cursor: 0,
        depth: 0,
    };
    let value = parser.sequence(None)?;
    if parser.cursor != chars.len() {
        return Err(formula_error("Unexpected trailing formula content."));
    }
    if !value.is_finite() || value <= 0.0 {
        return Err(formula_error("Formula produced an invalid molar mass."));
    }
    Ok(value)
}

#[derive(Serialize)]
pub struct RegistrySnapshot {
    pub id: &'static str,
    pub source: &'static str,
    pub scope: &'static str,
    pub element_count: usize,
    pub elements: &'static [Element],
}

pub fn registry_snapshot() -> RegistrySnapshot {
    RegistrySnapshot {
        id: REGISTRY_ID,
        source: REGISTRY_SOURCE,
        scope: "abridged standard atomic weights for explicitly listed common elements; not isotope-specific",
        element_count: ELEMENTS.len(),
        elements: ELEMENTS,
    }
}

pub fn registry_sha256() -> Result<String> {
    hash_canonical_json(&registry_snapshot())
}

struct FormulaParser<'a> {
    chars: &'a [char],
    cursor: usize,
    depth: usize,
}

impl FormulaParser<'_> {
    fn sequence(&mut self, stop: Option<char>) -> Result<f64> {
        if self.depth > 8 {
            return Err(formula_error("Formula nesting exceeds eight groups."));
        }
        let start = self.cursor;
        let mut total = 0.0;
        while self.cursor < self.chars.len() {
            let current = self.chars[self.cursor];
            if Some(current) == stop {
                break;
            }
            let mass = if current == '(' {
                self.cursor += 1;
                self.depth += 1;
                let group = self.sequence(Some(')'))?;
                self.depth -= 1;
                if self.chars.get(self.cursor) != Some(&')') {
                    return Err(formula_error(
                        "Formula group is missing a closing parenthesis.",
                    ));
                }
                self.cursor += 1;
                group
            } else if current.is_ascii_uppercase() {
                let mut symbol = String::from(current);
                self.cursor += 1;
                if self
                    .chars
                    .get(self.cursor)
                    .is_some_and(|next| next.is_ascii_lowercase())
                {
                    symbol.push(self.chars[self.cursor]);
                    self.cursor += 1;
                }
                element(&symbol)?.abridged_standard_atomic_weight
            } else {
                return Err(formula_error(format!(
                    "Unsupported formula character {current:?} at position {}.",
                    self.cursor + 1
                )));
            };
            let count = self.count()?;
            total += mass * count as f64;
            if !total.is_finite() {
                return Err(formula_error("Formula molar mass overflowed."));
            }
        }
        if self.cursor == start {
            return Err(formula_error("Formula group cannot be empty."));
        }
        Ok(total)
    }

    fn count(&mut self) -> Result<u32> {
        let start = self.cursor;
        while self
            .chars
            .get(self.cursor)
            .is_some_and(|ch| ch.is_ascii_digit())
        {
            self.cursor += 1;
        }
        if start == self.cursor {
            return Ok(1);
        }
        let text = self.chars[start..self.cursor].iter().collect::<String>();
        let count = text
            .parse::<u32>()
            .map_err(|_| formula_error("Formula count is too large."))?;
        if count == 0 || count > 1_000_000 {
            return Err(formula_error(
                "Formula counts must be between 1 and 1,000,000.",
            ));
        }
        Ok(count)
    }
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
            "{name}() requires non-negative {field}."
        )));
    }
    Ok(())
}

fn require_positive(value: Quantity, name: &str, field: &str) -> Result<()> {
    if value.value_si <= 0.0 {
        return Err(GoblinError::numeric(format!(
            "{name}() requires {field} greater than zero."
        )));
    }
    Ok(())
}

fn formula_error(message: impl Into<String>) -> GoblinError {
    GoblinError::data(format!(
        "UNSUPPORTED CHEMICAL FORMULA\n\n{}\n\nSupported syntax: exact element symbols, positive integer counts, and parentheses. Charges, isotopes, brackets, and hydrate dots are not inferred.",
        message.into()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_formulae_and_groups() {
        assert!((molar_mass_g_per_mol("H2O").unwrap() - 18.015).abs() < 1e-12);
        assert!((molar_mass_g_per_mol("C6H12O6").unwrap() - 180.156).abs() < 1e-12);
        assert!((molar_mass_g_per_mol("Ca(OH)2").unwrap() - 74.092).abs() < 1e-12);
    }

    #[test]
    fn refuses_ambiguous_or_unsupported_formula_syntax() {
        for formula in ["", "h2O", "Na+", "CuSO4.5H2O", "Mg[OH]2", "Xx2"] {
            assert!(molar_mass_g_per_mol(formula).is_err(), "accepted {formula}");
        }
    }
}
