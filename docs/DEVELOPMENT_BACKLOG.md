# Goblin++ development backlog

Logged 2026-10-02 from Malin Hess's WB-1 experience. Baseline:
`0.1.0-alpha.19`, source commit
`f4c69201f42f6b3d560a3befc32ddf362e01e2c2`.

This document records work to investigate, fix, or add. **None of these items
is marked implemented merely by creating this backlog.** Alpha.20 implements
GBL-001 through GBL-003; alpha.21 implements GBL-004. Alpha.22 implements the
GBL-005 policy/comparison stage; measured cross-platform CI validation passed
in Source checks #27 on the published alpha.22 source commit
`c3fff61a74cb3a1e4535d9c308bb23ce4e812691`. This is a finite fixture gate,
not a universal bitwise math guarantee.
Alpha.23 implements the GBL-006 CSV/TSV extraction stage locally; a general
exact-integer arithmetic type and FITS subset writer remain pending.
GBL-007 onward remain queued unless noted. No released tag or asset
should be overwritten. WB-1/WB-2 are the user's workflow labels; the complete
workflow, catalogue fixtures, and independent expected results have not been
reviewed in this logging pass. The 2026-10-04 WB-3 additions and proposed
strict research policy are logged in [WB3_ROADMAP.md](WB3_ROADMAP.md).

## Order and evidence

First resolve **GBL-001 through GBL-003**: accepted programs can silently do
something different from the user's intent. GBL-004 and GBL-005 belong to the
same correctness/integrity tier and must not be treated as cosmetic issues.
Next prioritise **GBL-006 through GBL-009** for the WB-2 workflow. Bootstrap
needs the seeded-RNG contract; large workflows also need the table-output and
resource-limit work in GBL-011/012. Do not claim WB-2 runs wholly in Goblin++
until the actual end-to-end workflow is tested.

Evidence labels:

- **Reproduced:** a minimal program was run against the local alpha.19
  release executable. This does not establish compiled or cross-platform
  behaviour unless explicitly stated.
- **Source-confirmed:** current implementation was inspected; the reported
  real-data workload was not reproduced.
- **Reported / validation needed:** preserve the user's observation without
  presenting it as an independently verified result.

| ID | Tier | Work item | Current evidence |
| --- | --- | --- | --- |
| GBL-001 | 1 | Refuse assignments to registered constants | Implemented alpha.20; both engines covered |
| GBL-002 | 1 | Prevent silently discarded pure-call results | Implemented alpha.20 for direct value-only builtin statements |
| GBL-003 | 1 | Make stored strings independent of later interpolation | Implemented alpha.20; user chose immediate capture |
| GBL-004 | 1 | Lossless default number-to-text formatting | Implemented alpha.21; bit-level/default export and both-engine gates |
| GBL-005 | 1 | Explicit cross-platform numeric reproducibility policy | Alpha.22 policy, hashed identity, explicit tolerances and fixtures; macOS/Linux CI gate passed |
| GBL-006 | 2 | Multi-column FITS cuts and subset export | Alpha.23 local: combined scalar cuts, exact unscaled i64 IDs and CSV/TSV export; FITS writing/blinding not implemented |
| GBL-007 | 2 | Median, quantile, sort, standard deviation, bootstrap | Source-confirmed gap in requested builtins |
| GBL-008 | 2 | Seeded, named, cross-platform RNG | Requested addition; no current RNG dependency/API found |
| GBL-009 | 2 | Declarable audited loop budget | Fixed cap source-confirmed |
| GBL-010 | 2 | Dimension-aware element-wise array maths | Requested extension to current vector/array support |
| GBL-011 | 2 | CSV/TSV table output from arrays | Scalar-cell API and text limits source-confirmed |
| GBL-012 | 2 | Larger or streaming input and array capacity | Current limits source-confirmed |
| GBL-013 | 3 | Compound unit literals matching rendered units | Previously reproduced; existing next-build note |
| GBL-014 | 3 | Explicit deliberate refusal | Requested addition; no `refuse` builtin found |
| GBL-015 | 3 | Exact 64-bit catalogue integers | Alpha.23 exact unscaled i64 FITS selection/text/export; general integer value/arithmetic type still queued |
| GBL-016 | 3 | Validate and explain minimum Rust version | Manifest says 1.92; 1.91 success is user-reported |
| GBL-017 | 3 | Gzip scientific input with original-file custody | Requested addition; gzip support not found |
| GBL-018 | WB-3 P0 | Matrix primitives and checked vector normalization | dot/cross/magnitude exist; matrices and norm/unit APIs queued |
| GBL-019 | WB-3 P0 | Covariance validation, Cholesky and multivariate sampling | Requires matrix, unit and RNG contracts; queued |
| GBL-020 | WB-3 P1 | Deterministic ID-based row partitioning | Requires exact ID encoding and versioned hash mapping; queued |
| GBL-021 | WB-3 P1 | Enforced allowed-column access and blind execution boundary | Not provided by GO_PARANOID; queued design/security gates |
| GBL-022 | WB-3 | Versioned science regression suite | Eight entropic runs and WB-1 proposed; fixtures/results not yet inspected |
| GBL-023 | WB-3 | GO_MAD strict research policy profile | Proposed name and contract only; not syntax or implemented isolation |
| GBL-024 | WB-3 P1 | Explicit constants namespace | Silent h collision fixed alpha.20; namespace design remains queued |
| GBL-025 | WB-3 P2 | Exact-source-ID table joins | Requires table and ID semantics; queued |

