# 0.1.0-alpha.27 (local, unreleased, auditable inference foundations)

- Explicit-stream resampling, bootstrap means/medians and Box-Muller standard
  normal draws using the existing PCG32 v1 seed/stream contract. No implicit
  interval or seed. Invalid helper calls leave their RNG transaction unchanged.
- Flat row-major matrix transpose/multiplication, safe vector norm/unit,
  covariance with explicit ddof, positive-definite Cholesky with explicit
  symmetry tolerance, and multivariate-normal sampling. No silent regularization.
  Homogeneous units only; mixed-unit Gaia covariance requires standardization.
- `GO_LOOP_BUDGET` supports one main-source declaration up to 50,000,000 shared
  loop-body entries. Requested/effective/used counts and resource policy are
  preserved and native/reference checked; builtin work limits remain separate.
- `csv_scan_stats`/`tsv_scan_stats` incrementally reduce a named numeric column:
  [count, sum, mean, min, max], with exact parsed-byte hash/evidence preservation.
  Files up to 1 GiB, records up to 1 MiB, at most 1,024 columns. Missing or
  malformed selected data refuses; ordinary array/table caps remain unchanged.
- Scientific helper summaries and versioned policies are in receipts/freezes
  and exact diffs. Summaries are integrity evidence, not mathematical replay.
  Existing raw PCG trace replay remains intact. Exact outputs are not rounded
  to conceal cross-platform normal-transform differences.
- VS Code 0.1.22 and JetBrains 0.3.12 include completion/help/highlighting.
  Rust remains 1.92.0, dependencies unchanged; Chinese/community-review files
  are untouched. This local stage is not yet a GitHub/Zenodo publication.
- Encryption, `COMPLETELY_MAD` and a custom IDE are not shipped features.
- Parser policy v2 preserves alpha.25/26 grammar for historical verification,
  including formerly ordinary `GO_LOOP_BUDGET` identifiers. Existing v1 freezes
  require an explicit revision before execution with the new syntax policy.

See [RESAMPLING.md](docs/RESAMPLING.md),
[MATRICES_COVARIANCE.md](docs/MATRICES_COVARIANCE.md), and
[RESOURCES_STREAMING.md](docs/RESOURCES_STREAMING.md).
Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.26 (seeded scientific randomness)

- Explicit PCG32 XSH-RR setseq v1 seeds/streams in both engines. Four new calls:
  `rng_seed`, `rng_word`, `rng_uniform`, `rng_integer`. Half-open mappings,
  exact seed text encoding, no automatic seed/reseeding, bounded resources.
- Run receipts include explicit no-RNG usage or seeded operation traces, exact
  raw-word/result digests and final state. Independent verification replays the
  bounded trace. Compiled runs compare native/reference evidence; standalone
  successful binaries preserve a native-data manifest.
- Freeze policy enforcement, tamper/refusal tests and exact RNG diff fields.
  Earlier non-RNG receipt/freeze behavior is preserved; historical artifacts
  are not rewritten. New builtins cannot silently replace user functions.
- VS Code 0.1.21 and JetBrains 0.3.11 provide completions and help. Rust stays
  at 1.92.0; Cargo dependencies are unchanged. PCG reference Apache-2.0
  attribution/license is bundled. Cross-platform RNG fixture comparison is
  added to CI; local passing tests are not a claim that remote CI has run.
- RNG is not cryptographic. Bootstrap, normal/covariance sampling, encryption
  and `COMPLETELY_MAD` remain future work. A custom IDE is not planned.

See [RANDOMNESS.md](docs/RANDOMNESS.md).
Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.25 (compound-unit input)

- Both engines accept numeric compound-unit suffixes, including the reported
  `1.13e-10 m/s^2` case, SI scaling, signed integer powers and grouped denominators.
- Coefficients are separate from unit powers: `3 m^2` is 3 square metres;
  `(3 m)^2` is 9. Review old unfrozen quantity powers before adopting this stage.
