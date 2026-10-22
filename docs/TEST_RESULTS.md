# Alpha.37 named numerical consistency audit (local, unreleased)

Verified locally on 2026-10-22, macOS arm64, retained Rust 1.92.0:

- Full `cargo test --locked --offline --all-targets --quiet`: **492 passed**,
  no failures/ignored tests. The final audit target then passed **25 tests**,
  including one subsequently added CSV/library audit gate: **493 distinct Rust
  tests** covered in total (do not sum the overlapping audit reruns).
- Formatting, warning-denying Clippy and optimized builds pass. Cargo locks
  change only the Goblin++ package version; no dependency/compiler upgrade.
- New gates cover exact scalar values/dimensions/signed zero, missing and array
  seals, malformed/unknown/duplicate/oversized/symlink configurations, directive
  placement, imports, frozen configuration changes, core-resealed tampering,
  missing compiler, interpreter-only invocation, inline Rust refusal, incomplete
  previews, preserved failures, RNG/clock replay, CSV/module evidence and diffs.
  An injected native-result mismatch is explicitly a **synthetic fixture**,
  not an observed platform discrepancy or independent accuracy benchmark.
- VS Code 0.1.28: **23 passed**, including actual CLI workflows. JetBrains
  0.3.18: **22 passed**, offline build successful. Both use 277 unique editor
  vocabulary entries. No live IDE interaction or Marketplace publication is
  claimed by the automated plugin tests.
- Python evidence-checker tests: **14 passed**. Separate cube extractor/regional
  study suites: **14 + 5 passed**. Chinese/community-review files unchanged.
- The actual immutable alpha.36 package executable generated interpreted and
  compiled receipts, both verified by alpha.37. An alpha.36 freeze also allowed
  a successful new execution and verification. Original archive SHA-256 remains
  `4450cdca424deda0398cd1ab3f39ed89325137c2f2ac106b907e548abea58218`.
- Full regression caught an initial parser dependency missing from generated
  standalone scientific crates. Path validation was moved into the shared
  parser, and those plotting/compiled regressions passed on rerun. Two initial
  test-fixture mistakes (dimension order and random stream selection) were
  corrected. Local HTTPS tests initially required permission for loopback test
  servers; all passed once permitted. Editor-count/version expectations were
  updated for the additional directive and both suites rerun successfully.

`tools/verify_local.py <package-receipt>` checks the final immutable archive,
inventory, plugin hashes and temporary-prefix installation, then executes the
existing 32 example runs plus the new explicit compiled audit and independent
verification. Its post-packaging result is recorded separately at handoff.
No user-prefix install, commit, push, public release or Zenodo submission is
performed by this local validation. Linux/cross-platform CI is still a separate
release gate. Same-machine consistency is not scientific truth, independent
algorithms, a signature, extra precision or a universal reproducibility promise.

## Previous local stage: Alpha.31 embedded plot provenance

Verified locally on 2026-10-11, macOS arm64, retained Rust 1.92.0:

- Rust: **363 passed**, 0 failed/ignored/filtered in the final full
  `cargo test --locked --offline --all-targets` suite. Formatting,
  warning-denying Clippy and optimized build pass. No new dependency crates.
- **18 additional provenance gates** (6 unit/compatibility + 12 integration)
  cover independent canonical manifest hashes, PNG/SVG embedding, repeat
  exact bytes, settings/data/order/name changes, stripping and full ordinary
  rehash attempts, duplicate metadata, altered SHA-256 with recomputed PNG CRC,
  unsafe names, compressed/oversized/noncanonical/unknown/duplicate JSON fields,
  malformed PNG signatures/lengths/CRC/trailing bytes, invalid SVG escaping/
  UTF-8/entities, exact legacy v1 reconstruction, old-freeze migration and
  receipt/pair policy mismatch. The original 30 plotting gates also pass,
  including interpreted/audited-native/standalone output parity.
- Existing VS Code 0.1.25: **20 passed**, 0 failed/skipped, including real
  alpha.31 CLI preview/run/compile/verify and `PLOT_PROVENANCE` checks.
  Its installer is retained unchanged: no new function signatures or reinstall.
- Existing JetBrains 0.3.15 installer/source retained unchanged from the
  alpha.30 local stage's 18 passing tests below. No new IDE-version or new
  JetBrains test-run claim is made for this metadata-only engine stage.
- Report-checker Python tests: **14 passed**. All **179** tracked Chinese/
  community-review files remain unchanged.
- `tools/verify_plot_compat.py` extracted the immutable real alpha.30 bundle
  into a disposable directory. The new engine verified its eight plot/evidence
  artifacts by original v1 reconstruction, without claiming embedded links.
  Its old plot freeze inspected correctly, refused new v2 execution before
  output, and succeeded after explicit revision and a child freeze. Its old
  non-plot energy freeze still executed and verified. All **seven** legacy
  FITS/output-demo artifacts retained exact hashes, byte counts and metadata.
- Original alpha.30 ZIP and both retained editor installer hashes were checked
  unchanged. No commit, push, public release or user-prefix installation is
  performed by these checks.

`tools/verify_local.py <package-receipt>` independently checks all ZIP/plugin
hashes and inventory files, installs only under a temporary prefix, and exercises
fifteen examples in both modes (**30 audited runs**). It recomputes canonical
manifest digests with Python, reads PNG iTXt/CRC and SVG metadata independently,
checks all four fields, and compares exact interpreted/compiled artifact hashes
(clock outputs are deliberately separate readings). Package validation occurs
after creating the immutable ZIP; its result is included in the handoff.

This establishes tested consistency/reconstruction, **not authenticated authorship,
certified time, an OS sandbox or proof of scientific correctness**. A complete
internally consistent replacement still needs a trusted original digest or
future signature to distinguish it.

## Previous local stage: Alpha.30 evidence-first array plotting

Verified locally on 2026-10-11, macOS arm64, retained Rust 1.92.0:

- Rust: **345 passed**, 0 failed/ignored/filtered in the full
  `cargo test --locked --offline --all-targets` suite. Formatting,
  warning-denying Clippy and optimized build pass. No dependency additions;
  Cargo.lock changes only the Goblin++ version.
- Thirty plotting gates cover line/scatter/histogram and PNG/SVG reconstruction,
  exact f64 bits/signed zero and input order, SI conversion and independent
  axis dimensions, mismatched/empty/nonnumeric/mixed-unit/nonfinite refusal,
  histogram edge inclusion/counts/constant data/collapsed edges, safe spans,
  bin types/limits, filenames, escaped SVG Unicode, missing PNG glyph refusal,
  label limits/bidirectional controls, case-sensitive units, malformed evidence,
  wrong arity/shadowing, atomic output-pair collisions, missing sidecars,
  changed settings/images and resealed tampering, native/standalone parity,
  pinned policy and old freeze/receipt compatibility, 100,000-item boundaries,
  and distinct tick labels for narrow/adjacent-value ranges.
- VS Code 0.1.25: **20 passed**, 0 failed/skipped, including real preview,
  interpreted/compiled plotting and independent run verification. Both
  editors use **275 unique vocabulary entries**.
- JetBrains 0.3.15: **18 passed**, 0 failed/skipped; offline package build
  against cached IntelliJ 2024.3.7/JDK 21 passes. The packaged vocabulary
  and version were checked. This is not certification of every IDE build.
- Report-checker Python tests: **14 passed**. All **179** tracked Chinese/
  community-review files remain byte-identical to pre-change HEAD.
- The previous alpha.29 bundled engine and alpha.30 were both run on the same
  `output_demo.gbl`/sample FITS bytes. All **seven output artifacts**, including
  its **three FITS plots**, matched exactly in SHA-256, byte count and metadata.
- A generated PNG was visually inspected for readable axes/ticks, case-correct
  units and unclipped labels. The exact data/settings sidecar was independently
  verified; tick-display rounding never changes authoritative values or seals.

`tools/verify_local.py <package-receipt>` checks archive/plugin hashes and every
inventory item, installs under a temporary prefix, and exercises fifteen shipped
examples in both modes (**30 audited runs**). It independently verifies receipts
and plotting reconstruction, checks sidecar links/hashes and preserves existing
clock/RNG/inference/resource parity rules. Packaging results are reported after
creation because this record is included in the immutable archive beforehand.

This is local evidence, not new Linux/Windows CI or full WB-3 validation.
Plot reconstruction is consistency checking, not authenticated authorship.
No Git commit, push, GitHub/Zenodo publication or user-prefix installation was
performed for this local build. Error bars, multiple series and log axes are
later stages. See [ARRAY_PLOTS.md](ARRAY_PLOTS.md).

## Previous stage: Alpha.29 captured local/UTC date/time

Verified locally on 2026-10-11, macOS arm64, retained Rust 1.92.0:

- Rust: **315 passed**, 0 failed/ignored/filtered, full
  `cargo test --locked --offline --all-targets`. Formatting,
  warning-denying Clippy and optimized build pass. Cargo.lock changes only
  the Goblin++ version; no new dependency or compiler upgrade.
- Twenty clock gates cover UTC/default/explicit local calls, leap day,
  date rollover, non-hour offsets, repeated daylight-saving local time,
  pre-epoch dates, four-digit years, strict modes/types/arity, discarded
  results/shadowing, call/replay budgets, snapshot strings, preserved later
  failures, clock preview, native replay/export parity, standalone evidence,
  resealed tampering/removal detection, old freeze/receipt compatibility and
  exact clock-input diffs.
- VS Code 0.1.24: **19 passed**, 0 failed/skipped, including real CLI
  clock preview and interpreted/compiled execution with independent
  verification. Shared vocabulary: 272 unique spellings.
- JetBrains 0.3.14: **17 passed**, 0 failed/skipped; offline build passes
  against cached IntelliJ 2024.3.7/JDK 21. This does not certify every IDE
  version or a new live CLion/PyCharm installation.
- Report-checker Python tests: **14 passed**. All **179** tracked Chinese/
  community-review files remain byte-identical to pre-change HEAD.

Clock verification checks preserved evidence consistency, not actual clock
accuracy or independently certified time. Display text uses second resolution;
receipt UTC instants retain nanoseconds and captured offsets. Successful
compiled audits replay reference readings; separate executions naturally have
different clock inputs. No authoritative seals are rounded to conceal that.

After packaging, `tools/verify_local.py <package-receipt>` checks hashes,
ZIP/inventory integrity and isolated installation; exercises fourteen shipped
examples in both modes (28 audited runs); independently verifies every receipt;
and checks exact native/reference clock parity within each compiled run.
It does not require separate live-clock runs to have identical timestamp seals.
Packaging results are reported separately because this record is included in
the immutable archive before those checks run.

The alpha.28 large-file memory benchmark below is retained historical evidence,
not a newly repeated alpha.29 RSS benchmark. No cross-platform CI claim or
GitHub/Zenodo publication is made by this local stage.
See [CLOCK_TIME.md](CLOCK_TIME.md) and `examples/clock_time.gbl`.

## Previous stage: Alpha.28 serial table batches

Verified locally on 2026-10-10, macOS arm64, retained Rust 1.92.0:

- Rust: **295 passed**, 0 failed/ignored/filtered in the full
  `cargo test --locked --offline --all-targets` run. Formatting,
  warning-denying Clippy and optimized build pass. No new dependencies or
  compiler upgrade; Cargo.lock changes only the Goblin++ version.
- Nineteen batch integration gates cover exact catalogue-ID text, requested
  column order, CSV/TSV quoting and multiline records, byte/row/handle limits,
  immutable input snapshots, interpreter/native/standalone execution, output
  parity, malformed and oversized input, discarded results, lifecycle refusal,
  legacy-output collisions, freeze-policy migration, independent tamper
  detection and partial-output preservation, including late ledger failure.