WB-3 promotes GBL-006/007/008/009/012 to P0; adds ECDF to GBL-007;
promotes GBL-013 to P1; and retains bootstrap as P1 dependent on GBL-008.
These are workflow priorities, not claims that the existing correctness
regressions or exact-ID dependencies may be skipped. See the roadmap for
acceptance gates and already-implemented distinctions.

## Tier 1: silent or misleading results

### User-supplied acceptance examples (2026-10-02)

Preserve these exact examples as the intended behaviour contract:

```goblin
h = 2                 # must refuse: h is a registered constant
a = []
append(a, 1)          # must refuse (or mutate a)
x = 1
s = "{x}"             # must interpolate now, or refuse
x = 2
write_text("t.txt", s)   # must never silently write 2
```

These are **requirements for the fix**, not a claim that alpha.19 currently
passes them. Implement as three isolated regression programs, not one
combined test: the first intentional refusal must not mask the other cases.

1. **Constant binding (GBL-001):** `h = 2` must fail with a diagnostic naming
   `h` as a registered constant, before it can be used as a variable.
2. **Empty-array append (GBL-002):** `a = []; append(a, 1)` must not silently
   pass with `a` still empty. Under the preferred copy-based policy, refuse
   the discarded result and explain `a = append(a, 1)`. A deliberate mutable
   design would instead need to demonstrate `a == [1]`; this alternative is
   not chosen or implemented by logging the example. Also test that assigned
   append from an empty array succeeds under the copy-based policy.
