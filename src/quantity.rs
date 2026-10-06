use crate::error::{GoblinError, Result};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

// SI base-dimension order: mass, length, time, temperature, amount, current.
// Current was appended in alpha.17 so the first five positions remain stable
// for historical Goblin++ evidence.
pub type Dimension = [i32; 6];
pub const DIMENSIONLESS: Dimension = [0, 0, 0, 0, 0, 0];
pub const MASS: Dimension = [1, 0, 0, 0, 0, 0];
pub const LENGTH: Dimension = [0, 1, 0, 0, 0, 0];
pub const TIME: Dimension = [0, 0, 1, 0, 0, 0];
pub const TEMPERATURE: Dimension = [0, 0, 0, 1, 0, 0];
pub const AMOUNT: Dimension = [0, 0, 0, 0, 1, 0];
pub const CURRENT: Dimension = [0, 0, 0, 0, 0, 1];
pub const ENERGY: Dimension = [1, 2, -2, 0, 0, 0];
pub const POWER: Dimension = [1, 2, -3, 0, 0, 0];
pub const CHARGE: Dimension = [0, 0, 1, 0, 0, 1];
pub const VOLTAGE: Dimension = [1, 2, -3, 0, 0, -1];
pub const RESISTANCE: Dimension = [1, 2, -3, 0, 0, -2];
pub const CONDUCTANCE: Dimension = [-1, -2, 3, 0, 0, 2];
pub const CAPACITANCE: Dimension = [-1, -2, 4, 0, 0, 2];
pub const INDUCTANCE: Dimension = [1, 2, -2, 0, 0, -2];
pub const FREQUENCY: Dimension = [0, 0, -1, 0, 0, 0];

#[derive(Debug, Clone, Copy)]
pub struct Unit {
    pub name: &'static str,
    pub dimension: Dimension,
    pub factor: f64,
    pub status: &'static str,
    pub registry: &'static str,
}

