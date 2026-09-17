# Rust engine architecture

The engine is intentionally split at trust boundaries:

- `lexer`, `parser`, and `ast` turn UTF-8 source into canonical meaning;
- `quantity` and `constants` implement deterministic scalar science semantics;
- `evaluator` interprets the AST and records constants, output, seals, and data access;
- `fits` streams read-only multi-HDU metadata and image/table values without a C or Python bridge;
- `output` validates artifact names and deterministically renders delimited text, SVG, and lossless PNG bytes;
- `compiler` emits reviewable Rust and invokes `rustc` without a shell;
- `inline_rust` extracts exact code bytes and enforces digest authorization;
- `runtime` performs custody preflight, execution, and evidence preservation;
- `custody`, `ledger`, and `audit` freeze, chain, and independently verify history;
- `main` exposes the `goblin++` user interface.

Interpretation is the default. Compilation is never inferred from source contents. Inline Rust cannot cause an implicit switch to native execution.

The compiler emits direct operations over generated quantity values. It does not bundle or invoke the source parser at runtime. This distinction is tested by inspecting generated source and by sealing the generated source and executable.

## FITS data path

FITS headers are read in 2,880-byte blocks. Values use positional reads from a held-open file descriptor; numeric column statistics scan bounded chunks rather than allocating the file. The full input hash is streamed separately. `fits-info --quick` deliberately skips that full hash and labels its output as un-hashed reconnaissance.

Run evidence is content-addressed under `.goblin/imports/SHA256.fits`. The first use streams a candidate copy, hashes that copy, and registers it only if it matches the bytes opened for evaluation. Each run receives a hard link to the stored object when supported, or an ordinary copy as a fallback. Independent run verification hashes the attached evidence again.

## Generated output path

Output calls construct bytes during interpreter evaluation but cannot choose an operating-system path. The runtime materializes those bytes with exclusive creation beneath the new run's `outputs` directory. The receipt binds filename, media type, byte count, producer, metadata, and SHA-256. The verifier accepts only flat `outputs/NAME` receipt paths and rehashes every artifact.

FITS plots select deterministic evenly spaced row indexes and record the population, requested, examined, and valid counts. PNG encoding uses fixed RGB dimensions, filter type zero, and stored DEFLATE blocks; SVG numeric layout is likewise deterministic. Rendering does not depend on system fonts for PNG and introduces no plotting-library runtime dependency.
