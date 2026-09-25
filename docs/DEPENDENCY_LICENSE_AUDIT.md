# Dependency-license review: macOS arm64 Alpha.12

Snapshot: `Cargo.lock` at Goblin++ 0.1.0-alpha.13, reviewed on 2026-09-25.
This is an evidence record for the macOS arm64 alpha binary, not legal advice
or a claim about future targets.

## What the source checkout uses

The five direct runtime crates are `chrono`, `clap`, `serde`, `serde_json`, and
`sha2`. The direct test-only crate is `tempfile`. Each declares
`MIT OR Apache-2.0` in its installed package manifest. The VS Code extension
declares no npm package dependencies.

The locked normal dependency tree used to build the macOS arm64 binary contains
40 external packages. This count includes build-time procedural-macro
dependencies and is deliberately conservative: preserving a notice for a
build-time package is safer than silently omitting a potentially applicable
notice.

The installed manifests for packages in this normal tree all offer an
MIT licensing path. Notable expressions are:

| Package | Declared license expression | Review note |
| --- | --- | --- |
| `unicode-ident` 1.0.24 | `(MIT OR Apache-2.0) AND Unicode-3.0` | The Unicode-3.0 terms apply in addition to the chosen MIT/Apache path. Its bundled `LICENSE-UNICODE` requires preservation of its copyright and permission notice when applicable. |
| `memchr` 2.8.3 | `Unlicense OR MIT` | MIT is an available path. |
| `generic-array` 0.14.7, `strsim` 0.11.1, `zmij` 1.0.23 | `MIT` | MIT notice applies where these components are redistributed. |

Other packages in this normal tree declare `MIT OR Apache-2.0` or
`Apache-2.0 OR MIT`. This inventory does not override a crate's own license
files, file-level notices, or the rights of the Rust standard library.

## Preserved release evidence

The repository now preserves:

1. [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md), an index of all 40
   packages in the exact locked macOS arm64 normal dependency tree;
2. the upstream license, copying, notice, copyright, and Unlicense files found
   at the roots of those exact locally cached crate versions under
   `third-party/licenses/crates/`;
3. the active Rust toolchain identity, Rust library copyright document, and
   every license text shipped in the toolchain's Rust documentation under
   `third-party/licenses/rust-standard-library/`; and
4. both project license files inside VS Code extension 0.1.7. The VSIX declares
   no npm dependencies, and its packaged source was compared with the reviewed
   extension source. VSCE's expected README link rewriting was the only
   difference.

`tools/collect_third_party_notices.py` performs the collection offline and
refuses incomplete or ambiguous local registry matches. The generated notice
bundle must accompany the macOS arm64 binary archive.

This closes the identified Alpha.12 macOS arm64 notice-collection gate. It does
not pre-approve a Linux, Windows, embedded, differently featured, or later
release. Repeat the inventory when `Cargo.lock`, the Rust toolchain, target,
features, or packaging process changes.

To reproduce the dependency trees from a prepared local Cargo cache:

```console
cargo tree --locked --offline --target aarch64-apple-darwin -e normal
cargo tree --locked --offline --target x86_64-unknown-linux-gnu -e normal
```

The project source repository does not vendor third-party crate source. The
project's MIT license does not replace the licenses of its dependencies.