- VS Code 0.1.23: **18 passed**, 0 failed/skipped, including actual CLI
  preview and interpreted/compiled batch execution with verification.
  Shared vocabulary: 271 unique spellings.
- JetBrains 0.3.13: **16 passed**, 0 failed/skipped. Offline installer build
  passes against cached IntelliJ 2024.3.7/JDK 21. This does not certify new live
  CLion/PyCharm installations or every IDE version.
- Report-checker Python tests: **14 passed**, 0 failed. All **179** tracked
  Chinese/community-review files remain byte-identical to pre-change HEAD.
- The opt-in real-file memory harness processed 2 MiB, 64 MiB and
  **1,075,839,251 bytes (1.002 GiB; 1,028,527 rows)** with verified receipts and
  exact input/output byte hashes. Peak process RSS for the largest input was
  **19,300,352 bytes (18.4 MiB)**; peak decoded batch allocation was
  4,653,056 bytes (4.44 MiB). The measured engine hash and complete evidence
  are in [BATCH_MEMORY_alpha28.json](verification/BATCH_MEMORY_alpha28.json).

This is a measured bounded, serial copy workflow, not a general process-memory
quota or full WB-3 science validation. Programs can intentionally accumulate
arrays; callers must release each batch before fetching the next. Full input
snapshots consume disk and retain unselected columns: column projection is
not physical blinding. Batch outputs preserve exact values; this does not
guarantee universal bitwise transcendental mathematics across machines.
Abrupt termination, exhausted storage and hostile filesystem races are not
guaranteed recoverable by this stage. No worker threads were introduced.

After packaging, `tools/verify_local.py <package-receipt>` checks hashes,
ZIP/inventory integrity, preserved notices and an isolated installation; runs
thirteen shipped examples in both modes (26 audited runs); compares exact
batch/RNG/inference/resource evidence and seals; and independently verifies
each receipt. Packaging results are reported separately because this record
is included in the immutable archive before those checks run.
See [BATCHES_STREAMING.md](BATCHES_STREAMING.md) for the API, lifecycle and limits.

## Previous stage: Alpha.27 inference and bounded resources

Verified locally on 2026-10-07, macOS arm64, retained Rust 1.92.0:

- Rust: **276 passed**, 0 failed/ignored/filtered in the final full
  `cargo test --locked --offline --all-targets` run. Formatting,
  warning-denying Clippy and optimized build pass. Cargo.lock changes only
  the Goblin++ version; no dependencies or compiler upgrade were introduced.
- Seven resampling tests cover explicit-stream selection, hand-computed
  bootstrap reductions, units, fixed normal transformation/draw counts,
  invalid-input refusal and transactional rollback. Thirteen matrix tests
  cover known products, stable norm/unit, covariance/divisors, Cholesky
  reconstruction, singular/ill-conditioned refusal, units, bounded work and
  transactional multivariate sampling.
- Six inference integration gates cover interpreter/native/standalone
  parity, exact helper traces/seals/RNG evidence, arity before effects,
  builtin collisions, preserved policy violations and independent tamper
  detection, including re-hashed native evidence mismatching the receipt.
- Fifteen resource/streaming gates cover shared nested/function loop budgets,
  both execution modes and standalone refusal, more than the former fixed
  loop ceiling and 16 MiB input bound, malformed/null/quoted/multiline tables,
  exact imported bytes, mixed scan/array access and policy/evidence tampering.
- Four parser-migration gates preserve a genuine, unmodified alpha.26 run
  that used `GO_LOOP_BUDGET` as an ordinary identifier/function/parameter.
  Its old canonical hash verifies unchanged. New policy v2 preserves v1
  compound units, rejects alpha.27 grammar downgrade and requires explicit
  revision before executing an old v1 freeze. Nine compound-unit gates and
  all prior regression suites pass.
- VS Code 0.1.22: **17 passed**, 0 failed/skipped, including actual CLI
  preview, interpreted/compiled inference and streaming runs and independent
  verification. Shared vocabulary: 263 unique spellings.
- JetBrains 0.3.12: **15 passed**, 0 failed/skipped; offline installer build
  passes against cached IntelliJ 2024.3.7/JDK 21. This is not a new live
  CLion/PyCharm test or multi-version compatibility certification.
- Report-checker Python tests: **14 passed**, 0 failed. All **179** tracked
  Chinese/community-review files remain byte-identical to pre-change HEAD.

Local same-machine parity is not universal cross-platform bitwise mathematics.
The PCG primitive algorithm remains unchanged; Box-Muller transcendental
functions can vary in their last bits across platforms. Preserve exact seals
and compare scientific quantities separately against declared tolerances.
The helpers do not validate a scientific model, imply confidence-interval
coverage, provide mixed-unit Gaia covariance, or enforce physical blinding.
Streaming is bounded column reduction, not arbitrary out-of-core analysis.
Encryption and `COMPLETELY_MAD` remain proposed, not shipped capabilities.

After packaging, `tools/verify_local.py <package-receipt>` checks artifact
hashes, ZIP/inventory integrity, preserved notices and an isolated installation;
runs twelve shipped examples in both modes (24 runs); compares exact RNG,
inference/resource evidence and seals; and independently verifies each receipt.
Packaging results are reported separately because this record is included in
the immutable archive before those checks run. See [RESAMPLING.md](RESAMPLING.md),
[MATRICES_COVARIANCE.md](MATRICES_COVARIANCE.md) and
[RESOURCES_STREAMING.md](RESOURCES_STREAMING.md).

## Previous stage: Alpha.26 seeded scientific RNG

Verified locally on 2026-10-06, macOS arm64, retained Rust 1.92.0:

- Rust: **231 passed**, 0 failed/ignored/filtered in the final full
  `cargo test --locked --offline --all-targets` run. Formatting,
  warning-denying Clippy, whitespace checks and optimized build pass.
  Cargo.lock changes only the Goblin++ version; no new dependencies.
