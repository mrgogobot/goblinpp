# Goblin++ for Visual Studio Code 0.1.3

Editor support for the Goblin++ Rust engine **0.1.0-alpha.8**. This extension keeps the existing `goblinpp-project.goblinpp` identity, so it updates the earlier VS Code extension rather than creating a second language mode.

It provides `.gbl` recognition, file icons, syntax highlighting, completions, hover help, snippets, document symbols, an optional non-semantic Icon View, and explicit commands for run, compiled run, check, verify, freeze, revision, status, lineage, ledger audit, and doctor. The vocabulary covers loops, branches, arrays, interaction, Booleans, FITS readers, generated-output functions, and optional `GO_PARANOID`/`seal` directives. The bundled 0.0.7 lexicon is retained only for constant aliases and units; the Rust addendum is an editor aid, not a new normative language specification.

## Install

This source repository contains the extension code, not a packaged VSIX. The
installation instructions below apply to a separately built and reviewed VSIX
release.

In VS Code, open **Extensions → ⋯ → Install from VSIX…** and select `goblinpp-vscode-0.1.3.vsix`. Reload VS Code if prompted. Install the Goblin++ Rust engine separately; the extension does not bundle or install it.

The extension looks for the alpha.8 executable in this order:

1. `goblinpp.executablePath`, if you set it;
2. a macOS arm64 `dist/macos-arm64/goblin++` in the trusted workspace;
3. `~/.local/bin/goblin++` on macOS/Linux;
4. `goblin++` on `PATH`.

If VS Code cannot find your executable, set **Goblin++: Executable Path** to its absolute path. The old `goblinpp.pythonPath` setting is no longer used. The Rust engine is currently shipped with a macOS arm64 binary; other platforms need a source build.

## Everyday or paranoid

An everyday `.gbl` program needs neither `GO_PARANOID` nor `seal`:

```goblin
samples = 12
accepted = 9
fraction = accepted / samples
print("accepted fraction = {fraction:.3f}")
write_text("answer.txt", "fraction = {fraction:.3f}")
```

The generated file is still confined to `RUN_DIR/outputs` and hashed. `seal value` optionally preserves a named value as a separate typed artifact. `GO_PARANOID` opts into postflight source evidence and a receipt-verification gate before ledger registration. It is not an OS sandbox or continuous source monitor.

The `if` and `switch` snippets produce real Goblin++ blocks. Conditions must be Boolean; switch uses the first exact matching case with no fallthrough. For full examples, see the engine's `examples/branching.gbl` and `docs/CONTROL_FLOW.md`.

Run prompts from `input()` appear in a VS Code input box; responses are preserved as plaintext run evidence, so do not enter secrets. **Run Current File With Arguments** asks for a JSON array of arguments. Arrays have completions and snippets for `len`, `append`, and independent slices; see the engine's `docs/INTERACTION_AND_ARRAYS.md`.

## Trust boundaries

The extension launches `goblin++` with an argument array and no shell. Process commands are disabled in untrusted workspaces. Run and compiled run save the active file first. Freeze and revision require explicit confirmation; revision also requires a new path and reason. Check-on-save is off by default and is a preview, never run evidence. The Rust CLI—not syntax colors, suggestions, or this extension—decides validity and custody status.

Compiled runs do not yet support FITS or output calls. Inline Rust is arbitrary native code and requires exact-hash authorization through the CLI; the VS Code compiled-run button does not bypass that approval.

See [Editor Guide](docs/EDITOR_GUIDE.md), [Authoring Tutorial](docs/AUTHORING_TUTORIAL.md), and [Read-only Check](docs/CHECK.md).
