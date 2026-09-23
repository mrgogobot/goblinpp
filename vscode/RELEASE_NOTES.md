# VS Code extension 0.1.7

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