- Eleven RNG integration tests plus two internal tests cover PCG's published
  seed-42/stream-54 reference vector, specified uniform/integer mappings,
  explicit streams and interleaving, full-u64 text seeds/u63 streams,
  bounds/types/reseeding/unseeded refusals, transactional resource guards,
  compressed trace segments, builtin collisions, arity-before-effects,
  interpreter/native/standalone results, frozen policy/legacy compatibility,
  tamper detection after re-hashing, missing required evidence and exact diff.
- Measured native/interpreter RNG traces and exact sealed hashes match on
  macOS arm64. The fixed RNG fixture is preserved for the new exact cross-platform
  CI gate. Linux/macOS validation awaits this stage's CI; this is not a universal
  bitwise mathematics guarantee. The existing 36-case math fixture also passes.
- A genuine run made with the installed alpha.25 engine, including a then-legal
  user function named `rng_uniform`, independently verifies with alpha.26.
  Historical canonical parsing is retained; executing a newly conflicting
  user function under alpha.26 requires renaming it.
- VS Code 0.1.21: **16 passed**, 0 failed/skipped, including actual CLI
  preview, interpreted/compiled RNG execution and independent verification.
  Vocabulary has 248 unique spellings; RNG help/highlighting/snippet pass.
- JetBrains 0.3.11: **14 passed**, 0 failed/skipped; offline installer build
  passes with cached IntelliJ 2024.3.7/JDK 21. This is not a new live
  CLion/PyCharm test or multi-version compatibility certification.
- Report-checker Python tests: **14 passed**, 0 failed. All **179** tracked
  Chinese/community-review files are byte-identical to the pre-change HEAD.
- PCG reference attribution and Apache-2.0 license are retained in source and
  included by the package notice collector. No cryptographic capability is
  supplied by this RNG; encryption remains a separate future feature.

Local checks do not establish complete WB-3 validation, authenticated history
or correctness of a scientific model. Bootstrap, normal/covariance sampling,
streaming/budget extensions and strict research isolation remain future work.
See [RANDOMNESS.md](RANDOMNESS.md) for the precise contract and limits.

After packaging, `tools/verify_local.py <package-receipt>` checks hashes,
ZIP/inventory integrity, preserved reference licensing and an isolated
installation; runs ten shipped examples in both modes (20 runs); and independently
verifies every receipt. Packaging results are reported separately because this
record is included in the immutable archive before those checks run.

## Previous stage: Alpha.25 compound-unit input

Verified locally on 2026-10-06, macOS arm64 with the retained Rust 1.92.0:

- Rust: 218 passed, 0 failed, 0 ignored, no filtered tests in the final
  `cargo test --locked --offline --all-targets` run. Formatting,
  warning-denying Clippy, whitespace checks and the optimized build pass.
  Cargo.lock changes only the Goblin++ package version; no new dependencies.
- Nine compound-unit gates cover independent numeric/dimensional references,
  SI scaling, signed zero, Unicode/canonical spelling, ordinary variable
  arithmetic, grouped denominators, arrays and imported functions, both engines
  and standalone native execution, parser-policy tampering, explicit old-freeze
  migration, CLI status/compilation refusal and preserved failure evidence.
- All 15,625 small-exponent combinations across six SI axes round-trip through
  the rendered unit syntax and current parser. Invalid powers, ambiguous
  denominators, dimension/scale overflow, underflow-to-zero, excessive nesting
  and overlong suffixes refuse.
- The shared numerical fixture has 36 cases. Both engines pass locally against
  the declared exact-bit/tolerance references, with independently verified
  preserved runs. Linux/macOS cross-platform comparison awaits alpha.25 CI;
  seals remain exact and are not rounded to hide platform differences.
- A genuine alpha.24 run whose imported function used `3 m^2` (9 square metres
  under the old parser) independently verifies using alpha.25. The new engine
  refuses its unchanged old freeze under the new policy; that protocol-violation
  run also independently verifies. Historical artifacts were not rewritten.
- VS Code 0.1.20: 16 passed, 0 failed, none skipped, including actual CLI
  interpretation/compilation/verification and compound-unit help/highlighting.
- JetBrains 0.3.10: 14 passed, 0 failed, none skipped; offline installer build
  passes against cached IntelliJ 2024.3.7/JDK 21. This is not a new live
  CLion/PyCharm test or multi-version compatibility certification.
- Report-checker Python tests: 10 passed, 0 failed. All 179 tracked Chinese and
  community-review files match their pre-change bytes.

Local engineering checks do not establish complete WB-3 scientific validation,
universal bitwise reproducibility or authenticated authorship. A compiler
upgrade is deferred until preparation for the first non-alpha release; CI now
pins Rust 1.92.0. See [COMPOUND_UNITS.md](COMPOUND_UNITS.md) for the deliberate
`3 m^2` migration and explicit revision requirement for old freezes.

After packaging, `tools/verify_local.py <package-receipt>` verifies artifact
hashes, ZIP/inventory integrity and an isolated installation, then runs nine
shipped examples in both modes (18 runs), independently verifying each receipt.
Bundle-validation results are reported separately; this record is included in
the immutable archive before those final packaging checks run.

## Previous stage: Alpha.24 distribution statistics

Verified locally on 2026-10-06, macOS arm64 with Rust 1.92.0:

- Rust: 209 passed, 0 failed, no filtered tests in the full run of
  `cargo test --locked --offline --all-targets`. Formatting, warning-denying
  Clippy and whitespace checks pass. No new dependencies.
- Four new shared-runtime tests and seven distribution integration gates cover
  independent known results, copy sorting, signed-zero ties, type-7 boundaries,
  dimensions, large offsets, subnormals/extremes, explicit invalid-input refusal,
  standalone native validation without interpreter preflight, exact evidence,
  tampering, historical parsing, builtin collisions and frozen-policy migration.
- The shared numerical fixture now has 31 cases, including six statistics
  reference cases. Both engines pass locally with declared tolerances/exact-bit
  requirements. Linux/macOS cross-platform validation awaits this stage's CI;
  authoritative seals remain exact and are never rounded to hide differences.
