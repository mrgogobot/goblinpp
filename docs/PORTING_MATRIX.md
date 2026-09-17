# Python 0.0.7 to Rust porting matrix

Reference artifact: `goblinpp-v0.0.7.zip`, SHA-256 `898b7a710672f5f712bbe002f28af46bd0f98d96b02f2cce4aec014a11a4f500`.

“Implemented” means exercised by Rust tests. “Pending” is a refusal to claim parity without evidence.

| Capability | Alpha.8 status | Evidence or boundary |
|---|---|---|
| Lexer, parser, AST | Implemented | Explicit/implicit multiplication, Unicode superscripts, aliases, comments, strings |
| Everyday control flow | Extended, implemented | `for range`, `while`, `if`/`else if`/`else`, `switch`/`case`/`default`, comparisons, Boolean values, nested control flow, interpreter/compiler parity and loop ceiling |
| Input and arguments | New, implemented | `input`, `argc`, `argv`, `--` separator, preserved plaintext interaction evidence; no secrets or text-to-number conversion |
| One-dimensional arrays and slices | New, implemented | Homogeneous arrays; indexing, assignment, independent half-open slices, `len`, `append`; 100,000-item cap; interpreter/compiler parity |
| Dimensional quantities | Implemented | Five base dimensions; mismatch and numeric-domain failures tested |
| Constants and units | Implemented | Python 0.0.7 registry values and aliases |
| Output, interpolation, sealing | Extended, implemented | `print`, formats, sealed variables, audited TXT/Markdown/CSV/TSV/JSON files |
| Deterministic plotting | New, implemented | Sampled FITS histogram/scatter in SVG/PNG with sampling metadata and digest verification |
| Canonical program hashing | Implemented | Energy hash matches Python reference exactly |
| `GO_PARANOID` | Extended, opt-in | Postflight source-byte observation and evidence; receipt self-verification before ledger registration; not continuous monitoring or a sandbox |
| Immutable run evidence | Implemented | Source, logs, artifacts, checksum-addressed imported bytes, compiler artifacts |
| `check` preview | Implemented | Read-only; no custody writes |
| Freeze and strict enforcement | Implemented | Receipt seal, bytes, canonical form, constant registry, registration |
| Revision lineage | Implemented | Required reason, no overwrite, sealed parent reference |
| Verify and semantic diff | Implemented | Independent evidence rehashing and classifications |
| Checksum custody ledger | Implemented | Hash chain, head checkpoint, append lock, tamper refusal |
| Native scalar and array compilation | New, implemented | Direct Rust operations, value-copy arrays, generated source and binary sealed |
| Inline Rust | New, implemented | Compiled mode only; exact SHA-256 authorization; immutable Goblin values |
| FITS image import | New, implemented | Pure Rust primary/image HDUs, explicit indexes, bounded-memory reads |
| FITS HDU/header discovery | New, implemented | Hashed or explicitly un-hashed `fits-info`; JSON and human output |
| FITS binary tables | New, implemented | Fixed-width logical/text/integer/real columns, repeats, nulls, scaling, statistics |
| Large FITS evidence | New, implemented | Streaming SHA-256 and one checksum-addressed stored copy, hard-linked per run |
| FITS ASCII tables/compression/special columns | Pending | Discoverable metadata; unsupported values are explicitly refused |
| FITS calls in compiled programs | Pending | Compiler refuses; interpreter is native Rust |
| Output calls in compiled programs | Pending | Compiler refuses; interpreter output remains native Rust |
| Bulk table export and FITS writing | Pending | Requires explicit filtering, schema, unit, and provenance contracts |
| Nested collections and user-defined functions | Pending | One-dimensional arrays are implemented; nested arrays and functions are not |
| Ed25519 ledger authorship | Pending | Alpha reports `CHECKSUM_ONLY` honestly |
| Interrupted-head recovery event | Pending | Do not use alpha ledger for crash-recovery claims |
| Adoption of pre-ledger freeze receipts | Pending | No compatibility command yet |
| Normative lexicon/grammar/semantics/tutorial CLI validators | Pending | Python reference remains normative during migration |
| VS Code extension backend switch | Implemented, separately packaged | Extension 0.1.3 invokes the Rust CLI and supports prompts, arguments, and array authoring; its hints are advisory |

## Promotion gates

The Rust engine should not replace Python 0.0.7 as the stable reference until:

1. The full normative grammar and semantic corpora run against both engines.
2. Receipt, freeze, lineage, and ledger schemas have explicit migration rules.
3. Signing, recovery, and legacy adoption are ported with negative-path tests.
4. FITS numerical semantics expand beyond the fixed-width `specObj-dr16.fits` gate to compressed images, special columns, declared units, and archive-diverse fixtures.
5. A clean-machine release build reproduces the test result and manifest.
