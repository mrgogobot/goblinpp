# Goblin++ for JetBrains IDEs

Plugin 0.3.12 adds alpha.27 completion/help for resampling, standard normals,
flat matrices, covariance/Cholesky, streaming scans and audited loop budgets.
Both editors share the same vocabulary; explicit statistical choices are visible.

It retains completions/quick help for alpha.26's four seeded scientific PCG32
functions. Explicit seeds/streams and half-open bounds; not cryptographic.
See [RANDOMNESS.md](../docs/RANDOMNESS.md).

Plugin 0.3.12 accompanies engine alpha.27. Unit completion and quick documentation
explain compound suffixes, whole-quantity powers and grouped denominators.
See `docs/COMPOUND_UNITS.md` in the engine package. Completion and quick documentation
include stable copy sorting, type-7 quantiles/median, explicit sample/population
standard deviation and unweighted ECDF. See `docs/STATISTICS.md` in the engine
package. Completion retains explicit FITS
predicate builders, CSV/TSV subset export and exact catalogue-ID text access.
See `docs/FITS_SUBSETS.md`; projection is not physical blinding. It retains `is_close` with
four explicit arguments and no defaults, plus `same_bits` including signed zero.
See `docs/NUMERIC_REPRODUCIBILITY.md` in the engine package. Help retains lossless
default numeric text and signed zero, and marks constants
read-only, requires consuming `append` results, and documents immediate
source-template capture. See `docs/PROTECTED_VALUES.md` in the engine package.

![Goblin++ project logo](artwork/goblinpp.png)

This plugin adds Goblin++ language and workflow support to IntelliJ Platform
IDEs. It is intentionally a thin editor integration around the real Rust
engine: the plugin does not reimplement scientific execution or custody rules.

## Initial feature set

- `.gbl` recognition and the Goblin++ file icon
- **New Project | Goblin++** generation in IntelliJ-based IDEs, including
  PyCharm and CLion
- auditable (`GO_PARANOID`) or everyday starter modes
- native **Run** configurations with toolbar, context-menu, shortcut, and
  editor-gutter execution
- interactive console input plus quoted program arguments for `argc`/`argv`
- vocabulary-driven syntax highlighting
- keyword, function, constant, and unit completion
- quick documentation sourced from the current editor vocabulary
- chemistry and electrical `chem_`/`ee_` completions, including Unicode SI units
- dimension-preserving `sum`/`mean` completion and help (requires alpha.19 engine)
- local `import` and explicit CSV/TSV reader completion/help (alpha.19)
- matching `()`, `[]`, and `{}` plus `#` line comments
- **Run**, **Compile and Run**, **Check**, **Freeze**, **Status**, and **Doctor**
  actions under **Tools | Goblin++**
- executable discovery at `PROJECT/target/release/goblin++`, then
  `~/.local/bin/goblin++`, then `PATH`
- an explicit executable override under **Settings | Tools | Goblin++**

The editor vocabulary is generated at build time from
[`../vscode/spec/rust-alpha27-editor.json`](../vscode/spec/rust-alpha27-editor.json)
and [`../vscode/spec/lexicon.v0.json`](../vscode/spec/lexicon.v0.json). This keeps
the JetBrains plugin aligned with the reviewed Goblin++ editor contract instead
of maintaining another hand-copied keyword list.

## Build

The plugin targets IntelliJ Platform build 243 (2024.3) or newer and requires a
native JDK 21 plus Gradle 8.x:

```console
cd jetbrains
./gradlew clean test buildPlugin
```

The installable ZIP is created under `build/distributions/`.

## Install locally

1. Open **Settings | Plugins** in a compatible JetBrains IDE.
2. Use the gear menu and choose **Install Plugin from Disk…**.
3. Select the ZIP from `build/distributions/`.
4. Restart the IDE if requested.
5. Open a `.gbl` file and use **Tools | Goblin++**.

To create a project, choose **File | New Project**, select **Goblin++**, choose
whether the starter should include `GO_PARANOID`, and press **Create**. The
wizard writes `main.gbl`, `README.md`, and a `.gitignore` suitable for Goblin++
run evidence and generated build files.

To run a program, open a `.gbl` file and click the green triangle in the editor
gutter, right-click and choose **Run**, or use the IDE's Run shortcut. Goblin++
output and `input()` prompts appear in the **Run** panel. Persistent program
arguments and the working directory can be edited under **Run | Edit
Configurations**.

Goblin++ itself must be installed separately. The plugin never downloads or
silently replaces the scientific engine.

## Trust boundary

Editor highlighting and completion are authoring aids, not scientific evidence.
Only the Goblin++ engine defines execution, dimensions, custody, freeze, and
verification semantics. `GO_PARANOID` is not a sandbox. Inline Rust remains
arbitrary native code authorized by its exact hash.

## Branding and licensing

Plugin source code and functional editor assets are MIT-licensed. The Goblin++
project logo is separate branding governed by `artwork/BRANDING.md`; including
it in this official package does not grant third parties permission to brand
unrelated projects as Goblin++.