pub const UNITS: &[Unit] = &[
    Unit {
        name: "kg",
        dimension: MASS,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "g",
        dimension: MASS,
        factor: 1e-3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "m",
        dimension: LENGTH,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "km",
        dimension: LENGTH,
        factor: 1e3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "s",
        dimension: TIME,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "K",
        dimension: TEMPERATURE,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "mol",
        dimension: AMOUNT,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "J",
        dimension: ENERGY,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "L",
        dimension: [0, 3, 0, 0, 0, 0],
        factor: 1e-3,
        status: "exact",
        registry: "SI-accepted-unit",
    },
    Unit {
        name: "mL",
        dimension: [0, 3, 0, 0, 0, 0],
        factor: 1e-6,
        status: "exact",
        registry: "SI-accepted-unit",
    },
    Unit {
        name: "uL",
        dimension: [0, 3, 0, 0, 0, 0],
        factor: 1e-9,
        status: "ASCII-alias-for-microlitre; exact",
        registry: "Goblin++-chemistry-v1",
    },
    Unit {
        name: "µL",
        dimension: [0, 3, 0, 0, 0, 0],
        factor: 1e-9,
        status: "exact",
        registry: "SI-accepted-unit",
    },
    Unit {
        name: "nm",
        dimension: LENGTH,
        factor: 1e-9,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "pm",
        dimension: LENGTH,
        factor: 1e-12,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "angstrom",
        dimension: LENGTH,
        factor: 1e-10,
        status: "exact",
        registry: "SI-accepted-unit",
    },
    Unit {
        name: "Å",
        dimension: LENGTH,
        factor: 1e-10,
        status: "alias-for-angstrom; exact",
        registry: "SI-accepted-unit",
    },
    Unit {
        name: "Pa",
        dimension: [1, -1, -2, 0, 0, 0],
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "kPa",
        dimension: [1, -1, -2, 0, 0, 0],
        factor: 1e3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "bar",
        dimension: [1, -1, -2, 0, 0, 0],
        factor: 1e5,
        status: "exact",
        registry: "SI-accepted-unit",
    },
    Unit {
        name: "atm",
        dimension: [1, -1, -2, 0, 0, 0],
        factor: 101_325.0,
        status: "standard-atmosphere; exact",
        registry: "SI-accepted-unit",
    },
    Unit {
        name: "Da",
        dimension: MASS,
        factor: 1.660_539_068_92e-27,
        status: "measured; CODATA-2022",
        registry: "CODATA-2022",
    },
    Unit {
        name: "A",
        dimension: CURRENT,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "mA",
        dimension: CURRENT,
        factor: 1e-3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "uA",
        dimension: CURRENT,
        factor: 1e-6,
        status: "ASCII-alias-for-microampere; exact",
        registry: "Goblin++-electrical-v1",
    },
    Unit {
        name: "µA",
        dimension: CURRENT,
        factor: 1e-6,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "C",
        dimension: CHARGE,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "V",
        dimension: VOLTAGE,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "mV",
        dimension: VOLTAGE,
        factor: 1e-3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "kV",
        dimension: VOLTAGE,
        factor: 1e3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "ohm",
        dimension: RESISTANCE,
        factor: 1.0,
        status: "ASCII-alias-for-ohm; exact",
        registry: "Goblin++-electrical-v1",
    },
    Unit {
        name: "Ω",
        dimension: RESISTANCE,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "kohm",
        dimension: RESISTANCE,
        factor: 1e3,
        status: "ASCII-alias-for-kiloohm; exact",
        registry: "Goblin++-electrical-v1",
    },
    Unit {
        name: "kΩ",
        dimension: RESISTANCE,
        factor: 1e3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "Mohm",
        dimension: RESISTANCE,
        factor: 1e6,
        status: "ASCII-alias-for-megaohm; exact",
        registry: "Goblin++-electrical-v1",
    },
    Unit {
        name: "MΩ",
        dimension: RESISTANCE,
        factor: 1e6,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "F",
        dimension: CAPACITANCE,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "mF",
        dimension: CAPACITANCE,
        factor: 1e-3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "uF",
        dimension: CAPACITANCE,
        factor: 1e-6,
        status: "ASCII-alias-for-microfarad; exact",
        registry: "Goblin++-electrical-v1",
    },
    Unit {
        name: "µF",
        dimension: CAPACITANCE,
        factor: 1e-6,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "nF",
        dimension: CAPACITANCE,
        factor: 1e-9,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "pF",
        dimension: CAPACITANCE,
        factor: 1e-12,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "H",
        dimension: INDUCTANCE,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "mH",
        dimension: INDUCTANCE,
        factor: 1e-3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "uH",
        dimension: INDUCTANCE,
        factor: 1e-6,
        status: "ASCII-alias-for-microhenry; exact",
        registry: "Goblin++-electrical-v1",
    },
    Unit {
        name: "µH",
        dimension: INDUCTANCE,
        factor: 1e-6,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "W",
        dimension: POWER,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "mW",
        dimension: POWER,
        factor: 1e-3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "kW",
        dimension: POWER,
        factor: 1e3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "S",
        dimension: CONDUCTANCE,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "mS",
        dimension: CONDUCTANCE,
        factor: 1e-3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "uS",
        dimension: CONDUCTANCE,
        factor: 1e-6,
        status: "ASCII-alias-for-microsiemens; exact",
        registry: "Goblin++-electrical-v1",
    },
    Unit {
        name: "µS",
        dimension: CONDUCTANCE,
        factor: 1e-6,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "Hz",
        dimension: FREQUENCY,
        factor: 1.0,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "kHz",
        dimension: FREQUENCY,
        factor: 1e3,
        status: "exact",
        registry: "SI-2019",
    },
    Unit {
        name: "MHz",
        dimension: FREQUENCY,
        factor: 1e6,
        status: "exact",
        registry: "SI-2019",
    },
];

#[derive(Debug, Serialize)]
pub struct UnitSnapshot<'a> {
    pub name: &'a str,
    pub dimension: Dimension,
    pub factor: f64,
    pub status: &'a str,
    pub registry: &'a str,
}

pub fn unit_snapshots() -> Vec<UnitSnapshot<'static>> {
    let mut values = UNITS
        .iter()
        .map(|unit| UnitSnapshot {
            name: unit.name,
            dimension: unit.dimension,
            factor: unit.factor,
            status: unit.status,
            registry: unit.registry,
        })
        .collect::<Vec<_>>();
    values.sort_by_key(|value| value.name);
    values
}

pub fn unit(name: &str) -> Option<Unit> {
    UNITS.iter().copied().find(|unit| unit.name == name)
}
pub fn is_unit(name: &str) -> bool {
    unit(name).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    pub value_si: f64,
    pub dimension: Dimension,
}

impl Quantity {
    pub fn new(value_si: f64, dimension: Dimension) -> Result<Self> {
        if !value_si.is_finite() {
            return Err(GoblinError::numeric(
                "NON-FINITE NUMERIC VALUE\n\nGoblin++ refuses NaN and infinity in scientific values.",
            ));
        }
        Ok(Self {
            value_si,
            dimension,
        })
    }

    pub fn scalar(value: f64) -> Result<Self> {
        Self::new(value, DIMENSIONLESS)
    }

