#!/usr/bin/env python3
"""Collect license evidence for the locked macOS arm64 release binary.

The script is deliberately offline. It reads Cargo's locked normal-dependency
tree, copies the license/notice files from the local Cargo registry, and adds
the license bundle shipped with the active Rust standard library.
"""

from __future__ import annotations

import re
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "third-party"
CRATE_LICENSES = OUTPUT / "licenses" / "crates"
RUST_LICENSES = OUTPUT / "licenses" / "rust-standard-library"
NOTICE = ROOT / "THIRD_PARTY_NOTICES.md"
TARGET = "aarch64-apple-darwin"
LICENSE_NAME = re.compile(r"^(license|copying|notice|copyright|unlicense)", re.I)
PACKAGE_LINE = re.compile(r"^([A-Za-z0-9_-]+) v([^ |]+)(?: .*)?\|([^|]*)$")


def run(*args: str) -> str:
    return subprocess.run(
        args,
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout.strip()


def registry_package(name: str, version: str) -> Path:
    registry = Path.home() / ".cargo" / "registry" / "src"
    matches = sorted(registry.glob(f"*/{name}-{version}"))
    if len(matches) != 1:
        raise SystemExit(
            f"expected one local registry directory for {name} {version}; found {matches}"
        )
    return matches[0]


def copy_notices(source: Path, destination: Path) -> list[str]:
    files = sorted(
        path for path in source.iterdir() if path.is_file() and LICENSE_NAME.match(path.name)
    )
    if not files:
        raise SystemExit(f"no top-level license or notice file found in {source}")
    destination.mkdir(parents=True, exist_ok=False)
    for path in files:
        shutil.copy2(path, destination / path.name)
    return [path.name for path in files]


def main() -> None:
    if OUTPUT.exists() or NOTICE.exists():
        raise SystemExit(
            "third-party output already exists; move or remove it before regenerating"
        )

    tree = run(
        "cargo",
        "tree",
        "--locked",
        "--offline",
        "--target",
        TARGET,
        "-e",
        "normal",
        "--prefix",
        "none",
        "--format",
        "{p}|{l}",
    )

    packages: dict[tuple[str, str], str] = {}
    for raw_line in tree.splitlines():
        line = raw_line.replace(" (*)", "").replace(" (proc-macro)", "")
        match = PACKAGE_LINE.match(line)
        if not match:
            continue
        name, version, license_expression = match.groups()
        if name == "goblinpp":
            continue
        packages[(name, version)] = license_expression.strip() or "NOT DECLARED"

    if not packages:
        raise SystemExit("Cargo dependency tree yielded no external packages")

    collected: list[tuple[str, str, str, list[str]]] = []
    for (name, version), license_expression in sorted(packages.items()):
        source = registry_package(name, version)
        copied = copy_notices(source, CRATE_LICENSES / f"{name}-{version}")
        collected.append((name, version, license_expression, copied))

    sysroot = Path(run("rustc", "--print", "sysroot"))
    rust_doc = sysroot / "share" / "doc" / "rust"
    RUST_LICENSES.mkdir(parents=True, exist_ok=False)
    shutil.copy2(rust_doc / "COPYRIGHT-library.html", RUST_LICENSES / "COPYRIGHT-library.html")
    for path in sorted((rust_doc / "licenses").iterdir()):
        if path.is_file():
            shutil.copy2(path, RUST_LICENSES / path.name)
    toolchain = run("rustc", "--version", "--verbose")
    (RUST_LICENSES / "RUST_TOOLCHAIN.txt").write_text(toolchain + "\n", encoding="utf-8")

    lines = [
        "# Third-party notices",
        "",
        "This notice bundle accompanies the Goblin++ 0.1.0-alpha.12 macOS arm64 binary.",
        "It was collected offline from the exact `Cargo.lock` normal-dependency tree",
        f"for `{TARGET}` and from the active Rust standard-library documentation.",
        "It records upstream terms; it does not replace them or constitute legal advice.",
        "Goblin++ itself remains licensed as stated in `LICENSE` and `LICENSE-DOCS.md`.",
        "",
        f"External Rust packages in this binary dependency tree: **{len(collected)}**.",
        "",
        "| Package | Declared license expression | Preserved files |",
        "| --- | --- | --- |",
    ]
    for name, version, license_expression, copied in collected:
        preserved = ", ".join(
            f"[`{filename}`](third-party/licenses/crates/{name}-{version}/{filename})"
            for filename in copied
        )
        lines.append(
            f"| `{name}` `{version}` | `{license_expression}` | {preserved} |"
        )
    lines.extend(
        [
            "",
            "## Rust standard library",
            "",
            "The release binary is built with Rust and contains standard-library code. The",
            "toolchain identity, Rust library copyright notices, and the license texts shipped",
            "with that toolchain are preserved under",
            "[`third-party/licenses/rust-standard-library/`](third-party/licenses/rust-standard-library/).",
            "",
            "## Reproduction",
            "",
            "Run this collector from a prepared checkout with the locked dependencies already",
            "present in the local Cargo cache:",
            "",
            "```console",
            "python3 tools/collect_third_party_notices.py",
            "```",
            "",
            "Review and regenerate the bundle whenever `Cargo.lock`, the release target, Rust",
            "toolchain, enabled features, or packaging process changes.",
        ]
    )
    NOTICE.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"collected {len(collected)} package notice sets")
    print(NOTICE)
    print(OUTPUT)


if __name__ == "__main__":
    main()
