# WB-3 requirements and proposed strict research profile

Logged 2026-10-04 from Malin Hess's requirements. Baseline: published
`0.1.0-alpha.22`, source commit
`c3fff61a74cb3a1e4535d9c308bb23ce4e812691`.

This is a design/backlog document, not an implementation or a claim that WB-3
can run completely in Goblin++. `GO_MAD`, `allow_columns()` and
`forbid_columns()` are proposed names only. Do not use them as current syntax.
The WB-3 analysis, its data model and independent expected results still need
review. Existing Chinese documentation and community-review snapshots are
historical and are not modified by this note.

Implementation update: alpha.23 supplies combined scalar FITS cuts and CSV/TSV
projection in both engines, including exact unscaled signed 64-bit IDs as text.
See [FITS_SUBSETS.md](FITS_SUBSETS.md). This is the first extraction stage only;
FITS writing, general integer arithmetic, streaming output and physical
blinding are not implemented. Alpha.24 implements the core GBL-007 distribution
functions in both engines: stable copy sorting, type-7 quantiles/median,
explicit population/sample standard deviation and unweighted ECDF. Policies
are recorded in new run/freeze receipts; see [STATISTICS.md](STATISTICS.md).
Bootstrap, seeded RNG and weighted distributions remain acceptance goals.

## Requested priorities and present coverage

| Priority | Requirement | Existing coverage and remaining work |
| --- | --- | --- |
| P0 | sort, median, quantile, ECDF | Core GBL-007 implemented locally in alpha.24, including explicit population/sample standard deviation; bootstrap/weights pending |
| P0 | Deterministic seeded RNG | GBL-008; pin algorithm/variant/version, seed encoding, streams and sampling mappings |
| P0 | Multi-column FITS filtering/export | GBL-006 alpha.23 local CSV/TSV stage; exact unscaled IDs and both engines covered; FITS writer/physical blinding pending |
| P0 | dot, cross, norm, unit, matrix multiply | dot/cross/magnitude exist; GBL-018 adds checked normalization and explicit matrix representation/operations |
| P0 | Covariance, Cholesky, multivariate normal | GBL-019; depends on matrix, units, RNG and normal-transform contracts |
| P0 | Streaming and declared iteration budget | GBL-009/012; loop cap remains 1,000,000; measure the reported 400,000-orbit workload |
| P1 | Hash-based row partitioning | GBL-020; exact stable IDs, fixed hash/encoding and declared fold mapping |
| P1 | Formal permitted-column controls | GBL-021; enforcement and evidence disclosure must be designed, not only logged |
| P1 | Bootstrap/resampling | GBL-007 plus GBL-008; declare resampling unit, replacement, statistic and interval assumptions |
| P1 | Specified float serialization | GBL-004 implemented alpha.21; GBL-005 comparison/identity implemented alpha.22; no universal deterministic math guarantee |
| P1 | Compound unit literals | GBL-013 remains open: `1.13e-10 m/s^2` must parse without changing expression precedence silently |
| P1 | Constants namespace | GBL-024; alpha.20 already prevents silent constant shadowing, but does not implement a namespace |
| P2 | String and append semantics | GBL-001/002/003 covered alpha.20; keep regressions and copy semantics, not a new hidden mutable mode |
| P2 | Joins by source ID | GBL-025; depends on exact ID representation and table/streaming contracts |

The old WB-1 tiers are retained in DEVELOPMENT_BACKLOG.md; WB-3's priorities
are an additional workflow view, not a retroactive reclassification of releases.
Exact IDs (GBL-015) are a prerequisite wherever Gaia identifiers would otherwise
pass through f64, even though this table lists joins as P2.

## P0 contracts

### Statistics and ECDF — GBL-007

- Keep sort copy-based. Specify stable tie ordering, signed zero, finite-value
  rules, dimensions and empty/singleton behavior; no silently dropped values.
- Pin the quantile definition/interpolation by a named, versioned convention.
  Define median consistently, including even-length input and overflow-safe
  interpolation. Sample versus population standard deviation remains explicit.
