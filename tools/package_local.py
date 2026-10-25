#!/usr/bin/env python3
"""Package the reviewed macOS arm64 engine and both editor installers.

Build/test the engine and editor plugins first. This tool copies source into a
fresh staging directory, regenerates locked third-party notices offline, and
records a SHA-256 inventory. It never includes live runs or custody files.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE_DIRS = ("src", "tests", "examples", "docs", "assets", "tools", "vscode", "jetbrains", "community-review", "studies", ".github")
SOURCE_FILES = (
    "Cargo.toml", "Cargo.lock", "build.rs", "CITATION.cff", "README.md", "RELEASE_NOTES.md",
    "RELEASE_CHECKLIST.md", "LICENSE", "LICENSE-DOCS.md", "BRANDING.md", "install.sh", ".gitignore",
    ".zenodo.json", "CONTRIBUTING.md", "CODE_OF_CONDUCT.md", "SECURITY.md",
)
EXCLUDED_DIRS = {".git", ".gradle", ".intellijPlatform", ".idea", "build", "target", "dist", "node_modules", "runs", ".goblin", "__pycache__", ".vscode-test", "vsix"}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def copy_tree(source: Path, destination: Path) -> None:
    def ignored(_directory: str, names: list[str]) -> list[str]:
        return [name for name in names if name in EXCLUDED_DIRS or name == ".DS_Store"
                or name.endswith((".zip", ".vsix", ".freeze.json", ".lineage.json", ".goblin.rs", ".pyc"))]
    shutil.copytree(source, destination, ignore=ignored)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=ROOT / "dist/releases",
                        help="Fresh destination for a corrected bundle; never replace existing packages.")
    args = parser.parse_args()
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    extension_version = json.loads((ROOT / "vscode/package.json").read_text())["version"]
    plugin_version = next(line.split("=", 1)[1] for line in
                          (ROOT / "jetbrains/gradle.properties").read_text().splitlines()
                          if line.startswith("pluginVersion="))
    editor_inputs = ROOT / "dist/releases"
    releases = args.output_dir.resolve()
    binary = ROOT / "target/release/goblinpp"
    orbit_binary = ROOT / "target/release/goblin-orbit"
    vsix = editor_inputs / f"goblinpp-vscode-{extension_version}.vsix"
    plugin = editor_inputs / f"goblinpp-jetbrains-{plugin_version}.zip"
    for artifact in (binary, orbit_binary, vsix, plugin):
        if not artifact.is_file():
            raise SystemExit(f"Build the required artifact first: {artifact}")
    observed = subprocess.check_output([str(binary), "--version"], text=True).strip()
    if observed != f"goblin++ {version}":
        raise SystemExit(f"Release binary has the wrong version: {observed}")
    architecture = subprocess.check_output(["file", str(binary)], text=True)
    if "Mach-O" not in architecture or "arm64" not in architecture:
        raise SystemExit("This local bundle requires the macOS arm64 binary.")
    if subprocess.check_output([str(orbit_binary), "--version"], text=True).strip() != f"goblin-orbit {version}":
        raise SystemExit("Standalone Orbit has the wrong version.")
    architecture = subprocess.check_output(["file", str(orbit_binary)], text=True)
    if "Mach-O" not in architecture or "arm64" not in architecture:
        raise SystemExit("This local bundle requires macOS arm64 Orbit.")

    bundle_name = f"goblinpp-rust-v{version}-macos-arm64-local"
    archive = releases / f"{bundle_name}.zip"
    receipt_path = releases / f"Goblin++_v{version}_PACKAGE_RECEIPT.json"
    sums_path = releases / f"Goblin++_v{version}_SHA256SUMS.txt"
    for output in (archive, receipt_path, sums_path):
        if output.exists():
            raise SystemExit(f"Refusing to replace an existing package: {output}")
    releases.mkdir(parents=True, exist_ok=True)
    for installer in (vsix, plugin):
        output = releases / installer.name
        if output == installer:
            continue
        if output.exists():
            if not output.is_file() or sha256(output) != sha256(installer):
                raise SystemExit(f"Refusing to replace a different editor installer: {output}")
        else:
            with output.open("xb") as stream:
                with installer.open("rb") as source:
                    shutil.copyfileobj(source, stream)

    with tempfile.TemporaryDirectory(prefix="goblinpp-package-") as temporary:
        staged = Path(temporary) / bundle_name
        staged.mkdir()
        for name in SOURCE_FILES:
            if (ROOT / name).is_file():
                shutil.copy2(ROOT / name, staged / name)
        for name in SOURCE_DIRS:
            copy_tree(ROOT / name, staged / name)
        destination = staged / "dist/macos-arm64/goblin++"
        destination.parent.mkdir(parents=True)
        shutil.copy2(binary, destination)
        destination.chmod(0o755)
        orbit_destination = destination.with_name("goblin-orbit")
        shutil.copy2(orbit_binary, orbit_destination)
        orbit_destination.chmod(0o755)
        editors = staged / "editors"
        editors.mkdir()
        for installer in (vsix, plugin):
            shutil.copy2(installer, editors / installer.name)

        subprocess.run([sys.executable, str(staged / "tools/collect_third_party_notices.py")],
                       cwd=staged, check=True)
        inventory = [{"path": str(path.relative_to(staged)), "bytes": path.stat().st_size,
                      "sha256": sha256(path)} for path in sorted(staged.rglob("*")) if path.is_file()]
        manifest = {"schema": "goblin.local-bundle.v1", "version": version,
                    "target": "aarch64-apple-darwin", "files": inventory}
        (staged / "BUNDLE_MANIFEST.json").write_text(json.dumps(manifest, indent=2) + "\n")
        with zipfile.ZipFile(archive, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as output:
            for path in sorted(staged.rglob("*")):
                if path.is_file():
                    info = zipfile.ZipInfo(str(path.relative_to(staged.parent)), (1980, 1, 1, 0, 0, 0))
                    info.compress_type = zipfile.ZIP_DEFLATED
                    info.create_system = 3
                    info.external_attr = (path.stat().st_mode & 0xFFFF) << 16
                    output.writestr(info, path.read_bytes())

    artifacts = [{"file": path.name, "bytes": path.stat().st_size, "sha256": sha256(path)}
                 for path in (archive, releases / vsix.name, releases / plugin.name)]
    receipt = {"schema": "goblin.package-receipt.v1", "version": version,
               "target": "aarch64-apple-darwin", "artifacts": artifacts}
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    sums_path.write_text("".join(f"{item['sha256']}  {item['file']}\n" for item in artifacts))
    print(json.dumps(receipt, indent=2))
    print(receipt_path)
    print(sums_path)


if __name__ == "__main__":
    main()
