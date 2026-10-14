import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


root = Path(__file__).resolve().parents[2]
source = root / "scripts" / "check-spec-coverage.py"
spec = importlib.util.spec_from_file_location("spec_coverage_check", source)
checker = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = checker
spec.loader.exec_module(checker)


class SpecCoverageMapTest(unittest.TestCase):
    def test_external_rust_crate_tests_are_found_without_generated_outputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            fixture_root = Path(temporary)
            sources = {
                "src/clap-adapter/tests/dynamic_host.rs": "dynamic_plugin_fixture_test",
                "src/rust-engine/tests/public_api.rs": "engine_public_fixture_test",
                "src/clap-adapter/target/generated.rs": "generated_target_fixture_test",
                "src/rust-engine/llvm-cov-target/generated.rs": "generated_coverage_fixture_test",
            }
            for relative_path, name in sources.items():
                path = fixture_root / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(f"#[test]\nfn {name}() {{}}\n")
            with mock.patch.object(checker, "ROOT", fixture_root):
                names = checker.rust_test_functions()
            self.assertTrue("dynamic_plugin_fixture_test" in names, "external crate test was not found")
            self.assertTrue("engine_public_fixture_test" in names, "engine public test was not found")
            self.assertNotIn("generated_target_fixture_test", names)
            self.assertNotIn("generated_coverage_fixture_test", names)
            self.assertFalse(checker.test_exists("rust:missing_fixture_test", names))

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
