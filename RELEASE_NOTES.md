# 0.1.0-alpha.41 (local, unreleased, native scientific readers)

- Native NPY 1/2/3 and stored/DEFLATE NPZ; logical C-order copies, explicit shape/dtype/storage order, exact integer ID text and no pickle/object execution.
- Native real SciPy CSR/CSC/COO NPZ; checked indices/pointers, duplicate-coordinate refusal and bounded explicit densification or individual rows.
- Native ECSV 1.0 scalar subset; exact text columns, typed reads, registered units converted to SI. Unsupported masks, mixins, YAML metadata and units are refused explicitly.
- Original-byte snapshots, bounded access evidence and versioned implementation policy in receipts/freezes; offline operation replay and native/reference parity. Failures preserve safely captured originals.
- New independent synthetic fixtures written by NumPy, SciPy and Astropy; Rust 1.92.0 retained. ZIP/DEFLATE dependencies added with locked versions and bundled third-party notices.
- VS Code 0.1.32 / JetBrains 0.3.22 include completion, help and highlighting.

See [scope, limits and examples](docs/SCIENTIFIC_READERS.md). This stage does not implement Python library calls, general Astropy objects, MAT/HDF5 or streaming ECSV. Integrity checks do not certify scientific validity or authenticated authorship.

[Goblin++ merch](https://h4k3rl1f3.myspreadshop.co.uk)

# 0.1.0-alpha.40 (local, unreleased, text toolkit and fluid mechanics)

- Adds Unicode upper/lower, UTF-8 byte count (`str_bytes_len`, text-only
  `sizeof`), explicit locale-independent `str_compare`, reversible fixed-width
  UTF-8 hex/octal/binary codecs, and exact-range `to_base`/`from_base`.
  Invalid UTF-8, malformed groups, over-limit expansion, noninteger/unitful
  radix arguments and out-of-range integers refuse explicitly.
- Adds nine scalar SI-dimension-checked `fluid_*` helpers for density,
  viscosity conversions, continuity, Reynolds number, dynamic/total pressure
  and laminar circular-pipe pressure drop. Model assumptions are documented;
  pipe Reynolds >= 2000 refuses, without implying the other assumptions hold.
- Both engines share core implementations; independent hand-calculated
  fixtures, native standalone cases, failure preservation and offline
  receipt verification cover the new behavior.
- Existing direct text comparison, decimal conversion, evidence profiles,
  parser policy and historical evidence are unchanged. No new dependencies;
  Rust remains 1.92.0. Historical Chinese/community-review material untouched.
- VS Code 0.1.31 / JetBrains 0.3.21 add completion, parameter help and highlighting.

See [strings](docs/STRINGS.md), [fluid mechanics](docs/FLUID_MECHANICS.md),
`examples/text_toolkit.gbl` and `examples/fluid_mechanics.gbl`.
Optional [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: alpha.39

# 0.1.0-alpha.39 (local, unreleased, bounded automatic final-state evidence)

- Adds `COMPLETELY_MAD` stage 1: a first top-level directive implying
  `GO_PARANOID`, with final user-global values sealed in `final-state.json`.
- Corrected local build identifies `COMPLETELY_MAD` in its banner, postflight
  labels, verification labels and explicit `execution_profile` receipt/preview
  field. Inherited checks and ordinary `GO_PARANOID` output remain unchanged;
  historical receipts without the new field still verify.
- Explicit `seal` snapshots are unchanged and separate. Reached global
  assignments are captured once; function locals and induction-only counters
  are not traced. Flat arrays, dimensional numbers, text and Booleans are supported.
- Capture admits at most 256 globals and 8 MiB of JSON; refusal is explicit,
  never silent truncation. Failed attempts label captured state partial and,
  for compiled attempts, interpreter-reference origin rather than native success.
- Both engines compare exact final values/dimensions/inventory. Offline
  verification checks evidence binding and compiled final-state parity; diff
  includes final captures. Preview remains preview-only.
- Inline Rust and unaudited standalone native execution are refused in this
  profile. This is not encryption, authenticated authorship, a sandbox,
  scientific validation, higher precision or an implicit numerical audit.
- Parser policy versions the new keyword; historical evidence remains
  verifiable. Older freezes require explicit revision/review before execution.
- VS Code 0.1.30 / JetBrains 0.3.20 help, completion and highlighting. Rust
  remains 1.92.0, with no new dependencies. Original research evidence and
  historical Chinese/community-review documents remain unchanged.
- Also documents automatic quantity units and adds the interpreted/native
  formatting regression from the prior R3 label review. No global stripping of
  literal text; the research script's extra unit is not an engine-formatting bug.

See [COMPLETELY_MAD.md](docs/COMPLETELY_MAD.md) and
[the example](examples/completely_mad.gbl).
Optional [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.38 (exact-ID table joins)

- Adds `csv_join` / `tsv_join` in interpreted and native/standalone data builds.
- Exact UTF-8 text IDs: no f64 conversion, trimming or Unicode normalization.
  Explicit inner/left join kind and column prefixes; one-to-one cardinality;
  missing and duplicate keys refuse with side/row diagnostics, even unmatched.
- Left order, both key columns, explicit match flag, strict quoted CSV/TSV and
  bounded inputs/output. No partial join table is published after failure.
- Input snapshots/hashes, matched/unmatched counts, complete versioned rules,
  output hashes, frozen policy and offline reconstruction preserve evidence.
  Checksum-only integrity is not authenticated authorship or physical blinding.
- VS Code 0.1.29 and JetBrains 0.3.19 completion/help/highlighting. Rust 1.92.0,
  dependencies and Chinese/community-review documents remain unchanged.
- Streaming/FITS/many-to-many joins, extra precision, encryption and
  `COMPLETELY_MAD` are not part of this stage. No end-to-end WB claim.

See [TABLE_JOINS.md](docs/TABLE_JOINS.md).
Optional [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.37 (named numerical consistency audit)

- Adds `GO_NUMERIC_AUDIT "config.json"` with explicit same-machine
  interpreter/native exact-bit and dimension comparison for named scalar seals.
- Requires `run --compile`, explicit schema/mode/names; no hidden compilation,
  tolerance defaults, array reduction or inline Rust. Standalone compilation
  refuses this launcher-owned audit; check preview is explicitly unaudited.
- Preserves PASS/FAIL/INCONCLUSIVE/ERROR evidence; no requested incomplete audit
  may pass a run. Configurations are snapshotted/hashed, pinned by freeze and
  independently reconstructed offline during `verify`.
- Historical non-audit receipts/freezes remain usable; differences in named
  audit evidence are explicit in `diff`. No change to binary64 math semantics,
  authoritative seals, existing parity gates, compiler version or dependencies.
- VS Code 0.1.28 and JetBrains 0.3.18 add directive completion/help/highlighting.
  Chinese/community-review documents are not modified.
- No scientific accuracy, independent algorithms, cross-platform guarantee,
  arbitrary precision or interval claim. `COMPLETELY_MAD` remains a proposed
  strict policy profile; AES/key management remains separate future work.

See [NUMERIC_AUDIT.md](docs/NUMERIC_AUDIT.md).
Optional [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.36 (explicit HTTPS acquisition)

- Adds Unix `goblin++ fetch` and fully offline `goblin++ verify-fetch`.
- HTTPS GET only, port 443, an explicit exact hostname and vetted public DNS
  candidates handed directly to the connector; rustls/platform certificate and
  hostname validation remain mandatory.
- No redirects, proxies, cookies, credential headers, uploads, listeners,
  automatic HTTP content decoding, retries of application requests or insecure
  override. Downloaded content is never automatically executed or interpreted.
- Bounds payloads (default 256 MiB; hard 1 GiB), headers (16 KiB), transfer time
  and receipt reads; private no-overwrite Unix bundles preserve partial failures.
- Separates expected-SHA256 matches from explicit `--record-only`. Acquisitions
  have a versioned receipt, client-lock hash, redacted source identity, payload
  hash and optional external receipt anchor. Integrity is not authorship or
  scientific validation; no implicit custody-ledger or `.gbl` parent linkage.
- Existing language/runtime semantics and editor vocabularies are unchanged.
  There is no download builtin, cube-handle integration or 3-D compression yet.
  Rust remains 1.92.0; maintained HTTP/TLS dependencies are newly locked.
- See [NETWORK_ACQUISITION.md](docs/NETWORK_ACQUISITION.md) for use, threat model,
  acceptance coverage and remaining publication gates. No public release or
  Zenodo submission is implied by these local source changes.

Optional [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk): no purchase
is required to use the language.

# 0.1.0-alpha.35 (local, unreleased, Orbit scoped baseline checks)

- Shared Rust module plus `goblin++ orbit` / standalone `goblin-orbit`, both
  installed by the local bundle and source installer.
- Explicit local directory, mandatory authorisation/trust declarations/reason,
  separately expected baseline SHA-256, exclusive outside-scope destinations.
- Bounded streaming file hashes, sorted entry inventories, descriptor-relative
  Unix no-follow traversal; unsupported/unreadable/unstable/budget gaps are
  incomplete, never successful coverage. Missing cannot be asserted across gaps.
- Preserved observations and derived comparison reports through the alpha.34
  reporting module; offline strict-schema/copy/classification verification and
  optional independently retained report receipt anchor.
- Explicit re-baselining preserves/pins immediate parent bytes; no overwrite,
  automatic repair, elevation, network scanning, vulnerability diagnosis or
  image/audio origin detection. No invented analyst findings or clean verdicts.
- Disposable walkthrough and tests; permission/ACL auditing, atomic snapshots,
  hostile-host assurance, signing and media analysis explicitly deferred.
- Rust 1.92.0 retained; existing locked libc is additionally a direct Unix
  dependency, without version changes. Scientific runtime semantics unchanged.
  VS Code 0.1.27 / JetBrains 0.3.17 reused; Chinese snapshots unchanged.

See [ORBIT.md](docs/ORBIT.md).
Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.34 (published, offline evidence reports)

- `evidence-report CASE.json --output NEW_DIRECTORY`: offline, bounded,
  expected-SHA-256-pinned evidence snapshots and structured Markdown/HTML reports.
- `verify-report DIRECTORY`: strict schemas, receipt core, exact copies and
  independent byte-for-byte report reconstruction; optional trusted receipt hash.
- Explicit analyst scope/authorisation, UTC declarations, severity/confidence,
  observations, impact, recommendations and nonempty evidence references.
- Safe text escaping, inert HTML with no network resources/scripts, exclusive
  destinations, symlink/traversal refusal, Unix private file/directory modes.
- Clear checksum-only boundary: no truth/authorship/permission certification,
  certified time, acquisition, legal admissibility, scanning or exploitation.
  No automated redaction/encryption or forensic-validation claim.
- Synthetic case and `.gbl` lab example; plaintext confidentiality and bounded
  limits documented. Existing analysis/freeze policies and editors unchanged.
- Rust 1.92.0 and dependencies retained; Mandarin/community-review unchanged.
  AES-256-GCM/COMPLETELY_MAD and remaining science stages stay on the roadmap.

See [EVIDENCE_REPORTING.md](docs/EVIDENCE_REPORTING.md).
Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.33 (published, element-wise numeric arrays)

- Array/scalar and equal-length array/array `+ - * / % ^`, unary signs, and
  existing root/log/rounding/trigonometric/extrema/hypot functions in both engines.
- Finite homogeneous numeric quantities, dimensional checks, independent copies,
  ascending zero-based failing-element diagnostics, no partial assignment.
- Explicit scalar repetition only; no length-one array broadcast, concatenation,
  truncation, masks, hidden reductions or automatic vectorization of all helpers.
- Existing 100,000-item limit; bounded serial mapping, not a RAM sandbox or threads.
- Versioned policy in run/freeze/check/compile/native data evidence; policy-aware
  verification/diff. Historical receipts remain verifiable; older freezes need
  explicit revision/child freeze before running with new arithmetic semantics.
- Checked unit exponent overflow refuses rather than wrapping/panicking.
- VS Code 0.1.27 / JetBrains 0.3.17 help; Rust 1.92.0 and dependencies unchanged.
  Chinese/community-review unchanged. Joins/richer plots/real workflow fixtures
  remain separate stages. Encryption and COMPLETELY_MAD remain roadmap proposals.

See [ARRAY_MATH.md](docs/ARRAY_MATH.md) and `examples/elementwise_arrays.gbl`.
Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.32 (published, bounded FITS batches)

- `fits_batch_open(path, hdu, columns, rows, batch_bytes, file_bytes)` projects
  supported scalar/fixed-ASCII BINTABLE fields into bounded row-major text batches.
  Original row order, exact unscaled i64 IDs, explicit null markers and finite
  scaling are preserved; selected names are exact/case-sensitive and units audited.
- Existing incremental CSV/TSV writers, EOF/close requirements, immutable snapshots,
  hashes, native/reference parity and `.partial` failure output behavior apply.
- Independent preserved-schema checks and versioned FITS policy in runs/freezes;
  older non-FITS-batch evidence remains supported. Historical freezes require an
  explicit revision/child freeze before adopting the new API.
- Explicit limits: <=64 GiB files, <=8 MiB/100,000 cells per returned batch,
  <=1 MiB raw rows/total raw headers and <=128 HDUs. No process-wide RAM quota,
  gzip, FITS writer, threads, hidden cuts or physical-blinding claim.
- VS Code 0.1.26 and JetBrains 0.3.16; Rust 1.92.0 and dependencies unchanged.
  Chinese/community-review snapshots untouched. Encryption and COMPLETELY_MAD
  remain roadmap proposals, not shipped features.

See [FITS_BATCHES.md](docs/FITS_BATCHES.md) and `examples/fits_batches.gbl`.
Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.31 (published, embedded plot provenance)

- Standard PNG uncompressed iTXt/SVG metadata embed the SHA-256 of an
  independent canonical data/settings manifest, without circular hashes.
- Finished figure/sidecar hashes and manifest links remain in the run receipt;
  verification checks policy, metadata and exact independent reconstruction.
- Missing/duplicate/changed provenance, malformed lengths/CRC, unsupported
  encodings, unsafe filenames and altered links refuse. Same inputs/settings/
  filename under the same policy reproduce identical bytes; no hidden timestamp.
- Historical alpha.30 v1 plots still verify unchanged. New plots use v2;
  frozen older plotting programs require an explicit revision/child freeze.
- Existing FITS outputs retain their bytes. Interpreter/native parity;
  both existing alpha.30 editor plugins retained, no reinstall needed.
- Checksum-only, not signing, secrecy, fraud detection or certified provenance.
  Trusted external hashes/signatures remain necessary against complete rewrites.
- Rust stays 1.92.0; no new dependencies or Chinese/community-review edits.

See [PLOT_PROVENANCE.md](docs/PLOT_PROVENANCE.md).
Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.30 (evidence-first array plotting)

- `plot_line`, `plot_scatter`, `plot_histogram` accept numeric arrays and explicit
  labels, output PNG/SVG, and preserve all points. x/y lengths and dimensions
  within each axis must match; nonfinite/missing/nonnumeric data refuses.
- Actual SI units, linear axes, ticks/grid, supplied line order, explicit
  histogram bins and zero count baseline. No hidden sorting or sampling.
- Exact data/settings sidecars with linked hashes; independent verification
  rerenders figure bytes, including resealed-image tamper refusal. Both names
  are collision-checked before either output is added.
- Native/interpreter parity and standalone evidence. Plot policy pinned in new
  freezes; historical non-plot evidence remains supported.
- Case-sensitive PNG labels; Unicode via escaped SVG, explicit refusal for
  unsupported PNG characters. No silent truncation or unit-case changes.
- VS Code 0.1.25 and JetBrains 0.3.15. Rust stays 1.92.0, no new dependencies.
  Error bars/multi-series/log axes remain later work. AES-256-GCM and
  COMPLETELY_MAD remain roadmap proposals. Chinese/community-review unchanged.

See [ARRAY_PLOTS.md](docs/ARRAY_PLOTS.md) and `examples/array_plots.gbl`.
Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.29 (captured date/time)

- `time_date()` and `time_date("local")` capture system local date/time;
  `time_date("utc")` captures UTC. Immutable ordinary text, formatted
  DD-MM-YYYY:HH:MM:SS, usable with or without GO_PARANOID/seal.
- Versioned clock policy/readings in run receipts and new freezes, exact
  native/reference replay, standalone native evidence, independent consistency
  and tamper checks, and CLOCK_INPUT_CHANGE diff classification.
- Historical non-clock receipts/freezes remain supported. Explicit revision
  is required before an unpinned older frozen program adopts clock calls.
- VS Code 0.1.24 and JetBrains 0.3.14 add completion/help/highlighting.
  No compiler upgrade or new dependencies; Rust remains 1.92.0.
- System clocks are not certified time. AES-256-GCM encryption and
  COMPLETELY_MAD strict policy remain roadmap proposals, not shipped features.
  Chinese/community-review material is unchanged.

See [CLOCK_TIME.md](docs/CLOCK_TIME.md) and `examples/clock_time.gbl`.
Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.28 (serial table batches)

- Eight explicit CSV/TSV batch/writer functions, available in interpreted,
  audited compiled and standalone execution. Flat row-major text batches retain
  exact catalogue IDs; original row order and lossless quoting are preserved.
- Caller-selected row, decoded batch and file budgets: <=8 MiB/100,000 cells per
  batch, <=64 GiB per file, <=1 MiB per logical record, <=1024 columns. Existing
  eager/scan APIs keep their limits. No threading or general process RAM quota.
- Private disk snapshots protect the parsed input identity; SHA-256, full input
  evidence, requested columns, limits, counts and lifecycle states are recorded.
  PASS requires observed EOF and explicit closes. Failed stream outputs carry
  `.partial` names and `complete=false`, including late ledger failures.
- Native/reference checks compare batch evidence and on-disk output hashes;
  standalone failure paths preserve partial outputs and batch input snapshots.
- New freezes pin the batch contract; old non-batch freezes and historical
  verification remain supported. Preview checks validate lifecycle without
  registering evidence; temporary I/O staging is cleaned up.
- VS Code 0.1.23 and JetBrains 0.3.13 add shared completion/help/highlighting.
  No Rust upgrade, dependency additions, encryption or `COMPLETELY_MAD` changes.

See [BATCHES_STREAMING.md](docs/BATCHES_STREAMING.md) and
`examples/batch_catalogue.gbl`. Large snapshots require sufficient disk space.
Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).

## Previous stage: 0.1.0-alpha.27 (auditable inference foundations)

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
