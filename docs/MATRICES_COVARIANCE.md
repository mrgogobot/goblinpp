# Matrices, covariance and multivariate-normal sampling

Goblin++ alpha.27 uses ordinary flat arrays with explicit shapes. There is no
implicit matrix type, reshape, broadcasting, mixed-unit coercion or storage
aliasing. Every returned array is an independent value.

## Row-major layout

For two rows and three columns, `[1, 2, 3, 4, 5, 6]` means:

```text
1 2 3
4 5 6
```

Entry `(row, column)` is at index `row * columns + column`. Indices are zero
based. Rows and columns must be positive dimensionless integers. Array lengths
must match exactly; nested arrays are not accepted by these functions.

| Function | Arguments and result |
|---|---|
| `matrix_transpose(values, rows, columns)` | Flat transpose; result shape `columns × rows`, same units. |
| `matrix_multiply(left, left_rows, left_columns, right, right_rows, right_columns)` | Require `left_columns == right_rows`; flat result shape `left_rows × right_columns`; units multiply. |
| `vector_norm(vector)` | Euclidean magnitude, same units as the homogeneous input. Zero vector returns zero. |
| `vector_unit(vector)` | Dimensionless unit-direction array; refuses the zero vector. |
| `covariance(values, observations, variables, ddof)` | Row-major observations; flat `variables × variables` covariance, squared input units. |
| `cholesky(values, dimension, symmetry_tolerance)` | Flat lower-triangular factor `L`; zero above the diagonal; square-root input units. |
| `mvnormal(mean, covariance_values, dimension, symmetry_tolerance, stream)` | One sample array with `dimension` entries and the units of `mean`. |

All entries must be finite numeric quantities. Each input array must have one
homogeneous dimension. Matrix multiplication can use different dimensions on
its two input arrays, but not within either array. Dimension arithmetic is
checked: exponent overflow is refused. Cholesky input dimensions must have even
exponents.

```goblin
GO_PARANOID

left_values = [1, 2, 3, 4, 5, 6]
right_values = [7, 8, 9, 10, 11, 12]
product = matrix_multiply(left_values, 2, 3, right_values, 3, 2)
transposed = matrix_transpose(left_values, 2, 3)

displacement = [3 m, 4 m, 0 m]
distance = vector_norm(displacement)
direction = vector_unit(displacement)

print("product = {product}")   # [58, 64, 139, 154]
print("transpose = {transposed}")   # [1, 4, 2, 5, 3, 6]
print("distance = {distance}")   # 5 m
print("direction = {direction}")   # [0.6, 0.8, 0]
seal product
seal distance
seal direction
```

Normalization works in scaled coordinates. `vector_unit([1e308, 1e308])` remains
usable even though its unscaled magnitude cannot be represented as finite f64.
`vector_norm` refuses an overflowing magnitude rather than returning infinity.
Matrix products use compensated accumulation in a fixed traversal order;
non-finite intermediate products and sums are refused.

## Covariance has an explicit denominator

`covariance(values, observations, variables, ddof)` computes each entry as the
sum of centered products divided by `observations - ddof`:

- `ddof = 0`: the empirical population covariance of the supplied observations.
- `ddof = 1`: the usual sample covariance. Its unbiasedness requires the usual
  statistical assumptions; choosing this denominator does not establish that
  the data are independent, representative or free from measurement bias.

No other `ddof` is accepted. There must be more observations than `ddof`. A
constant column can produce zero variance; an estimated covariance matrix is
allowed to be singular. Cholesky and sampling will refuse a singular matrix.

```goblin
measurements = [1 m, 2 m, 2 m, 4 m, 3 m, 6 m]
sample_covariance = covariance(measurements, 3, 2, 1)
print("sample covariance = {sample_covariance}")
# Flat entries: [1 m^2, 2 m^2, 2 m^2, 4 m^2].
seal sample_covariance
```

Centering anchors each column to its first observation when finite differences
can be formed. It then scales and uses compensated summation. If subtracting
opposite extremes overflows, it scales the originals instead. Covariances are
filled symmetrically from one calculation per pair. Rescaling multiplies the
smallest and largest factor magnitudes first to avoid unnecessary intermediate
overflow or underflow. Unrepresentable/non-finite results are refused; ordinary
finite f64 rounding, including very small values rounding to zero, still applies.

