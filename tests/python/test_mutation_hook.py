"""Exercise the push hook's changed-owner contract without running mutation jobs."""
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class MutationHookTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="dandrum hook ")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "checkout with spaces"
        self.root.mkdir()
        self.calls = Path(self.temporary.name) / "cargo-calls.jsonl"
        self.cargo = Path(self.temporary.name) / "cargo-driver"
        self.cargo.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, sys\n"
            "with open(os.environ['DANDRUM_HOOK_CALLS'], 'a') as output:\n"
            "    output.write(json.dumps(sys.argv[1:]) + '\\n')\n"
            "key = 'DANDRUM_HOOK_TEST_EXIT' if sys.argv[1] == 'test' else 'DANDRUM_HOOK_MUTATION_EXIT'\n"
            "sys.exit(int(os.environ.get(key, '0')))\n"
        )
        self.cargo.chmod(0o755)
        self.git("init", "-q", "-b", "baseline")
        self.git("config", "user.name", "Hook test")
        self.git("config", "user.email", "hook-test@example.invalid")
        self.write("README.md", "baseline\n")
        self.commit("Baseline")
        self.git("checkout", "-q", "-b", "candidate")
        self.git("branch", "--set-upstream-to=baseline")

    def git(self, *args):
        return subprocess.run(
            ["git", *args], cwd=self.root, check=True,
            capture_output=True, text=True,
        )

    def write(self, relative, content):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def commit(self, message):
        self.git("add", "--all")
        self.git("commit", "-qm", message)

    def invoke(self, test_exit=0, mutation_exit=0):
        env = dict(os.environ, DANDRUM_CARGO=str(self.cargo),
                   DANDRUM_HOOK_CALLS=str(self.calls),
                   DANDRUM_HOOK_TEST_EXIT=str(test_exit),
                   DANDRUM_HOOK_MUTATION_EXIT=str(mutation_exit))
        result = subprocess.run(
            ["bash", str(ROOT / ".githooks/pre-push")],
            cwd=self.root, env=env, capture_output=True, text=True,
        )
        calls = [json.loads(line) for line in self.calls.read_text().splitlines()]
        return result, calls

    def test_selects_changed_owners_relative_to_crate_with_spaces_intact(self):
        self.write("src/rust-engine/src/preparation.rs", "pub fn prepare() {}\n")
        self.write("src/rust-engine/src/kernel/voice with spaces.rs", "pub fn voice() {}\n")
        self.write("tests/cpp/ignored.cpp", "int ignored;\n")
        self.commit("Change two Rust owners and unrelated C++")
        result, calls = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        manifest = str(self.root / "src/rust-engine/Cargo.toml")
        self.assertEqual(calls[0], ["test", "--manifest-path", manifest])
        self.assertEqual(calls[1][:3], ["mutants", "--manifest-path", manifest])
        selectors = calls[1][3:]
        self.assertEqual(selectors[::2], ["--file", "--file"])
        self.assertEqual(set(selectors[1::2]),
                         {"src/preparation.rs", "src/kernel/voice with spaces.rs"})
        self.assertEqual(len(calls), 2)

    def test_non_rust_change_runs_tests_without_starting_mutation(self):
        self.write("README.md", "documentation only\n")
        self.commit("Documentation")
        result, calls = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][0], "test")
        self.assertIn("No changed Rust source files", result.stdout)

    def test_missing_upstream_selects_owners_from_last_commit(self):
        self.git("branch", "--unset-upstream")
        self.write("src/rust-engine/src/fallback.rs", "pub fn fallback() {}\n")
        self.commit("Fallback owner")
        result, calls = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls[1][-2:], ["--file", "src/fallback.rs"])

    def test_failed_unit_gate_propagates_exit_without_starting_mutation(self):
        self.write("src/rust-engine/src/failing.rs", "pub fn failing() {}\n")
        self.commit("Changed owner")
        result, calls = self.invoke(test_exit=23)
        self.assertEqual(result.returncode, 23)
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][0], "test")

    def test_failed_mutation_gate_propagates_exit(self):
        self.write("src/rust-engine/src/failing.rs", "pub fn failing() {}\n")
        self.commit("Changed owner")
        result, calls = self.invoke(mutation_exit=17)
        self.assertEqual(result.returncode, 17)
        self.assertEqual([call[0] for call in calls], ["test", "mutants"])


if __name__ == "__main__":
    unittest.main()