- Multiple denominator factors require parentheses; ambiguous slash chains
  refuse. Rendered multi-factor denominators now have explicit parentheses.
- New receipts/freezes/checks declare the parser policy. Historical evidence
  and module graphs use the legacy parser, not a reinterpreted canonical hash.
  Old freezes require explicit revision; violations remain preserved evidence.
- Unit scales, exponent bounds and dimensional arithmetic refuse invalid input;
  arrays, libraries, native standalone programs and simple historical hashes
  are covered. No new units, constants, dependencies or RNG have been added.
- VS Code 0.1.20 and JetBrains 0.3.10 provide compound-unit help/highlighting.
- Retain Rust 1.92.0; CI pins it instead of automatically updating stable Rust.
  No user compiler installation or existing released artifact was changed.
- Chinese/community-review snapshots remain unchanged. Cross-platform results
  await this stage's CI; exact seals are not rounded to conceal differences.

See `docs/COMPOUND_UNITS.md` and `examples/compound_units.gbl`.

Spread the word with [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk).
Buying merchandise is optional; Goblin++ remains free to use.

## Previous stage: 0.1.0-alpha.24 (distribution statistics)

- `sort` returns a stable ascending numeric copy; signed-zero ties preserve
  source order. Empty sorting is allowed; reductions require finite samples.
- `median` and `quantile(values, p)` use Hyndman–Fan type 7 with explicit
  dimensionless p in [0, 1], no boundary fuzz and overflow-safe interpolation.
- `std_population` uses divisor n; `std_sample` uses n-1 and requires two
  observations. Anchored, scaled compensated computation avoids avoidable
  large-offset cancellation and intermediate overflow.
- `ecdf(values, query)` is unweighted count(value <= query)/n, including ties.
  Query dimensions must match observations; results are dimensionless.
- Both engines share the implementation and refuse invalid types, dimensions,
  empty reductions and nonfinite results without silently dropping rows.
- New run and freeze receipts pin the algorithm-source hash and conventions;
  policy changes are distinct in exact diffs and frozen execution refuses them.
  Historical evidence remains verifiable without inventing undeclared policy.
- New builtin collisions refuse execution; historical user-function syntax
  remains parseable for canonical verification. Discarded results refuse.
- VS Code 0.1.19 and JetBrains 0.3.9 supply matching completion and help.
- Tests extend the shared platform fixtures. Local verification is not a claim
  of universal bitwise reproducibility or complete WB-3 validation. No new
  dependencies; historical Chinese/community-review material is unchanged.

See `docs/STATISTICS.md` and `examples/statistics_distribution.gbl`.
Seeded RNG, bootstrap, weighted distributions and covariance remain future work.

## Previous stage: 0.1.0-alpha.23 (combined FITS extraction)

- Versioned `fits_where`, `fits_all` and `fits_any` predicates support explicit
  scalar-column comparisons, nested AND/OR, and null/valid tests.
- `fits_export_csv` / `fits_export_tsv` project selected columns in original
  row order and record source hash, cuts, schema, counts and null policy in
  generated artifact metadata. Outputs stay inside the run and are hashed.
- Unscaled signed 64-bit FITS IDs are selected/exported without f64 conversion;
  `fits_column_text` reads them exactly. Existing numeric FITS access refuses
  unsafe integer cells. This is not a general exact integer arithmetic type.
- FITS scaling preserves signed zero when the physical offset is zero. This
  fixes a round-trip regression found while testing the subset exporter.
- Bounded 8 MiB scans work beyond array/loop limits; generated output remains
  bounded to 64 MiB and is not a streaming writer. Empty selections produce
  header-only files; unsupported types/scales and ambiguous exports refuse.
- Interpreter and compiled support share the same extraction helpers. New
  predicates do not prevent historical canonical source verification.
- Function argument evaluation is kept outside the large builtin dispatch
  frame so recursion reaches the existing 16-call refusal instead of aborting
  the process. Linux-sized stack, caller restoration, side-effect ordering and
  standalone native depth-boundary regressions cover this repair.
