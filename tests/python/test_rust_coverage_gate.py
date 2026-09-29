import contextlib
import importlib.util
import io
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


source = Path(__file__).resolve().parents[2] / "scripts" / "check-rust-coverage.py"
spec = importlib.util.spec_from_file_location("rust_coverage_check", source)
checker = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = checker
spec.loader.exec_module(checker)


class RustCoverageGateTest(unittest.TestCase):
    def run_gate(self, strict_name, reported_name, missed_line=None, create_source=True):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_root = root / "src" / "rust-engine" / "src"
            source_root.mkdir(parents=True)
            strict_path = f"src/rust-engine/src/{strict_name}"
            if create_source:
                (source_root / strict_name).write_text("fn example() {}\n")
            policy = root / "coverage-allowlist.txt"
            policy.write_text(f"strict {strict_path}\n")
            missed = 1 if missed_line is not None else 0
            report = (
                "Filename Regions Missed Regions Cover Functions Missed Functions Executed "
                "Lines Missed Lines Cover Branches Missed Branches Cover\n"
                "--------------------------------\n"
                f"{reported_name} 1 0 100.00% 1 0 100.00% 1 {missed} 100.00% 0 0 -\n"
                f"TOTAL 1 0 100.00% 1 0 100.00% 1 {missed} 100.00% 0 0 -\n"
                "Uncovered Lines:\n"
            )
            if missed_line is not None:
                report += f"{source_root / reported_name}: {missed_line}\n"
            output = io.StringIO()
            with (
                mock.patch.object(checker, "ROOT", root),
                mock.patch.object(checker, "POLICY", policy),
                mock.patch.object(
                    checker.subprocess,
                    "run",
                    return_value=SimpleNamespace(returncode=0, stdout=report),
                ),
                contextlib.redirect_stdout(output),
            ):
                result = checker.main()
            return result, output.getvalue()

    def test_missing_strict_source_fails_closed(self):
        result, output = self.run_gate("missing.rs", "present.rs", create_source=False)
        self.assertEqual(result, 1)
        self.assertIn("missing.rs", output)

    def test_strict_source_absent_from_measured_report_fails_closed(self):
        result, output = self.run_gate("strict.rs", "other.rs")
        self.assertEqual(result, 1)
        self.assertIn("strict.rs", output)

    def test_uncovered_executable_line_has_a_specific_diagnostic(self):
        result, output = self.run_gate("strict.rs", "strict.rs", missed_line=1)
        self.assertEqual(result, 1)
        self.assertIn("uncovered lines 1", output)

    def test_empty_measurement_cannot_pass_as_full_coverage(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            policy = root / "coverage-allowlist.txt"
            policy.write_text("strict src/rust-engine/src/strict.rs\n")
            output = io.StringIO()
            with (
                mock.patch.object(checker, "ROOT", root),
                mock.patch.object(checker, "POLICY", policy),
                mock.patch.object(
                    checker.subprocess,
                    "run",
                    return_value=SimpleNamespace(returncode=0, stdout=""),
                ),
                contextlib.redirect_stdout(output),
            ):
                result = checker.main()
            self.assertNotEqual(result, 0)


if __name__ == "__main__":
    unittest.main()
