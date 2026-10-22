"""Fast fixture lifecycle checks; no compiler, Cargo or CMake execution."""
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import test_engine_only_build as fixture


@unittest.skipUnless(sys.platform.startswith("linux"), "engine build fixture is Linux-specific")
class EngineBuildFixtureCleanupTest(unittest.TestCase):
    def test_finished_snapshots_are_removed_but_shared_targets_and_logs_survive(self):
        for outcome in ("success", "failure", "setup-error"):
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                (root / "CMakeLists.txt").write_text("# minimal source snapshot\n")
                for name in ("src", "tests", "examples", "schema", "scripts", "spikes"):
                    (root / name).mkdir()
                    (root / name / "input.txt").write_text(name)
                work = root / "build/engine-only-regression"
                cache = work / "cargo-target"
                cache.mkdir(parents=True)
                sentinel = cache / "retained-dependency"
                sentinel.write_bytes(b"compiled dependency sentinel")
                log = work / "retained-command.log"
                log.write_bytes(b"raw command evidence")

                class Probe(fixture.EngineBuildFixture):
                    def test_probe(self):
                        self.assertTrue(self.source.is_dir())
                        (self.source / "shared-target").symlink_to(
                            self.cargo_target, target_is_directory=True)
                        if outcome == "failure":
                            self.fail("deliberate test failure")

                result = unittest.TestResult()
                with patch.object(fixture, "ROOT", root):
                    if outcome == "setup-error":
                        with patch.object(fixture.shutil, "copytree", side_effect=
                                          OSError("deliberate snapshot copy failure")):
                            unittest.defaultTestLoader.loadTestsFromTestCase(Probe).run(result)
                    else:
                        unittest.defaultTestLoader.loadTestsFromTestCase(Probe).run(result)

                self.assertEqual(result.testsRun, int(outcome != "setup-error"))
                self.assertEqual(len(result.failures), int(outcome == "failure"))
                self.assertEqual(len(result.errors), int(outcome == "setup-error"))
                if result.failures:
                    self.assertIn("deliberate test failure", result.failures[0][1])
                if result.errors:
                    self.assertIn("deliberate snapshot copy failure", result.errors[0][1])
                self.assertEqual(list(work.glob("fixture-*")), [],
                                 "completed or failed source snapshots must be removed")
                self.assertEqual(sentinel.read_bytes(), b"compiled dependency sentinel")
                self.assertEqual(log.read_bytes(), b"raw command evidence")


if __name__ == "__main__":
    unittest.main()
