# Numerical reproducibility — alpha.22

Goblin++ now distinguishes **unchanged evidence**, **identical floating-point
bits**, and **agreement within a tolerance you explicitly choose**. They are
different claims. None establishes scientific truth or authenticates authorship.

## Quick start

```goblin
GO_PARANOID

observed = 1.000000000001 m
expected = 1 m
absolute_tolerance = 2e-12 m
relative_tolerance = 1e-12

acceptable = is_close(observed, expected, absolute_tolerance, relative_tolerance)
identical = same_bits(observed, expected)
print("within my tolerance = {acceptable}; identical bits = {identical}")
seal observed
seal absolute_tolerance
seal relative_tolerance
seal acceptable
```

Run `goblin++ examples/numeric_comparison.gbl`, or add `--compile` to test the
native engine. The expected Boolean results are `true` and `false` respectively.

`is_close(a, b, absolute_tolerance, relative_tolerance)` requires **all four
arguments**. There are no default tolerances. Both operands and the absolute
tolerance must have matching physical dimensions; relative tolerance must be
dimensionless. Values and tolerances must be finite, and tolerances nonnegative.
Matching dimensions do not imply matching physical meaning: you still choose
whether comparing those measurements makes scientific sense.

The symmetric, inclusive comparison policy is:

```text
abs(a - b) <= max(absolute_tolerance,
                  relative_tolerance * max(abs(a), abs(b)))
```

The implementation tests the absolute condition first, then a normalized relative
difference, avoiding overflow in the tolerance product and in differences of
opposite extreme values. All operations are finite IEEE f64 arithmetic, not
arbitrary-precision real arithmetic. The policy is **max**, not the additive
absolute-plus-relative rule used by some other libraries. A relative tolerance
of 2 may accept opposite values: that is your declared policy, not a useful
default. Choose tolerances from measurement uncertainty and the analysis goal.

`same_bits(a, b)` requires matching dimensions and compares the finite SI f64
bits. Unlike numeric `==`, it distinguishes positive and negative zero.
`is_close(0, -0, 0, 0)` is true; `same_bits(0, -0)` is false. These helpers return
Booleans, not quantities, and support scalar quantities only. Consume their
results; a discarded direct call is refused. New user functions cannot redefine
these builtin names. Rename any colliding user function; use a revision for a
frozen source. Historical syntax/canonical evidence remains verifiable.

## Comparing preserved runs

```sh
goblin++ compare-value PATH_TO_RUN_A PATH_TO_RUN_B observed \
  --absolute-tolerance-si 2e-12 --relative-tolerance 1e-12
```

Both flags are required. The absolute flag is in the sealed quantity's **SI
units**, not its display units: for a length it means metres. The report includes
the dimension vector and SI unit label. Relative tolerance is dimensionless.
Add `--json` for a structured report including both receipt hashes, exact bit
patterns, math policies and build environments.

Both runs must have status `PASS` and pass existing exact SHA-256 verification.
The named seal must be a scalar quantity with matching dimensions in both runs.
Arrays, text, Booleans, missing/duplicate names, damaged evidence and unsafe
artifact paths are refused. Partial outputs from failed runs are not compared.

Possible labels:

| Label | Meaning |
| --- | --- |
| `BITWISE_IDENTICAL` | The selected SI f64 values have identical bits. |
| `WITHIN_DECLARED_TOLERANCE` | Different bits, accepted by your explicit tolerance. |
| `OUTSIDE_DECLARED_TOLERANCE` | Different bits, rejected by that tolerance. |

Exit codes are 0 for either accepted label, 1 outside tolerance, 2 for refusal.
The command is read-only; it does not amend receipts, seals, ledgers or freezes.
Its scope is the **one named scalar**, not the whole program or experiment.
Comparing values from different math policies is allowed only as this explicitly
limited claim; both policies remain visible, including `null` for legacy runs.

## Evidence stays exact

