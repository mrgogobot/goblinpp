"""Catch syntax/indentation errors in packaging and validation helpers."""
import unittest
from pathlib import Path


class ToolSyntaxTests(unittest.TestCase):
    def test_python_helpers_parse_without_running_them(self):
        tools = Path(__file__).resolve().parents[1]
        for source in tools.rglob("*.py"):
            if "__pycache__" in source.parts:
                continue
            with self.subTest(source=str(source.relative_to(tools))):
                compile(source.read_text(encoding="utf-8"), str(source), "exec")