- VS Code 0.1.19: 16 passed, 0 failed, none skipped. Activation, 244 unique
  vocabulary entries, statistics highlighting/completion/help and actual
  engine check/run/compiled-run/independent-verification contracts pass.
- JetBrains 0.3.9: 13 passed, 0 failed, none skipped; offline installer build
  passes against cached IntelliJ 2024.3.7/JDK 21. This is not a new live
  CLion/PyCharm test or multi-version compatibility certification.
- Report-checker Python tests: 10 passed, 0 failed.
- New statistics conventions and shared-source identity are hashed into run
  receipts and new freezes. Policy drift preserves a protocol-violation run;
  exact diff labels policy differences separately from output equality.
- Existing source hashing, dimensions, constants, protected values, old receipt
  compatibility, recursion safety, FITS extraction and custody gates pass.
  Chinese and community-review documentation is unchanged.

These are local engineering results, not full WB-3 scientific validation,
arbitrary precision or universal cross-platform bitwise reproducibility.
GBL-007 core distributions are implemented; bootstrap, RNG and weights remain
future work. See [STATISTICS.md](STATISTICS.md). After packaging,
`tools/verify_local.py <package-receipt>` checks hashes, ZIP/inventory integrity,
an isolated installation and eight examples in both modes with independent
verification. Bundle-validation output is reported separately.

## Previous stage: Alpha.21 lossless numeric text

- Rust: 170 passed, 0 failed, no filtered tests in the final full run of
  `cargo test --locked --offline --all-targets` on macOS arm64/Rust 1.92.0.
  Formatting, warning-denying Clippy and whitespace checks pass.
- VS Code 0.1.16: 16 passed, 0 failed, none skipped; actual engine
  check/run/compile/verify gates and round-trip completion help included.
- JetBrains 0.3.6: 13 passed, 0 failed; installer builds offline against
  cached IntelliJ 2024.3.7/JDK 21. No new live CLion/PyCharm test or
  multi-version compatibility claim is made.
- Five new Rust gates check 100,000 deterministic finite f64 bit patterns,
  including signed zero, subnormals, extremes and notation boundaries;
  both-engine 32-value print/template/to_text/parse round trips, CSV/TSV/text
  bytes, JSON/seals and independent verification; unchanged explicit
  precision, unitful SI output, numeric refusals, alpha.20 freeze revision
  enforcement and preserved protocol-violation evidence.
- The untouched, SHA-256-verified alpha.20 local binary reproduced truncated
  numeric text and a false round-trip comparison. Its real preserved run
  independently verifies under alpha.21 without rewriting old evidence.
- Existing energy canonical hashing, dimensions, old receipt/registry
  compatibility, scientific functions and paranoid custody gates pass.

These are local results, not external CI, exhaustive enumeration of all f64
values, arbitrary precision or cross-platform deterministic maths. No
dependencies were added. GBL-004 is implemented; GBL-005 onward remain queued.
See [the numeric contract and migration guide](NUMERIC_TEXT.md).
`tools/verify_local.py <package-receipt>` additionally checks artifact hashes,
ZIP/inventory integrity and an isolated install, running five examples in
both modes with independent verification. Package-validation output is
reported separately after creating the immutable archive.

## Previous stage: Alpha.20 protected values

- Rust: 165 passed, 0 failed, using `cargo test --locked --offline --all-targets`
  on macOS arm64 with Rust 1.92.0. Formatting, warning-denying Clippy and
  whitespace checks pass. No dependencies were added.
- VS Code 0.1.15: 16 passed, 0 failed, none skipped. Includes actual engine
  check/run/compile/verify contracts and updated constant/append/prompt help.
- JetBrains 0.3.5: 13 passed, 0 failed; installer builds offline with cached
  IntelliJ 2024.3.7/JDK 21. This is not a new live CLion/PyCharm smoke test or
  multi-version compatibility verification.
- Eleven new Rust gates cover every registered constant alias, nested binding
  validation, discarded value-only calls, independent append copies, preserved
  refusals and preview diagnostics, eager templates across loops/arrays/returns/
  concatenation/text/CSV/TSV/JSON output, literal/nested JSON braces, unknown
  fields, formats/size limits, constant evidence/seals, input/argv/imported
  braces, standalone native execution, old freeze revision enforcement in
  run/compile, historical receipt integrity, policy diffs and marker tampering.
- A real alpha.19 bug-reproduction run still independently verifies under
  alpha.20. Synthetic historical freeze/receipt tests additionally cover
  migration and policy boundaries without rewriting real user evidence.

These are local results, not external CI, clean-machine dependency installation,
authenticated authorship or scientific validation of the complete WB-1/WB-2
workflow. GBL-004 onward remain queued. See [migration](PROTECTED_VALUES.md)
and the backlog. Use `tools/verify_local.py <package-receipt>` to check the
archive inventory and isolated installation after packaging.

## Previous stage: Alpha.19 tables, modules and compiled I/O

- Rust tests (`cargo test --locked --offline --all-targets`): 154 passed,
  0 failed on macOS arm64 with Rust 1.92.0.
- Rust formatting, warning-denying Clippy and whitespace checks: pass.
- VS Code 0.1.14: 16 passed, 0 failed; activation, 230 unique vocabulary entries, import/table
  highlighting and completion checks; actual CLI check/run/verify gates cover
  CSV + imported functions and compiled FITS in addition to previous features.
- JetBrains 0.3.4: 12 passed, 0 failed; cached IntelliJ 2024.3.7/JDK 21 plugin
  build passes. No new live CLion/PyCharm smoke test or multi-version verifier
  is claimed in this stage.
- Nine new Rust integration gates cover combined CSV/TSV + transitive/dedup
  libraries + native FITS/output/plots, quoted Unicode/BOM/CRLF/multiline data,
  explicit numeric refusals and preserved failures, table limits, symlinks,
  invalid library execution, path traversal/cycles/duplicates, frozen library
  notation refusal, native interpolation-name safety, module-only semantic
  diffs, module changes at paranoid postflight, and independent verification
  without live libraries. Tampering table/module bytes, native output bytes,
  support sources or native manifests fails verification.
