import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("rng_reports", Path(__file__).resolve().parents[1] / "compare_rng_reports.py")
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class RngReportTests(unittest.TestCase):
    def setUp(self):
        engine = {"rng": {"usage": "USED", "policy": gate.POLICY, "streams": [{"seed": "42", "results_sha256": "digest"}]},
                  "sealed_artifacts": {"x": {"sha256": "exact"}}}
        self.reports = [{"schema": gate.SCHEMA, "goblin_version": "test", "source_sha256": "source",
                         "rng_policy": {"id": gate.POLICY}, "interpreter": copy.deepcopy(engine),
                         "native": copy.deepcopy(engine), "os": os_name, "arch": arch}
                        for os_name, arch in [("macos", "aarch64"), ("linux", "x86_64")]]

    def test_exact_measured_platforms_pass(self):
        self.assertTrue(gate.compare_reports(self.reports)["cross_platform_gate"])

    def test_missing_or_same_platform_does_not_pass(self):
        with self.assertRaises(ValueError):
            gate.compare_reports([])
        with self.assertRaisesRegex(ValueError, "measured"):
            gate.compare_reports(self.reports[:1])
        self.reports[1]["os"] = "macos"
        with self.assertRaisesRegex(ValueError, "measured"):
            gate.compare_reports(self.reports)

    def test_identity_and_exact_digest_drift_refuse(self):
        for field in ("goblin_version", "source_sha256", "rng_policy"):
            changed = copy.deepcopy(self.reports)
            changed[1][field] = {"id": "changed"} if field == "rng_policy" else "changed"
            with self.assertRaises(ValueError):
                gate.compare_reports(changed)
        self.reports[1]["interpreter"]["rng"]["streams"][0]["results_sha256"] = "altered"
        self.reports[1]["native"] = copy.deepcopy(self.reports[1]["interpreter"])
        with self.assertRaisesRegex(ValueError, "Cross-platform"):
            gate.compare_reports(self.reports)

    def test_native_drift_and_missing_evidence_refuse(self):
        self.reports[1]["native"]["rng"]["streams"][0]["seed"] = "43"
        with self.assertRaisesRegex(ValueError, "Native"):
            gate.compare_reports(self.reports)
        changed = copy.deepcopy(self.reports[:1])
        changed[0]["interpreter"]["rng"]["usage"] = "NOT_USED"
        with self.assertRaises(ValueError):
            gate.compare_reports(changed, False)
