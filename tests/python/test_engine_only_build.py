"""Prove the actual engine build with the JUCE/browser source trees absent."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
CMAKE = shutil.which("cmake")
CTEST = shutil.which("ctest")


@unittest.skipUnless(sys.platform.startswith("linux"), "existing Rust static-library link recipe is Linux-specific")
class EngineBuildFixture(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Keep the task-local Cargo/build cache reusable across regression runs.
        cls.work = ROOT / "build" / "engine-only-regression"
        cls.work.mkdir(parents=True, exist_ok=True)
        # Fresh snapshots cannot retain removed source files or discovery caches.
        cls.fixture_root = Path(tempfile.mkdtemp(prefix="fixture-", dir=cls.work))
        cls.source = cls.fixture_root / "source"
        cls.source.mkdir()
        cls.cargo_target = cls.work / "cargo-target"
        cls.cargo_target.mkdir(exist_ok=True)
        shutil.copy2(ROOT / "CMakeLists.txt", cls.source / "CMakeLists.txt")
        for name in ("src", "tests", "examples", "schema", "scripts", "spikes"):
            shutil.copytree(ROOT / name, cls.source / name, ignore=shutil.ignore_patterns("target", "build", "__pycache__"))
        assert not (cls.source / "third_party").exists()
        assert not (cls.source / "web").exists()
        assert not (cls.source / "ui").exists()

    def command(self, label, args):
        result = subprocess.run(args, capture_output=True, text=True,
                                env=dict(os.environ, CARGO_NET_OFFLINE="true"))
        output = result.stdout + result.stderr
        (self.work / (label + ".log")).write_text(output)
        return result.returncode, output

    def configure(self, label, *options):
        # DefaultPatch searches upward from CTest; keep it inside this snapshot.
        build = self.source / "build" / label
        build.mkdir(parents=True)
        if "-DDANDRUM_ENGINE_ONLY=ON" in options:
            # All fixtures belong to this task; CTest serializes this guard.
            (build / "rust-target").symlink_to(self.cargo_target, target_is_directory=True)
        code, output = self.command(label, [CMAKE, "-S", str(self.source), "-B", str(build),
                                          "-DCMAKE_BUILD_TYPE=Release", *options])
        return build, code, output


class EngineOnlyBuildTest(EngineBuildFixture):
    def test_engine_build_without_host_dependencies(self):
        build, code, output = self.configure("engine", "-DDANDRUM_ENGINE_ONLY=ON")
        self.assertEqual(code, 0, output)
        cache = (build / "CMakeCache.txt").read_text()
        self.assertNotIn("DANDRUM_NODE_EXECUTABLE:", cache)
        code, output = self.command("engine-targets", [CMAKE, "--build", str(build), "--target", "help"])
        self.assertEqual(code, 0, output)
        for target in ("juceaide", "dandrum-plugin", "dandrum-sampler-plugin",
                       "dandrum-drum-machine-demo", "dandrum-native-knob-fonts"):
            self.assertNotRegex(output, r"(?m)(?:^|\s)" + target + r"(?:\s|$)")
        # Build every independent native target, so accidental adapter links fail.
        code, output = self.command("engine-build", [CMAKE, "--build", str(build), "--parallel", "2"])
        self.assertEqual(code, 0, output)
        code, output = self.command("engine-inventory", [CTEST, "--test-dir", str(build),
                                                       "--show-only=json-v1"])
        self.assertEqual(code, 0, output)
        names = {test["name"] for test in json.loads(output)["tests"]}
        self.assertNotIn("clap-dynamic-host", names)
        self.assertTrue({"rust-engine-tests", "cxx-rust-engine-ffi-smoke",
                         "cxx-rust-engine-sample-accurate-midi-smoke", "sound-lab-controller",
                         "cxx-plugin-waveform-service", "cxx-plugin-meter-capture",
                         "filter-spike-engine"}.issubset(names), names)
        # Existing signed gain/reset and sample-offset tests exercise real Rust.
        code, output = self.command("engine-runtime", [CTEST, "--test-dir", str(build),
            "--output-on-failure", "-R", "^cxx-rust-engine-(ffi-smoke|sample-accurate-midi-smoke)$"])
        self.assertEqual(code, 0, output)
        self.assertIn("100% tests passed", output)

    def test_default_and_disabled_engine_mode_require_comparison_host(self):
        for label, options in (("default-host", []), ("explicit-host", ["-DDANDRUM_ENGINE_ONLY=OFF"])):
            with self.subTest(mode=label):
                _, code, output = self.configure(label, "-DDANDRUM_NATIVE_ONLY=ON", *options)
                self.assertNotEqual(code, 0, output)
                self.assertIn("third_party/JUCE", output)
                self.assertNotIn("Node.js is required", output)

    def test_conflicting_product_options_fail_before_dependency_discovery(self):
        options = ("DANDRUM_NATIVE_ONLY", "DANDRUM_TRIGGER_REACT",
                   "DANDRUM_BUILD_FILTER_UI_SPIKES", "DANDRUM_BUILD_TRIGGER_SLINT",
                   "DANDRUM_SLINT_LIBRARY_ONLY", "DANDRUM_ADVANCED_SAMPLER_ONLY")
        for option in options:
            with self.subTest(option=option):
                _, code, output = self.configure("conflict-" + option,
                    "-DDANDRUM_ENGINE_ONLY=ON", "-D" + option + "=ON")
                self.assertNotEqual(code, 0, output)
                self.assertIn("DANDRUM_ENGINE_ONLY cannot be combined with " + option, " ".join(output.split()))
                self.assertNotIn("third_party/JUCE", output)
                self.assertNotIn("React rendering test dependencies", output)


class ClapProofBuildTest(EngineBuildFixture):
    def test_clap_selection_requires_independent_engine_mode(self):
        _, code, output = self.configure("clap-needs-engine-mode", "-DDANDRUM_CLAP_PROOF=ON")
        self.assertNotEqual(code, 0, output)
        self.assertIn("DANDRUM_CLAP_PROOF requires DANDRUM_ENGINE_ONLY", output)
        self.assertNotIn("Node.js is required", output)
        self.assertNotIn("third_party/JUCE", output)

    def test_exported_clap_build_and_dynamic_tests_without_host_sources(self):
        build, code, output = self.configure("clap", "-DDANDRUM_ENGINE_ONLY=ON",
                                             "-DDANDRUM_CLAP_PROOF=ON")
        self.assertEqual(code, 0, output)
        cache = (build / "CMakeCache.txt").read_text()
        self.assertNotIn("DANDRUM_NODE_EXECUTABLE:", cache)
        code, output = self.command("clap-targets", [CMAKE, "--build", str(build), "--target", "help"])
        self.assertEqual(code, 0, output)
        self.assertRegex(output, r"(?m)(?:^|\s)dandrum-clap-proof(?:\s|$)")
        self.assertNotRegex(output, r"(?m)(?:^|\s)(?:juceaide|dandrum-plugin)(?:\s|$)")
        code, output = self.command("clap-build", [CMAKE, "--build", str(build),
                                                   "--target", "dandrum-clap-proof", "--parallel", "2"])
        self.assertEqual(code, 0, output)
        self.assertTrue((build / "clap" / "DandrumHeadlessProof.clap").is_file())
        code, output = self.command("clap-runtime", [CTEST, "--test-dir", str(build),
                                                     "--output-on-failure", "-R", "^clap-dynamic-host$"])
        self.assertEqual(code, 0, output)
        self.assertIn("100% tests passed", output)


class ComparisonBuildTest(EngineBuildFixture):
    def test_comparison_configure_retains_juce_targets(self):
        build = self.work / "comparison"
        code, output = self.command("comparison-configure", [CMAKE, "-S", str(ROOT), "-B", str(build),
            "-DCMAKE_BUILD_TYPE=Release", "-DDANDRUM_NATIVE_ONLY=ON", "-DDANDRUM_ENGINE_ONLY=OFF"])
        self.assertEqual(code, 0, output)
        code, output = self.command("comparison-targets", [CMAKE, "--build", str(build), "--target", "help"])
        self.assertEqual(code, 0, output)
        for target in ("dandrum-plugin", "dandrum-sampler-plugin", "dandrum-drum-machine-demo", "dandrum-ffi-smoke"):
            self.assertRegex(output, r"(?m)(?:^|\s)" + target + r"(?:\s|$)")
        code, output = self.command("comparison-inventory", [CTEST, "--test-dir", str(build), "--show-only=json-v1"])
        self.assertEqual(code, 0, output)
        names = {test["name"] for test in json.loads(output)["tests"]}
        self.assertTrue({"rust-engine-tests", "cxx-rust-engine-ffi-smoke", "cxx-juce-kernel-source",
                         "native-editor-smoke"}.issubset(names), names)


if __name__ == "__main__":
    unittest.main()