- Native executables run with no Goblin++ engine available on PATH. Interpreted
  and native output/seal descriptors and payload bytes agree. Existing energy
  canonical hashes and historical receipt/freeze checks remain covered.
- Former blanket compiled-FITS refusal tests now prove lazy branches and
  uncalled functions do not read missing data. Selected/weighted FITS summaries
  independently verify in compiled mode.

These are local test results, not external CI, a clean-machine dependency-cache
test, authenticated authorship or a claim of scientific correctness. Cargo data
builds are locked/offline; standalone custody boundaries are documented in
[the I/O guide](DATA_MODULES_NATIVE.md).

## Previous stage: Alpha.18 statistics step 1

- Rust tests (`cargo test --locked --all-targets`): 145 passed, 0 failed.
- Rust formatting, warning-denying Clippy, and whitespace checks: pass.
- VS Code 0.1.13: 16 passed, 0 failed, including actual alpha.18 interpreted
  and compiled statistics runs and independent receipt verification.
- JetBrains 0.3.3: 11 passed, 0 failed; plugin builds successfully with cached
  IntelliJ Platform 2024.3.7 and JDK 21. This step does not repeat the broader
  alpha.17 multi-IDE compatibility verifier or claim a live IDE smoke test.
- `sum` and `mean` cover known values, compatible unit normalization, SI
  dimensions including current, singleton/all-zero/negative observations,
  cancellation compensation, large/tiny means, invalid type/arity/empty/mixed
  arrays, overflow refusal, both evidence modes, nonmutation, function calls,
  and strict freeze refusal. Failure receipts independently verify.
- The reduction algorithm is shared verbatim by interpreted and generated
  native programs; raw non-finite values are also directly rejected in tests.

These results describe the local alpha.18 step-1 sources, not a published
release, external CI run, or claim that the remaining statistics are available.

## Previous stage: Alpha.17 acceptance results

Current alpha.17 local source-check gates (macOS arm64, Rust 1.92.0):

- `cargo test --locked --all-targets`: 137 passed, 0 failed. Electrical gates
  cover all 18 functions, 34 unit spellings, expected values and dimensions,
  Unicode input, passive-component domains, tiny parallel resistances, reserved
  names, scalar/array native evidence, and independently verifiable failures
  in both engines;
- `cargo clippy --locked --all-targets -- -D warnings`,
  `cargo fmt --all -- --check`, and `git diff --check`: pass;
- release build reports `goblin++ 0.1.0-alpha.17`;
- VS Code extension 0.1.12: 16 passed, 0 failed against the actual alpha.17
  binary, including compiled chemistry/electrical and Unicode-unit input;
- JetBrains plugin 0.3.2: 11 passed, 0 failed; Plugin Verifier reports
  compatible with IntelliJ IC-243.28141.18, PyCharm PY-253.33514.19, and CLion
  CL-262.10968.117;
- the complete `examples/electrical.gbl` passes interpreted and compiled,
  with independently verified receipts and matching sealed artifacts;
- real archived alpha.15 and alpha.16 binaries generated frozen energy runs
  in both modes; alpha.17 independently verifies all four runs and recognizes
  each historical freeze as `FROZEN_VERIFIED`;
- electrical registry SHA-256 at this gate is
  `982afcad3968913c96fcf82e2123d654b64580aa11738f7e114ea161f7513da1`.

These are local source-check results for the alpha.17 working tree, not a new
public release or clean-machine reproduction. IDE verification covers only
the named builds. The engine tests and prebuilt bundle target macOS arm64;
other engine platforms have not been validated at this gate.

## Previous stage: Alpha.16 acceptance results

Current alpha.16 local source-check gates (macOS arm64, Rust 1.92.0):

- `cargo test --locked --all-targets`: 125 passed, 0 failed. Chemistry gates
  cover values and dimensions, unsupported syntax, all 43 registry elements in
  both engines, verifiable run evidence, freeze evidence, and historical
  pre-alpha.16 freeze compatibility;
- `cargo clippy --locked --all-targets -- -D warnings`,
  `cargo fmt --all -- --check`, and `git diff --check`: pass;
- release build reports `goblin++ 0.1.0-alpha.16`;
- VS Code extension 0.1.11: 16 passed, 0 failed against the actual alpha.16
  binary, including compiled chemistry and Unicode-unit input;
- JetBrains plugin 0.3.1: 10 passed, 0 failed; Plugin Verifier reports
  compatible with IntelliJ IC-243.28141.18, PyCharm PY-253.33514.19, and CLion
  CL-262.10968.117;
- `examples/chemistry.gbl` passes in interpreted and native-compiled modes with
  registry `IUPAC-2021-ABRIDGED-COMMON-v1`; both paths are also covered by the
  independently verified integration tests;
- chemistry registry SHA-256 at this gate is
  `c26c5bf4c5df38fa25ffad332a7608727167650bb946f88f372e646c33059f9e`.

These are local source-check results for the alpha.16 working tree, not a new
public release or clean-checkout reproduction. The platform verifier results
cover the named IDE builds; the Rust engine tests ran on macOS arm64.

## Previous stage: Alpha.15 acceptance results

Current alpha.15 local source-check gates (macOS arm64, Rust 1.92.0):

- `cargo test --locked --all-targets`: 118 passed, 0 failed; five new science
  integration tests cover reference conversions, vector algebra, coordinate
  conventions, linear and rotational motion, explicit failure paths, reserved
  names, native parity, and independently verifiable runs;
- `cargo clippy --locked --all-targets -- -D warnings` and
  `cargo fmt --all -- --check`: pass;
- release build reports `goblin++ 0.1.0-alpha.15`;
- VS Code extension 0.1.10 tests against the current binary: 16 passed, 0
  failed, including the real CLI check/run/verify gate;
