# Scientific mathematics in Goblin++ 0.1.0-alpha.14

Alpha.25 accepts compound numeric units, e.g. `1.13e-10 m/s^2`.
`3 m^2` now means three square metres; `(3 m)^2` squares the whole quantity.
See [the syntax and historical-evidence migration guide](COMPOUND_UNITS.md).

Alpha.22 additionally provides dimension-aware `is_close` (four explicit
arguments, no default tolerance) and `same_bits` (including signed zero).
See [numeric reproducibility and comparison](NUMERIC_REPRODUCIBILITY.md).

The scientific-math built-ins execute in both the Rust interpreter and the
native compiler. Their results remain ordinary Goblin++ quantities and may be
printed, placed in arrays, returned from `g_func`, or sealed.

## Root, magnitude, extrema, and distance

```goblin
root = sqrt(81)
length = sqrt((3 m)^2)
magnitude = abs(-4 kg)
smallest = min(3 m, 1 m, 2 m)
largest = max(3 m, 1 m, 2 m)
diagonal = hypot(3 m, 4 m)
```

- `abs(value)` preserves the input dimension.
- `sqrt(value)` requires a non-negative value and an even exponent for every
  base dimension. It halves those exponents in the result. Fractional
  dimensions are not silently introduced.
- `min(first, second, ...)` and `max(first, second, ...)` require at least two
  arguments with identical dimensions.
- `hypot(x, y)` requires matching dimensions and returns that dimension.

## Rounding

`floor`, `ceil`, and `round` accept one dimensionless value. `round` follows
Rust `f64` behavior: halfway cases round away from zero. Goblin++ does not
silently round a physical quantity in SI units because the intended rounding
unit would be ambiguous.

## Exponential and logarithmic functions

`exp`, `ln`, and `log10` require dimensionless inputs. `ln` and `log10`
require a value greater than zero. Overflow, infinity, and NaN are refused.

```goblin
growth = exp(1)
natural = ln(growth)
decades = log10(1000)
```

## Trigonometry

Angle conventions are part of each preferred function name:

- `sind`, `cosd`, and `tand` interpret their dimensionless input as degrees;
- `sinr`, `cosr`, and `tanr` interpret their dimensionless input as radians;
- `asind`, `acosd`, and `atand` return a dimensionless numeric angle measured
  in degrees;
- `asinr`, `acosr`, and `atanr` return a dimensionless numeric angle measured
  in radians;
- `atan2d(y, x)` and `atan2r(y, x)` accept matching dimensions, refuse the
  undefined pair `(0, 0)`, and return degrees or radians respectively;
- `deg2rad` and `rad2deg` perform explicit conversion of dimensionless numeric
  angles.

Inverse sine and cosine restrict their input to the closed interval `[-1, 1]`.
Degree and radian values are not yet distinct quantity types: do not write
`30 deg` or `0.5 rad`. The suffix is the enforced angle convention.

```goblin
opposite = sinr(pi / 2)
half = sind(30)
angle_r = atan2r(1 m, 1 m)
angle_d = atan2d(1 m, 1 m)
converted = deg2rad(180)
```

For migration, the older `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, and
`atan2` names retain their alpha.13 radians behavior. Each used name emits one
preserved `G302` warning per run directing the author to its explicit `r` or
`d` form. These aliases are deprecated and may be removed only at a declared
breaking language version; they never silently change to degrees.

## Explicit refusals

These examples fail rather than inventing a result:

```goblin
bad_root = sqrt(-1)        # real-number domain error
bad_unit = sqrt(3 m)       # fractional dimension would be required
bad_log = ln(0)            # logarithm domain error
bad_trig = sind(1 m)       # trigonometry requires a dimensionless angle
bad_pair = hypot(1 m, 1 s) # dimensions do not match
```

Goblin++ currently implements real-valued `f64` mathematics. Complex numbers,
fractional-dimension quantities, uncertainty propagation, and first-class
angle quantity units are separate future design decisions, not implied by
these built-ins.