- VS Code 0.1.18 and JetBrains 0.3.8 include matching completion and help.
- No new dependencies. Historical Chinese/community-review material is unchanged.
  Extraction retains full input evidence and is not physical blinding.

See `docs/FITS_SUBSETS.md`, `examples/fits_subset.gbl` and `docs/WB3_ROADMAP.md`.
Statistics, seeded RNG and GO_MAD remain future stages. No claim of full WB-3
validation is made.

## Previous stage: 0.1.0-alpha.22 (explicit numerical comparison)

- `is_close(a, b, absolute_tolerance, relative_tolerance)` requires four explicit
  arguments; dimensions, finiteness and nonnegative tolerances are checked in
  both engines. `same_bits(a, b)` includes the sign of zero.
- Read-only `compare-value` requires exact verified successful runs and two
  explicit tolerance flags; scalar-only numeric agreement is never evidence
  verification or scientific validation. `diff` remains exact.
- New receipts hash math policy, launcher build/binary identity and native
  compiler/binary identity. New freezes pin policy; historical evidence and
  undeclared legacy policy are not rewritten.
- A shared 25-case fixture and CI artifact gate measure both engines on actual
  macOS arm64/Linux x86_64. Cross-platform results remain pending until CI passes.
- VS Code 0.1.17 and JetBrains 0.3.7 include comparison completion and help.
- Packaging retains the Chinese documentation snapshot and community-review
  proposals unchanged. No dependencies or deterministic math backend added.

See `docs/NUMERIC_REPRODUCIBILITY.md` and `examples/numeric_comparison.gbl`.

## Previous stage: 0.1.0-alpha.21 (lossless numeric text)

- One shared shortest-round-trip formatter for default finite numeric text in
  both engines: print, eager placeholders, to_text, text/CSV/TSV cells and arrays.
  Signed zero is preserved. Explicit presentation precision remains unchanged.
- JSON/sealed serialization and exact-bit native manifests remain intact.
  Nonfinite and unsafe integer input refusals are retained.
- The hashed semantics policy advances to round-trip-numbers.v2. Historical
  evidence still verifies; alpha.20 and older frozen execution requires an
  explicit revision. Output hashes may change.
- VS Code 0.1.16 and JetBrains 0.3.6 include matching completion help.

See `docs/NUMERIC_TEXT.md` and `examples/numeric_text.gbl`. No dependencies
were added. Cross-platform math, compound-unit literals and GBL-005 onward
remain queued. This is not an arbitrary-precision or exact-integer feature.

## Previous stage: 0.1.0-alpha.20 (protected values)

- Every registered constant alias is read-only, including `h` (Planck's
  constant). Assignments are refused before evaluating their right-hand
  side; nested bindings are checked. Expressions, templates and `seal h`
  resolve the same registry value and record its use.
- Discarded calls to registered value-only builtins are refused. `append`
  remains copy-based: use `a = append(a, value)`. Effectful output/input and
  user-function calls remain valid statements.
- Source string literals capture `{name}` / `{name:.Nf}` placeholders
  when evaluated, including assignments, function returns and array items.
  Output and input prompts never reinterpret stored text. User input,
  arguments and imported data remain plain text. `{{name}}` writes `{name}`.
- Interpreter and native programs share the bounded template scanner. New
  runs/freezes record a hashed `language_semantics` policy. Historical runs
  and old freeze integrity remain verifiable, but old frozen sources require
  explicit revision to adopt changed execution semantics. Diff reports flag
  `LANGUAGE_SEMANTICS_CHANGE` even if outputs agree.
- VS Code 0.1.15 and JetBrains 0.3.5 document protection, append result
  consumption and immediate capture in completion help.

No dependencies were added. Numeric round-trip formatting, deterministic
math, bulk FITS selection/export, seeded RNG and larger budgets remain queued.
See `docs/PROTECTED_VALUES.md` and `examples/protected_values.gbl`.

## Previous stage: 0.1.0-alpha.19 (tables/modules/native I/O)