- `examples/vector_motion.gbl` passes in interpreted and native-compiled modes,
  and both preserved runs independently verify;
- alpha.13 unsuffixed trig calls retain radians results and emit one preserved
  `G302` warning per used legacy function name; older canonical-hash and Python
  0.0.7 compatibility gates remain unchanged.

These are local source-check results for the alpha.15 working tree, not a new
public release, clean-checkout reproduction, or cross-platform result. The
public alpha.13 release and Zenodo record remain unchanged.

## Previous stage: Alpha.14 acceptance results

Alpha.14 passed 113 Rust tests and 16 VS Code tests before the alpha.15 work.
Its gates covered explicit degree/radian forward and inverse trigonometry,
`atan2`, angle conversion, domain and dimension refusals, reserved names,
legacy warnings, interpreter/native parity, and independently verifiable runs.

## Previous stage: Alpha.13 acceptance results

Current alpha.13 local source-check gates (macOS arm64, Rust 1.92.0):

- `cargo test --locked --all-targets`: 112 passed, 0 failed; five new
  alpha.13 integration tests cover scientific-math values and dimensions,
  interpreter/native agreement, verifiable success and failure runs, domain
  and arity refusal, and reserved builtin names;
- `cargo clippy --locked --all-targets -- -D warnings` and
  `cargo fmt --all -- --check`: pass;
- debug build used by the test gates reports `goblin++ 0.1.0-alpha.13`;
- VS Code extension 0.1.8 tests against the current binary: 16 passed, 0
  failed, including the real CLI check/run/verify gate;
- `examples/scientific_math.gbl` is covered by the same interpreter/compiler
  and receipt-verification contract in `tests/math_builtins.rs`;
- older canonical hashes and the retained Python 0.0.7 compatibility corpus:
  pass unchanged.

These are local source-check results for the alpha.13 working tree, not a new
public release, clean-checkout reproduction, or cross-platform result. The
public alpha.12 archive remains unchanged.

## Previous stage: Alpha.12 acceptance results

Alpha.12 passed 107 Rust tests and 16 VS Code tests before its public release.
Its control-flow gates covered short-circuit Boolean logic, nearest-loop
`break`/`continue`, independent direct-array iteration, checked remainder and
integer parsing, nesting, type/range failures, and parse-time loop-control
refusal.

## Previous stage: Alpha.11 acceptance results

Current alpha.11 local source-check gates (macOS arm64, Rust 1.92.0):

- `cargo test --locked`: 101 passed, 0 failed; three new integration tests cover
  scaled FITS selection, missing cells, optional positive weights, a scan
  crossing the 8 MiB chunk boundary, verifiable negative paths, and native
  compilation refusal for FITS calls;
- `cargo clippy --locked --all-targets -- -D warnings`,
  `cargo fmt --all -- --check`, and `git diff --check`: pass;
- `cargo build --locked --release`: pass; binary reports
  `goblin++ 0.1.0-alpha.11`;
- VS Code extension 0.1.6 tests against that binary: 16 passed, 0 failed,
  including a real `fits_select_stats` check/run/verify;
- isolated `examples/fits_selection.gbl` check and paranoid run: pass;
  selected rows 2, used rows 2, weight sum 10, weighted mean Z 1.6125;
  independent `verify`: pass.

The bundled three-row FITS fixture tests functionality, not a DESI science
claim. Real-catalogue cuts, weights, and results still need independent
scientific validation. This is a local check, not a public release, clean
checkout reproduction, or cross-platform result.

## Previous stage: Alpha.10 acceptance results

Current alpha.10 local source-check gates (macOS arm64, Rust 1.92.0):

- `cargo test --locked`: 98 passed, 0 failed, including six new `g_strings` tests and one explicit Python-reference-divergence test;
- `cargo clippy --locked --all-targets -- -D warnings`, formatting, and `git diff --check`: pass;
- `cargo build --locked --release`: pass; binary reports `goblin++ 0.1.0-alpha.10`;
- VS Code 0.1.5 tests against the built alpha.10 CLI: 16 passed, 0 failed, including a compiled text-operation smoke test;
- `examples/strings.gbl` read-only check: `CHECK_STATUS=PASS`;
- interpreted/native text operations, checked conversion of supplied input, Unicode-scalar length, source freeze, sealed values, and independent run verification: pass;
- invalid numeric text, precision-dangerous plain integers, empty separators, mixed types, and text-size/part limits: explicit failures tested.

Text `+` is an intentional alpha.10 extension: Python 0.0.7 rejected it with `G000`. The retained reference corpus still passes, but exact semantic parity is not claimed for this case. These are local checks, not a public release or cross-platform reproduction.

## Previous stage: Alpha.9 acceptance results

Alpha.9 source-check gates (macOS arm64, Rust 1.92.0):

- `cargo test --locked`: 91 passed, 0 failed, including seven new `g_func` tests;
- `cargo clippy --locked --all-targets -- -D warnings`: pass;
- `cargo fmt --all` and `git diff --check`: pass;
- VS Code 0.1.4 tests against the built alpha.9 CLI: 16 passed, 0 failed;
- interpreted and compiled function calls, nested calls, early return, forward declarations, local array copies, confined interpreted output, frozen source, sealed result, and independent run verification: pass;
- missing `return`, wrong arity, recursion limit, unknown symbols, invalid declarations, and compiled FITS-call refusal inside a function: explicit failures tested.

These are local source-check results, not a public release, a clean-checkout reproduction, or a cross-platform result. Python 0.0.7 remains the normative reference while the porting gates are open.

## Previous stage: Alpha.8 acceptance results

Alpha.8 gates (macOS arm64, Rust 1.92.0):

