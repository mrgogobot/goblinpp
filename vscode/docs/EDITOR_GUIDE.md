# Goblin++ VS Code editor guide

This guide targets the Rust engine 0.1.0-alpha.12 and extension 0.1.7.

## 1. Open a trusted folder

Open your `.gbl` project folder in VS Code and trust it only after you trust the files and executable. The extension does not start Goblin++ processes in an untrusted workspace. A `.gbl` file receives Goblin++ highlighting and the Goblin icon automatically.

## 2. Select the executable

If the Goblin++ command already works in a terminal, the extension may find it automatically. On macOS arm64 it first checks the trusted workspace's `dist/macos-arm64/goblin++`, then `~/.local/bin/goblin++`, then `PATH`. Set `goblinpp.executablePath` to an absolute path if discovery fails or if you want a specific version. The extension does not use Python or `.venv`.

## 3. Edit

Completions and colors cover assignments, range and direct-array `for`, `while`, `break`/`continue`, `and`/`or`/`not`, `if`/`else if`/`else`, `switch`/`case`/`default`, `g_func`/`return`, `g_strings` calls, Booleans, comparisons, arrays, FITS functions, text/table/plot outputs, and optional `GO_PARANOID` and `seal`. Bracket pairs auto-close. Snippets include `everyday`, `paranoid`, `for`, `foreach`, `while`, `if`, `switch`, `g_func`, `parse_number`, `parse_integer`, `str_split`, `array`, `append`, `input`, `argv`, `fitsmean`, `fitsselect`, and `writecsv`.

Suggestions are aids, not proof that a scientific formula is correct. FITS metadata, column names, and units still need human interpretation. The Rust parser and evaluator are authoritative.

Use **Goblin++: Insert Scientific Symbol** for supported aliases and notation. `hbar`/`ħ` are registered constant spellings; `ω` is merely a valid identifier glyph unless you assign it. **Goblin++: Toggle Icon View** shows visual decorations without changing source bytes. **Set Icon for Identifier** creates an explicit *visual-only* mapping. There is no `[iconmode]` language syntax. Icon View skips strings, comments, and inline Rust blocks.

## 4. Check, run, and verify

The Command Palette offers:

- **Goblin++: Check Current File** — read-only preview, not evidence;
- **Goblin++: Run Current File** — interpreter run with receipt;
- **Goblin++: Run Current File With Arguments** — enter a JSON array of text arguments passed after `--`;
- **Goblin++: Compile and Run Current File** — native compiled run where supported;
- **Goblin++: Verify Run Directory** — rehash preserved evidence;
- **Goblin++: Show Current File Status** and **Show Current File Lineage**;
- **Goblin++: Audit Workspace Ledger** and **Check Workspace**;
- **Goblin++: Freeze Current File** and **Create Child Revision**.

The editor displays command output in the **Goblin++** output channel. Run saves the file first. Read-only Status and Lineage ask you to save a dirty buffer instead of inspecting stale on-disk text. Check-on-save is disabled until you explicitly enable **Goblin++: Toggle Check on Save**.

When a run calls `input()`, the extension shows an input box and forwards the response. Answers and arguments are preserved in plaintext interaction evidence; never use this for secrets. `check` does not ask for input. Array slices and `append` return independent copies, not shared Go-style views.

The status bar distinguishes passing commands, preview failures, machinery failures, protocol violations, and failed run verification. Treat it as a summary; the CLI receipt and output are the evidence.

Compiled FITS and output calls are currently refused. An inline Rust block needs explicit SHA-256 authorization at the CLI and runs with the user's OS authority. The compiled-run button does not authorize blocks automatically.

## 5. Freeze only after review

Freeze commits exact source bytes. The extension asks for modal confirmation. A later edit—even a notation-only edit—must go through an explicit child revision with a nonempty reason. The CLI independently enforces the rule. Do not delete receipts or ledger files to clear a refusal; use Status, Lineage, Audit Ledger, and Verify to understand it.

## Troubleshooting

If Goblin++ cannot start, set `goblinpp.executablePath` to the absolute Rust binary and run `goblin++ --version` in a terminal. If highlighting is missing, confirm the file ends in `.gbl` or select **Goblin++** from VS Code's language picker. If a preview passes but a run refuses, the preview intentionally did not check freeze or custody state; inspect the complete run output.
