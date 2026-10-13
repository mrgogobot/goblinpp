# Element-wise array maths — alpha.33

Numeric arrays use the same finite-value and dimensional rules as scalar
calculations. The result is a new, independent, one-dimensional array. Neither
input is mutated. Both interpreter and compiled execution support this feature.

```goblin
GO_PARANOID
areas = [9 m^2, 16 m^2, 25 m^2]
lengths = sqrt(areas)
speeds = lengths / (2 s)
print("lengths = {lengths}; speeds = {speeds}")
seal lengths
seal speeds
```

Output:

```text
lengths = [3 m, 4 m, 5 m]; speeds = [1.5 m/s, 2 m/s, 2.5 m/s]
```

## Shape, operators and scalar repetition

- Unary `+` and `-` map over numeric arrays.
- Binary `+`, `-`, `*`, `/`, `%` and `^` pair corresponding elements when both
  operands are arrays. Every array operand must have **exactly the same length**.
- When only one operand is an array, a numeric scalar is repeated at every
  index. Order matters: `8 / [2, 4]` yields `[4, 2]`; `[2, 4] / 8` does not.
- A one-element **array** is not a scalar: `[1, 2] * [3]` refuses, rather than
  broadcasting or dropping rows. Use `[1, 2] * 3` if repetition is intended.
- `*` does not mean dot product or matrix multiplication. Keep using `dot(a, b)`
  and `matrix_multiply(...)` for those explicit operations.
- Array addition is numeric addition, never concatenation. Text/Boolean arrays
  and nested arrays are refused by this feature. Scalar text `+` is unchanged.

```goblin
a = [2, 4, 6]
b = [3, 5, 7]
products = a * b       # [6, 20, 42]
inner = dot(a, b)      # 68
```

## Functions

The following existing functions also map over numeric arrays:

| Family | Functions |
| --- | --- |
| Magnitude/root | `abs`, `sqrt` |
| Element-wise extrema | `min`, `max` (two or more operands) |
| Rounding | `floor`, `ceil`, `round` |
| Exponential/logarithmic | `exp`, `ln`, `log10` |
| Explicit trigonometry | `sind`, `cosd`, `tand`, `sinr`, `cosr`, `tanr`, `asind`, `acosd`, `atand`, `asinr`, `acosr`, `atanr`, `atan2d`, `atan2r` |
| Angle conversions | `deg2rad`, `rad2deg` |
| Two-component length | `hypot` |
| Historical radians aliases | `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2` (still warn; prefer explicit names) |

Multi-argument calls follow the same equal-length/scalar-repetition rules:
`hypot([3 m, 5 m], [4 m, 12 m])` yields `[5 m, 13 m]`.
`min([2, 4], [1, 8], 3)` yields `[1, 3]`.
`min(values)` is **not** a reduction. For a distribution's minimum, for example,
use `ordered = sort(values)` and `smallest = ordered[0]` after checking nonempty.

`is_close` and `same_bits` remain scalar comparisons. Comparisons, equality,
Boolean conditions and logical operators have not become array masks. Statistics,
science/chemistry/electrical helpers, conversions such as `au2m`, and user
`g_func` calls are **not automatically vectorized**. A function may explicitly
use array expressions inside its body.

## Units and failure behavior

Each numeric input array must be homogeneous in dimensions. Multiplication and
division derive dimensions; addition/subtraction require matching dimensions.
`sqrt` requires nonnegative values and even unit exponents. Exponents must be
dimensionless signed 32-bit integers, as in scalar arithmetic. Unit exponent
overflow refuses instead of panicking or wrapping; the magnitude must be
representable as a positive signed 32-bit integer.

Output dimensions must also remain homogeneous. `[1 m, 2 m] ^ [1, 2]` refuses
at index 1: metres and square metres cannot be stored in the same numeric array.
This is not fixed by coercing units or stripping dimensions.

Shape is checked before scalar computation, then finite numeric input types and
within-array dimensions are checked. Functions validate arity even with an
empty array. Actual lanes run in ascending order. A scalar domain/unit failure
names the operation and the first failing **zero-based element index**, for
example `sqrt: element index 1 (zero-based)` for `sqrt([9, -1, 4])`.
Scalar numeric/dimension error categories are retained; shape/type/output
homogeneity errors use `G203`.

The whole expression must succeed before its destination is assigned. A failure
never publishes a partially calculated result array or replaces the old value.
Prior explicitly sealed values, output artifacts, imports and the failed run's
diagnostics are still preserved according to the existing audit rules.

## Empty arrays and limits

`sqrt([])`, `[] + []` and `min([], 2)` return `[]`. Shape, function arity and
numeric scalar types are validated, but there is **no scalar domain or
cross-operand dimension calculation without a lane**. Thus `[] / 0` and
`[] + (1 s)` return `[]`; they do not establish that the scalar operation is
valid on real data. Use an explicit nonempty check when an empty dataset should
refuse. `[] + [1]` and `[] + "text"` always refuse.

The existing 100,000-item array limit remains. All supplied array operands and
the result are bounded per operation. Mapping is serial and does not consume
user loop-body entries in `GO_LOOP_BUDGET`. Arrays/results are ordinary copies:
several retained arrays still use memory. This is not threading, lazy evaluation,
general out-of-core maths, a whole-process RAM limit, or a speed guarantee.
Use CSV/TSV/FITS batches to process larger catalogues in bounded chunks.

## Evidence and migration

New run/freeze receipts, `check` previews, compile reports and native data
manifests record `goblin.elementwise-numeric-arrays.v1` in `array_math_policy`.
Verification refuses a missing/unsupported policy on alpha.33+ receipts, even
if their core hash is recomputed. `diff` distinguishes `ARRAY_MATH_POLICY_CHANGE`
from an identical-result classification. Hashes remain checksums, not signatures.

Historical receipts without this field remain verifiable. **All older freezes
without the policy require an explicit revision before execution/compilation
with alpha.33**, even scalar-only programs; the whole-program freeze must not
silently acquire changed arithmetic semantics. Preserve the parent unchanged:

```sh
goblin++ revise parent.gbl child.gbl --reason "adopt alpha.33 array arithmetic policy"
goblin++ freeze child.gbl
goblin++ child.gbl
```

This does not change the existing parser, canonical source rules, RNG,
statistics or float serialization policies. Exact seals are preserved; platform
transcendental results can still differ in their last bits. Choose and declare
scientific comparison tolerances separately—never round authoritative evidence
to conceal those differences.

## Try and verify

The [local acceptance report](verification/ARRAY_MATH_ACCEPTANCE_alpha33.json)
and [actual alpha.32 compatibility gate](verification/ARRAY_MATH_COMPATIBILITY_alpha33.json)
record the tests and their bounded claims. These are engineering fixtures,
not an independently validated WB/entropic verdict or completed remote CI.

From the extracted bundle root:

```sh
./install.sh --prefix "$HOME/.local"
goblin++ --version
goblin++ examples/elementwise_arrays.gbl
goblin++ run examples/elementwise_arrays.gbl --compile
```

For each run, copy its actual printed `RUN_DIR` into the verify command:

```sh
goblin++ verify ./examples/runs/<actual-run-directory>
```

The angle brackets above are a placeholder, not a literal filename. Paths are
relative to the current terminal directory; IDE Run uses the source's actual
path. This feature does not implement exact-ID joins, new plot types or an
independently validated WB/entropic end-to-end regression suite.

Optional outreach: [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk).
No purchase is required to use Goblin++.