- `cargo test --offline --locked`: 84 passed, 0 failed (80 interaction-stage and regression tests plus four array tests);
- `cargo clippy --offline --locked --all-targets -- -D warnings`: pass;
- `cargo fmt --all -- --check`: pass;
- `node --test` for VS Code 0.1.3 with the alpha.8 CLI: 16 passed, 0 failed;
- release binary built with `cargo build --release --offline --locked` and installed to an isolated prefix: pass;
- arrays example run interpreted and compiled: both `PASS`, both independently verifiable, `diff` = `IDENTICAL_RESULT`;
- independent slice and assignment copies, `append`, `len`, indexed loop, typed array seals, invalid bounds/types, canonical hash, and frozen edit refusal: pass;
- interactive text input, zero-based arguments after `--`, preserved interaction evidence and tamper detection: pass;
- isolated checksum custody ledger audit: pass. No authorship authentication is claimed.

The original Python 0.0.7 reference remains normative while the pending migration gates in `PORTING_MATRIX.md` remain open. The macOS arm64 binary is not a cross-platform build.

## Previous stage: Alpha.6 acceptance results

Current alpha.6 gates (macOS arm64, Rust 1.92.0):

- `cargo test --locked --offline`: 72 passed, 0 failed, including seven new branching tests;
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass;
- `cargo fmt -- --check`: pass;
- `if`/`else if`/`else` and `switch`/`case`/`default` interpreter/compiler agreement, sealed values, and independent run verification: pass;
- lazy branch evaluation, first-match/no-fallthrough, default path, and nested loop branches: pass;
- non-Boolean conditions, unit/type mismatches, malformed branch syntax, preserved machinery failures, frozen-branch edit refusal, and compiled-data-call refusal inside an unreachable branch: pass;
- VS Code 0.1.1 editor/CLI integration: 15 passed, 0 failed.

## Inherited alpha.5 gates

Validation environment: macOS arm64, Rust 1.92.0.

Release gates:

- `cargo test --locked --offline`: 65 passed, 0 failed;
- test breakdown: 20 library, 17 acceptance, 5 CLI, 8 control-flow, 8 paranoid-policy, and 7 reference-compatibility tests;
- everyday program with neither `GO_PARANOID` nor `seal`: pass, with a generated text file and verifiable run;
- TXT/CSV/TSV/JSON/SVG/PNG outputs without `GO_PARANOID`: pass with original path, duplicate, size, and hash safeguards;
- paranoid source postflight evidence and independent rehash: pass; tampering detected;
- changed, missing, or unpreservable source-end evidence: refused with preserved verifiable failure;
- corrupted paranoid run evidence: self-check failure `G405`, no ledger registration;
- run-receipt v1 verification compatibility and malformed-source failure verification: pass;
- Python 0.0.7 normative compatibility: all retained grammar, canonical, semantic, and error-code cases passed; four prior comparison rejections are intentional alpha.4 additions;
- three additional exact Unicode canonical hashes matched Python 0.0.7;
- `cargo clippy --all-targets --locked -- -D warnings`: pass;
- `cargo fmt -- --check`: pass;
- interpreted/compiled `for` and `while` output, sealed values, and independent run verification: pass;
- signed and zero-length ranges, nested loops, Boolean conditions, dimension checks, and syntax refusals: pass;
- infinite `while` reaches the shared one-million-iteration ceiling and creates a verifiable machinery-failure run: pass;
- real FITS pixel accumulation through an interpreted loop with preserved input evidence: pass;
- frozen loop-body edit refused before execution: pass;
- compiled seal inside a loop snapshots the value at seal time: pass;
- release build and bundled-binary installer smoke test: pass;
- release `examples/loops.gbl` interpreted and compiled runs: pass; both verified and diffed as `IDENTICAL_RESULT` with different execution engines;
- installed executable identity: Mach-O 64-bit arm64, invoked as `goblin++`;
- deterministic FITS fixture reproduction from the standalone generator: byte-for-byte pass;
- output tutorial through the installed release binary: pass;
- TXT, CSV, TSV, JSON, SVG, and PNG type recognition: pass;
- PNG decoder and visual-inspection gate: pass;
- generated artifact path, SHA-256, and byte-count verification: pass;
- deterministic generated-output hashes across independent runs: pass;
- changed generated output: detected by `verify`;
- absolute, nested, and parent-traversing output names: refused;
- duplicate output name: refused without overwriting;
- output call without `GO_PARANOID`: accepted and independently verifiable in alpha.5;
- empty or comment-only executed source: preserved machinery failure rather than `PASS`;
- unit-bearing JSON value: SI value, dimension vector, and unit retained;
- compiled output/FITS call: explicitly refused rather than incompletely compiled;
- all alpha.2 FITS, evidence, freeze, revision, ledger, interpreter, compiler, and inline-Rust tests remain passing.

Real-file plotting gate:

- input: `specObj-dr16.fits`, 6,727,078,080 bytes, opened read-only;
- source SHA-256 remained `968ccae3f6ca7a166fcbf3d95e4a6d4b3be59d945e9328132b509f69f5687b0d`;
- binary table: 5,789,200 rows and 133 readable columns;
- a 5,000-row deterministic `Z` histogram sample decoded successfully;
- a 5,000-row deterministic paired `Z`/`Z_ERR` scatter sample decoded successfully;
- `goblin++ check` reported two generated-output previews with `CHECK_STATUS=PASS`;
- authority remained `PREVIEW_ONLY_NOT_EVIDENCE`; no run, plot files, or catalogue copy was created by this gate;
- no scientific interpretation was claimed for the unfiltered samples.

Existing FITS evidence remains:

- fixture SHA-256: `6724062c9bf311427ffa525ca547cb57bdbf461c36bc7321d4061d35295c3d71`;
- FITS evidence reuse uses hard-link identity on Unix and refuses a damaged store;
- large-file discovery and full hashing agree with the independent operating-system checksum;
- input symlinks, random groups, malformed structures, and unsupported value types remain refused.

The alpha.3 real-file FITS/plot gate above is inherited evidence; the 6.7 GB preview was not rerun for alpha.5. This evidence supports an alpha policy implementation, not a production-security claim. It does not waive any pending item in `PORTING_MATRIX.md`.
