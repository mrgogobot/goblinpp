# Statistics — alpha.24 distributions, alpha.18 reductions

## Distribution functions

These functions run in the interpreter and standalone compiled code. Inputs
are homogeneous finite numeric arrays; compatible units normalize to SI. Every
observation is retained and arguments are never mutated. No weights, row
selection, missing-value removal or scientific interpretation are inferred.

| Function | Definition and result |
| --- | --- |
| `sort(values)` | Stable ascending numeric copy, retaining dimensions; `sort([])` is `[]`. Text/Boolean sorting is not implemented. |
| `median(values)` | `quantile(values, 0.5)`; interpolated even middle, retaining dimensions. |
| `quantile(values, p)` | Hyndman–Fan type 7; finite dimensionless `p` in `[0,1]`; retains dimensions. |
| `std_population(values)` | Square root of summed squared deviations divided by `n`; retains dimensions. Singleton gives positive zero. |
| `std_sample(values)` | Standard deviation with divisor `n-1`; at least two observations; retains dimensions. |
| `ecdf(values, query)` | Unweighted `count(value <= query)/n`, including all ties; dimensionless scalar. Finite query must match observation dimensions. |

Except for `sort`, empty arrays refuse. Nonfinite values, text, Boolean values,
mixed dimensions and invalid probabilities refuse rather than being dropped.
There is no ambiguous `std()` default. Sample standard deviation is not
standard error; choosing `n-1` does not establish independent sampling.

### First example

```goblin
GO_PARANOID
measurements = [2 m, 4 m, 4 m, 4 m, 5 m, 5 m, 7 m, 9 m]
ordered = sort(measurements)
middle = median(measurements)
upper_quartile = quantile(measurements, 0.75)
spread = std_population(measurements)
sample_spread = std_sample(measurements)
fraction = ecdf(measurements, 5 m)
print("median = {middle}; Q3 = {upper_quartile}")
print("population SD = {spread:.6f}; sample SD = {sample_spread:.6f}")
print("fraction at or below 5 m = {fraction:.3f}")
seal middle
seal spread
seal fraction
```

Expected: median `4.5 m`, Q3 `5.5 m`, population SD approximately `2 m`, sample
SD approximately `2.138090 m`, ECDF `0.750`. Explicit `.6f` rounds presentation
only; seals retain exact calculated f64 outputs. Run the complete
`examples/statistics_distribution.gbl` normally or with `--compile`, then
independently verify the printed run directory.

For a CSV, pass its filename after `--`, retrieve `file = argv(1)` and use
`values = csv_numbers(file, "EXACT_HEADER")`. Columns are dimensionless; attach
physical units explicitly if defined by the experiment. Numeric CSV loading
refuses missing cells. Relative paths resolve from the entry `.gbl` directory.
The existing 100,000-item array and CSV input limits have not been raised.

### Quantile, ties and signed zero

For stable ascending observations `a[0]` through `a[n-1]`, set `h = (n-1)*p`,
`j = floor(h)`, `t = h-j`, and interpolate between `a[j]` and `a[j+1]`.
Exact endpoints/singletons return the stored value. The named convention is
[Hyndman–Fan type 7](https://stat.ethz.ch/R-manual/R-patched/library/stats/html/quantile.html).
This f64 implementation uses no boundary fuzz and does not promise bitwise
agreement with every library implementation. For example the 0.25 quantile
of `[1,2,3,4]` is `1.75`, not the second observation.

Numeric sorting treats both zero signs as equal, retaining their source order:
`sort([0, -0])` remains `[0, -0]`. Equal interpolation endpoints return the
lower stored value, including its sign. ECDF counts both zero signs as equal;
constant-array standard deviation returns positive zero.

### Numerical and audit boundaries

Interpolation avoids avoidable overflow from same-sign sums and opposite-sign
differences. Standard deviation anchors and scales deviations before a centered
compensated two-pass calculation; if anchor subtraction overflows, it normalizes
the original observations instead. It avoids forming an overflowing physical
variance merely to take a square root. An unrepresentable final result refuses.
This remains f64 arithmetic: rounding and underflow (including zero results)
are possible, and input precision already lost cannot be recovered. No universal
cross-platform bitwise or arbitrary-precision guarantee is made.

Run receipts and new freezes record `statistics_policy`: definitions and the
shared statistics source SHA-256. A changed pin refuses run/compile with
`STATISTICS_POLICY_CHANGED_AFTER_FREEZE`; use an explicit revision. Historical
freezes without the field acquire no invented pin. `diff` reports
`STATISTICS_POLICY_CHANGE` separately from equal output values. Verification
and seals stay exact; scientific tolerance comparisons remain separate.

Historical `g_func`/parameter names colliding with a new builtin remain
canonicalizable for evidence verification, but new execution refuses them.
Rename them in an explicit revision. Ordinary variables such as `median = 7`
still work; call syntax `median(...)` invokes the builtin. Discarded calls
refuse: use `ordered = sort(values)`, not `sort(values)`.

## Existing sum and mean

`sum(values)` returns the total. `mean(values)` returns the arithmetic mean.
Both take exactly one non-empty numeric array and run in interpreted and
native-compiled programs. They do not modify the array.

```goblin
GO_PARANOID
measurements = [10 m, 11 m, 12 m]
total = sum(measurements)
average = mean(measurements)
print("total = {total}")
print("average = {average}")
seal total
seal average
```

Expected program output (before the run evidence summary):

```text
total = 33 m
average = 11 m
```

Run the bundled example with `goblin++ examples/statistics_basics.gbl`, or add
`--compile` to run its compiled form. Neither `GO_PARANOID` nor `seal` is
required for everyday use; their existing evidence rules remain unchanged.

## Units and input rules

- Every element must be a finite numeric quantity with the same dimension.
- Compatible spellings are normalized to SI: `mean([1 km, 500 m])` is `750 m`.
- The result retains the input dimension, including current and compound units.
- Single-element arrays work; empty arrays fail for **both** functions. Goblin++
  does not invent the dimension of an empty sum or a mean with no observations.
- Text, Boolean, and nested arrays are not numeric observations. Values are
  never silently filtered, parsed, or treated as zero.
- Existing array limits, indexing, slices, and independent-copy semantics apply.
- `sum` and `mean` are reserved function/parameter names, as with other built-ins.
  Ordinary variables with these names still work; call syntax invokes the builtin.

These examples fail and their run evidence is still independently verifiable:

```goblin
x = mean([])
x = sum(["10", "11"])
x = mean([1 m, 1 s])
x = sum([1e308, 1e308])
```

## Floating-point policy

The shared runtime uses compensated (Neumaier) summation in source order.
For example, `sum([1e16, 1, -1e16])` recovers `1` rather than losing the small
term. Sum refuses non-finite intermediate totals, corrections, or final results:
even `sum([1e308, 1e308, -1e308])` fails instead of assuming later cancellation
can rescue an overflowing intermediate total.

Mean divides each value by the largest absolute input, compensates the scaled
sum, divides by the count, and rescales. Thus `mean([1e308, 1e308])` is `1e308`
even though `sum(...)` would fail. All-zero arrays return zero in their dimension.
This is `f64` arithmetic, not exact arithmetic: rounding remains possible,
extremely small terms may underflow during scaling, and subnormal results may
round to zero. Compensation improves stability; it does not prove arbitrary
precision or measurement accuracy.

## Follow-on work, not available yet

Seeded RNG, bootstrap/resampling, weighted statistics, variance functions,
missing-data policies, covariance and uncertainty propagation are separate
steps. No implicit sample selection or scientific interpretation is introduced.