Added ten explicit CSV/TSV readers: row/column counts, headers, text columns
and checked numeric columns. UTF-8, quotes, strict row shape, unique headers,
resource limits and refusal of missing/nonfinite numeric values are enforced.
Input bytes and requested accesses are preserved and independently verified.

Added top-level `import "relative/library.gbl"` for reusable `g_func` libraries.
Libraries contain definitions/imports only; no hidden top-level execution.
Cycles, symlinks, path traversal and duplicate names fail. Frozen programs pin
the raw library graph; canonical hashing and run verification include it.

The compiler now supports existing FITS readers, file writers and plots plus
the new table readers through shared Rust helpers. No Goblin++/Python process
or source parsing is used at native runtime. Audited runs compare native and
reference input/output evidence and bytes; generated support sources are
preserved in a hashed inventory. Data compilation uses locked offline Cargo
and requires cached dependencies. Standalone executables have explicit,
separate output/custody boundaries.

VS Code 0.1.14 and JetBrains 0.3.4 add matching highlighting and completion.
See `docs/DATA_MODULES_NATIVE.md` and `examples/csv_modules.gbl`.
No dependencies were added. No bulk export, FITS writing, automatic unit or
missing-value inference, external package imports or additional statistics
are claimed by this stage.

## Previous stage: 0.1.0-alpha.18 (statistics step 1)

Added `sum(array)` and `mean(array)` to both execution engines. They require
non-empty homogeneous numeric arrays, preserve SI dimensions, and use one
shared compensated summation implementation. Mean uses magnitude scaling to
avoid avoidable sum overflow. Empty, nonnumeric, incompatible, and overflowing
inputs fail explicitly; no values are silently skipped. Existing variables
named `sum` or `mean` remain valid, but those names are now reserved for built-in
function calls and cannot name user functions or function parameters.

The current editor sources add completion and help for both functions. This is
only the first statistics step: median, variance, standard deviation, weights,
CSV reading, and uncertainty propagation are not implemented by this change.
See `docs/STATISTICS.md` and `examples/statistics_basics.gbl`.

## Previous stage: 0.1.0-alpha.17

Added electric current as a sixth SI base dimension, 34 electrical unit
spellings, and 18 `ee_` functions in both execution engines. The helpers cover
Ohm's law, signed power and energy, charge, series/parallel resistance, RC time
constants, ideal capacitor/inductor energy, explicit KCL/KVL residual and
tolerance checks, and electrical unit reporting.

New run and freeze evidence includes the versioned electrical registry and
six-axis dimensions. Historical constant-only and alpha.16 scientific freezes
retain their original five-axis checks; old five-axis native manifests are
accepted with zero current exponent. Generic inverse-time display remains
`1/s` so existing angular-velocity output is preserved.

VS Code 0.1.12 and JetBrains 0.3.2 include matching completion, highlighting,
units, and function help. See `docs/ELECTRICAL_ENGINEERING.md` and the runnable
`examples/electrical.gbl`.

## Previous stage: 0.1.0-alpha.16

Added a deliberately scoped chemistry foundation to both execution engines:
a versioned 43-element common-element registry, exact-symbol atomic lookup,
bounded chemical-formula molar mass, and dimension-checked helpers for moles,
mass, amount concentration, and dilution. Added practical volume, wavelength,
pressure, and atomic-mass units plus `R` and `m_u`.

Chemistry registry identity and SHA-256 now appear in run evidence, and the
combined scientific registry used by freeze receipts includes constants,
units, and chemistry reference data. Unsupported formula syntax and dimension
errors fail explicitly. Isotopes, charge, hydrates, reactions, balancing, pH,
equilibrium, kinetics, thermodynamic state, uncertainty, and biological
sequence semantics remain intentionally outside this stage.

Historical freeze receipts through alpha.15 retain their original
constant-only registry check and remain verifiable. New alpha.16 freezes bind
the expanded scientific registry; verification labels the historical path
instead of claiming that an old receipt covered data that did not yet exist.