The sample-covariance definition is documented by the
[NIST Engineering Statistics Handbook](https://www.itl.nist.gov/div898/handbook/pmc/section5/pmc541.htm).
Goblin++ uses its own bounded implementation; it does not call NIST software.

## Cholesky: validate, do not repair

Cholesky requires a numerically positive-definite matrix. It returns `L` such
that the declared lower-triangle matrix is approximately `L * transpose(L)`.
Singular and indefinite inputs are refused. A tiny or ill-conditioned matrix
can be refused if its pivots are not strictly positive at available precision;
there is no silent jitter, pivot clipping, diagonal loading, nearest-positive-
definite repair or averaging of disagreeing triangles.

`symmetry_tolerance` is a finite nonnegative dimensionless **relative** tolerance.
For a pair of entries `a` and `b`, the check uses
`abs(a / scale - b / scale) <= tolerance`, where
`scale = max(abs(a), abs(b))`. A pair of zeros agrees exactly. There is no
absolute tolerance floor such as `max(scale, 1)`.

Use `0` to require exact symmetry. If a positive tolerance accepts a small
triangle discrepancy, **the lower triangle is the declared input**; the upper
triangle is checked but never averaged into it. Set the tolerance deliberately
and preserve it in the source/run evidence. Larger tolerances deliberately admit
larger discrepancies; they are not recommended defaults or a conditioning test.

```goblin
covariance_values = [4 m^2, 2 m^2, 2 m^2, 3 m^2]
lower = cholesky(covariance_values, 2, 0)
lower_transpose = matrix_transpose(lower, 2, 2)
reconstructed = matrix_multiply(lower, 2, 2, lower_transpose, 2, 2)
print("lower = {lower}")
print("reconstructed = {reconstructed}")
seal lower
seal reconstructed
```

The factorization uses global scaling and a fixed lower-triangle traversal.
`cholesky` does not promise to certify conditioning, solve linear systems or
prove the scientific validity of a covariance model. The positive-definite
lower-factor convention is also described in
[LAPACK's DPOTRF reference](https://netlib.org/lapack/explore-html/d2/d09/group__potrf_ga84e90859b02139934b166e579dd211d4.html).
Goblin++ does not bundle LAPACK or claim its numerical performance guarantees.

To avoid accepting a singular matrix because cancellation left a tiny positive
residual, a scaled diagonal pivot must exceed
`16 * f64::EPSILON * dimension * max(abs(scaled_diagonal), abs(product_sum))`.
This fixed conservative roundoff bound is separate from the user-declared
symmetry tolerance. It can refuse a mathematically positive-definite but
numerically near-singular matrix; it never changes a pivot to make one pass.

## One multivariate-normal sample per call

```goblin
GO_PARANOID
rng_seed(42, 54)

mean_position = [10 m, 20 m]
position_covariance = [4 m^2, 2 m^2, 2 m^2, 3 m^2]
position_draw = mvnormal(mean_position, position_covariance, 2, 0, 54)

print("sampled position = {position_draw}")
seal position_draw
```

Every input, shape, dimension, symmetry check and positive pivot is validated
before random draws. The specified stream must already have an explicit seed.
Each component obtains one standard normal from the fixed Box-Muller mapping in
the resampling policy; there is no cached spare. A `dimension`-component sample
consumes `2 * dimension` uniform operations and `4 * dimension` PCG raw words.
The result is `mean + L * z`. The RNG state is committed only when the complete
sample succeeds; refusal cannot partially advance the stream.

PCG raw words, uniform mapping, seeds, stream identities and replay evidence are
specified by [RANDOMNESS.md](RANDOMNESS.md). Box-Muller uses floating-point log,
cosine and square root. Their last bits can differ across platforms: an exact
uniform trace is **not** a guarantee of universally bit-identical normal samples.
Keep authoritative seals exact; compare scientifically derived quantities with
explicit tolerances. This is scientific simulation randomness, not encryption.

### Mixed-unit astrometry needs explicit standardization

This version does not directly accept a mean vector mixing angular position,
distance and velocity, or a covariance whose entries have differing physical
dimensions. For a scientifically justified model, form a dimensionless
correlation matrix, draw zero-mean standardized components, then explicitly
apply each component's scale and offset in its own unit:

```goblin
rng_seed(2026, 7)
correlation = [1, 0.25, 0.25, 1]
standardized = mvnormal([0, 0], correlation, 2, 0, 7)
distance_draw = 100 m + standardized[0] * 2 m
speed_draw = 3 m/s + standardized[1] * 0.1 m/s
```

This example illustrates dimensional bookkeeping, not a validated Gaia model.
Coordinate conventions, correlation matrices, uncertainty distributions and
physical constraints remain the researcher's explicit responsibility.

## Bounded work and current limitations

- Every input/output matrix or vector has at most 100,000 entries.
- Covariance variables, Cholesky dimension and `mvnormal` dimension are at most
  128; covariance observations must also fit the entry limit.
- Matrix multiplication additionally checks a 50,000,000-scalar-product work cap.
- These limits are independent of a program's declared loop budget. Raising the
  loop budget does not remove built-in bounds, allocation limits or RNG limits.
- Dense matrices are in-memory only. There is no sparse, GPU, distributed,
  streaming-matrix or automatic chunked matrix execution.
- Zero dimensions, empty vectors, ragged/nested arrays, NaN and infinity are
  refused; no implicit data filtering or missing-value policy is applied.
- No general matrix inverse, eigen decomposition or positive-semidefinite
  sampler is provided in this stage.

The matrix functions use the same implementation in the interpreter and
compiled data runtime. Audit receipts preserve exact sampled outputs and RNG
evidence; a hash is not evidence that a covariance assumption is scientifically
correct.