    pub fn from_unit(value: f64, name: &str) -> Result<Self> {
        let selected =
            unit(name).ok_or_else(|| GoblinError::unknown(format!("Unknown unit: {name}")))?;
        Self::new(value * selected.factor, selected.dimension)
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self> {
        self.require_same_dimension(rhs)?;
        Self::new(self.value_si + rhs.value_si, self.dimension)
    }

    pub fn checked_sub(self, rhs: Self) -> Result<Self> {
        self.require_same_dimension(rhs)?;
        Self::new(self.value_si - rhs.value_si, self.dimension)
    }

    pub fn checked_mul(self, rhs: Self) -> Result<Self> {
        Self::new(
            self.value_si * rhs.value_si,
            add_dimension(self.dimension, rhs.dimension),
        )
    }

    pub fn checked_div(self, rhs: Self) -> Result<Self> {
        if rhs.value_si == 0.0 {
            return Err(GoblinError::numeric(
                "DIVISION BY ZERO\n\nGoblin++ refuses an undefined numeric result.",
            ));
        }
        Self::new(
            self.value_si / rhs.value_si,
            sub_dimension(self.dimension, rhs.dimension),
        )
    }

    pub fn powi(self, power: i32) -> Result<Self> {
        Self::new(
            self.value_si.powi(power),
            mul_dimension(self.dimension, power),
        )
    }

    pub fn checked_sqrt(self) -> Result<Self> {
        if self.value_si < 0.0 {
            return Err(GoblinError::numeric(
                "SQUARE ROOT DOMAIN ERROR\n\nsqrt() requires a non-negative value.",
            ));
        }
        if self.dimension.iter().any(|power| power % 2 != 0) {
            return Err(GoblinError::dimension(format!(
                "SQUARE ROOT DIMENSION ERROR\n\nsqrt() requires even unit exponents, got {}.",
                format_dimension(self.dimension)
            )));
        }
        Self::new(self.value_si.sqrt(), self.dimension.map(|power| power / 2))
    }

    fn require_same_dimension(self, rhs: Self) -> Result<()> {
        if self.dimension != rhs.dimension {
            return Err(GoblinError::dimension(format!(
                "INCOMPATIBLE DIMENSIONS\n\nleft dimension  = {}\nright dimension = {}\n\nGoblin refuses dimensional nonsense.\nWHO PAID FOR THIS ASSUMPTION?",
                format_dimension(self.dimension),
                format_dimension(rhs.dimension)
            )));
        }
        Ok(())
    }

    pub fn render(self) -> String {
        let value = format_number(self.value_si);
        let dimension = format_dimension(self.dimension);
        if dimension == "1" {
            value
        } else {
            format!("{value} {dimension}")
        }
    }
}

impl Display for Quantity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.render())
    }
}

pub fn format_number(value: f64) -> String {
    crate::text_runtime::format_number(value)
}

pub fn format_dimension(dimension: Dimension) -> String {
    if dimension == DIMENSIONLESS {
        return "1".into();
    }
    let named = [
        (ENERGY, "J"),
        (POWER, "W"),
        (CURRENT, "A"),
        (CHARGE, "C"),
        (VOLTAGE, "V"),
        (RESISTANCE, "ohm"),
        (CONDUCTANCE, "S"),
        (CAPACITANCE, "F"),
        (INDUCTANCE, "H"),
    ];
    if let Some((_, label)) = named.iter().find(|(candidate, _)| *candidate == dimension) {
        return (*label).into();
    }
    let labels = ["kg", "m", "s", "K", "mol", "A"];
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    for (label, power) in labels.iter().zip(dimension) {
        if power == 0 {
            continue;
        }
        let text = if power.abs() == 1 {
            (*label).to_string()
        } else {
            format!("{label}^{}", power.abs())
        };
        if power > 0 {
            positive.push(text);
        } else {
            negative.push(text);
        }
    }
    let numerator = if positive.is_empty() {
        "1".into()
    } else {
        positive.join("*")
    };
    if negative.is_empty() {
        numerator
    } else if negative.len() > 1 {
        format!("{numerator}/({})", negative.join("*"))
    } else {
        format!("{numerator}/{}", negative.join("*"))
    }
}

fn add_dimension(left: Dimension, right: Dimension) -> Dimension {
    std::array::from_fn(|index| left[index] + right[index])
}

fn sub_dimension(left: Dimension, right: Dimension) -> Dimension {
    std::array::from_fn(|index| left[index] - right[index])
}

fn mul_dimension(dimension: Dimension, power: i32) -> Dimension {
    dimension.map(|item| item * power)
}
