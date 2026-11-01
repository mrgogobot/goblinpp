"""The distributable must not copy locally generated networking credentials."""
import importlib.util
from pathlib import Path
import tempfile
import unittest


class NetworkPackageTests(unittest.TestCase):
    def test_local_network_credentials_are_excluded_from_source_copy(self):
        path = Path(__file__).resolve().parents[1] / 'package_local.py'
        spec = importlib.util.spec_from_file_location('package_local', path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / 'source'
            (source / 'network-demo').mkdir(parents=True)
            (source / 'network-demo/recipient.key').write_text('public fake secret fixture')
            (source / 'network_client.gbl').write_text('GO_PRIVATE\n')
            module.copy_tree(source, root / 'copied')
            self.assertFalse((root / 'copied/network-demo').exists())
            self.assertTrue((root / 'copied/network_client.gbl').is_file())
