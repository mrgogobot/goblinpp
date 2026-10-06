#!/usr/bin/env python3
"""Compare the CI-measured RNG fixture exactly; no tolerance or rounded seals."""
import argparse
import json
from pathlib import Path

SCHEMA = "goblin.rng-platform-report.v1"
POLICY = "goblin.pcg32-xsh-rr-setseq.v1"


def compare_reports(reports, require_platforms=True):
    if not reports:
        raise ValueError("No measured RNG reports")
    reference = reports[0]
    for report in reports:
        if report.get("schema") != SCHEMA:
            raise ValueError("Unknown RNG report schema")
        for field in ("goblin_version", "source_sha256", "rng_policy", "interpreter", "native", "os", "arch"):
            if not report.get(field):
                raise ValueError(f"Missing report field: {field}")
        if report["rng_policy"].get("id") != POLICY:
            raise ValueError("Unknown RNG policy")
        engine = report["interpreter"]
        if not engine.get("sealed_artifacts") or engine.get("rng", {}).get("usage") != "USED":
            raise ValueError("Missing used RNG fixture/seals")
        if engine["rng"].get("policy") != POLICY or not engine["rng"].get("streams"):
            raise ValueError("Missing RNG trace/policy")
        if report["native"] != engine:
            raise ValueError("Native/interpreter RNG mismatch")
        for field in ("goblin_version", "source_sha256", "rng_policy", "interpreter"):
            if report[field] != reference[field]:
                raise ValueError(f"Cross-platform RNG mismatch: {field}")
    platforms = {(r["os"], r["arch"]) for r in reports}
    covered = ("macos", "aarch64") in platforms and ("linux", "x86_64") in platforms
    if require_platforms and not covered:
        raise ValueError("Need measured macOS arm64 and Linux x86_64 RNG reports")
    return {"status": "PASS", "schema": SCHEMA, "policy": POLICY,
            "classification": "EXACT_RNG_FIXTURE_MATCH",
            "cross_platform_gate": covered, "reports": len(reports),
            "scope": "Fixed tested PCG32/mapping fixture; not all scientific mathematics"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    reports = [json.loads(p.read_text()) for p in sorted(args.directory.rglob("rng-report.json"))]
    print(json.dumps(compare_reports(reports), indent=2))


if __name__ == "__main__":
    main()
