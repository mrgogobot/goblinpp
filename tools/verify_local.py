#!/usr/bin/env python3
"""Verify a local package receipt, inventory and isolated macOS installation.

Exercises the shipped engine and examples, not the repository's build binary.
Never installs into the user's normal prefix or publishes anything.
"""
import argparse
import hashlib
import json
import os
import subprocess
import tempfile
import zipfile
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt", type=Path)
    args = parser.parse_args()
    receipt = json.loads(args.receipt.read_text())
    releases = args.receipt.parent
    version = receipt["version"]
    for entry in receipt["artifacts"]:
        artifact = releases / entry["file"]
        assert artifact.stat().st_size == entry["bytes"]
        assert hashlib.sha256(artifact.read_bytes()).hexdigest() == entry["sha256"]
    archive = releases / receipt["artifacts"][0]["file"]
    with tempfile.TemporaryDirectory(prefix="goblinpp-local-smoke-") as temporary:
        target = Path(temporary)
        with zipfile.ZipFile(archive) as zipped:
            assert zipped.testzip() is None
            for item in zipped.infolist():
                path = Path(item.filename)
                assert not path.is_absolute() and ".." not in path.parts
                assert not item.is_dir()
                assert (item.external_attr >> 16) & 0o170000 != 0o120000
            zipped.extractall(target)
            for item in zipped.infolist():
                os.chmod(target / item.filename, (item.external_attr >> 16) & 0o777)
        root = target / f"goblinpp-rust-v{version}-macos-arm64-local"
        inventory = json.loads((root / "BUNDLE_MANIFEST.json").read_text())
        assert inventory["version"] == version
        actual = {str(path.relative_to(root)) for path in root.rglob("*") if path.is_file()}
        expected = {entry["path"] for entry in inventory["files"]} | {"BUNDLE_MANIFEST.json"}
        assert actual == expected
        assert (root / "third-party/licenses/pcg/LICENSE-APACHE").is_file()
        assert (root / "third-party/licenses/pcg/NOTICE.md").read_bytes() == (root / "tools/license_sources/pcg/NOTICE.md").read_bytes()
        for entry in inventory["files"]:
            payload = (root / entry["path"]).read_bytes()
            assert len(payload) == entry["bytes"]
            assert hashlib.sha256(payload).hexdigest() == entry["sha256"], entry["path"]
        for entry in receipt["artifacts"][1:]:
            payload = (root / "editors" / entry["file"]).read_bytes()
            assert hashlib.sha256(payload).hexdigest() == entry["sha256"]
        prefix = target / "isolated-install"
        subprocess.run([str(root / "install.sh"), "--prefix", str(prefix)], check=True)
        binary = prefix / "bin/goblin++"
        assert subprocess.check_output([str(binary), "--version"], text=True).strip() == f"goblin++ {version}"
        for example in ["energy", "csv_modules", "output_demo", "protected_values", "numeric_text", "numeric_comparison", "fits_subset", "statistics_distribution", "compound_units", "seeded_random", "inference", "streaming_catalogue", "batch_catalogue"]:
            rng_evidence = None
            parity_evidence = None
            for extra in [[], ["--compile"]]:
                run = subprocess.run([str(binary), f"examples/{example}.gbl", *extra],
                                     cwd=root, text=True, capture_output=True, check=True)
                assert "RUN_STATUS=PASS" in run.stdout, run.stdout + run.stderr
                directory = next(line.removeprefix("RUN_DIR=") for line in run.stdout.splitlines() if line.startswith("RUN_DIR="))
                report = json.loads((root / directory / "receipt.json").read_text())
                exact_evidence = {key: report[key] for key in
                                  ("rng_policy", "rng", "inference_policy", "inference",
                                   "resource_policy", "resources", "batch_policy", "batches", "sealed_artifacts")}
                if parity_evidence is not None:
                    assert exact_evidence == parity_evidence, example
                parity_evidence = exact_evidence
                if example == "energy":
                    assert "CANONICAL_SOURCE_SHA256=6a1d72b91490bfc247bdf22322f7eb2ec59d5a88344d6c13a469e92883185a2d" in run.stdout
                if example == "protected_values":
                    assert "captured = 1; current = 2" in run.stdout
                    assert (root / directory / "outputs/snapshots.txt").read_text() == "1\niteration = 0\niteration = 1\niteration = 2\n"
                if example == "numeric_text":
                    assert "saved = 0.12345678901234566; restored exactly = true" in run.stdout
                    assert "rounded label = 0.123" in run.stdout
                    assert "signed zero = -0" in run.stdout
                    assert (root / directory / "outputs/numbers.txt").read_text() == "0.12345678901234566\n"
                if example == "fits_subset":
                    report = json.loads((root / directory / "receipt.json").read_text())
                    exported = [item for item in report["generated_artifacts"] if item["producer"].startswith("fits_export_")]
                    assert len(exported) == 2
                    assert all(item["metadata"]["physical_blinding"] is False for item in exported)
                    assert all(item["metadata"]["input_rows"] == 3 for item in exported)
                    assert "OBJECT,Z,QUALITY\n" in (root / directory / "outputs/subset.csv").read_text()
                if example == "statistics_distribution":
                    assert "median = 4.5 m" in run.stdout
                    assert "population standard deviation = 2.000000 m" in run.stdout
                    report = json.loads((root / directory / "receipt.json").read_text())
                    assert report["statistics_policy"]["quantile"] == "HYNDMAN_FAN_TYPE_7_F64_NO_BOUNDARY_FUZZ_V1"
                if example == "compound_units":
                    assert "area = 3 m^2; whole square = 9 m^2" in run.stdout
                    assert "pressure = 7 kg/(m*s^2)" in run.stdout
                    assert "matches old workaround = true" in run.stdout
                    report = json.loads((root / directory / "receipt.json").read_text())
                    assert report["parser_policy"] == "goblin.compound-units-loop-budget.v2"
                    assert report["math_environment"]["launcher_build"]["rustc"].startswith("rustc 1.92.0 ")
                if example == "seeded_random":
                    report = json.loads((root / directory / "receipt.json").read_text())
                    assert report["rng_policy"]["id"] == "goblin.pcg32-xsh-rr-setseq.v1"
                    assert report["rng"]["usage"] == "USED"
                    assert report["rng"]["streams"][0]["seed"] == "2026"
                    assert report["rng"]["streams"][1]["seed"] == "42"
                    assert report["rng"]["streams"][1]["stream"] == "54"
                    if rng_evidence is not None:
                        assert rng_evidence == report["rng"]
                    rng_evidence = report["rng"]
                if example == "inference":
                    assert "row-major product = [58, 64, 139, 154]" in run.stdout
                    assert report["inference_policy"]["id"] == "goblin.inference.v1"
                    assert report["resources"]["requested_loop_budget"] == 50_000_000
                    assert report["resources"]["effective_loop_budget"] == 50_000_000
                if example == "streaming_catalogue":
                    assert "rows = 3; sum = 33; mean = 11; min = 10; max = 12" in run.stdout
                    assert report["resource_policy"]["streaming"]["id"] == "goblin.delimited-scan.v1"
                if example == "batch_catalogue":
                    assert "rows = 4; total measurement = 11" in run.stdout
                    assert report["batches"]["readers"][0]["rows"] == 4
                    assert report["generated_artifacts"][0]["metadata"]["complete"] is True
                    assert (root / directory / "outputs/processed.csv").read_text() == "source_id,doubled\n9007199254740993,5\n18446744073709551615,8\n42,3\n7,6\n"
                verified = subprocess.run([str(binary), "verify", directory, "--json"],
                                          cwd=root, text=True, capture_output=True, check=True)
                assert json.loads(verified.stdout)["verified"] is True
                print(example, "compiled" if extra else "interpreted", "+ independent verification: PASS")
        print(f"Artifact hashes, ZIP integrity, {len(inventory['files'])} inventory files and isolated installation: PASS")


if __name__ == "__main__":
    main()
