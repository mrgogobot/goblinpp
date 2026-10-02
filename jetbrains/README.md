# Goblin++ for JetBrains IDEs

Plugin 0.3.6 accompanies engine alpha.21. Completion help documents lossless
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
[`../vscode/spec/rust-alpha19-editor.json`](../vscode/spec/rust-alpha19-editor.json)
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