`verify` still checks stored source, receipts, artifacts and evidence using exact
hashes. A tolerance can never make damaged evidence pass. Existing `diff` remains
an exact comparison and now also reports `MATH_POLICY_SAME` and
`MATH_ENVIRONMENT_SAME`. A policy mismatch is classified `MATH_POLICY_CHANGE`
(unless a language-semantics change takes precedence). Matching sealed bytes are
not the same as a matching build environment. Environment differences are
reported but do not themselves establish changed scientific meaning.

## Backend and build identity

Alpha.22 retains Rust's standard/platform floating-point backend. It does **not**
promise universal cross-platform bitwise agreement for transcendental functions.
Rust documents unspecified precision for functions such as
[sin](https://doc.rust-lang.org/std/primitive.f64.html#method.sin). No deterministic
math crate was added, and the platform math library is explicitly recorded as
`UNPINNED_PLATFORM_IMPLEMENTATION`.

New run receipts hash `math_policy` and `math_environment` with the receipt core:

- policy `goblin.rust-platform-finite-f64.v1`, comparison-policy identifier,
  exact evidence-verification mode, no default tolerance and no cross-platform
  bitwise guarantee;
- launcher build compiler version, target, profile, target features and the
  executing launcher binary's SHA-256;
- for native runs, the actual native compiler version and generated binary hash.

The launcher may be the CLI, an embedding application or a test harness. These
fields describe the observed build, not an authenticated or hermetic environment.
They do not pin OS library versions, all compiler flags or hardware behavior.
Existing receipt environment fields remain available. `compile --json` includes
policy/build metadata, but directly running a standalone native executable still
does not create an audited run; use `goblin++ run FILE --compile` for preserved
receipts. Inline Rust remains separately approved and outside any general math
reproducibility guarantee.

New freezes pin the math policy, **not** a false promise of matching all future
platform bits. A changed declared policy blocks execution with a preserved
`MATH_POLICY_CHANGED_AFTER_FREEZE` protocol violation, and blocks standalone
compilation. Freeze hash integrity and execution-policy compatibility remain
separate questions. Legacy freezes without a math policy are not retrospectively
claimed to pin one; their existing source/constants/language gates still apply.
Historical receipts are not rewritten or upgraded.

## Measured platform gate

`tests/fixtures/platform_math.json` defines 25 named finite cases: analytic
identities or rounded mathematical references, signed zero, roots, large/tiny
values, explicit-angle trig, inverse/domain-edge trig and logarithms. Five cases
require exact reference bits; the rest declare their own engineering-test
tolerances. These are regression tolerances, **not scientific defaults** or a
proof of accuracy over every input domain.

The Rust test runs the same source through both engines, verifies each run and
optionally preserves reports plus complete run evidence:

```sh
GOBLIN_MATH_REPORT_PATH=target/new-math-evidence/report.json \
  cargo test --locked --test numeric_reproducibility shared_platform_math_fixtures
```

The destination must be new; reports/evidence are never overwritten. CI preserves
macOS and Linux measurements separately, re-verifies both archived runs, checks
reported bits against seals, then compares all four engine/platform combinations.
The final gate requires actual macOS arm64 and Linux x86_64 observations and
refuses to infer coverage from labels. It never executes archived native binaries.
CI artifacts are named by runner and attempt, so retries preserve earlier
evidence. Retrying failed jobs can reuse the latest successful report for the
other platform, but source, version, fixture and policy identities must agree.

```sh
python3 tools/compare_math_reports.py MAC_REPORT.json LINUX_REPORT.json \
  --goblinpp target/debug/goblinpp --require-macos-linux
```

Cross-platform validation is pending until that CI gate actually passes. Local
Mac tests establish only local behavior. Do not publish a cross-platform claim
based on two copies of the same local report.

The maintained Chinese documentation remains its reviewed alpha.21 snapshot;
the community-review proposals are unchanged. This new English guide documents
the alpha.22 additions without silently editing that historical edition.