The VS Code 0.1.11 and JetBrains 0.3.1 editor sources add matching chemistry
completion, documentation, highlighting, constants, and units; runtime
semantics remain authoritative.

## Previous stage: 0.1.0-alpha.15

Added dimension-checked astronomical distance conversions, homogeneous vector
`magnitude`/`dot`/`cross`, explicit Cartesian/polar/spherical conversion, linear
velocity, explicitly named Galilean and collinear special-relativistic velocity
addition, angular velocity, tangential velocity, centripetal acceleration, and
angular momentum. All functions run in both the Rust interpreter and native
compiler.

Coordinate and angle conventions are part of the public interface: `d` means
degrees, `r` means radians, spherical inclination is measured from +z, and
azimuth is measured from +x toward +y. Inputs with invalid dimensions,
ambiguous origins, invalid radii or inclinations, non-positive elapsed time, or
superluminal relativistic operands are refused. This stage deliberately does
not claim general-relativistic, reference-frame, tensor, orbital-propagation,
or uncertainty semantics.

## Previous stage: 0.1.0-alpha.14

Added explicit angle-convention trigonometry to the Rust interpreter and native
compiler. `sind`/`cosd`/`tand` accept numeric degrees;
`sinr`/`cosr`/`tanr` accept numeric radians; inverse and two-argument forms use
matching `d` or `r` suffixes for their result. `deg2rad` and `rad2deg` provide
explicit numeric conversion. Inputs remain dimensionless quantities in this
stage; `deg` and `rad` are not silently introduced as physical units.

The alpha.13 unsuffixed names retain their radians behavior for compatibility
and emit one preserved `G302` migration warning per function name per run.
They do not silently change meaning. Both execution engines, warning evidence,
domain and dimension refusals, native result agreement, documentation,
examples, capabilities output, and VS Code authoring support are covered by
the alpha.14 gates.

## Previous stage: 0.1.0-alpha.13

