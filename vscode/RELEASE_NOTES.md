# VS Code extension 0.1.22 (local, unreleased)

Updated for Rust alpha.27: completion, highlighting, help and a bootstrap
snippet for explicit-stream resampling, normal sampling, flat row-major
matrices, covariance, Cholesky, multivariate normals, bounded CSV/TSV scans
and `GO_LOOP_BUDGET`. Help exposes required divisors, tolerance, stream and
resource choices. Both editors use the same 263-spelling vocabulary.
The engine, not the editor, owns execution and evidence semantics.

## Previous stage: 0.1.21

Added alpha.26 seeded PCG32 scientific RNG help, highlighting and snippets.
Seeds and streams are explicit; this generator is not cryptographic.

## Previous stage: 0.1.20

Updated for Rust alpha.25 with compound-unit help and suffix highlighting.
`3 m^2` denotes three square metres; `(3 m)^2` squares the whole quantity.
Multiple denominator factors require parentheses. The engine owns semantics,
legacy-evidence verification and explicit freeze migration, not this editor.

## Previous stage: 0.1.19

Updated for Rust alpha.24 with six distribution-function completions, snippets,
highlighting and help. Quantiles use type 7; standard deviation names specify
sample/population divisors, ECDF includes ties and sort returns a stable copy.
Requires alpha.24 for execution; the editor does not reimplement statistics.

## Previous stage: 0.1.18

Updated for Rust alpha.23 with highlighting, completion, help and snippets for
`fits_where`, `fits_all`, `fits_any`, `fits_column_text`, `fits_export_csv` and
`fits_export_tsv`. Help distinguishes exact ID text from integer arithmetic and
subset extraction from physical blinding. No execution semantics live in the editor.
Previous alpha.22 comparison and alpha.21 lossless-text help is retained.

## Previous stage: 0.1.14

Updated for Rust alpha.19: `import`, ten explicit CSV/TSV reader completions,
snippets and function help, plus syntax highlighting. Existing FITS and output
calls now work in compiled runs with an alpha.19 engine. Editor hints do not
replace the engine's strict table, module or evidence checks.

## Previous stage: 0.1.13

Updated for the first Rust alpha.18 statistics step: completion, snippets,
function help and highlighting for dimension-preserving `sum` and `mean`.
Requires an alpha.18 engine for execution of these functions.

## Previous stage: 0.1.12

Updated for Rust alpha.17. Added the 18 electrical `ee_` functions, SI electrical
unit completion and highlighting, snippets, and six-axis dimension labels.
The chemistry vocabulary remains included. Function semantics are defined by
the installed Goblin++ engine.

## Previous stage: 0.1.11

Updated for Rust 0.1.0-alpha.16. Added highlighting, completion help, and
snippets for the versioned chemistry registry, molar mass, mass/amount,
concentration, dilution, practical laboratory units, `R`, and `m_u`.

## Previous release: 0.1.10

Updated for Rust 0.1.0-alpha.15. Added highlighting and detailed completion
help for astronomical distance conversion, vector algebra, explicit
polar/spherical coordinate conversion, linear velocity, named Galilean and
collinear relativistic velocity addition, and rotational-motion helpers.

## Previous release: 0.1.9

Added highlighting, completion help, and snippets for explicit degree/radian
trigonometry, inverse functions, `atan2d`/`atan2r`, and
`deg2rad`/`rad2deg`. Unsuffixed trigonometric names are marked as deprecated
radians-compatible aliases; the engine owns warning and execution semantics.

## Previous release: 0.1.8

Updated for Rust 0.1.0-alpha.13. Added highlighting, completion help, and
snippets for the dimension-aware scientific-math built-ins, including `sqrt`,
trigonometry, logarithms, extrema, rounding, `atan2`, and `hypot`.

## Previous release: 0.1.7

Updated for Rust 0.1.0-alpha.12. Added highlighting, completion help, and
snippets for direct array iteration, `break`, `continue`, short-circuit
`and`/`or`/`not`, integer remainder, and `parse_integer`. Runtime semantics
remain authoritative in the Goblin++ engine.

## Previous release: 0.1.6

Updated for Rust 0.1.0-alpha.11. Added `fits_select_stats` highlighting,
completion help, and a snippet for explicit filtered/weighted FITS summaries.
This is editor source only until a reviewed VSIX is packaged.

## Previous release: 0.1.5

Updated for Rust 0.1.0-alpha.10. Added `g_strings` builtin highlighting, completions, and snippets for checked text-to-number conversion and split/join. Text `+` and `len(text)` work in the engine; the extension remains advisory.

## Previous release: 0.1.4

Updated for Rust 0.1.0-alpha.9. Added `g_func` and `return` highlighting, completion help, and a function snippet. The editor remains advisory; runtime semantics and custody decisions belong to the engine.

## Previous release: 0.1.3

Updated for Rust 0.1.0-alpha.8. Added `input()` prompt boxes, Run With Arguments, and highlighting/completions/snippets for copy-value arrays, `len`, and `append`. Interaction responses are stored in plaintext run evidence; the editor does not make them secret. The extension remains an editor aid; the CLI owns syntax and custody decisions.

## Previous release: 0.1.1

Updated for Goblin++ Rust 0.1.0-alpha.6. Added highlighting, completion help, and snippets for `if`, `else if`, `else`, `switch`, `case`, and `default`. The editor remains advisory: the Rust CLI decides syntax, branch semantics, and custody outcomes. The extension identity is unchanged, so installing this VSIX updates the existing extension.

## Previous release: 0.1.0

Updated for the Goblin++ Rust engine 0.1.0-alpha.5. Commands now launch `goblin++`, not Python. A new explicit compiled-run command is available. Read-only diagnostics understand the Rust `goblin.check.v1` response and lexical/parser stderr failures without inventing source ranges.

Highlighting, completions, snippets, and bundled guides now cover loops, Boolean comparisons, FITS import, audited output, and optional `GO_PARANOID`/`seal`. The Icon View remains visual-only and now ignores inline Rust blocks. The earlier extension identity, symbols, and icon mapping settings are preserved. `goblinpp.pythonPath` has been replaced by `goblinpp.executablePath`.

The extension is an editor aid. It does not bundle the engine, authorize inline Rust, authenticate authorship, or claim that a passing preview proves scientific correctness.
