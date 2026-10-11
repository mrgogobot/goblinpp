#!/usr/bin/env python3
"""Verify real alpha.30 evidence and freeze migration with the new local engine.

Runs only in a disposable extracted bundle. Never installs, publishes or edits
the original archive. This supplements synthetic historical Rust fixtures.
"""
import argparse
import json
import os
import subprocess
import tempfile
import zipfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("old_bundle", type=Path)
    parser.add_argument("new_binary", type=Path)
    args = parser.parse_args()
    new = args.new_binary.resolve()
    with tempfile.TemporaryDirectory(prefix="goblinpp-plot-compat-") as temporary:
        target = Path(temporary)
        with zipfile.ZipFile(args.old_bundle) as archive:
            assert archive.testzip() is None
            for item in archive.infolist():
                path = Path(item.filename)
                assert not path.is_absolute() and ".." not in path.parts
                assert (item.external_attr >> 16) & 0o170000 != 0o120000
            archive.extractall(target)
            for item in archive.infolist():
                os.chmod(target / item.filename, (item.external_attr >> 16) & 0o777)
        root, = target.iterdir()
        old = root / "dist/macos-arm64/goblin++"
        assert subprocess.check_output([old, "--version"], text=True).strip() == "goblin++ 0.1.0-alpha.30"

        def invoke(binary, *arguments, status=0):
            process = subprocess.run([str(binary), *map(str, arguments)], cwd=root,
                                     capture_output=True, text=True)
            assert process.returncode == status, process.stdout + process.stderr
            return process.stdout

        def run(binary, source, status=0):
            output = invoke(binary, source, status=status)
            directory = next(line.removeprefix("RUN_DIR=") for line in output.splitlines()
                             if line.startswith("RUN_DIR="))
            path = root / directory
            receipt = json.loads((path / "receipt.json").read_text())
            assert json.loads(invoke(new, "verify", path, "--json"))["verified"]
            return path, receipt

        for example in ("array_plots", "output_demo", "energy"):
            source = f"examples/{example}.gbl"
            original, receipt = run(old, source)
            assert receipt["status"] == "PASS"
            assert receipt["plot_policy"] == "goblin.array-plots.v1"
            if example == "array_plots":
                checks = json.loads(invoke(new, "verify", original, "--json"))["checks"]
                assert sum(check["check"].startswith("PLOT_RECONSTRUCTION:") and check["pass"]
                           for check in checks) == 8
                assert not any(check["check"].startswith("PLOT_PROVENANCE:") for check in checks)
                invoke(old, "freeze", source)
                assert "FROZEN_VERIFIED" in invoke(new, "status", source)
                refused, failed = run(new, source, status=1)
                assert failed["status"] == "PROTOCOL_VIOLATION"
                assert failed["generated_artifacts"] == []
                assert "current plot provenance policy" in (refused / "stderr.log").read_text()
                child = "examples/provenance_revision.gbl"
                invoke(new, "revise", source, child, "--reason", "Explicit plot provenance policy migration test")
                invoke(new, "freeze", child)
                _, revised = run(new, child)
                assert revised["status"] == "PASS" and revised["plot_policy"] == "goblin.array-plots.v2"
                print("Real alpha.30 plot receipt/reconstruction + explicit freeze revision: PASS")
            elif example == "output_demo":
                _, current = run(new, source)
                assert current["generated_artifacts"] == receipt["generated_artifacts"]
                assert len(current["generated_artifacts"]) == 7
                print("Seven legacy FITS/output artifacts retain exact hashes and metadata: PASS")
            else:
                invoke(old, "freeze", source)
                _, current = run(new, source)
                assert current["status"] == "PASS"
                assert current["sealed_artifacts"] == receipt["sealed_artifacts"]
                print("Real alpha.30 non-plot freeze still runs and verifies: PASS")


if __name__ == "__main__":
    main()
