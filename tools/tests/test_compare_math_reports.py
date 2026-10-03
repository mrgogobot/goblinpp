import copy
import importlib.util
import math
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("math_reports", Path(__file__).resolve().parents[1] / "compare_math_reports.py")
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class MathEvidenceGateTests(unittest.TestCase):
    def setUp(self):
        self.fixture = {"cases": [{"name": "value", "expected": 1., "absolute": 1e-12,
                                    "relative": 0., "exact": False}]}
        policy = {"comparison_policy": gate.POLICY}
        self.reports = []
        for os_name, architecture in [("macos", "aarch64"), ("linux", "x86_64")]:
            self.reports.append({"os": os_name, "arch": architecture, "math_policy": policy,
                                 "goblin_version": "test", "source_sha256": "source", "fixture_sha256": "fixture",
                                 "engines": [{"engine": engine, "results": [{"name": "value", "observed": 1.,
                                              "bits_hex": gate.bits(1.)}]} for engine in ("rust-interpreter", "rust-native-compiled")]})

    def test_exact_and_toleranced_labels_are_distinct(self):
        result = gate.compare_reports(self.reports, self.fixture, True)
        self.assertTrue(all(c["classification"] == "BITWISE_IDENTICAL" for c in result["comparisons"]))
        value = 1. + 1e-13
        self.reports[1]["engines"][0]["results"][0].update(observed=value, bits_hex=gate.bits(value))
        result = gate.compare_reports(self.reports, self.fixture, True)
        self.assertIn("WITHIN_FIXTURE_TOLERANCE", {c["classification"] for c in result["comparisons"]})

    def test_outside_tolerance_refuses(self):
        self.reports[1]["engines"][0]["results"][0].update(observed=2., bits_hex=gate.bits(2.))
        with self.assertRaisesRegex(ValueError, "fixture failed"):
            gate.compare_reports(self.reports, self.fixture, True)

    def test_exact_cases_cannot_use_tolerance_to_hide_changed_bits(self):
        self.fixture["cases"][0]["exact"] = True
        value = 1. + 1e-13
        self.reports[1]["engines"][0]["results"][0].update(observed=value, bits_hex=gate.bits(value))
        with self.assertRaises(ValueError):
            gate.compare_reports(self.reports, self.fixture, True)

    def test_platform_coverage_cannot_be_inferred(self):
        self.reports[1]["os"] = "macos"
        with self.assertRaisesRegex(ValueError, "measured"):
            gate.compare_reports(self.reports, self.fixture, True)
        self.assertFalse(gate.compare_reports(self.reports, self.fixture, False)["cross_platform_gate"])

    def test_identity_mismatch_refuses(self):
        for field in ("goblin_version", "source_sha256", "fixture_sha256", "math_policy"):
            reports = copy.deepcopy(self.reports)
            reports[1][field] = "changed"
            with self.assertRaisesRegex(ValueError, field):
                gate.compare_reports(reports, self.fixture, True)

    def test_invalid_tolerances_and_nonfinite_values_refuse(self):
        for args in [(1., 1., -1., 0.), (1., 1., 0., -1.), (math.nan, 1., 0., 0.),
                     (1., 1., math.inf, 0.)]:
            with self.assertRaises(ValueError):
                gate.close(*args)

    def test_overflow_and_signed_zero(self):
        self.assertFalse(gate.close(1e308, -1e308, 1e308, 1.99))
        self.assertTrue(gate.close(1e308, -1e308, 0., 2.))
        self.assertTrue(gate.close(-0., 0., 0., 0.))
        self.assertNotEqual(gate.bits(-0.), gate.bits(0.))

    def test_escaping_evidence_paths_refuse(self):
        with self.assertRaises(ValueError):
            gate.safe_child(Path("."), "../outside")
        with self.assertRaises(ValueError):
            gate.safe_child(Path("."), "/outside")

    def test_retry_reports_are_selected_without_replacing_earlier_evidence(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ["math-evidence-macos-latest-1", "math-evidence-macos-latest-2",
                         "math-evidence-ubuntu-latest-1"]:
                (root / name).mkdir()
                (root / name / "report.json").write_text("{}")
            chosen = gate.latest_reports(root)
            self.assertEqual({p.parent.name for p in chosen},
                             {"math-evidence-macos-latest-2", "math-evidence-ubuntu-latest-1"})
            self.assertTrue((root / "math-evidence-macos-latest-1/report.json").exists())

    def test_report_must_match_verified_seals_platform_and_fixture_source(self):
        fixture = {"cases": [{"name": "root", "expression": "sqrt(81)", "expected": 9.,
                              "absolute": 0., "relative": 0., "exact": True, "dimension": [0]*6}]}
        fixture_hash = "fixture-hash"
        source_hash = hashlib.sha256(b"GO_PARANOID\nroot = sqrt(81)\nseal root\n").hexdigest()
        policy = {"comparison_policy": gate.POLICY, "default_tolerance": None,
                  "cross_platform_bitwise_guarantee": False}
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            engines = []
            for engine in ("rust-interpreter", "rust-native-compiled"):
                run = root / engine
                run.mkdir()
                artifact = json.dumps({"value": {"value_si": 9., "dimension": [0]*6}}).encode()
                (run / "root.json").write_bytes(artifact)
                receipt = {"status": "PASS", "source": {"sha256": source_hash}, "goblin_version": "test",
                           "environment": {"os": "macos", "arch": "aarch64"}, "math_policy": policy,
                           "execution": {"engine": engine}, "receipt_core_sha256": "receipt-hash",
                           "math_environment": {}, "sealed_artifacts": [{"name": "root", "path": "root.json",
                              "sha256": hashlib.sha256(artifact).hexdigest()}]}
                (run / "receipt.json").write_text(json.dumps(receipt))
                engines.append({"engine": engine, "run_dir": engine, "receipt_core_sha256": "receipt-hash",
                                "math_environment": {}, "results": [{"name": "root", "observed": 9.,
                                   "bits_hex": gate.bits(9.), "dimension": [0]*6,
                                   "reference_bits_equal": True, "within_fixture_tolerance": True}]})
            report = {"schema": "goblin.platform-math-report.v1", "fixture_sha256": fixture_hash,
                      "source_sha256": source_hash, "goblin_version": "test", "os": "macos", "arch": "aarch64",
                      "math_policy": policy, "engines": engines}
            path = root / "report.json"
            path.write_text(json.dumps(report))
            with patch.object(gate.subprocess, "run") as verifier:
                gate.validate_report(path, fixture, fixture_hash, Path("verifier"))
                self.assertEqual(verifier.call_count, 2)
                for mutation, reason in [(lambda r: r["engines"][0]["results"][0].update(bits_hex=gate.bits(8.)), "bits"),
                                         (lambda r: r.update(os="linux"), "Platform"),
                                         (lambda r: r.update(source_sha256="other-source"), "fixture program")]:
                    changed = copy.deepcopy(report)
                    mutation(changed)
                    path.write_text(json.dumps(changed))
                    with self.assertRaisesRegex(ValueError, reason):
                        gate.validate_report(path, fixture, fixture_hash, Path("verifier"))
                path.write_text(json.dumps(report))
                verifier.side_effect = subprocess.CalledProcessError(1, "verify")
                with self.assertRaises(subprocess.CalledProcessError):
                    gate.validate_report(path, fixture, fixture_hash, Path("verifier"))


if __name__ == "__main__":
    unittest.main()
