"""Fast fixture lifecycle checks; no compiler, Cargo or CMake execution."""
from pathlib import Path
import sys
import tempfile
import unittest
import os
import fcntl
from unittest.mock import patch

import test_engine_only_build as fixture


@unittest.skipUnless(sys.platform.startswith("linux"), "engine build fixture is Linux-specific")
class EngineBuildFixtureCleanupTest(unittest.TestCase):
    def test_unknown_linked_or_active_snapshot_is_preserved(self):
        for case in ('unknown', 'linked-marker', 'active'):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                work = root / 'build/engine-only-regression'
                work.mkdir(parents=True)
                sentinel = root / 'important'
                sentinel.write_text('preserve')
                if case == 'unknown':
                    snapshot = work / 'fixture-stable'
                    snapshot.mkdir()
                    (snapshot / 'unknown').write_text('keep')
                elif case == 'linked-marker':
                    (work / '.snapshot-owner.json').symlink_to(sentinel)
                with (work / '.snapshot-lease').open('a') as lease:
                    if case == 'active':
                        fcntl.flock(lease, fcntl.LOCK_EX)
                    class Probe(fixture.EngineBuildFixture):
                        def test_probe(self):
                            self.fail('must not start')
                    result = unittest.TestResult()
                    with patch.object(fixture, 'ROOT', root):
                        unittest.defaultTestLoader.loadTestsFromTestCase(Probe).run(result)
                self.assertEqual(result.testsRun, 0)
                self.assertEqual(len(result.errors), 1)
                self.assertEqual(sentinel.read_text(), 'preserve')
                if case == 'unknown':
                    self.assertEqual((snapshot / 'unknown').read_text(), 'keep')

    def test_finished_snapshots_are_removed_but_shared_targets_and_logs_survive(self):
        for outcome in ("success", "failure", "setup-error"):
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                (root / "CMakeLists.txt").write_text("# minimal source snapshot\n")
                (root / ".ai-rules.json").write_text('{}')
                for name in (".githooks", "src", "tests", "examples", "schema", "scripts", "spikes"):
                    (root / name).mkdir()
                    (root / name / "input.txt").write_text(name)
                notices = root / "docs/migrations/trigger-clap-beta-notices.json"
                notices.parent.mkdir(parents=True)
                notices.write_bytes((fixture.ROOT / "docs/migrations/trigger-clap-beta-notices.json").read_bytes())
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
                            cache, target_is_directory=True)
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


    def test_repeated_snapshot_has_stable_path_and_no_stale_source_or_cmake_state(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "CMakeLists.txt").write_text("# source-free fixture\n")
            (root / ".ai-rules.json").write_text('{}')
            for name in (".githooks", "src", "tests", "examples", "schema", "scripts", "spikes"):
                (root / name).mkdir()
                (root / name / "input.txt").write_text(name)
            notices = root / "docs/migrations/trigger-clap-beta-notices.json"
            notices.parent.mkdir(parents=True)
            notices.write_bytes((fixture.ROOT / "docs/migrations/trigger-clap-beta-notices.json").read_bytes())
            scratch = root / "owned-run-scratch"
            scratch.mkdir()
            work = root / "build/engine-only-regression"
            cache = work / "cargo-target"
            cache.mkdir(parents=True)
            (cache / "sentinel").write_bytes(b"retained compiled dependency")
            (work / "retained.log").write_bytes(b"retained command log")

            paths = []
            class Probe(fixture.EngineBuildFixture):
                def test_probe(self):
                    paths.append(self.source)
                    self.assertFalse((self.source / 'build/stale/CMakeCache.txt').exists())
                    self.assertEqual((self.source / 'src/input.txt').exists(), len(paths) == 1)
                    self.assertEqual((self.source / ".githooks/input.txt").read_text(), ".githooks",
                                     "registered hook checks need their authored hook in the snapshot")
                    stale = self.source / 'build/stale'
                    stale.mkdir(parents=True)
                    (stale / 'CMakeCache.txt').write_text('stale')
                    (self.source / "shared-target").symlink_to(cache, target_is_directory=True)

            result = unittest.TestResult()
            with patch.object(fixture, "ROOT", root), patch.dict(
                    fixture.os.environ, {"DANDRUM_VERIFICATION_SCRATCH": str(scratch)}):
                unittest.defaultTestLoader.loadTestsFromTestCase(Probe).run(result)
                self.assertTrue(result.wasSuccessful(), result.failures + result.errors)
                (root / 'src/input.txt').unlink()
                result = unittest.TestResult()
                unittest.defaultTestLoader.loadTestsFromTestCase(Probe).run(result)
            self.assertTrue(result.wasSuccessful(), result.failures + result.errors)
            self.assertEqual(result.testsRun, 1)
            self.assertEqual(paths[0], paths[1])
            self.assertEqual(list(scratch.iterdir()), [])
            self.assertEqual((cache / "sentinel").read_bytes(), b"retained compiled dependency")
            self.assertEqual((work / "retained.log").read_bytes(), b"retained command log")


if __name__ == "__main__":
    unittest.main()
