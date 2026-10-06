# Rust engine architecture

The engine is intentionally split at trust boundaries:

- `lexer`, `parser`, and `ast` turn UTF-8 source into canonical meaning;
- `quantity` and `constants` implement deterministic scalar science semantics;
- `evaluator` interprets the AST and records constants, output, seals, and data access;
- `fits` streams read-only multi-HDU metadata and image/table values without a C or Python bridge;
- `delimited` snapshots bounded UTF-8 CSV/TSV with explicit numeric conversion;
- `modules` statically expands definitions-only local imports and preserves the raw source graph;
- `output` validates artifact names and deterministically renders delimited text, SVG, and lossless PNG bytes;
- `compiler` emits reviewable Rust and invokes `rustc`, or offline locked Cargo for shared data helpers, without a shell;
- `inline_rust` extracts exact code bytes and enforces digest authorization;
- `runtime` performs custody preflight, execution, and evidence preservation;
- `custody`, `ledger`, and `audit` freeze, chain, and independently verify history;
- `main` exposes the `goblin++` user interface.

Interpretation is the default. Compilation is never inferred from source contents. Inline Rust cannot cause an implicit switch to native execution.

Alpha.25 declares `goblin.compound-unit-literals.v1` in current evidence.
Numeric unit suffixes lower to existing quantity AST operations; the coefficient
is not raised by a unit power. Historical receipts without this declaration
use the legacy parser, including preserved module graphs, for verification
only. New execution/compilation refuses older freezes until an explicit revision
adopts the current policy. See [compound-unit syntax and migration](COMPOUND_UNITS.md).

The compiler emits direct operations over generated quantity values. It never invokes the source parser at runtime. Data programs link a shared support library whose sources include the existing parser because of module dependencies; no Goblin source evaluation occurs in the generated entry point. This distinction is tested by inspecting generated source, running standalone without the engine on PATH, and sealing generated source, support inventory and executable.

## FITS data path

FITS headers are read in 2,880-byte blocks. Values use positional reads from a held-open file descriptor; numeric column statistics scan bounded chunks rather than allocating the file. The full input hash is streamed separately. `fits-info --quick` deliberately skips that full hash and labels its output as un-hashed reconnaissance.

Run evidence is content-addressed under `.goblin/imports/SHA256.fits`. The first use streams a candidate copy, hashes that copy, and registers it only if it matches the bytes opened for evaluation. Each run receives a hard link to the stored object when supported, or an ordinary copy as a fallback. Independent run verification hashes the attached evidence again.

## Generated output path

Output calls construct bytes during interpreter evaluation but cannot choose an operating-system path. The runtime materializes those bytes with exclusive creation beneath the new run's `outputs` directory. The receipt binds filename, media type, byte count, producer, metadata, and SHA-256. The verifier accepts only flat `outputs/NAME` receipt paths and rehashes every artifact.

Alpha.19 native calls use the same Rust FITS/table/output helpers, with already
evaluated arguments from generated code. Audited compiled runs compare native
input access/hash descriptors, seals, output metadata and bytes against a
reference evaluation. Native manifest and output copies are independently
verified. This performs two evaluations; standalone execution has different
custody and working-directory boundaries. See [the I/O guide](DATA_MODULES_NATIVE.md).

## Library and table evidence

Modules are statically expanded into the canonical AST without executing
library top-level statements. Raw source graph entries are included in freeze
receipts and run evidence. Verification resolves only preserved module bytes.
CSV/TSV access operates on a bounded starting snapshot; tables are retained
under the existing checksum-addressed input evidence mechanism with `.csv`
or `.tsv` extensions and explicit operation/column access descriptors.

FITS plots select deterministic evenly spaced row indexes and record the population, requested, examined, and valid counts. PNG encoding uses fixed RGB dimensions, filter type zero, and stored DEFLATE blocks; SVG numeric layout is likewise deterministic. Rendering does not depend on system fonts for PNG and introduces no plotting-library runtime dependency.