- For the unweighted ECDF, define `F(x) = count(x_i <= x) / n`, including all
  ties at x. Results are dimensionless; observations and query values must have
  compatible dimensions. Empty samples refuse; weighted ECDF is separate work.
- Tests: hand-calculated fixtures, ties, units, extrema, invalid observations,
  mutation independence, interpreter/native agreement and platform tolerances.

### RNG and transformed sampling — GBL-008

- Choose a particular algorithm/variant/version, not just "PCG" or "ChaCha".
  Specify seed bytes, endianness, initialization, stream derivation, raw-word
  sequence, integer range mapping and float interval/bit mapping.
- Require an explicit seed in strict audited runs. Record algorithm identity,
  implementation/version, seed, stream IDs and draws/operations sufficiently
  to replay. Never silently seed from time or reseed a bootstrap loop.
- Bind stream assignment to stable row/orbit/replicate identities if order or
  parallel scheduling should not change the simulation. Define chunk-boundary
  and worker-count behavior before promising scheduling-independent results.
- Raw RNG sequences can be bitwise reproducible while a normal transform that
  uses platform log/sqrt/trig is not. Version and test that transform separately;
  declare its exact-bit or explicit-tolerance promise. A seed alone is not a
  complete reproducibility contract.
- Published raw algorithm vectors, cross-engine/platform replay, boundary and
  invalid-parameter tests, stream independence design and preserved evidence
  are required. Scientific reproducibility is not cryptographic suitability.

### FITS selection and export — GBL-006

- Explicit named HDU, permitted columns, typed predicates, AND/OR grouping,
  inclusive/exclusive boundaries, null/invalid-value policy and stable row order.
- Preserve input hash and identity, predicate, requested columns, selected
  counts, row-order policy and output hashes. Distinguish access logs from a
  scientifically justified blind selection.
- CSV/TSV subset output is the first bounded scope; FITS writing is separate
  unless explicitly implemented and tested. Existing FITS readers/writers must
  participate in interpreter/native evidence parity.
- Never export numeric source IDs through f64. Design exact integer support or
  a lossless typed/text ID path first; forbid lossy implicit conversions.
- Fixtures cover combined cuts, nulls, empty/all selections, ties/boundaries,
  exact IDs, streaming/chunk independence, round-trip and tampering failures.

### Vector/matrix primitives — GBL-018

- Reuse existing `dot`, `cross` and `magnitude`; `norm` could be a documented
  Euclidean alias, but its meaning must not conflict with future matrix norms.
  `unit(vector)` would return a dimensionless normalized copy and refuse zero
  or nonfinite magnitude. Avoid avoidable overflow/underflow.
- Current arrays are one-dimensional and homogeneous. Choose an explicit matrix
  representation and shape contract; adding nested arrays is not implicit in
  this proposal. Define row/column orientation and matmul separately from
  element-wise multiplication, with no silent broadcasting or truncation.
- Products derive dimensions. Rotations state axes, handedness, angle units,
  frame, epoch and multiplication order. Coordinate transforms are not an
  orbital integrator or a Gaia uncertainty model by themselves.
- Test shape/unit refusal, zero vectors, known orthogonal rotations and basis
  vectors, inverse round trips, numerical extremes and native parity.

### Covariance and Cholesky — GBL-019

- Define variable ordering and reference frame explicitly. In physical units,
  covariance entry C_ij has dimension unit_i * unit_j; this cannot be silently
  stored as one homogeneous unitful array. A dimensionless correlation matrix
  plus declared scales, or a typed covariance object, needs a separate contract.
- Validate square shape, finite entries, nonnegative diagonal, declared
  symmetry tolerance and positive-definiteness. Ordinary Cholesky requires
  positive-definite input; singular positive-semidefinite input needs a separate
  refusal or explicitly selected factorization policy.
- Never silently add diagonal jitter, clip eigenvalues, rescale errors or
  "repair" correlations. If an optional regularization is later introduced,
  record its method and magnitude and preserve the original covariance.
- State the factor convention (for example C = L L^T) and sampling convention
  (mu + L z). Sampling depends on GBL-008's normal-transform contract.
