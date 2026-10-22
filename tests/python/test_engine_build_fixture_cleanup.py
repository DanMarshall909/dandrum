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
                for name in (".githooks", "src", "tests", "examples", "schema", "scripts", "spikes"):
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


    def test_supervised_snapshot_uses_owned_scratch_and_keeps_cache_and_logs_external(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "CMakeLists.txt").write_text("# source-free fixture\n")
            for name in (".githooks", "src", "tests", "examples", "schema", "scripts", "spikes"):
                (root / name).mkdir()
                (root / name / "input.txt").write_text(name)
            scratch = root / "owned-run-scratch"
            scratch.mkdir()
            work = root / "build/engine-only-regression"
            cache = work / "cargo-target"
            cache.mkdir(parents=True)
            (cache / "sentinel").write_bytes(b"retained compiled dependency")
            (work / "retained.log").write_bytes(b"retained command log")

            class Probe(fixture.EngineBuildFixture):
                def test_probe(self):
                    snapshots = list(scratch.glob("fixture-*/source"))
                    self.assertEqual(len(snapshots), 1,
                                     "the supervisor must own the active source snapshot")
                    self.assertEqual((snapshots[0] / "src/input.txt").read_text(), "src")
                    self.assertEqual((snapshots[0] / ".githooks/input.txt").read_text(), ".githooks",
                                     "registered hook checks need their authored hook in the snapshot")
                    self.assertEqual(self.cargo_target, cache)
                    (snapshots[0] / "shared-target").symlink_to(cache, target_is_directory=True)

            result = unittest.TestResult()
            with patch.object(fixture, "ROOT", root), patch.dict(
                    fixture.os.environ, {"DANDRUM_VERIFICATION_SCRATCH": str(scratch)}):
                unittest.defaultTestLoader.loadTestsFromTestCase(Probe).run(result)
            self.assertTrue(result.wasSuccessful(), result.failures + result.errors)
            self.assertEqual(result.testsRun, 1)
            self.assertEqual(list(scratch.iterdir()), [])
            self.assertEqual((cache / "sentinel").read_bytes(), b"retained compiled dependency")
            self.assertEqual((work / "retained.log").read_bytes(), b"retained command log")


if __name__ == "__main__":
    unittest.main()
