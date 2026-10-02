#!/usr/bin/env python3
"""Package current editor sources using an existing reviewed VSIX scaffold.

No marketplace publication or dependency downloads. New outputs are exclusive.
Build and test the JetBrains plugin separately before running this tool.
"""
from __future__ import annotations

import argparse
import json
import shutil
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--template", required=True, type=Path)
    args = parser.parse_args()
    releases = ROOT / "dist/releases"
    manifest = json.loads((ROOT / "vscode/package.json").read_text())
    version = manifest["version"]
    plugin_version = next(line.split("=", 1)[1] for line in
                          (ROOT / "jetbrains/gradle.properties").read_text().splitlines()
                          if line.startswith("pluginVersion="))
    vsix = releases / f"goblinpp-vscode-{version}.vsix"
    plugin = releases / f"goblinpp-jetbrains-{plugin_version}.zip"
    built_plugin = ROOT / "jetbrains/build/distributions" / plugin.name
    if not built_plugin.is_file():
        raise SystemExit(f"Build the JetBrains installer first: {built_plugin}")
    if vsix.exists() or plugin.exists():
        raise SystemExit("Refusing to overwrite an existing reviewed editor artifact.")
    specs = list((ROOT / "vscode/spec").glob("rust-alpha*-editor.json"))
    if len(specs) != 1:
        raise SystemExit("Exactly one current Rust editor addendum is required.")
    current_spec = specs[0]
    with zipfile.ZipFile(args.template) as template:
        scaffold = template.read("extension.vsixmanifest").decode()
        identity = ET.fromstring(scaffold).find(
            "{http://schemas.microsoft.com/developer/vsx-schema/2011}Metadata/"
            "{http://schemas.microsoft.com/developer/vsx-schema/2011}Identity")
        if identity is None or identity.attrib["Id"] != manifest["name"] or identity.attrib["Publisher"] != manifest["publisher"]:
            raise SystemExit("The VSIX scaffold has a different extension identity.")
        scaffold = scaffold.replace(f'Version="{identity.attrib["Version"]}"', f'Version="{version}"')
        payload = {"extension.vsixmanifest": scaffold.encode(),
                   "[Content_Types].xml": template.read("[Content_Types].xml")}
        for name in template.namelist():
            if not name.startswith("extension/") or name.endswith("/"):
                continue
            relative = name.removeprefix("extension/")
            if relative.startswith("spec/rust-alpha") and relative.endswith("-editor.json"):
                relative = "spec/" + current_spec.name
            source_name = {"LICENSE.txt": "LICENSE", "readme.md": "README.md"}.get(relative, relative)
            source = ROOT / "vscode" / source_name
            if not source.is_file():
                raise SystemExit(f"Missing VSIX source payload: {source}")
            payload["extension/" + relative] = source.read_bytes()
    with zipfile.ZipFile(vsix, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as output:
        for name, content in sorted(payload.items()):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            output.writestr(info, content)
    with zipfile.ZipFile(vsix) as archive:
        if archive.testzip() is not None:
            raise SystemExit("VSIX integrity check failed.")
        assert json.loads(archive.read("extension/package.json"))["version"] == version
        for name, content in payload.items():
            assert archive.read(name) == content
    # Both outputs were checked absent before creation; preserve the build unchanged.
    with built_plugin.open("rb") as source, plugin.open("xb") as destination:
        shutil.copyfileobj(source, destination)
    print(vsix)
    print(plugin)


if __name__ == "__main__":
    main()