- Hand-checked matrices, singular/indefinite/asymmetric cases, mixed-unit
  variables, reconstruction, reproducible draws and predeclared statistical
  tests are required. Gaia parameter ordering, angular units, correlations and
  frame/epoch must come from the actual dataset schema, not column-name guesses.

### Catalogue-scale execution — GBL-009/012

- `GO_LOOP_BUDGET 50000000` remains proposed. Record requested/effective budget,
  configuration and host cap; loops/functions share it predictably in both
  engines. Higher loop budgets do not solve array, file, memory or wall-time caps.
- Use bounded chunked processing with stable ordering, exact ID handling and
  preserved import/output evidence. No silent sampling or chunkwise changes to
  reduction semantics. Test bounded memory, late malformed rows, input mutation,
  exhaustion/refusal evidence and workload size.

## P1/P2 extensions

### Deterministic partitions — GBL-020

Version the hash algorithm, domain separator, ID encoding, declared partition
key/seed and fold mapping. Never use process-randomized hashes, row position or
f64-rounded IDs. Decide whether duplicates stay together and whether grouped
objects require one group identity. Reordering/chunking must not change an ID's
fold. Hash collisions must not accidentally merge distinct join identities.
Freeze the partition rule before examining outcomes; deterministic partitioning
does not stop a researcher trying many keys and reporting only the best one.

### Constants namespace — GBL-024

`h` currently resolves to Planck's constant and assigning it is refused clearly
in alpha.20 onward. An explicit namespace is an ergonomic extension, not an
unfixed silent-collision bug. Namespace syntax, compatibility with unqualified
aliases, ordinary-variable shadowing and migration require a design decision.
Do not silently change the meaning of existing `h`, expressions, templates or
seals. Retain the protected-alias regression suite.

### Exact-ID joins — GBL-025

Declare inner/left/other join kinds, duplicate-key cardinality, missing/null
keys, exact string/integer identity, output row order and column-name conflicts.
No guessed coercion, silent row loss or many-to-many explosion. Record both
input hashes, join key/rule, matched/unmatched counts and output hash; require
bounded-memory and interpreter/native fixtures above 2^53 where integers apply.

## Enforced blinding and GO_MAD — GBL-021 and GBL-023

`GO_MAD` is proposed as a versioned strict research profile that **includes**
GO_PARANOID's existing checks and refuses when required policy is unavailable.
It must not become a dramatic label for ordinary runs, claim scientific truth,
or imply encryption/authenticated authorship. Exact syntax remains undecided.

Proposed required contract:

- Explicit immutable analysis/policy manifest, source/canonical/module hashes,
  runtime/engine/build identity and math/serialization policy.
- Explicit dataset/HDU identities and permitted-column capability set. If a
  denylist is also supplied, deny wins; contradictory/unknown rules refuse.
  No implicit wildcard access to newly added columns or alternate filenames.
- Conditional RNG algorithm/seed/stream/distribution-transform metadata, or an
  explicit no-RNG-used record. Explicit budgets and scientifically justified
  tolerances; no hidden scientific defaults.
- Required declared outputs/seals, generated-file hashes and postflight
  verification. Missing/invalid requirements produce preserved refusal, not
  a warning followed by PASS.
- Exact verified parent-run/selection/partition/policy hashes, with acyclic
  references and completeness checks; distinguish experiment lineage from
  source revision lineage. Parents must satisfy the declared accepted-status
  policy, not be treated as scientifically valid just because hashes match.
- Freeze the analysis and partition policy before unblinding. Any later change
  is an explicit revision/event, not a rewritten receipt. An unblinding event
  names its reason and authorization; checksum-only evidence cannot establish
  the real identity of the person who authorized it.

### The critical boundary: access control is not logging

Checking column permissions in every FITS/table entry point can restrict the
supported language API. It must cover data values, filters, weights, plots,
headers/metadata and exports consistently, including native support and its
reference evaluation. Unknown/alternate access routes fail closed.

However, the current evidence store preserves complete FITS/CSV/TSV input
bytes. Archiving a full catalogue beside a "blind" run exposes forbidden
columns outside the accessor. Hashing a dataset is a commitment to bytes, not
redaction, confidentiality, or proof of column-access control. Headers, counts,
IDs and diagnostic/output channels can also leak outcomes; the threat model
must state which are permitted.

