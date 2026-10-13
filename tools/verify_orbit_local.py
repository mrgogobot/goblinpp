#!/usr/bin/env python3
"""Smoke-test both installed Orbit entry points using disposable local fixtures only."""
import argparse
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--goblinpp", type=Path, required=True)
    parser.add_argument("--orbit", type=Path, required=True)
    args = parser.parse_args()

    def run(binary, *options, expected=0):
        prefix = [str(binary)] + (["orbit"] if binary == args.goblinpp else [])
        result = subprocess.run(prefix + [str(o) for o in options] + ["--json"],
                                text=True, capture_output=True)
        assert result.returncode == expected, result.stdout + result.stderr
        return json.loads(result.stdout)

    with tempfile.TemporaryDirectory(prefix="goblin-orbit-smoke-") as temporary:
        base = Path(temporary).resolve()
        root = base / "selected"
        root.mkdir()
        (root / "sample.txt").write_bytes(b"abc")
        captured = run(args.orbit, "baseline", root, "--output", base / "baseline",
                       "--reason", "Synthetic smoke test", "--trust-declaration",
                       "Fixture authored here, not a system security assertion",
                       "--authorization", "Own disposable directory")
        digest = captured["baseline_sha256"]
        assert digest == hashlib.sha256((base / "baseline/baseline.json").read_bytes()).hexdigest()
        run(args.goblinpp, "verify-baseline", base / "baseline", "--expected-baseline-sha256", digest)
        for i, binary in enumerate([args.goblinpp, args.orbit]):
            report = base / f"check-{i}"
            result = run(binary, "check", root, "--baseline", base / "baseline/baseline.json",
                         "--expected-baseline-sha256", digest, "--output", report,
                         "--author", "Synthetic analyst", "--authorization", "Own disposable directory")
            assert result["check"]["status"] == "MATCH"
            run(binary, "verify", report, "--expected-baseline-sha256", digest,
                "--expected-receipt-sha256", result["receipt_core_sha256"])
            assert (root / "sample.txt").read_bytes() == b"abc"
        (root / "sample.txt").write_bytes(b"legitimate fixture change")
        changed = run(args.orbit, "check", root, "--baseline", base / "baseline/baseline.json",
                      "--expected-baseline-sha256", digest, "--output", base / "changed",
                      "--author", "Synthetic analyst", "--authorization", "Own lab", expected=1)
        assert changed["check"]["results"][0]["classification"] == "CONTENT_CHANGED"
        incomplete = run(args.goblinpp, "check", root, "--baseline", base / "baseline/baseline.json",
                         "--expected-baseline-sha256", digest, "--output", base / "budget",
                         "--author", "Synthetic analyst", "--authorization", "Own lab",
                         "--max-file-bytes", "2", expected=2)
        assert incomplete["check"]["status"] == "INCOMPLETE"
        assert list(root.iterdir()) == [root / "sample.txt"]
        (base / "check-0/report/evidence/0002.bin").write_bytes(b"fudged copy")
        command = [str(args.orbit), "verify", str(base / "check-0"), "--expected-baseline-sha256", digest]
        assert subprocess.run(command, capture_output=True).returncode == 2
    print("Orbit installed-binary checks passed: both entry points, hashes, mismatch, budget, tamper refusal.")


if __name__ == "__main__":
    main()
