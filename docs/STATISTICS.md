# Statistics basics — alpha.18, step 1

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

## Next steps, not available yet

Median; explicit sample/population variance and standard deviation; weighted
mean; missing-data policies; and measurement-uncertainty propagation are separate
steps. No implicit sample selection or scientific interpretation is introduced.
