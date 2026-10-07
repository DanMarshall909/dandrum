import contextlib
import io
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import demo_launcher as launcher


class DemoLauncherTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="dandrum demos ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "checkout with spaces"
        (self.root / "scripts").mkdir(parents=True)
        shutil.copy(ROOT / "scripts/demos.json", self.root / "scripts/demos.json")

    def invoke(self, *args):
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            code = launcher.main(list(args), self.root)
        return code, stdout.getvalue(), stderr.getvalue()

    def test_list_help_and_unknown_name(self):
        for args in [(), ("--list",), ("--help",), ("-h",)]:
            with self.subTest(args=args):
                code, output, error = self.invoke(*args)
                self.assertEqual(code, 0, error)
                for name in ["tb303", "sampler", "drums", "react303", "trigger", "filter-slint", "filter-jive"]:
                    self.assertIn(name, output)
                self.assertIn("silent mock", output)
                self.assertIn("embedded WebView", output)
                self.assertIn("unavailable", output)
        code, output, error = self.invoke("does-not-exist")
        self.assertEqual(code, 2)
        self.assertIn("does-not-exist", error)
        self.assertIn("Usage: ./demo", output)

    def test_resolve_local_then_sorted_compatible_worktrees(self):
        other = Path(self.temp.name) / "z other checkout"
        first = Path(self.temp.name) / "a first checkout"
        for checkout in [other, first]:
            (checkout / "web/trigger").mkdir(parents=True)
            (checkout / "web/trigger/package.json").write_text('{}')
            (checkout / "src/rust-engine").mkdir(parents=True)
            (checkout / "src/rust-engine/Cargo.toml").write_text('[package]')
        porcelain = f"worktree {other}\nHEAD abc\n\nworktree {first}\nHEAD def\n\n"
        with patch("subprocess.run") as run:
            run.return_value.stdout = porcelain
            code, output, error = self.invoke()
            self.assertEqual(code, 0, error)
            self.assertIn(str(first), output)
            self.assertNotIn(str(other), output)
            (self.root / "web/trigger").mkdir(parents=True)
            (self.root / "web/trigger/package.json").write_text('{}')
            code, output, error = self.invoke()
            self.assertIn(str(self.root), output)
            self.assertNotIn(str(first), output)
        (first / "src/rust-engine/Cargo.toml").unlink()
        with patch("subprocess.run") as run:
            run.return_value.stdout = porcelain
            (self.root / "web/trigger/package.json").unlink()
            _, output, _ = self.invoke()
            self.assertIn(str(other), output)
            self.assertNotIn(str(first), output)

    def test_native_availability_requires_its_target(self):
        (self.root / "CMakeLists.txt").write_text('juce_add_plugin(other PRODUCT_NAME "Other")')
        with patch("subprocess.run") as run:
            run.return_value.stdout = ""
            code, _, error = self.invoke("drums")
            self.assertEqual(code, 1)
            self.assertIn("unavailable", error)
            (self.root / "CMakeLists.txt").write_text('juce_add_console_app(dandrum-drum-machine-demo PRODUCT_NAME "Drums")')
            code, output, error = self.invoke()
            self.assertEqual(code, 0, error)
            self.assertEqual(output.count(str(self.root)), 1)

    def test_missing_sources_and_git_do_not_start_preparation(self):
        with patch("subprocess.run", side_effect=FileNotFoundError("git")) as run:
            code, _, error = self.invoke("trigger")
            self.assertEqual(code, 1)
            self.assertIn("web/trigger/package.json", error)
            self.assertEqual(run.call_count, 1)
        with patch("subprocess.run") as run:
            code, _, _ = self.invoke("unknown")
            self.assertEqual(code, 2)
            run.assert_not_called()

    def native_fixture(self, name="drums", build="build", configuration=""):
        entry = next(item for item in json.loads((self.root / "scripts/demos.json").read_text()) if item["name"] == name)
        target = entry["target"].removesuffix("_Standalone")
        (self.root / "CMakeLists.txt").write_text(f'juce_add_console_app({target} PRODUCT_NAME "Demo")')
        source = self.root / entry["source"]
        if not source.exists():
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text("Demo source")
        parts = Path(entry["artifact"]).parts
        artifact = self.root / build / parts[0] / configuration / Path(*parts[1:])
        artifact.parent.mkdir(parents=True, exist_ok=True)
        artifact.write_text('#!/usr/bin/env python3\nimport json, os, sys\nfrom pathlib import Path\nPath("launched.json").write_text(json.dumps([os.getcwd(), sys.argv[1:]]))\n')
        artifact.chmod(0o755)
        return artifact

    def recorded_tools(self, commands):
        real_run = subprocess.run

        def run(command, **kwargs):
            if command[0] == "git":
                return subprocess.CompletedProcess(command, 0, stdout="")
            commands.append((command, kwargs))
            if Path(command[0]).name in {"cmake", "cmake.exe"} or command[0] == "npm":
                return subprocess.CompletedProcess(command, 0)
            return real_run(command, **kwargs)

        return patch("subprocess.run", side_effect=run)

    def test_native_build_launch_and_argument_forwarding(self):
        artifact = self.native_fixture()
        commands = []
        with self.recorded_tools(commands), patch.object(sys, "platform", "linux"):
            code, output, error = self.invoke("drums", "--", "--midi-input", "port with spaces")
        self.assertEqual(code, 0, error)
        self.assertIn(str(self.root), output)
        self.assertEqual(json.loads((self.root / "launched.json").read_text()), [str(self.root), ["--midi-input", "port with spaces"]])
        self.assertEqual(commands[0][0][1:], ["-S", str(self.root), "-B", str(self.root / "build")])
        self.assertEqual(commands[1][0][1:], ["--build", str(self.root / "build"), "--target", "dandrum-drum-machine-demo", "--config", "Release"])
        self.assertEqual(commands[2][0][0], str(artifact))
        for _, options in commands:
            self.assertEqual(options["cwd"], self.root)
        self.assertTrue(commands[0][1]["env"]["PATH"].startswith("/usr/bin:/bin:"))

    def test_empty_single_config_cache_launches_default_artifact(self):
        artifact = self.native_fixture()
        (self.root / "build/CMakeCache.txt").write_text('// ignored comment\nOTHER:STRING=value\nCMAKE_BUILD_TYPE:STRING=\n')
        commands = []
        with self.recorded_tools(commands):
            code, _, error = self.invoke("drums")
        self.assertEqual(code, 0, error)
        self.assertEqual(commands[-1][0], [str(artifact)])

    def test_native_configured_artifact_wins_over_stale_default(self):
        cases = [("sampler", "CMAKE_BUILD_TYPE:STRING=Debug", "Debug"),
                 ("drums", "CMAKE_BUILD_TYPE:STRING=RelWithDebInfo", "RelWithDebInfo"),
                 ("sampler", "CMAKE_CONFIGURATION_TYPES:STRING=Debug;Release", "Release"),
                 ("drums", "CMAKE_CONFIGURATION_TYPES:STRING=Debug;Release", "Release"),
                 ("sampler", "CMAKE_CONFIGURATION_TYPES:STRING=Debug", "Debug")]
        for name, cache, config in cases:
            with self.subTest(name=name, cache=cache):
                artifact = self.native_fixture(name)
                parts = artifact.relative_to(self.root / "build").parts
                configured = self.root / "build" / parts[0] / config / Path(*parts[1:])
                configured.parent.mkdir(parents=True, exist_ok=True)
                artifact.rename(configured)
                artifact.write_text('#!/usr/bin/env python3\nfrom pathlib import Path\nPath("stale.json").write_text("wrong app")\n')
                artifact.chmod(0o755)
                (self.root / "build/CMakeCache.txt").write_text(cache + '\n')
                commands = []
                with self.recorded_tools(commands), patch.object(sys, "platform", "darwin"):
                    code, _, error = self.invoke(name, "hello world")
                self.assertEqual(code, 0, error)
                self.assertEqual(commands[-1][0], [str(configured), "hello world"])
                self.assertEqual(commands[1][0][-2:], ["--config", config])
                self.assertEqual(json.loads((self.root / "launched.json").read_text()), [str(self.root), ["hello world"]])
                self.assertFalse((self.root / "stale.json").exists())

    def web_fixture(self, package="web/trigger"):
        directory = self.root / package
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "package.json").write_text('{"scripts":{"dev":"vite"}}')
        (directory / "package-lock.json").write_text('{}')
        return directory

    def test_native_app_bundle_and_exe_artifact_paths(self):
        for layout in ["Dandrum.app/Contents/MacOS/Dandrum", "Dandrum.exe"]:
            with self.subTest(layout=layout):
                artifact = self.native_fixture("tb303")
                moved = artifact.parent / layout
                moved.parent.mkdir(parents=True, exist_ok=True)
                artifact.rename(moved)
                commands = []
                with self.recorded_tools(commands):
                    code, _, error = self.invoke("tb303")
                self.assertEqual(code, 0, error)
                self.assertEqual(commands[-1][0], [str(moved)])
                moved.unlink()

    def test_browser_dependencies_and_argument_forwarding(self):
        package = self.web_fixture()
        commands = []
        with self.recorded_tools(commands):
            code, _, error = self.invoke("trigger", "--port", "9876")
        self.assertEqual(code, 0, error)
        self.assertEqual([cmd for cmd, _ in commands], [["npm", "ci"], ["npm", "run", "dev", "--", "--port", "9876"]])
        self.assertTrue(all(options["cwd"] == package for _, options in commands))
        (package / "node_modules").mkdir()
        (package / "node_modules/.package-lock.json").write_text('{}')
        commands.clear()
        with self.recorded_tools(commands):
            code, _, error = self.invoke("trigger")
        self.assertEqual(code, 0, error)
        self.assertEqual([cmd for cmd, _ in commands], [["npm", "run", "dev", "--"]])

    def test_embedded_react_uses_isolated_webview_build_and_required_dependency(self):
        self.native_fixture("react303", "build/demo-webview")
        self.web_fixture("web/tb303")
        sampler = self.web_fixture("web/sampler")
        commands = []
        with self.recorded_tools(commands):
            code, _, error = self.invoke("react303")
        self.assertEqual(code, 0, error)
        self.assertEqual(commands[0][0], ["npm", "ci"])
        self.assertEqual(commands[0][1]["cwd"], sampler)
        self.assertEqual(commands[1][0][1:], ["-S", str(self.root), "-B", str(self.root / "build/demo-webview"), "-DDANDRUM_NATIVE_ONLY=OFF"])
        self.assertEqual(commands[2][0][3:5], ["--target", "dandrum-plugin_Standalone"])

    def test_native_build_metadata_is_generic_and_skips_browser_dependencies(self):
        catalog = self.root / "scripts/demos.json"
        demos = json.loads(catalog.read_text())
        next(demo for demo in demos if demo["name"] == "drums").update(
            nativeOnly=True, buildDirectory="build/filter-ui-spikes",
            cmakeOptions=["DANDRUM_BUILD_FILTER_UI_SPIKES=ON", "CMAKE_BUILD_TYPE=Release"])
        catalog.write_text(json.dumps(demos))
        build = self.root / "build/filter-ui-spikes"
        artifact = self.native_fixture(build="build/filter-ui-spikes", configuration="Release")
        (build / "CMakeCache.txt").write_text("CMAKE_BUILD_TYPE:STRING=Release\n")
        default_cache = self.root / "build/CMakeCache.txt"
        default_cache.write_text("CMAKE_BUILD_TYPE:STRING=Debug\n")
        self.web_fixture("web/sampler")
        commands = []
        with self.recorded_tools(commands):
            code, _, error = self.invoke("drums", "--", "--input", "source with spaces.wav")
        self.assertEqual(code, 0, error)
        self.assertEqual(commands[0][0][1:], ["-S", str(self.root), "-B", str(build),
                         "-DDANDRUM_NATIVE_ONLY=ON", "-DDANDRUM_BUILD_FILTER_UI_SPIKES=ON", "-DCMAKE_BUILD_TYPE=Release"])
        self.assertEqual(commands[1][0][1:], ["--build", str(build), "--target", "dandrum-drum-machine-demo", "--config", "Release"])
        self.assertEqual(commands[2][0], [str(artifact), "--input", "source with spaces.wav"])
        self.assertEqual(json.loads((self.root / "launched.json").read_text()),
                         [str(self.root), ["--input", "source with spaces.wav"]])
        self.assertEqual(default_cache.read_text(), "CMAKE_BUILD_TYPE:STRING=Debug\n")

    def test_optional_build_metadata_preserves_react_flags_and_dependency_preparation(self):
        catalog = self.root / "scripts/demos.json"
        demos = json.loads(catalog.read_text())
        next(demo for demo in demos if demo["name"] == "react303").update(
            buildDirectory="build/custom-webview", cmakeOptions=["CMAKE_BUILD_TYPE=Debug"])
        catalog.write_text(json.dumps(demos))
        self.native_fixture("react303", "build/custom-webview", "Debug")
        self.web_fixture("web/sampler")
        (self.root / "build/custom-webview/CMakeCache.txt").write_text("CMAKE_BUILD_TYPE:STRING=Debug\n")
        commands = []
        with self.recorded_tools(commands):
            code, _, error = self.invoke("react303")
        self.assertEqual(code, 0, error)
        self.assertEqual(commands[0][0], ["npm", "ci"])
        self.assertEqual(commands[1][0][1:], ["-S", str(self.root), "-B", str(self.root / "build/custom-webview"),
                         "-DDANDRUM_NATIVE_ONLY=OFF", "-DCMAKE_BUILD_TYPE=Debug"])
        self.assertEqual(commands[2][0][-2:], ["--config", "Debug"])

    def test_filter_spikes_launch_their_registered_artifacts_and_forward_arguments(self):
        for name, product in [("filter-slint", "Dandrum Filter Slint"), ("filter-jive", "Dandrum Filter JIVE")]:
            with self.subTest(name=name):
                self.native_fixture(name, "build/filter-ui-spikes", "Release")
                build = self.root / "build/filter-ui-spikes"
                (build / "CMakeCache.txt").write_text("CMAKE_BUILD_TYPE:STRING=Release\n")
                commands = []
                with self.recorded_tools(commands):
                    code, _, error = self.invoke(name, "--", "argument with spaces")
                self.assertEqual(code, 0, error)
                expected = build / f"dandrum-{name}_artefacts/Release/Standalone" / product
                self.assertEqual(commands[-1][0], [str(expected), "argument with spaces"])
                self.assertEqual(json.loads((self.root / "launched.json").read_text()),
                                 [str(self.root), ["argument with spaces"]])

    def assert_native_failures(self, name, build="build", configuration=""):
        artifact = self.native_fixture(name, build, configuration)
        if configuration:
            (self.root / build / "CMakeCache.txt").write_text(f"CMAKE_BUILD_TYPE:STRING={configuration}\n")
        for fail_at, status in [("configure", 7), ("build", 8), ("app", 9), ("app", -15)]:
            commands = []

            def run(command, **options):
                if command[0] == "git":
                    return subprocess.CompletedProcess(command, 0, stdout="")
                commands.append(command)
                stage = "app" if command[0] == str(artifact) else "build" if "--build" in command else "configure"
                if stage == fail_at:
                    raise subprocess.CalledProcessError(status, command)
                return subprocess.CompletedProcess(command, 0)

            with self.subTest(stage=fail_at, status=status), patch("subprocess.run", side_effect=run):
                code, _, error = self.invoke(name)
                self.assertEqual(code, status if status > 0 else 128 - status)
                self.assertIn("failed", error.lower())
                self.assertEqual(len(commands), {"configure": 1, "build": 2, "app": 3}[fail_at])

    def test_preparation_and_child_failures_stop_and_preserve_exit_codes(self):
        self.assert_native_failures("drums")
        self.web_fixture()
        with patch("subprocess.run") as run:
            run.side_effect = [subprocess.CompletedProcess(["git"], 0, stdout=""), subprocess.CalledProcessError(17, ["npm", "ci"])]
            code, _, error = self.invoke("trigger")
            self.assertEqual(code, 17)
            self.assertIn("npm", error)
            self.assertEqual(run.call_count, 2)

    def test_native_only_failures_stop_before_launching_or_preserve_child_status(self):
        self.web_fixture("web/sampler")
        self.assert_native_failures("filter-slint", "build/filter-ui-spikes", "Release")

    def test_missing_command_artifact_and_interruption_are_actionable(self):
        artifact = self.native_fixture()
        with patch("subprocess.run") as run:
            run.side_effect = [subprocess.CompletedProcess(["git"], 0, stdout=""), FileNotFoundError("cmake is missing")]
            code, _, error = self.invoke("drums")
            self.assertEqual(code, 127)
            self.assertIn("cmake", error)
        artifact.unlink()
        with self.recorded_tools([]):
            code, _, error = self.invoke("drums")
        self.assertEqual(code, 1)
        self.assertIn("missing artifact", error)
        self.assertIn("dandrum-drum-machine-demo", error)
        with patch("subprocess.run", side_effect=KeyboardInterrupt):
            code, _, error = self.invoke("drums")
        self.assertEqual(code, 130)
        self.assertNotIn("Traceback", error)


if __name__ == "__main__":
    unittest.main()
