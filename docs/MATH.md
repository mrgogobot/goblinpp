# Scientific mathematics in Goblin++ 0.1.0-alpha.13

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

`sin`, `cos`, `tan`, `asin`, `acos`, and `atan` accept one dimensionless
value. `asin` and `acos` restrict their input to the closed interval `[-1, 1]`.
`atan2(y, x)` accepts two values with matching dimensions and refuses
`atan2(0, 0)`. Angles are represented as dimensionless radians; there is no
implicit degree conversion.

```goblin
opposite = sin(pi / 2)
angle = atan2(1 m, 1 m)
```

## Explicit refusals

These examples fail rather than inventing a result:

```goblin
bad_root = sqrt(-1)        # real-number domain error
bad_unit = sqrt(3 m)       # fractional dimension would be required
bad_log = ln(0)            # logarithm domain error
bad_trig = sin(1 m)        # trigonometry requires a dimensionless angle
bad_pair = hypot(1 m, 1 s) # dimensions do not match
```

Goblin++ currently implements real-valued `f64` mathematics. Complex numbers,
fractional-dimension quantities, uncertainty propagation, and angle units are
separate future design decisions, not implied by these built-ins.
