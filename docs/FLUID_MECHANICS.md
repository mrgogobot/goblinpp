# Introductory fluid mechanics — alpha.40

These scalar helpers run in the interpreter and generated native code using the
same bounded implementation. They check SI dimensions, argument counts, finite
values, physical sign constraints, overflow and a nonzero calculation collapsing
to a zero final result. They are not a CFD solver, material-property database,
interval-arithmetic implementation or a guarantee that a chosen model applies.
Ordinary binary64 rounding and intermediate underflow remain possible.

Inputs carry units: for example `1000 kg/m^3`, `0.001 Pa*s`, `2 m/s` and
`1e-6 m^3/s`. No implicit density, viscosity, temperature or gravity is supplied.
Arrays are refused; loop or select scalar inputs explicitly.

| Function | Calculation and result |
|---|---|
| `fluid_density(mass, volume)` | mass / volume, kg/m³; mass ≥ 0, volume > 0 |
| `fluid_kinematic_viscosity(mu, rho)` | dynamic viscosity / density, m²/s; both > 0 |
| `fluid_dynamic_viscosity(nu, rho)` | kinematic viscosity × density, Pa·s; both > 0 |
| `fluid_volume_flow(area, mean_normal_velocity)` | area × mean normal velocity, m³/s; area > 0; signed flow |
| `fluid_flow_speed(volume_flow, area)` | flow / area, m/s; area > 0; signed normal velocity |
| `fluid_reynolds(rho, velocity, characteristic_length, mu)` | rho × abs(velocity) × length / mu; dimensionless; rho, length, mu > 0 |
| `fluid_dynamic_pressure(rho, velocity)` | rho × velocity² / 2, Pa; rho > 0 |
| `fluid_bernoulli_total_pressure(static_pressure, rho, velocity)` | static pressure + rho × velocity² / 2, Pa; rho > 0 |
| `fluid_pipe_pressure_drop(rho, mu, length, radius, volume_flow)` | 8 × mu × length × flow / (pi × radius⁴), Pa; rho, mu, length, radius > 0 |

Velocity is a signed scalar normal to the area; `fluid_volume_flow` does not
integrate a nonuniform velocity field. Steady incompressible continuity between
sections can be written by finding Q at the first section and passing the same Q
to `fluid_flow_speed` at the second. Compressible continuity instead conserves
mass flux, and is not provided by this shortcut.

Reynolds number is a diagnostic; the characteristic length depends on geometry.
Use **diameter** for circular-pipe Reynolds number. No universal flow-regime
classification is inferred by `fluid_reynolds`.

The Bernoulli helper is explicitly the **same-height, steady, incompressible,
inviscid streamline model**, without pumps, dissipative losses or heat addition.
Keep all static pressures on the same gauge/absolute basis. It does not transfer
pressure between sections or include elevation terms. See
[NASA's Bernoulli equation and restrictions](https://www1.grc.nasa.gov/beginners-guide-to-aeronautics/bernoullis-equation/).

The pipe helper uses **radius**, not diameter. Its signed result is inlet minus
outlet pressure for positive flow along the declared pipe direction. The model
requires a rigid, straight circular tube, Newtonian incompressible fluid,
steady fully developed laminar flow and no-slip walls; entrance/minor losses
are omitted. The implementation conservatively refuses pipe Reynolds ≥ 2000
using abs(Q)/(pi r²) and diameter 2r. Passing that gate does not prove the other
assumptions. No turbulent fallback or silent model substitution is performed.
See [OpenStax's Poiseuille derivation](https://openstax.org/books/university-physics-volume-1/pages/14-7-viscosity-and-turbulence).

```goblin
GO_PARANOID
rho = 1000 kg/m^3
mu = 0.001 Pa*s
radius = 0.01 m
flow = 1e-6 m^3/s
mean_velocity = fluid_flow_speed(flow, pi * radius^2)
reynolds = fluid_reynolds(rho, mean_velocity, 2 * radius, mu)
pressure_drop = fluid_pipe_pressure_drop(rho, mu, 1 m, radius, flow)
print("Re = {reynolds}; pressure drop = {pressure_drop}")
seal reynolds
seal pressure_drop
```

The illustrative expected pressure drop is approximately 0.254647908947 Pa;
Re is approximately 63.661977. These are arithmetic fixture expectations,
not measured water properties or validation of an experimental setup.

Run and preserve the full example, then verify its printed run directory:

```sh
goblin++ run examples/fluid_mechanics.gbl
goblin++ run examples/fluid_mechanics.gbl --compile
goblin++ verify <RUN_DIR>
```

`GO_PARANOID`, `COMPLETELY_MAD` and explicit `seal` retain their existing roles.
Verification establishes preserved evidence integrity, not scientific validity.
