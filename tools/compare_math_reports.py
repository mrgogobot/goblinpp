#!/usr/bin/env python3
"""Validate preserved engine evidence and compare explicitly toleranced fixtures.

This is a CI fixture gate, not a general scientific-agreement test. It never
executes the archived native binaries and never modifies an evidence directory.
"""
from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import math
import re
import struct
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
POLICY = "goblin.symmetric-max-absolute-relative.v1"


def require(condition: bool, reason: str) -> None:
    if not condition:
        raise ValueError(reason)


def bits(value: float) -> str:
    require(math.isfinite(value), "Nonfinite result in fixture report")
    return struct.pack(">d", value).hex()


def close(a: float, b: float, absolute: float, relative: float) -> bool:
    require(all(math.isfinite(x) for x in (a, b, absolute, relative)),
            "Comparison values and tolerances must be finite")
    require(absolute >= 0 and relative >= 0, "Negative tolerance")
    difference = abs(a - b)
    if difference <= absolute:
        return True
    if relative == 0:
        return False
    scale = max(abs(a), abs(b))
    ratio = difference / scale if math.isfinite(difference) else abs(a / scale - b / scale)
    return ratio <= relative


def safe_child(root: Path, relative: str) -> Path:
    path = Path(relative)
    require(not path.is_absolute() and ".." not in path.parts, "Non-local evidence path")
    candidate = root / path
    require(not candidate.is_symlink() and candidate.resolve().is_relative_to(root.resolve()),
            "Evidence path escapes report directory")
    return candidate


def validate_report(path: Path, fixture: dict, fixture_hash: str, binary: Path) -> dict:
    report = json.loads(path.read_text())
    require(report["schema"] == "goblin.platform-math-report.v1", "Unsupported report schema")
    require(report["fixture_sha256"] == fixture_hash, "Fixture hash mismatch")
    expected_source = "GO_PARANOID\n" + "".join(
        f"{case['name']} = {case['expression']}\nseal {case['name']}\n" for case in fixture["cases"])
    require(report["source_sha256"] == hashlib.sha256(expected_source.encode()).hexdigest(),
            "Executed source is not the shared fixture program")
    policy = report["math_policy"]
    require(policy["comparison_policy"] == POLICY, "Unsupported comparison policy")
    require(policy["cross_platform_bitwise_guarantee"] is False, "Misleading bitwise guarantee")
    require(policy["default_tolerance"] is None, "Implicit default tolerance")
    engines = report["engines"]
    require({e["engine"] for e in engines} == {"rust-interpreter", "rust-native-compiled"}
            and len(engines) == 2, "Both execution engines are required")
    cases = {case["name"]: case for case in fixture["cases"]}
    for engine in engines:
        run = safe_child(path.parent, engine["run_dir"])
        subprocess.run([str(binary), "verify", str(run)], check=True, capture_output=True)
        receipt = json.loads((run / "receipt.json").read_text())
        require(receipt["status"] == "PASS", "Failed execution is not fixture success")
        require(receipt["source"]["sha256"] == report["source_sha256"], "Source identity mismatch")
        require(receipt["goblin_version"] == report["goblin_version"], "Version mismatch")
        require(receipt["environment"]["os"] == report["os"]
                and receipt["environment"]["arch"] == report["arch"], "Platform identity mismatch")
        require(receipt["math_policy"] == policy, "Receipt policy mismatch")
        require(receipt["execution"]["engine"] == engine["engine"], "Engine mismatch")
        require(receipt["receipt_core_sha256"] == engine["receipt_core_sha256"], "Receipt hash mismatch")
        require(receipt["math_environment"] == engine["math_environment"], "Build identity mismatch")
        observed = {result["name"]: result for result in engine["results"]}
        require(len(observed) == len(engine["results"]) and observed.keys() == cases.keys(),
                "Missing, duplicate or unexpected fixture results")
        artifacts = {entry["name"]: entry for entry in receipt["sealed_artifacts"]}
        for name, result in observed.items():
            entry = artifacts[name]
            artifact_bytes = safe_child(run, entry["path"]).read_bytes()
            require(hashlib.sha256(artifact_bytes).hexdigest() == entry["sha256"],
                    f"Artifact changed after verification: {name}")
            value = json.loads(artifact_bytes)["value"]
            require(bits(value["value_si"]) == result["bits_hex"] == bits(result["observed"]),
                    f"Report bits differ from verified value: {name}")
            case = cases[name]
            require(value["dimension"] == result["dimension"] == case["dimension"],
                    f"Dimension mismatch: {name}")
            identical = result["bits_hex"] == bits(case["expected"])
            accepted = close(result["observed"], case["expected"], case["absolute"], case["relative"])
            require(identical == result["reference_bits_equal"]
                    and accepted == result["within_fixture_tolerance"], f"Incorrect report label: {name}")
            require(accepted and (not case["exact"] or identical), f"Reference fixture failed: {name}")
    return report