3. **Stored template (GBL-003):** run the four statements from `x = 1` through
   `write_text` independently. Snapshot interpolation must export the captured
   value `1` (with the writer's documented newline), or the assignment must
   explicitly refuse an unresolved stored template. A successful export of
   `2` is forbidden. Under the refusal policy, no `t.txt` should be generated.

Alpha.20 chooses independent-copy append plus discarded-call refusal, and
immediate template capture (not the alternative refusal policy).

Run each contract through checking, interpreted execution, and compiled
execution as appropriate to the chosen diagnostic stage. Verify preserved
failure evidence separately from scientific success. Test direct and stored
strings, loops, and escaped literal braces without introducing accidental
late interpolation or banning ordinary text.

### GBL-001 — Registered constants must not collide with variable assignments

**Alpha.20:** all constant aliases are protected and expression/template/seal
resolution agrees. The reproduction below records old alpha.19 behaviour.

Minimal reproduction:

```goblin
h = 9
print("{h}")  # Prints 9 from the environment.
print(h)      # Prints Planck's constant from the constant registry.
seal h        # Reads the assigned environment value.
```

The assignment is stored; it is not simply ignored. Expression-name
resolution prefers a parser-assigned constant ID, whereas interpolation and
sealing read the environment. This disagreement is itself an integrity risk.
`src/parser.rs`, `src/evaluator.rs`, and `src/compiler.rs` are relevant.

- Refuse ordinary assignment to every registered constant spelling/alias
  with a direct diagnostic, e.g. `h is a registered constant; choose another
  variable name`. Do not silently permit shadowing.
- Audit every binding route, including indexed assignment, function names,
  parameters, and loop variables. Some routes already reject reserved names;
  apply a consistent rule rather than assuming all are broken.
- Acceptance: enumerate aliases from the registry; cover `h`, `G`, `c`, `R`,
  `k_B`, `N_A`, and `pi` when registered; verify interpreter/compiler and
  `check` agree, failures are preserved, and expression/interpolation/seal
  cannot disagree about a successfully assigned value.

### GBL-002 — Discarded `append` must not silently succeed

**Alpha.20:** refuse discarded direct value-only builtin calls; retain copies.
User-defined/effectful statements are not a general unused-result checker.

```goblin
a = [1, 2]
append(a, 3)
print("{a}")  # Currently [1, 2], with RUN_STATUS=PASS.
```

`append` returns an independent array; an expression statement evaluates and
discards that result. This matches the existing copy-based array design, but
the statement's silent no-op is misleading.

- Preferred direction: preserve value/copy semantics and reject discarded
  pure-call results with `Use a = append(a, x)` guidance. Do not switch
  `append` to mutation without an explicit language-design decision.
- Distinguish pure/value-producing calls from legitimate effectful statements
  such as `print`, file writers, and deliberately side-effecting user
  functions. A blanket ban on every discarded call would break useful code.
- Acceptance: assigned append still works, copies remain independent,
  discarded append fails clearly, nested/block/function contexts are covered,
  and permitted effectful statements still run in both execution engines.

### GBL-003 — Stored text must not acquire values from a later environment

**Alpha.20:** Malin chose immediate capture. Source string expressions resolve
once; output never re-expands text. See [migration](PROTECTED_VALUES.md).
The following records the historical bug, not current expected behaviour.

```goblin
x = 1
line = "saved x = {x}"
x = 2
print(line)                         # Currently saved x = 2.
write_text("late_template.txt", line) # Currently writes saved x = 2.
```

Strings are stored literally. Output functions then interpolate their text
against the current environment (`export_cell` also does this for text
exports). Accumulating these strings in a loop can therefore capture the
wrong values at export time. `src/evaluator.rs` and generated interpolation
in `src/compiler.rs` need joint review.

- Decide and document when interpolation occurs. Options include snapshotting
  interpolated values at string creation or requiring an explicit formatting
  operation outside direct output templates. Ordinary literal braces must
  remain usable; do not mistake JSON or scientific text for templates.
- No implicit second interpolation of already constructed strings. Undefined
  placeholders, brace escapes, formatting specs, and function-local values
  need explicit semantics and helpful errors.
- Acceptance: a loop with changing values produces distinct correct rows;
  reassignment after text creation cannot silently alter exported content;
  repeated exports, CSV/TSV/text/JSON paths, direct print templates, and
  interpreter/compiler parity are tested.

### GBL-004 — Lossless default numeric text

```goblin
value = 0.12345678901234567
text = to_text(value)
restored = parse_number(text)
same = value == restored
print("{text}; {same}") # Alpha.19/20: 0.123456789012346; false.
```

The alpha.19/20 formatter uses 15 significant digits in scientific notation,
but 15 **decimal places** in its fixed-format branch. Neither is a general
f64 round-trip guarantee. There are separate interpreter/generated formatter
implementations. JSON output and receipt serialization have distinct paths;
do not assume every sealed value is already truncated in the same way.

Alpha.21 uses one shared shortest-round-trip formatter; the example now
prints `0.12345678901234566; true`. Signed zero is preserved. Explicit
precision remains rounded presentation. New freezes record the formatting
policy epoch; old frozen execution requires revision, not receipt rewriting.
See [the contract](NUMERIC_TEXT.md) and `tests/numeric_text.rs`.

- Use shortest round-trip decimal formatting for default numeric text and
  machine-readable exports. Up to 17 significant digits may be necessary;
  always printing 17 is not required. Retain explicitly requested rounded
  presentation such as `{value:.3f}`.
- Define signed-zero handling separately from visual formatting. Audit CSV,
  TSV, `to_text`, stdout, seal serialization, and compiled output paths.
- Acceptance: bit-level round trips for finite f64 values (with any documented
  signed-zero exception), subnormals, extremes and deterministic generated
  cases; preserve rejection of nonfinite values. Version formatting changes
  because output hashes will change; old preserved runs must still verify.

### GBL-005 — Cross-platform math and comparison policy

**Alpha.22:** retains the platform backend with explicit no-bitwise-guarantee
policy, hashes build identity, adds dimension-aware explicit-tolerance comparison
and a 25-case two-engine platform gate. Historical metadata stays historical;
exact verification remains exact. See [policy](NUMERIC_REPRODUCIBILITY.md).
The macOS/Linux CI comparison passed in Source checks #27 for the published
alpha.22 commit; no deterministic backend is claimed. Published provenance is
at https://github.com/mrgogobot/goblinpp/releases/tag/v0.1.0-alpha.22 and
https://zenodo.org/records/23126818.
The following preserves the original report and acceptance requirements.

User reports last-digit trigonometric differences between macOS arm64 and
Linux x86. This two-platform difference was **not reproduced in this pass**.
Current math uses Rust/platform floating-point operations rather than a
pinned deterministic math backend. Receipts already include runtime version,
OS, and architecture (`src/runtime.rs`); `diff` already reports environment
differences, but that is not a numerical tolerance policy.

- Decide whether supported operations promise bitwise reproducibility or
  scientific agreement within an explicitly declared tolerance. Investigate
  a pinned pure-software backend such as `libm`; do not claim a dependency
  alone guarantees all reductions, conversions, compiler optimisations, and
  floating-point behaviour across every target.
- Record math backend/version and relevant build/runtime identity. If numeric
  comparisons are added, use stated absolute/relative or ULP tolerances with
  dimension checks; never weaken byte/hash verification of stored evidence.
- Acceptance: shared fixtures on macOS arm64/Linux x86, both execution
  engines, domain boundaries and extreme inputs; exact and tolerant results
  have separate labels. Changing math policy must not silently change old
  receipt meaning or hide a real discrepancy.

## Tier 2: scientific workflow capability

### GBL-006 — Multi-column FITS selection and subset export

**Alpha.23 local:** implemented explicit scalar comparisons and nested AND/OR,
null/valid tests, stable-order CSV/TSV projection, exact unscaled signed i64
ID transport and recorded extraction policy/counts. Both engines and direct
native failure paths are covered. See [FITS_SUBSETS.md](FITS_SUBSETS.md).
General exact integer arithmetic, a FITS writer, streaming output, formal
allowed-column enforcement and real WB workflow validation remain pending.
The bullets below retain the original acceptance targets.

`fits_select_stats` currently supports a single selection column with a
half-open interval, target statistics, and optional weights. It does not
provide a general combined predicate and writable selected table.

- Add explicit combined cuts with documented AND/OR, boundary, null/invalid,
  weight, and row-order rules. Decide supported export formats: CSV/TSV first
  and/or FITS binary-table writing; do not imply FITS writing already exists.
- Preserve original data hashes, HDU/column identities, predicates, selected
  row counts, and generated subset hashes. Exact IDs depend on GBL-015 when
  exporting numeric identifiers; do not cast them through f64.
- Acceptance: synthetic independent expected subsets, empty/all selections,
  multiple columns, nulls, equal boundaries, stable ordering, export/readback,
  tampering refusal, and interpreter/compiler parity. Validate WB-2 itself
  with appropriately redistributable or local-only catalogue fixtures.

### GBL-007 — Statistics and bootstrap

- Add `median`, `quantile`, `sort`, and standard deviation with declared
  quantile interpolation, population/sample convention (or explicit `ddof`),
  ordering, finite-value policy, empty/singleton behaviour, and dimensions.
  Specify whether `sort` returns a copy; do not introduce hidden mutation.
- WB-3 P0 adds ECDF: specify right-continuous `count(x_i <= x) / n`, treatment
  of ties, finite inputs, compatible dimensions, query ordering and empty-data
  refusal. A weighted ECDF needs a separate explicit weight contract; do not
  silently substitute it for the unweighted distribution.
- Bootstrap is a follow-on using GBL-008: specify resampling unit, replacement,
  statistic, repetitions, interval method, and provenance. It is not a
  scientifically valid default for every dependent or weighted dataset.
- Acceptance: independent small reference cases, ties, extremes, units,
  invalid inputs, stable computation, and engine parity; bootstrap fixtures
  reproduce from seed and algorithm and expose assumptions explicitly.

### GBL-008 — Audited seeded random numbers

- Choose a fully specified named algorithm/version, such as a particular PCG
  or ChaCha variant, with fixed seed encoding, state transition, stream
  selection, and integer/float sampling mappings. No clock-derived implicit
  seed in an audited workflow.
- Record seed, algorithm/version, and stream/draw provenance in receipts and
  freeze/replay policy. State that scientific reproducibility is not a claim
  of cryptographic suitability.
- Acceptance: published algorithm vectors, identical sequences across target
  platforms and engines, replay, bounds, independent streams, receipt
  verification, and explicit failures for invalid seeds/parameters.

### GBL-009 — Declarable loop budget

Both engines currently cap total loop-body iterations at 1,000,000.
`GO_LOOP_BUDGET 50000000` is **proposed syntax**, not presently supported.

- Allow an explicit validated top-level budget with a documented default,
  maximum, and host safety policy. Record effective/requested budget in run
  evidence and include it in canonical/freeze semantics.
- Acceptance: exact boundary, nested and function loops share the budget,
  malformed/conflicting declarations refuse, interpreter/native parity,
  preserved budget-exhaustion runs, and unchanged defaults for old programs.

### GBL-010 — Element-wise array maths

- Add explicit array arithmetic and mapped functions such as `sqrt(a)` with
  dimension checking. Define scalar broadcasting and equal-length rules;
  distinguish element-wise multiplication from `dot` and `cross`. No silent
  zip truncation, shape inference, or skipped invalid elements.
- Acceptance: length mismatch, empty arrays, scalar/array cases, incompatible
  dimensions, domain failures identifying the element, resource limits, copy
  independence, stable ordering, and interpreter/compiler parity.

### GBL-011 — Table output from arrays

Current `write_csv`/`write_tsv` take flattened individual cells plus a column
count. `write_text` accepts `.txt`/`.md`. A constructed text value is limited
to **1 MiB** (`src/text_runtime.rs`); generated files have a distinct **64 MiB
per-file** limit (`src/output.rs`). Do not describe 1 MiB as the universal
output-file limit.

- Add arrays-to-table output with headers, explicit numeric/unit representation,
  matching column lengths or validated rows, and checked quoting/escaping.
  Prefer a bounded/streaming writer over hand-building one enormous string.
- Acceptance: thousands of rows, commas/tabs/quotes/newlines/Unicode, empty
  tables, mismatched lengths, exact IDs, lossless numeric cells, safe output
  names, no overwrites, size-limit boundary, partial-write failure policy,
  sealed artifact verification, and native/interpreter parity.

### GBL-012 — Catalogue-scale limits and streaming

Current delimited input: 16 MiB, 100,000 data rows, 1,000,000 cells (also
1,024 columns). Arrays: 100,000 items. These are confirmed bounds, not proof
that increasing them alone makes the current eager evaluator scale safely.

- Define bounded chunked/streaming ingestion and processing, and/or explicit
  audited configurable limits. Distinguish file size, row/cell count, array
  count, text size, memory, output bytes, and loop budget.
- Preserve input evidence, parse/error locations, deterministic ordering and
  aggregation semantics. Audit chunk boundaries, repeated reads, import
  caching, memory copies, source changes, and decompression limits.
- Acceptance: representative large catalogues, exact threshold tests,
  malformed late rows, bounded memory measurements, partial-read refusal,
  identical results where semantics are unchanged, and parity of supported
  native operations. No unbounded "remove all limits" mode by accident.

## Tier 3: usability and representational gaps

### GBL-013 — Compound unit literals

Already reproduced: `1.13e-10 m/s^2` fails with `G101 UNKNOWN SYMBOL s`, while
`1.13e-10 m / (1 s)^2` passes and renders the compact form. Keep the detailed
design/precedence and compatibility gates in [NEXT_BUILD.md](NEXT_BUILD.md).

### GBL-014 — Explicit refusal

- Add deliberate failure such as `refuse("reason")` with a distinct diagnostic
  and preserved reason. Decide a dedicated status versus an explicitly
  classified failure; do not call every scientific refusal a machinery bug
  or freeze protocol violation.
- Acceptance: top-level/block/function use, compiled parity, message bounds,
  execution stops immediately, prior/partial output disposition is explicit,
  preserved verification passes, and secrets/privacy warnings still apply.

### GBL-015 — Exact 64-bit integers

**Alpha.23 interim transport:** unscaled signed i64 FITS IDs are compared and
exported exactly, and `fits_column_text` returns exact decimal text. Numeric
FITS access refuses unsafe integer cells. This does not implement the general
integer type or unsigned/scaled integer representation described below.

Numeric values currently use f64; `parse_integer` also returns f64 and rejects
integer text beyond the safe exact range. A floating-point quantity is not
an exact 64-bit ID container. Users report this blocks Gaia catalogue IDs.

- Design signed/unsigned 64-bit representation as needed; specify literals,
  parsing, arithmetic overflow, comparisons, arrays, conversions, and
  mixed-int/float/unit rules. Never silently round a catalogue identifier.
- Acceptance: values around 2^53 and signed/unsigned extrema, leading-zero
  text policy, checked overflow, FITS/CSV/TSV import/export, JSON/receipt
  round-trip and schema compatibility, native parity, and explicit errors
  for lossy conversions. Text IDs remain a practical interim storage option.

### GBL-016 — Minimum supported Rust version

`Cargo.toml` declares `rust-version = "1.92"`. User reports build/run success
with 1.91; this has not been verified here and the exact build path matters
(building the engine versus compiling generated Rust).

- Test the original manifest/lockfile on candidate toolchains, supported
  targets, interpreter build, generated native programs, tests, and linting.
  Check APIs such as `as_chunks` and dependency MSRVs, rather than guessing
  from one successful executable.
- Either lower the declaration after the full gate passes or explain the
  specific 1.92 requirement. Keep install docs, CI, package metadata, and
  offline data-compilation instructions consistent.

### GBL-017 — Gzip input with original-file provenance

- Add direct `.fits.gz` support (and decide whether delimited gzip input is
  in scope). Hash/preserve the original compressed bytes; separately record
  the decoded stream hash, decompressor identity, and access metadata where
  needed. No substitution of a temporary decompressed file for the original
  input's identity.
- Define supported concatenated-member/trailing-data policy, CRC checks,
  truncated input refusal, decoded-size/ratio limits, safe bounded temporary
  storage for FITS random access, and no surprise overwrite or network access.
- Acceptance: equivalence to the uncompressed scientific data, corrupt/large
  inputs, compressed-byte changes, preserved independent verification,
  original-file postflight checks, and native/interpreter parity.

## WB-3 extensions (logged 2026-10-04)

Detailed requirements, dependencies, missing evidence and the proposed
`GO_MAD` boundary are in [WB3_ROADMAP.md](WB3_ROADMAP.md). GBL-018 through
GBL-025 are planned work only. No runtime, editor vocabulary, receipt schema,
or scientific test result is changed by recording them.

The proposed science fixtures must preserve measured outputs and declare
independently justified tolerances before a new candidate is tested. Existing
run bytes still verify exactly; reference agreement is a separate gate. Do
not round authoritative seals, infer a constants namespace from alpha.20's
protection, or infer physical blinding from an access log.

## Documentation and outreach notes

### DOC-001 — Goblin++ merchandise link

Requested by Malin Hess on 2026-10-03. Include the following optional link in
a future documentation update to help spread the word about Goblin++:

- [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk)
- Suggested placement: a short merchandise/community section in the main README
  and a link in the next documentation package.
- Keep merchandise separate from installation and scientific instructions;
  no purchase is required to use Goblin++.

Status: noted for the next documentation update, not yet added to published
documentation or existing release archives.

## Gates shared by all implementation work

- Add minimal regressions before claiming a fix; record separately what was
  tested on each platform and in each execution engine.
- Preserve dimensions, explicit failure, freeze/lineage/custody, resource
  bounds, generated-artifact hashes, and old preserved-run verification.
- Review changes to canonical programs, rendered values, math policy,
  schemas, and source semantics as versioned compatibility decisions.
- Update documentation, examples, both editor integrations, release notes,
  and packages where the language or builtin vocabulary changes.
- Commit/push/publish only as separately authorised. Logging this list does
  not authorise implementing all features or publishing another release.
