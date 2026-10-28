// Shared verbatim with native programs: SI mass, length, time, temperature, amount, current.
pub type Dim = [i32; 6];
pub const DENSITY: Dim = [1, -3, 0, 0, 0, 0];
pub const DYNAMIC_VISCOSITY: Dim = [1, -1, -1, 0, 0, 0];
pub const KINEMATIC_VISCOSITY: Dim = [0, 2, -1, 0, 0, 0];
pub const PRESSURE: Dim = [1, -1, -2, 0, 0, 0];
pub const FLOW: Dim = [0, 3, -1, 0, 0, 0];
const SPEED: Dim = [0, 1, -1, 0, 0, 0];
const AREA: Dim = [0, 2, 0, 0, 0, 0];
const LENGTH: Dim = [0, 1, 0, 0, 0, 0];
const ZERO: Dim = [0; 6];

#[derive(Debug)]
pub struct Error {
    pub dimension: bool,
    pub message: String,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}
fn fail(message: impl Into<String>) -> Error {
    Error {
        dimension: false,
        message: message.into(),
    }
}
pub const FUNCTIONS: &[&str] = &[
    "fluid_density",
    "fluid_kinematic_viscosity",
    "fluid_dynamic_viscosity",
    "fluid_volume_flow",
    "fluid_flow_speed",
    "fluid_reynolds",
    "fluid_dynamic_pressure",
    "fluid_bernoulli_total_pressure",
    "fluid_pipe_pressure_drop",
];
pub fn arity(name: &str) -> Option<usize> {
    match name {
        "fluid_density"
        | "fluid_kinematic_viscosity"
        | "fluid_dynamic_viscosity"
        | "fluid_volume_flow"
        | "fluid_flow_speed"
        | "fluid_dynamic_pressure" => Some(2),
        "fluid_bernoulli_total_pressure" => Some(3),
        "fluid_reynolds" => Some(4),
        "fluid_pipe_pressure_drop" => Some(5),
        _ => None,
    }
}

/// Scalar, dimension-checked textbook helpers, not a flow solver.
pub fn call(name: &str, values: &[(f64, Dim)]) -> Result<(f64, Dim), Error> {
    let expected = arity(name).ok_or_else(|| fail(format!("Unknown fluid function: {name}")))?;
    if values.len() != expected {
        return Err(fail(format!(
            "{name}() expects {expected} arguments, got {}.",
            values.len()
        )));
    }
    // range: 0 = signed, 1 = nonnegative, 2 = strictly positive.
    let arg = |i: usize, dim: Dim, range: u8, field: &str| -> Result<f64, Error> {
        let (v, actual) = values[i];
        if actual != dim {
            return Err(Error {
                dimension: true,
                message: format!(
                    "{name}() argument {} ({field}) requires SI dimensions {dim:?}, got {actual:?}.",
                    i + 1
                ),
            });
        }
        if !v.is_finite() || (range == 1 && v < 0.0) || (range == 2 && v <= 0.0) {
            return Err(fail(format!(
                "{name}() {field} must be finite{}.",
                match range {
                    1 => " and nonnegative",
                    2 => " and greater than zero",
                    _ => "",
                }
            )));
        }
        Ok(v)
    };
    let result = match name {
        "fluid_density" => (
            arg(0, [1, 0, 0, 0, 0, 0], 1, "mass")? / arg(1, [0, 3, 0, 0, 0, 0], 2, "volume")?,
            DENSITY,
        ),
        "fluid_kinematic_viscosity" => (
            arg(0, DYNAMIC_VISCOSITY, 2, "dynamic viscosity")? / arg(1, DENSITY, 2, "density")?,
            KINEMATIC_VISCOSITY,
        ),
        "fluid_dynamic_viscosity" => (
            arg(0, KINEMATIC_VISCOSITY, 2, "kinematic viscosity")? * arg(1, DENSITY, 2, "density")?,
            DYNAMIC_VISCOSITY,
        ),
        "fluid_volume_flow" => (
            arg(0, AREA, 2, "area")? * arg(1, SPEED, 0, "mean normal velocity")?,
            FLOW,
        ),
        "fluid_flow_speed" => (
            arg(0, FLOW, 0, "volume flow")? / arg(1, AREA, 2, "area")?,
            SPEED,
        ),
        "fluid_reynolds" => (
            arg(0, DENSITY, 2, "density")?
                * arg(1, SPEED, 0, "velocity")?.abs()
                * arg(2, LENGTH, 2, "characteristic length")?
                / arg(3, DYNAMIC_VISCOSITY, 2, "dynamic viscosity")?,
            ZERO,
        ),
        "fluid_dynamic_pressure" => {
            let rho = arg(0, DENSITY, 2, "density")?;
            let v = arg(1, SPEED, 0, "velocity")?;
            (0.5 * rho * v * v, PRESSURE)
        }
        "fluid_bernoulli_total_pressure" => {
            let p = arg(
                0,
                PRESSURE,
                0,
                "static pressure (consistent gauge or absolute)",
            )?;
            let rho = arg(1, DENSITY, 2, "density")?;
            let v = arg(2, SPEED, 0, "velocity")?;
            (p + 0.5 * rho * v * v, PRESSURE)
        }
        "fluid_pipe_pressure_drop" => {
            let rho = arg(0, DENSITY, 2, "density")?;
            let mu = arg(1, DYNAMIC_VISCOSITY, 2, "dynamic viscosity")?;
            let length = arg(2, LENGTH, 2, "pipe length")?;
            let radius = arg(3, LENGTH, 2, "pipe radius")?;
            let flow = arg(4, FLOW, 0, "volume flow")?;
            let speed = flow / (std::f64::consts::PI * radius * radius);
            let reynolds = rho * speed.abs() * (2.0 * radius) / mu;
            if !reynolds.is_finite() || reynolds >= 2000.0 {
                return Err(fail(format!(
                    "{name}() requires pipe Reynolds number < 2000; got {reynolds}. This gate does not establish fully developed Newtonian laminar flow."
                )));
            }
            (
                8.0 * mu * length * flow / (std::f64::consts::PI * radius.powi(4)),
                PRESSURE,
            )
        }
        _ => unreachable!(),
    };
    if !result.0.is_finite()
        || (result.0 == 0.0
            && values.iter().all(|(v, _)| *v != 0.0)
            && name != "fluid_bernoulli_total_pressure")
    {
        return Err(fail(format!(
            "{name}() result overflowed or underflowed; refusing an unreliable numeric result."
        )));
    }
    Ok(result)
}
