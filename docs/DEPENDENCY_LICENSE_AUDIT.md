# Dependency-license review in progress

Snapshot: `Cargo.lock` at Goblin++ 0.1.0-alpha.8, reviewed on 2026-09-17.
This is a manifest-level inventory for release preparation, **not** a complete
third-party notice bundle or a legal conclusion about a finished binary.

## What the source checkout uses

The five direct runtime crates are `chrono`, `clap`, `serde`, `serde_json`, and
`sha2`. The direct test-only crate is `tempfile`. Each declares
`MIT OR Apache-2.0` in its installed package manifest. The VS Code extension
declares no npm package dependencies.

`Cargo.lock` has 80 package entries including Goblin++ and packages for
multiple targets and development. The locked normal dependency trees contain
40 external packages for `aarch64-apple-darwin` and 38 for
`x86_64-unknown-linux-gnu`; these counts include build-time procedural-macro
dependencies and do not claim that every package is embedded in the finished
executable. The two trees differ by `core-foundation-sys` and `libc`, present
only in the macOS tree in this snapshot.

The installed manifests for packages in these two normal trees all offer an
MIT licensing path. Notable expressions are:

| Package | Declared license expression | Review note |
| --- | --- | --- |
| `unicode-ident` 1.0.24 | `(MIT OR Apache-2.0) AND Unicode-3.0` | The Unicode-3.0 terms apply in addition to the chosen MIT/Apache path. Its bundled `LICENSE-UNICODE` requires preservation of its copyright and permission notice when applicable. |
| `memchr` 2.8.3 | `Unlicense OR MIT` | MIT is an available path. |
| `generic-array` 0.14.7, `strsim` 0.11.1, `zmij` 1.0.23 | `MIT` | MIT notice applies where these components are redistributed. |

Other packages in the two normal trees declare `MIT OR Apache-2.0` or
`Apache-2.0 OR MIT`. This inventory does not override a crate's own license
files, file-level notices, or the rights of the Rust standard library.

## Still required before publishing binaries

1. Inspect the exact macOS/Linux binaries and extension package to identify
   which third-party material is shipped, as distinct from build-only crates.
2. Collect and include the applicable copyright, license, and notice texts for
   the shipped components, including any applicable Unicode notice.
3. Repeat the inventory when `Cargo.lock`, target platforms, features, or the
   packaging process changes.

To reproduce the dependency trees from a prepared local Cargo cache:

```console
cargo tree --locked --offline --target aarch64-apple-darwin -e normal
cargo tree --locked --offline --target x86_64-unknown-linux-gnu -e normal
```

The project source repository does not vendor third-party crate source. The
project's MIT license does not replace the licenses of its dependencies.
