#!/usr/bin/env python3
"""Verify actual alpha.32 runs, scalar results and explicit freeze migration.

All work occurs inside a disposable copy of the hash-verified old local bundle.
No installed executable, original package or user custody history is modified.
"""
import argparse
import json
import os
from pathlib import Path
import tempfile
import zipfile

from check_batch_memory import digest
from check_fits_batch_compatibility import invoke, run, verify


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("engine", type=Path)
    parser.add_argument("old_receipt", type=Path)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    if args.report.exists():
        raise SystemExit("Refusing to replace an existing compatibility report")
    engine = args.engine.resolve()
    package = json.loads(args.old_receipt.read_text())
    assert package["version"] == "0.1.0-alpha.32"
    archive = args.old_receipt.parent / package["artifacts"][0]["file"]
    assert digest(archive) == package["artifacts"][0]["sha256"]
    records = []
    with tempfile.TemporaryDirectory(prefix="goblin-array-history-") as temporary:
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
        bundle = root / "goblinpp-rust-v0.1.0-alpha.32-macos-arm64-local"
        old = bundle / "dist/macos-arm64/goblin++"
        inventory = json.loads((bundle / "BUNDLE_MANIFEST.json").read_text())
        expected = next(i["sha256"] for i in inventory["files"]
                        if i["path"] == "dist/macos-arm64/goblin++")
        assert digest(old) == expected
        assert invoke(old, ["--version"], bundle).stdout.strip() == "goblin++ 0.1.0-alpha.32"
        for example in ["energy", "arrays", "fits_batches", "array_plots"]:
            for extra in [[], ["--compile"]]:
                path, receipt, _ = run(old, bundle / f"examples/{example}.gbl", bundle, extra)
                assert receipt["status"] == "PASS"
                assert "array_math_policy" not in receipt
                verify(engine, path, bundle)
                record = {"case": f"actual-alpha32-{example}-{'native' if extra else 'interpreter'}",
                          "historical_receipt_core_sha256": receipt["receipt_core_sha256"],
                          "verification": "PASS"}
                if example in ["energy", "arrays"]:
                    new_path, new_receipt, _ = run(engine, bundle / f"examples/{example}.gbl", bundle, extra)
                    assert new_receipt["status"] == "PASS"
                    assert (path / "stdout.log").read_bytes() == (new_path / "stdout.log").read_bytes()
                    assert receipt["source"]["sha256"] == new_receipt["source"]["sha256"]
                    verify(engine, new_path, bundle)
                    record["unchanged_existing_output"] = "PASS"
                records.append(record)
        source = bundle / "examples/energy.gbl"
        invoke(old, ["freeze", str(source)], bundle)
        for extra in [[], ["--compile"]]:
            path, receipt, _ = run(engine, source, bundle, extra, success=False)
            assert receipt["status"] == "PROTOCOL_VIOLATION"
            assert receipt["protocol_violation"]["classification"] == "ARRAY_MATH_POLICY_NOT_FROZEN"
            verify(engine, path, bundle)
        native = bundle / "blocked-compile"
        denied = invoke(engine, ["compile", str(source), "-o", str(native)], bundle, success=False)
        assert "did not pin" in denied.stderr and not native.exists()
        child = source.with_name("energy_R1.gbl")
        invoke(engine, ["revise", str(source), str(child), "--reason", "explicit array policy migration"], bundle)
        invoke(engine, ["freeze", str(child)], bundle)
        for extra in [[], ["--compile"]]:
            path, receipt, _ = run(engine, child, bundle, extra)
            assert receipt["status"] == "PASS"
            verify(engine, path, bundle)
        records.append({"case": "actual-alpha32-freeze-remains-inspectable-but-requires-explicit-revision-both-engines", "gate": "PASS"})
    report = {"schema": "goblin.array-math-history-check.v1", "gate": "PASS",
              "new_engine": invoke(engine, ["--version"], Path.cwd()).stdout.strip(),
              "new_engine_sha256": digest(engine), "old_engine_sha256": expected,
              "old_archive_sha256": package["artifacts"][0]["sha256"], "cases": records,
              "scope": "Actual alpha.32 runs/scalar results/freeze in a disposable copy; not every historical release or real WB workflow"}
    with args.report.open("x") as stream:
        json.dump(report, stream, indent=2)
        stream.write("\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