For the stronger claim that a blind process physically cannot read forbidden
data, use a separate trusted extraction stage and isolated execution boundary:

1. A trusted extractor reads the original catalogue, commits its identity and
   writes only the approved blind subset and provenance manifest.
2. The blind worker receives only that subset, approved source/modules and
   explicit policy. Original inputs and full raw evidence are inaccessible
   under host/process/container access controls; confidential upstream evidence
   stays with the trusted custodian, not the worker's run directory.
3. All supported interpreter/native access honors the capability set. Arbitrary
   inline Rust, external processes and undeclared file/network access are
   prohibited in this profile unless an independently tested sandbox really
   enforces the same boundary. Goblin++ currently provides no such OS sandbox.
4. Deliberate denied-read tests cover interpreter, compiled code, imports,
   alternate paths/columns, raw evidence, diagnostics and policy tampering.
   Document the malicious/accidental access threat model and what remains
   outside it. A process boundary does not blind a human custodian who retains
   access to the original catalogue.

No GO_MAD or physical-blinding claim should ship before these contracts and
negative tests are implemented. Local hash chains alone also cannot prevent
wholesale replacement of an experiment history; authenticated/external anchors
would be a separate capability, not implied by parent-run hashes.

## Science regression suite — GBL-022

Malin proposes the eight entropic runs and WB-1 as candidate fixtures and reports
cross-platform reproduction with matching source hashes and all paranoid
postflight checks passing. This note preserves that report; it has **not**
inspected those run directories, inputs, values or independent expectations.
The inline chat reference markers are not usable file paths or source URLs.

Before calling them frozen regression references:

- Obtain the exact source/module/data manifests, all relevant run hashes,
  policies, units, platforms and independently justified expected quantities.
  Distinguish analytic reference values from historical measured outputs.
- Define per-quantity absolute/relative tolerances and reference roles before
  testing a new candidate; never fit tolerances to make that candidate pass.
  Matching source/postflight evidence alone does not verify a scientific model.
- Split small redistributable fixtures for routine CI from optional large/local
  catalogue gates. Review data redistribution, privacy and blinded-field
  exposure before putting evidence in a public repository or release.
- Verify historical bytes exactly first, then assess numerical agreement as a
  separate gate. Preserve reference files unchanged; engine/policy migrations
  get explicit revised fixtures without overwriting old runs/freezes.
- Exercise both engines and actual macOS/Linux runners, including denied access,
  tampering, incomplete/failed runs, altered units and invalid scientific inputs.
  Archive reports that name exactly which reference role passed or failed.

Shortest-round-trip text is already specified. Do **not** quantize authoritative
seals to make different platform values hash alike. Keep raw measured values and
exact hashes; any rounded presentation or tolerant comparison is separate and
explicit. Receipts intentionally include environment/run identity, so complete
receipt hashes can differ even when scientific results agree exactly.

## Suggested implementation sequence (not an execution authorization)

1. Continue GBL-006: combined FITS predicates plus bounded CSV/TSV export,
   preserving exact IDs. Define blinding data flow now so it is not retrofitted
   onto a full-data evidence leak.
2. Add GBL-007's sort/median/quantile/ECDF core and adopt reviewed small science
   fixtures as they become available. No WB-3 end-to-end claim yet.
3. Implement the RNG/stream/normal-transform contract, with a replayable test
   corpus, then explicit resampling helpers.
4. Matrix/normalization primitives, followed by covariance/Cholesky/sampling;
   streaming and budgets proceed as dependencies for catalogue-scale gates.
5. Partitions, enforced blinding and a versioned strict GO_MAD profile once its
   prerequisites and host isolation are testable. Exact-ID joins follow their
   table/identity foundations.

Every delivered stage needs runtime tests, preservation/refusal evidence, both
editor updates, current docs and a separately approved release. Logging WB-3
does not authorize implementing all of it, changing published alpha.22 assets,
or editing the historical Mandarin/community-review documentation.
