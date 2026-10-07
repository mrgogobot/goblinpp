# Explicit-stream resampling and normal draws (alpha.27)

These helpers use alpha.26's **non-cryptographic**, explicitly seeded PCG32
streams. Every helper requires a stream argument: no clock seeds, implicit
streams, or silent reseeding. Numeric quantities keep their SI dimensions.

```goblin
GO_PARANOID

rng_seed(20261007, 3)
observations = [1 m, 3 m, 8 m, 10 m]
sample = resample(observations, 4, 3)
means = bootstrap_mean(observations, 1000, 3)
medians = bootstrap_median(observations, 1000, 3)

print("sample = {sample}")
print("bootstrap mean distribution center = {mean(means)}")
print("bootstrap median distribution center = {median(medians)}")
seal means
seal medians
```

## Function contracts

| Function | Result and algorithm |
|---|---|
| `resample(values, count, stream)` | A new array of `count` observations drawn with replacement, uniformly by index. The source is not edited. |
| `bootstrap_mean(values, repetitions, stream)` | An array of `repetitions` bootstrap means. Each replicate draws exactly `len(values)` observations with replacement. |
| `bootstrap_median(values, repetitions, stream)` | The analogous distribution of medians, using the existing type-7 median implementation. |
| `rng_normal(stream)` | One dimensionless standard-normal draw with the specified Box-Muller transformation below. |
| `rng_normal_array(count, stream)` | An array equivalent to `count` consecutive `rng_normal(stream)` calls. |

Counts/repetitions must be positive, dimensionless numeric integers no greater
than 100,000. The source must contain 1–100,000 finite numeric quantities of
one shared dimension. Text, booleans, nested arrays, mixed dimensions and
nonfinite numbers are refused. There is no automatic missing-value removal.
Canonical decimal stream text is supported just as in alpha.26; a stream must
already have been seeded. Invalid helper arguments consume no RNG state.
If a budget or numerical refusal occurs after drawing begins, the helper rolls
back *all* its draws: returned RNG state and evidence remain unchanged.

Bootstrap selections are limited to 1,000,000 per helper call, computed as
`len(values) * repetitions`, in addition to the existing per-run RNG caps.
Sampling consumes one `rng_integer(0, len(values), stream)` operation per
selection. Normal draws consume two uniform operations per result, not one.
These explicit limits are separate from a user-program loop budget: increasing
that loop budget does not silently remove RNG evidence or bootstrap caps.

## Statistical assumptions: do not smuggle a confidence interval

These are **unweighted IID observation bootstraps**. They are not block,
cluster, stratified, survey-weighted, paired-multicolumn, or parametric
bootstraps. Independent resampling of separate columns destroys their pairing;
do not use it to propagate correlated Gaia astrometry. Bootstrap sampling is
conditional on the supplied observations and does not account for selection
effects or repair an unrepresentative sample. A singleton input gives a
degenerate bootstrap distribution; it does not establish zero uncertainty in
the underlying population.

The helpers return distributions, not default confidence intervals.
For example, `quantile(means, 0.025)` and `quantile(means, 0.975)` are two
empirical percentile endpoints using Goblin++'s specified type-7 interpolation.
Calling these a 95% confidence interval requires declaring the percentile
method, sampling assumptions and its limitations. They are not BCa,
studentized, or guaranteed-coverage intervals. The repetition count controls
Monte Carlo precision, not the validity of those assumptions.

## Standard-normal transform and reproducibility boundary

The versioned transform takes two successive existing 53-bit uniforms
`u` and `v` in `[0,1)` and evaluates, in this order:

```text
sqrt(-2 * ln(1 - u)) * cos(2 * pi * v)
```

`1-u` lies in `(0,1]`, so the logarithm never receives zero. No rejection loop,
cached spare or hidden state is used. The second Box-Muller normal is discarded.
The finite uniform grid means this is a finite-precision approximation to the
continuous normal law, not an unlimited-tail distribution or a cryptographic
generator.

The receipt's PCG evidence contains the existing integer/uniform operations,
their counts, state and exact primitive-result hashes. Its independent replay
checks those primitives. It does **not**, by itself, replay the application,
prove a chosen statistical model, or certify that the derived normal outputs
are bitwise-identical across CPUs. `ln`, `sqrt` and `cos` may differ in their
last bits across platforms. Preserve exact normal/derived seals and compare
scientific results separately using declared tolerances. Interpreter and native
programs use this same Rust helper; no new random-number algorithm is substituted.

## Algorithm references

- [NIST Dataplot: Bootstrap Sample](https://www.itl.nist.gov/div898/software/dataplot/refman2/ch2/bootsamp.pdf)
  describes observation resampling with replacement at the original sample size.
- [US Social Security Administration, Actuarial Study 117, Appendix C](https://www.ssa.gov/oact/NOTES/as117/as117.pdf)
  gives the Box-Muller transform. Goblin++ specifies its own uniform-grid
  endpoint mapping and discards the spare; it does not copy that report's PRNG
  or its alternative polar/rejection implementation.
- [RANDOMNESS.md](RANDOMNESS.md) specifies the unchanged alpha.26 PCG stream,
  uniform and integer mappings, replay evidence and non-cryptographic boundary.

Tests include hand-computed bootstrap reductions, direct-index equivalence,
singleton/unit preservation, fixed-seed normal transform and draw-count checks,
pre-draw argument refusal and post-draw transactional rollback. A fixed-seed
moment smoke test is a regression check, not a proof of statistical quality.