def compare_reports(reports: list[dict], fixture: dict, require_platforms: bool) -> dict:
    require(len(reports) >= 2, "At least two reports required")
    if require_platforms:
        require({(r["os"], r["arch"]) for r in reports}
                == {("macos", "aarch64"), ("linux", "x86_64")},
                "Gate requires measured macOS arm64 and Linux x86_64, not inferred coverage")
    for field in ("goblin_version", "math_policy", "source_sha256", "fixture_sha256"):
        require(all(r[field] == reports[0][field] for r in reports), f"Different {field}")
    cases = {case["name"]: case for case in fixture["cases"]}
    observations = []
    for index, report in enumerate(reports):
        for engine in report["engines"]:
            label = f"report{index}:{report['os']}/{report['arch']}:{engine['engine']}"
            observations.append((label, {r["name"]: r for r in engine["results"]}))
    comparisons = []
    for (left_name, left), (right_name, right) in itertools.combinations(observations, 2):
        for name, case in cases.items():
            a, b = left[name], right[name]
            identical = a["bits_hex"] == b["bits_hex"]
            accepted = close(a["observed"], b["observed"], case["absolute"], case["relative"])
            require(accepted and (not case["exact"] or identical),
                    f"Cross-engine/platform fixture failed: {name} ({left_name} vs {right_name})")
            comparisons.append({"name": name, "left": left_name, "right": right_name,
                                "classification": "BITWISE_IDENTICAL" if identical else "WITHIN_FIXTURE_TOLERANCE",
                                "absolute_tolerance_si": case["absolute"], "relative_tolerance": case["relative"]})
    return {"schema": "goblin.platform-math-comparison.v1", "status": "PASS",
            "scope": "FINITE_NAMED_FIXTURES_ONLY", "comparison_policy": POLICY,
            "cross_platform_gate": require_platforms, "fixture_sha256": reports[0]["fixture_sha256"],
            "scientific_validity": "NOT_ESTABLISHED_BY_COMPARISON", "comparisons": comparisons}


def latest_reports(root: Path) -> list[Path]:
    """Keep retries immutable; choose the latest preserved attempt per runner.

    A successful earlier job may be reused when only failed jobs are retried.
    validate_report/compare_reports still require matching source/version/policy.
    """
    latest = {}
    for directory in root.iterdir():
        match = re.fullmatch(r"math-evidence-(macos-latest|ubuntu-latest)-(\d+)", directory.name)
        if match and (directory / "report.json").is_file():
            runner, attempt = match[1], int(match[2])
            if runner not in latest or attempt > latest[runner][0]:
                latest[runner] = (attempt, directory / "report.json")
    require(set(latest) == {"macos-latest", "ubuntu-latest"}, "Missing preserved platform report")
    return [latest[runner][1] for runner in sorted(latest)]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reports", nargs="*", type=Path)
    parser.add_argument("--latest-per-platform", type=Path,
                        help="Choose latest immutable CI attempt for each required runner")
    parser.add_argument("--goblinpp", required=True, type=Path)
    parser.add_argument("--require-macos-linux", action="store_true")
    args = parser.parse_args()
    fixture_bytes = (ROOT / "tests/fixtures/platform_math.json").read_bytes()
    fixture = json.loads(fixture_bytes)
    try:
        require(bool(args.reports) != bool(args.latest_per_platform),
                "Supply report paths OR --latest-per-platform, not both")
        paths = args.reports if args.reports else latest_reports(args.latest_per_platform)
        reports = [validate_report(path, fixture, hashlib.sha256(fixture_bytes).hexdigest(),
                                   args.goblinpp.resolve()) for path in paths]
        print(json.dumps(compare_reports(reports, fixture, args.require_macos_linux), indent=2))
    except (ValueError, KeyError, TypeError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Math evidence gate failed: {error}\n")


if __name__ == "__main__":
    main()
