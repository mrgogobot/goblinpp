"""Verify offline license fallbacks fail closed on altered terms or identities."""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

TOOLS = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('license_collector', TOOLS / 'collect_third_party_notices.py')
collector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(collector)


class LicenseEvidenceTests(unittest.TestCase):
    def test_real_fallbacks_match_registry_identity_and_hashes(self):
        for evidence in sorted((TOOLS / 'license_sources/crates').iterdir()):
            metadata = json.loads((evidence / 'PROVENANCE.json').read_text())
            source = collector.registry_package(metadata['crate'], metadata['version'])
            with tempfile.TemporaryDirectory() as temporary:
                result = collector.crate_notices(source, Path(temporary) / 'out', metadata['crate'], metadata['version'])
                self.assertIn('PROVENANCE.json', result)

    def test_altered_license_is_rejected_before_copying(self):
        import shutil
        evidence = TOOLS / 'license_sources/crates/age-0.11.5'
        source = collector.registry_package('age', '0.11.5')
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target = root / 'tools/license_sources/crates/age-0.11.5'
            shutil.copytree(evidence, target)
            (target / 'LICENSE-MIT').write_text('altered terms')
            with patch.object(collector, 'ROOT', root), self.assertRaisesRegex(SystemExit, 'hash mismatch'):
                collector.crate_notices(source, root / 'out', 'age', '0.11.5')
            self.assertFalse((root / 'out').exists())