Added dimension-aware scientific mathematics to both the Rust interpreter and
native compiler: `abs`, `sqrt`, variadic `min`/`max`, `floor`, `ceil`,
`round`, `exp`, `ln`, `log10`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`,
`atan2`, and `hypot`. Square root accepts only non-negative quantities with
even unit exponents. Dimension-sensitive functions require compatible units;
transcendental and rounding functions require dimensionless values. Domain,
dimension, arity, overflow, and interpreter/compiler agreement paths are
covered by preserved and independently verifiable tests.

The VS Code editor source now highlights, completes, and documents these
built-ins. This stage deliberately does not add implicit angle units,
fractional dimensions, complex numbers, or silent NaN/infinity propagation.

## Previous stage: 0.1.0-alpha.12

Everyday-language usability stage. Added short-circuit Boolean `and`, `or`,
and `not`; nearest-loop `break` and `continue`; direct iteration over an
independent one-dimensional array value; checked integer `%`; and
`parse_integer(text)`. All additions run in both the Rust interpreter and
native compiler, retain the shared loop budget, produce new canonical AST
forms without changing older program hashes, and have positive and negative
acceptance tests. The VS Code extension source is updated for the new syntax.

The integer facilities deliberately remain within exactly representable
`f64` integers (±(2^53−1)); this is not an arbitrary-precision integer type.
Direct iteration does not stream FITS rows. Compiled FITS/output parity and a
module system remain later, separately reviewable milestones.

### Packaging and third-party notices

The macOS arm64 binary archive includes `THIRD_PARTY_NOTICES.md` and the
`third-party/` notice bundle generated from the exact locked normal dependency
tree and active Rust standard-library documentation. The bundle preserves the
upstream license files for all 40 external packages in that target tree,
including the additional Unicode terms used by `unicode-ident`, together with
the Rust library copyright and license files. The collector is reproducible
offline with `tools/collect_third_party_notices.py`. This evidence is
target-specific and must be regenerated when the lockfile, Rust toolchain,
target, features, or packaging changes.

## Previous stage: 0.1.0-alpha.11

Added `fits_select_stats` for a bounded, full-table scalar-numeric FITS
selection and optional positive weights. The half-open interval and column
names are explicit; selected and usable row counts are returned alongside the
weight sum and mean. FITS null/scaling rules apply, non-positive weights and
non-finite accumulation fail, and run receipts record selection parameters.
The bundled three-row fixture is a functionality test, not a scientific DESI
result. FITS calls remain interpreter-only. Everyday text operations and
prompted `input()` were already implemented and are now called out explicitly
in the public-release checklist.

## Previous stage: 0.1.0-alpha.8

Added one-dimensional homogeneous arrays, zero-based indexing and indexed assignment, half-open slices, `len`, and `append` to both Rust execution engines. Array assignment, slicing, and append use independent value copies. Array seals, native parity manifests, canonical hashes, freeze refusal, and negative paths are tested. The 0.1.0-alpha.7 interaction work is included: `input`, read-only `argc`, `argv`, `--` program arguments, preserved interaction evidence, and an editor prompt bridge. Arrays are capped at 100,000 elements; nested arrays and numeric conversion of input text are not yet supported.

## Previous stage: 0.1.0-alpha.6

Added Goblin++ `if`/`else if`/`else` and `switch`/`case`/`default` to both the Rust interpreter and native compiler. `if` conditions must be Boolean. `switch` evaluates the selector once, tests labels lazily in order with exact, dimension-aware equality, executes the first matching case without fallthrough, and optionally executes a final `default`. Branches compose with loops and share the existing run, freeze, verification, and optional sealing policies. Existing canonical hashes remain unchanged.

The branch acceptance tests cover interpreter/compiler agreement, no fallthrough, lazy conditions and labels, nested loops, type/dimension failures, malformed syntax, verifiable failures, and frozen-source refusal. FITS and generated-output calls still cannot be natively compiled, even in an unreachable branch; interpret those programs instead. Collections, user-defined functions, Boolean combinators, and authenticated ledger authorship remain future work.

## Previous stage: 0.1.0-alpha.5

Everyday programming stage: `GO_PARANOID` and `seal` are optional. `print`, the safe generated-output functions, FITS access, and loops work without either directive. File output remains confined to a unique run directory, capped, hashed, and verifiable. `seal` continues to snapshot named values when explicitly requested.

`GO_PARANOID` now has a concrete additional evidence policy: postflight source re-observation with preserved bytes, mismatch/missing-evidence refusal, and an independent receipt/evidence verification gate before custody-ledger registration. A failed self-check becomes `G405` and cannot register an unverified receipt. Run receipt schema v2 records this evidence, while the verifier continues to accept v1. The postflight check detects bytes that differ at that instant; it is not continuous monitoring or an OS sandbox. Existing freeze, inline-Rust authorization, FITS and output safeguards apply in both modes.

The existing alpha.4 loop implementation remained unchanged. Its limits included missing conditionals, collections, user-defined functions, compiled FITS/output calls, and authenticated ledger authorship.

## Previous stage: 0.1.0-alpha.4

Everyday control-flow stage. `for ... in range(...) { ... }` and `while condition { ... }` now execute as Goblin++ syntax in both the Rust interpreter and native compiler.

Added Boolean literals, comparison operators, nested blocks, signed half-open ranges, and a global one-million-loop-body-iteration ceiling per run. Out-of-budget loops and invalid range/condition types fail explicitly and remain preserved as verifiable runs. Boolean seals are checked against compiled results. A compiled seal now snapshots its value at the `seal` statement, matching the interpreter even inside loops.

The old 0.0.7 grammar corpus remains under regression test; four formerly rejected comparison cases are explicitly reclassified as intentional alpha.4 extensions. Legacy canonical hashes, including the energy example, remain unchanged.

This is a useful control-flow foundation, not a claim that Goblin++ is already a complete everyday language. Conditionals, collections, user-defined functions, and bulk/weighted FITS row operations remain future work. The loop limit is not a sandbox for authorized inline Rust.
