use crate::quantity::{AMOUNT, DIMENSIONLESS, Dimension, Quantity};
use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub struct Constant {
    pub id: &'static str,
    pub aliases: &'static [&'static str],
    pub value_si: f64,
    pub dimension: Dimension,
    pub status: &'static str,
    pub registry: &'static str,
}

pub const CONSTANTS: &[Constant] = &[
    Constant {
        id: "math.pi",
        aliases: &["pi", "π"],
        value_si: std::f64::consts::PI,
        dimension: DIMENSIONLESS,
        status: "mathematical-exact; floating-evaluation",
        registry: "builtin-math-v1",
    },
    Constant {
        id: "physical.speed_of_light",
        aliases: &["c", "speed_of_light"],
        value_si: 299_792_458.0,
        dimension: [0, 1, -1, 0, 0],
        status: "exact",
        registry: "SI-2019",
    },
    Constant {
        id: "physical.planck_constant",
        aliases: &["h", "planck_constant"],
        value_si: 6.626_070_15e-34,
        dimension: [1, 2, -1, 0, 0],
        status: "exact",
        registry: "SI-2019",
    },
    Constant {
        id: "physical.reduced_planck_constant",
        aliases: &["hbar", "ħ"],
        value_si: 1.054_571_817_646_156_5e-34,
        dimension: [1, 2, -1, 0, 0],
        status: "derived-exact-relation; floating-evaluation",
        registry: "SI-2019",
    },
    Constant {
        id: "physical.gravitational_constant",
        aliases: &["G", "gravitational_constant"],
        value_si: 6.674_30e-11,
        dimension: [-1, 3, -2, 0, 0],
        status: "measured",
        registry: "CODATA-2022",
    },
    Constant {
        id: "physical.boltzmann_constant",
        aliases: &["k_B", "boltzmann_constant"],
        value_si: 1.380_649e-23,
        dimension: [1, 2, -2, -1, 0],
        status: "exact",
        registry: "SI-2019",
    },
    Constant {
        id: "physical.avogadro_constant",
        aliases: &["N_A", "avogadro_constant"],
        value_si: 6.022_140_76e23,
        dimension: [0, 0, 0, 0, -AMOUNT[4]],
        status: "exact",
        registry: "SI-2019",
    },
];

pub fn resolve(alias: &str) -> Option<&'static Constant> {
    CONSTANTS
        .iter()
        .find(|constant| constant.aliases.contains(&alias))
}

pub fn by_id(id: &str) -> Option<&'static Constant> {
    CONSTANTS.iter().find(|constant| constant.id == id)
}

impl Constant {
    pub fn quantity(self) -> Quantity {
        Quantity::new(self.value_si, self.dimension)
            .expect("constant registry must contain finite values")
    }
}

#[derive(Debug, Serialize)]
pub struct ConstantSnapshot<'a> {
    pub id: &'a str,
    pub aliases: &'a [&'a str],
    pub value_si: f64,
    pub dimension: Dimension,
    pub status: &'a str,
    pub registry: &'a str,
}

pub fn snapshots() -> Vec<ConstantSnapshot<'static>> {
    let mut values = CONSTANTS
        .iter()
        .map(|constant| ConstantSnapshot {
            id: constant.id,
            aliases: constant.aliases,
            value_si: constant.value_si,
            dimension: constant.dimension,
            status: constant.status,
            registry: constant.registry,
        })
        .collect::<Vec<_>>();
    values.sort_by_key(|value| value.id);
    values
}
