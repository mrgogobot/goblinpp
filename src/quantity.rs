use crate::error::{GoblinError, Result};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub type Dimension = [i32; 5];
pub const DIMENSIONLESS: Dimension = [0, 0, 0, 0, 0];
pub const MASS: Dimension = [1, 0, 0, 0, 0];
pub const LENGTH: Dimension = [0, 1, 0, 0, 0];
pub const TIME: Dimension = [0, 0, 1, 0, 0];
pub const TEMPERATURE: Dimension = [0, 0, 0, 1, 0];
pub const AMOUNT: Dimension = [0, 0, 0, 0, 1];
pub const ENERGY: Dimension = [1, 2, -2, 0, 0];

#[derive(Debug, Clone, Copy)]
pub struct Unit {
    pub name: &'static str,
    pub dimension: Dimension,
    pub factor: f64,
}

pub const UNITS: &[Unit] = &[
    Unit {
        name: "kg",
        dimension: MASS,
        factor: 1.0,
    },
    Unit {
        name: "g",
        dimension: MASS,
        factor: 1e-3,
    },
    Unit {
        name: "m",
        dimension: LENGTH,
        factor: 1.0,
    },
    Unit {
        name: "km",
        dimension: LENGTH,
        factor: 1e3,
    },
    Unit {
        name: "s",
        dimension: TIME,
        factor: 1.0,
    },
    Unit {
        name: "K",
        dimension: TEMPERATURE,
        factor: 1.0,
    },
    Unit {
        name: "mol",
        dimension: AMOUNT,
        factor: 1.0,
    },
    Unit {
        name: "J",
        dimension: ENERGY,
        factor: 1.0,
    },
];

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
    if value == 0.0 {
        return "0".to_string();
    }
    let scientific = value.abs() >= 1e15 || value.abs() < 1e-4;
    if scientific {
        // Match Python's `.15g`: fifteen significant digits total.
        let mut rendered = format!("{value:.14e}");
        if let Some(index) = rendered.find('e') {
            let mut mantissa = rendered[..index]
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
            if mantissa == "-0" {
                mantissa = "0".into();
            }
            let exponent: i32 = rendered[index + 1..].parse().unwrap_or(0);
            rendered = format!("{mantissa}e{exponent:+}");
        }
        rendered
    } else {
        format!("{value:.15}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

pub fn format_dimension(dimension: Dimension) -> String {
    if dimension == DIMENSIONLESS {
        return "1".into();
    }
    if dimension == ENERGY {
        return "J".into();
    }
    let labels = ["kg", "m", "s", "K", "mol"];
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
