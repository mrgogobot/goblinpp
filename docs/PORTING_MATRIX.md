# Python 0.0.7 to Rust porting matrix

Reference artifact: `goblinpp-v0.0.7.zip`, SHA-256 `898b7a710672f5f712bbe002f28af46bd0f98d96b02f2cce4aec014a11a4f500`.

“Implemented” means exercised by Rust tests. “Pending” is a refusal to claim parity without evidence.

| Capability | Alpha.19 status | Evidence or boundary |
|---|---|---|
| Lexer, parser, AST | Implemented | Explicit/implicit multiplication, Unicode superscripts, aliases, comments, strings |
| Everyday control flow | Extended, implemented | Range/direct-array `for`, `while`, nearest-loop `break`/`continue`, short-circuit `and`/`or`/`not`, `if`/`else if`/`else`, `switch`/`case`/`default`, comparisons, nested control flow, interpreter/compiler parity and loop ceiling |
| Input and arguments | Extended, implemented | `input`, `argc`, `argv`, `--` separator, preserved plaintext interaction evidence; explicit checked `parse_number` and `parse_integer` conversion |
| Text operations (`g_strings`) | New, implemented | Text `+`, `len(text)` in Unicode scalar values, `to_text`, `parse_number`, `parse_integer`, `str_trim`, `str_contains`, `str_replace`, `str_split`, `str_join`; interpreter/native parity and bounded results. Python 0.0.7 rejected text `+` with G000; this is an explicit language extension, not reference parity. |
| One-dimensional arrays and slices | New, implemented | Homogeneous arrays; indexing, assignment, independent half-open slices and iteration values, `len`, `append`; 100,000-item cap; interpreter/compiler parity |
| Dimensional quantities | Implemented | Six base dimensions including electric current; historical five-axis evidence retained; mismatch and numeric-domain failures tested |
| Scientific mathematics | New, implemented | Dimension-aware `abs`, `sqrt`, extrema and `hypot`; checked dimensionless rounding, exponential, logarithmic and trigonometric functions; interpreter/compiler parity and preserved failures |
| Array statistics | New, implemented | Numeric `sum`/`mean`, stable copy `sort`, type-7 `median`/`quantile`, explicit population/sample standard deviation and unweighted ECDF; dimensions, shared runtime, policy receipts and native parity; variance, weights, RNG and bootstrap pending |
| Checked integer remainder | New, implemented | `%` requires safe dimensionless integers, rejects zero divisor, and has interpreter/compiler parity |
| Constants and units | Extended, implemented | Python 0.0.7 registry plus alpha.16 chemistry and alpha.17 SI electrical units; registry contents are freeze evidence |
| Chemistry foundation | New, implemented | Scoped 43-element abridged registry, formula molar mass, mass/amount/concentration/dilution helpers, interpreter/native parity, explicit unsupported-syntax refusal; no isotope, reaction, pH, equilibrium, kinetics, or biology inference |
| Electrical engineering foundation | New, implemented | SI current axis and 34 electrical unit spellings; 18 dimension-checked DC/passive helpers, explicit signed KCL/KVL residuals and caller-supplied tolerances, native parity and verifiable failures; no AC phasor or circuit-topology solver |
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
| Filtered/weighted FITS table summary | New, implemented in working tree | `fits_select_stats` streams scalar numeric rows with explicit half-open bounds, optional positive weights, selected/used row counts, and sealed/verifiable output; no implicit survey cuts or unit conversion |
| Large FITS evidence | New, implemented | Streaming SHA-256 and one checksum-addressed stored copy, hard-linked per run |
| FITS ASCII tables/compression/special columns | Pending | Discoverable metadata; unsupported values are explicitly refused |
| FITS calls in compiled programs | New, implemented | Shared Rust helpers; native input hashes and accesses must match reference; support inventory preserved |
| Output calls in compiled programs | New, implemented | Native output metadata and bytes compared, then independently verified; standalone custody boundary documented |
| Bulk table export and FITS writing | Pending | Requires explicit output schema, unit, and provenance contracts; the summary operation does not export rows |
| User-defined functions | Implemented in working tree | `g_func`, parameters, explicit `return`, local copy-value scope, forward calls and 16-call limit; interpreter/native parity tested |
| Local function libraries | New, implemented | Static definitions-only imports, cycle/traversal/symlink/duplicate refusal, frozen raw graph, independent module-evidence verification |
| CSV/TSV reading | New, implemented | Bounded UTF-8 snapshots, explicit delimiter and text/numeric column choice, strict headers/row shape, no silently skipped numeric cells, both engines |
| Nested collections | Pending | Arrays remain one-dimensional |
| Ed25519 ledger authorship | Pending | Alpha reports `CHECKSUM_ONLY` honestly |
| Interrupted-head recovery event | Pending | Do not use alpha ledger for crash-recovery claims |
| Adoption of pre-ledger freeze receipts | Pending | No compatibility command yet |
| Normative lexicon/grammar/semantics/tutorial CLI validators | Pending | Python reference remains normative during migration |
| Editor integrations | Implemented; updated sources not yet released | VS Code 0.1.14 and JetBrains 0.3.4 sources add table and import hints alongside chemistry/electrical; their hints are advisory |

## Promotion gates

The Rust engine should not replace Python 0.0.7 as the stable reference until:

1. The full normative grammar and semantic corpora run against both engines, with deliberate versioned extensions (including text `+`) recorded as explicit exceptions and tested separately; no unlabelled divergence is accepted.
2. Receipt, freeze, lineage, and ledger schemas have explicit migration rules.
3. Signing, recovery, and legacy adoption are ported with negative-path tests.
4. FITS numerical semantics expand beyond the fixed-width `specObj-dr16.fits` gate to compressed images, special columns, declared units, and archive-diverse fixtures.
5. A clean-machine release build reproduces the test result and manifest.
