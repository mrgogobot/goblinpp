#!/usr/bin/env python3
"""Check actual alpha.31 evidence/freezes with alpha.32 in an isolated directory.

The archived package hash and its engine inventory hash are checked first.
No release files, installed executables or user custody histories are modified.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import zipfile

from check_batch_memory import digest


def invoke(engine, args, root, success=True):
    result = subprocess.run([str(engine), *args], cwd=root, capture_output=True, text=True)
    if success:
        assert result.returncode == 0, result.stdout + result.stderr
    else:
        assert result.returncode != 0, result.stdout + result.stderr
    return result


def run(engine, source, root, extra=(), success=True):
    result = invoke(engine, ["run", str(source), *extra], root, success)
    directory = next(line.removeprefix("RUN_DIR=") for line in result.stdout.splitlines()
                     if line.startswith("RUN_DIR="))
    path = root / directory
    return path, json.loads((path / "receipt.json").read_text()), result


def verify(engine, path, root):
    result = invoke(engine, ["verify", str(path), "--json"], root)
    assert json.loads(result.stdout)["verified"] is True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("engine", type=Path)
    parser.add_argument("old_receipt", type=Path)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    if args.report.exists():
        raise SystemExit("Refusing to overwrite an existing compatibility report")
    engine = args.engine.resolve()
    package = json.loads(args.old_receipt.read_text())
    assert package["version"] == "0.1.0-alpha.31"
    archive = args.old_receipt.parent / package["artifacts"][0]["file"]
    assert digest(archive) == package["artifacts"][0]["sha256"]
    records = []
    with tempfile.TemporaryDirectory(prefix="goblin-fits-history-") as temporary:
        root = Path(temporary)
        with zipfile.ZipFile(archive) as zipped:
            assert zipped.testzip() is None
            for item in zipped.infolist():
                path = Path(item.filename)
                assert not path.is_absolute() and ".." not in path.parts
                assert (item.external_attr >> 16) & 0o170000 != 0o120000
            zipped.extractall(root)
            for item in zipped.infolist():
                os.chmod(root / item.filename, (item.external_attr >> 16) & 0o777)
        bundle = root / "goblinpp-rust-v0.1.0-alpha.31-macos-arm64-local"
        old = bundle / "dist/macos-arm64/goblin++"
        inventory = json.loads((bundle / "BUNDLE_MANIFEST.json").read_text())
        expected = next(i["sha256"] for i in inventory["files"]
                        if i["path"] == "dist/macos-arm64/goblin++")
        assert digest(old) == expected
        assert invoke(old, ["--version"], root).stdout.strip() == "goblin++ 0.1.0-alpha.31"
        for example in ["energy", "batch_catalogue", "array_plots"]:
            for extra in [[], ["--compile"]]:
                path, receipt, _ = run(old, bundle / f"examples/{example}.gbl", bundle, extra)
                assert "fits_batch_policy" not in receipt
                verify(engine, path, bundle)
                records.append({"case": f"alpha31-{example}-{'native' if extra else 'interpreter'}",
                                "receipt_core_sha256": receipt["receipt_core_sha256"],
                                "verification": "PASS"})
        source = bundle / "examples/energy.gbl"
        invoke(old, ["freeze", str(source)], bundle)
        for extra in [[], ["--compile"]]:
            path, receipt, _ = run(engine, source, bundle, extra)
            assert receipt["status"] == "PASS"
            verify(engine, path, bundle)
        records.append({"case": "actual-alpha31-energy-freeze-runs-in-both-new-engines", "gate": "PASS"})
        # An old parser could freeze an unknown call; adopting its new semantics
        # now requires the explicit revision route, not implicit policy invention.
        source = bundle / "examples/new_fits.gbl"
        current_root = Path(__file__).resolve().parent.parent
        source.write_bytes((current_root / "examples/fits_batches.gbl").read_bytes())
        invoke(old, ["freeze", str(source)], bundle)
        path, receipt, result = run(engine, source, bundle, success=False)
        assert receipt["status"] == "PROTOCOL_VIOLATION"
        assert "did not pin FITS batches" in result.stderr
        verify(engine, path, bundle)
        child = source.with_name("new_fits_R1.gbl")
        invoke(engine, ["revise", str(source), str(child), "--reason", "explicit FITS batch policy migration test"], bundle)
        invoke(engine, ["freeze", str(child)], bundle)
        for extra in [[], ["--compile"]]:
            path, receipt, _ = run(engine, child, bundle, extra)
            assert receipt["status"] == "PASS"
            verify(engine, path, bundle)
        records.append({"case": "actual-alpha31-unpinned-FITS-refuses-then-explicit-revision-passes-both-engines", "gate": "PASS"})
    report = {"schema": "goblin.fits-batch-history-check.v1", "gate": "PASS",
              "new_engine": invoke(engine, ["--version"], Path.cwd()).stdout.strip(),
              "new_engine_sha256": digest(engine), "old_engine_sha256": expected,
              "old_archive_sha256": package["artifacts"][0]["sha256"], "cases": records,
              "scope": "Actual local alpha.31 fixtures and freezes, not every historical release"}
    with args.report.open("x") as stream:
        json.dump(report, stream, indent=2)
        stream.write("\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
