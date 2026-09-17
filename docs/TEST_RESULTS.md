# Alpha.8 acceptance results

Current alpha.8 gates (macOS arm64, Rust 1.92.0):

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
