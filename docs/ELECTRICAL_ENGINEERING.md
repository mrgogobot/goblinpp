# Electrical engineering in Goblin++

Alpha.17 adds strict SI electrical quantities and 18 functions for scalar DC
calculations, ideal passive components, and explicit Kirchhoff balance checks.
All run in the interpreter and native compiler. The definitions follow the
[BIPM SI Brochure, ninth edition, version 3.02](https://www.bipm.org/documents/20126/41483022/SI-Brochure-9.pdf).

## First calculation

Save this as `load.gbl`:

```goblin
GO_PARANOID
voltage = 12 V
resistance = 4.7 kohm
current = ee_current(voltage, resistance)
power = ee_power(voltage, current)
milliamps = ee_in_unit(current, "mA")
milliwatts = ee_in_unit(power, "mW")
print("current = {milliamps:.3f} mA")
print("power = {milliwatts:.3f} mW")
seal current
seal power
```

Run it with `goblin++ load.gbl`. Expected calculation output:

```text
current = 2.553 mA
power = 30.638 mW
```

`goblin++ load.gbl --compile` runs the compiled equivalent and checks its sealed
values against the interpreter. `goblin++ compile load.gbl -o load` creates a
standalone executable. `GO_PARANOID` and `seal` retain their existing optional
meaning; see [SECURITY.md](SECURITY.md) for evidence storage and trust boundaries.

## Units and conversions

Every quantity stores an SI value and six exponents in the order
`[mass, length, time, temperature, amount, electric current]`.

| Quantity | Recognized unit spellings |
| --- | --- |
| Electric current | `A`, `mA`, `uA`, `µA` |
| Electric charge | `C` |
| Voltage | `V`, `mV`, `kV` |
| Resistance | `ohm`, `Ω`, `kohm`, `kΩ`, `Mohm`, `MΩ` |
| Capacitance | `F`, `mF`, `uF`, `µF`, `nF`, `pF` |
| Inductance | `H`, `mH`, `uH`, `µH` |
| Power | `W`, `mW`, `kW` |
| Conductance | `S`, `mS`, `uS`, `µS` |
| Frequency | `Hz`, `kHz`, `MHz` |

Spellings are case-sensitive. The ASCII `u` forms and `ohm` forms are convenient
aliases; for example, `100 uF` and `100 µF` have the same SI value and dimension.
`C`, `F`, and `H` in quoted chemical formula strings still denote element
symbols; unit suffixes occur in numeric expressions.

Unit suffixes perform conversion on input. Thus `1 kohm + 500 ohm` produces
`1500 ohm`. Ordinary arithmetic also checks dimensions: `(12 V) / (4.7 kohm)`
produces current, and adding volts to amperes fails.

`ee_in_unit(quantity, "unit")` returns a **dimensionless number for reporting**
in a compatible electrical unit. Preserve the original quantity in equations
and `seal` statements:

```goblin
r = 4700 ohm
r_kohm = ee_in_unit(r, "kohm")
print("R = {r_kohm:.1f} kohm")
seal r
```

It refuses unknown spellings, incompatible dimensions, and non-electrical
units. SI dimensions do not distinguish cycle frequency from angular frequency
because radians are dimensionless: `Hz` and `1/s` share an exponent vector.
Generic inverse-time output therefore retains `1/s`; choose the reporting
label and any `2*pi` conversion explicitly.

## Functions

| Function | Calculation and requirements |
| --- | --- |
| `ee_voltage(I, R)` | `V = I R`; current and non-negative resistance |
| `ee_current(V, R)` | `I = V / R`; voltage and strictly positive resistance |
| `ee_resistance(V, I)` | `R = V / I`; nonzero current; result must be non-negative |
| `ee_power(V, I)` | `P = V I`; signed power under the passive sign convention |
| `ee_power_i2r(I, R)` | `P = I² R`; non-negative resistance |
| `ee_power_v2r(V, R)` | `P = V² / R`; strictly positive resistance |
| `ee_energy(P, t)` | Constant signed power times non-negative duration; joules |
| `ee_charge(I, t)` | Constant signed current times non-negative duration; coulombs |
| `ee_series_resistance(values)` | Sum a non-empty array of non-negative resistances |
| `ee_parallel_resistance(values)` | Reciprocal-sum equivalent; non-empty array of strictly positive resistances |
| `ee_rc_time_constant(R, C)` | `tau = R C`; non-negative resistance and capacitance; seconds |
| `ee_capacitor_energy(C, V)` | `E = C V² / 2`; non-negative capacitance; joules |
| `ee_inductor_energy(L, I)` | `E = L I² / 2`; non-negative inductance; joules |
| `ee_kcl_residual(currents)` | Signed sum of a non-empty current array |
| `ee_kvl_residual(voltages)` | Signed sum of a non-empty voltage array |
| `ee_kcl_balanced(currents, tolerance)` | Boolean: absolute residual ≤ non-negative current tolerance |
| `ee_kvl_balanced(voltages, tolerance)` | Boolean: absolute residual ≤ non-negative voltage tolerance |
| `ee_in_unit(value, "unit")` | Dimensionless reporting number in a compatible electrical unit |

`ee_power` assumes that positive current enters the positive-voltage terminal.
Positive power represents absorption; negative power represents delivery. The
programmer chooses terminal orientation. The resistor helpers describe ideal
passive resistances; negative resistance and division by zero fail explicitly.
The parallel helper requires strictly positive resistances; represent a known
ideal short explicitly instead of passing a zero branch to that helper.

## Kirchhoff's laws

For a node, choose a consistent sign convention, such as entering positive and
leaving negative. For a closed loop, choose a traversal direction and encode
voltage rises and drops with signs:

```goblin
currents = [3 mA, -2.001 mA, -1 mA]
residual = ee_kcl_residual(currents)
balanced = ee_kcl_balanced(currents, 0.01 mA)
print("node residual = {residual}; balanced = {balanced}")

voltages = [12 V, -7 V, -5 V]
loop_ok = ee_kvl_balanced(voltages, 0.001 V)
print("loop balanced = {loop_ok}")
```

The first residual is approximately `-0.001 mA`, and its balance check is true
within the stated `0.01 mA` tolerance. A tolerance is a caller-supplied decision
threshold, not measurement uncertainty. These helpers sum values supplied by
the user; they do not discover connections or solve simultaneous circuits.

## Scope and evidence

The stage covers scalar DC values and ideal stored-energy/time-constant
relations. AC phasors, complex impedance, circuit topology, semiconductor
models, component tolerances, uncertainty propagation, and time-domain circuit
simulation remain future work. Electrical dimensions catch unit mistakes;
they cannot validate an assumed model or a circuit's topology.

Numerics retain finite `f64` behavior. Overflow is refused; rounding and
underflow follow floating-point arithmetic. Parallel resistance uses scaling
to avoid spurious reciprocal overflow for very small positive inputs.

Electrical registry identity and SHA-256 appear in run evidence. New freeze
receipts bind the six-axis constants, unit definitions, chemistry registry, and
electrical helper registry. Historical receipts keep their original hashes:
pre-alpha.16 constants use five axes, and alpha.16's five-axis scientific
registry remains recognized. This does not claim that older freezes covered
the new electrical vocabulary. Old native result manifests with five axes
are read with zero electric-current exponent.

Try [`examples/electrical.gbl`](../examples/electrical.gbl) for the complete
worked example. Both editor plugins include `ee_` completion, units, and help.
