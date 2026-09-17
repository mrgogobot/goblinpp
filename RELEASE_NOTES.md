# 0.1.0-alpha.8

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
