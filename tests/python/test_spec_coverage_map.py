import importlib.util
import sys
import unittest
from pathlib import Path


root = Path(__file__).resolve().parents[2]
source = root / "scripts" / "check-spec-coverage.py"
spec = importlib.util.spec_from_file_location("spec_coverage_check", source)
checker = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = checker
spec.loader.exec_module(checker)


class SpecCoverageMapTest(unittest.TestCase):
    def test_python_demo_tests_are_valid_test_references(self):
        self.assertTrue(checker.test_exists("py:tests/python/test_demo_launcher.py", set()))
        self.assertTrue(checker.test_exists("py:tests/python/test_demo_inventory.py", set()))
        self.assertFalse(checker.test_exists("py:tests/python/missing.py", set()))
        self.assertFalse(checker.test_exists("py:scripts/demo_launcher.py", set()))
        self.assertFalse(checker.test_exists("py:tests/python/../../scripts/demo_launcher.py", set()))

    def test_javascript_page_tests_are_valid_test_references(self):
        self.assertTrue(checker.test_exists("js:tests/js/Tb303PageTest.mjs", set()))
        self.assertFalse(checker.test_exists("js:tests/js/does-not-exist.mjs", set()))
        self.assertFalse(checker.test_exists("js:tests/cpp/PluginEditorBridgeTest.cpp", set()))


if __name__ == "__main__":
    unittest.main()
