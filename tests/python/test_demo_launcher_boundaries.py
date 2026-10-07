"""Real Git/CMake/npm contracts; separate from the fast launcher tests."""
import contextlib
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import demo_launcher as launcher


class DemoLauncherBoundaryTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="dandrum demo boundary ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "source checkout"
        (self.root / "scripts").mkdir(parents=True)
        shutil.copy(ROOT / "scripts/demos.json", self.root / "scripts/demos.json")

    def invoke(self, *args):
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            status = launcher.main(list(args), self.root)
        return status, stdout.getvalue(), stderr.getvalue()

    def test_executable_lists_demos_from_another_current_directory(self):
        result = subprocess.run([str(ROOT / "demo"), "--list"], cwd=self.root, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("react303", result.stdout)
        self.assertIn("silent mock engine", result.stdout)

    def test_real_cmake_build_launch_and_configure_failure(self):
        (self.root / "CMakeLists.txt").write_text('''cmake_minimum_required(VERSION 3.22)
project(LauncherBoundary LANGUAGES NONE)
option(DANDRUM_NATIVE_ONLY "Disable browser dependencies" OFF)
option(DANDRUM_BUILD_FILTER_UI_SPIKES "Build optional filter experiments" OFF)
function(juce_add_console_app target)
  add_custom_target(${target}
    COMMAND ${CMAKE_COMMAND} -E make_directory "${CMAKE_BINARY_DIR}/${target}_artefacts/$<CONFIG>"
    COMMAND ${CMAKE_COMMAND} -E copy "${CMAKE_SOURCE_DIR}/app.py" "${CMAKE_BINARY_DIR}/${target}_artefacts/$<CONFIG>/${target}")
endfunction()
function(juce_add_plugin target)
  add_custom_target(${target}_Standalone
    COMMAND ${CMAKE_COMMAND} -E make_directory "${CMAKE_BINARY_DIR}/${target}_artefacts/$<CONFIG>/Standalone"
    COMMAND ${CMAKE_COMMAND} -E copy "${CMAKE_SOURCE_DIR}/app.py" "${CMAKE_BINARY_DIR}/${target}_artefacts/$<CONFIG>/Standalone/Dandrum")
endfunction()
juce_add_console_app(dandrum-drum-machine-demo PRODUCT_NAME "Drums")
juce_add_plugin(dandrum-plugin FORMATS Standalone PRODUCT_NAME "Dandrum")
''')
        app = self.root / "app.py"
        app.write_text('#!/usr/bin/env python3\nimport json, os, sys\nfrom pathlib import Path\nPath("observed.json").write_text(json.dumps([os.getcwd(), sys.argv[1:]]))\n')
        app.chmod(0o755)
        status, _, error = self.invoke("drums", "--", "--patch", "path with spaces.yaml")
        self.assertEqual(status, 0, error)
        observed = self.root / "observed.json"
        self.assertEqual(json.loads(observed.read_text()), [str(self.root), ["--patch", "path with spaces.yaml"]])
        cmake = Path.home() / ".local/bin/cmake"
        cmake = str(cmake) if cmake.is_file() else "cmake"
        subprocess.run([cmake, "-S", str(self.root), "-B", str(self.root / "build"), "-DCMAKE_BUILD_TYPE=Release"], check=True)
        stale = self.root / "build/dandrum-plugin_artefacts/Standalone/Dandrum"
        stale.parent.mkdir(parents=True)
        stale.write_text('#!/usr/bin/env python3\nfrom pathlib import Path\nPath("stale.json").write_text("wrong app")\n')
        stale.chmod(0o755)
        status, _, error = self.invoke("tb303", "configured plugin")
        self.assertEqual(status, 0, error)
        self.assertEqual(json.loads(observed.read_text()), [str(self.root), ["configured plugin"]])
        self.assertFalse((self.root / "stale.json").exists())
        status, _, error = self.invoke("drums", "configured console")
        self.assertEqual(status, 0, error)
        self.assertEqual(json.loads(observed.read_text()), [str(self.root), ["configured console"]])
        default_cache = self.root / "build/CMakeCache.txt"
        preserved_cache = default_cache.read_text()
        catalog = self.root / "scripts/demos.json"
        demos = json.loads(catalog.read_text())
        next(demo for demo in demos if demo["name"] == "drums").update(
            nativeOnly=True, buildDirectory="build/filter-ui-spikes",
            cmakeOptions=["DANDRUM_BUILD_FILTER_UI_SPIKES=ON", "CMAKE_BUILD_TYPE=Release"])
        catalog.write_text(json.dumps(demos))
        sampler = self.root / "web/sampler"
        sampler.mkdir(parents=True)
        (sampler / "package.json").write_text('{}')
        status, _, error = self.invoke("drums", "--", "native only argument")
        self.assertEqual(status, 0, error)
        self.assertEqual(json.loads(observed.read_text()), [str(self.root), ["native only argument"]])
        isolated = self.root / "build/filter-ui-spikes"
        cache = (isolated / "CMakeCache.txt").read_text()
        self.assertIn("DANDRUM_NATIVE_ONLY:BOOL=ON\n", cache)
        self.assertIn("DANDRUM_BUILD_FILTER_UI_SPIKES:BOOL=ON\n", cache)
        self.assertRegex(cache, r"(?m)^CMAKE_BUILD_TYPE:[^=]+=Release$")
        self.assertTrue((isolated / "dandrum-drum-machine-demo_artefacts/Release/dandrum-drum-machine-demo").is_file())
        self.assertEqual(default_cache.read_text(), preserved_cache)
        self.assertFalse((sampler / "node_modules").exists())
        observed.unlink()
        with (self.root / "CMakeLists.txt").open("a") as source:
            source.write('\nmessage(FATAL_ERROR "injected configure failure")\n')
        status, _, error = self.invoke("drums")
        self.assertNotEqual(status, 0)
        self.assertIn("failed", error)
        self.assertFalse(observed.exists(), "Configuration failure must prevent launching an existing artifact")

    def test_artifact_layout_is_calibrated_against_vendored_juce_target_files(self):
        juce = (ROOT / "third_party/JUCE").as_posix()
        (self.root / "CMakeLists.txt").write_text('''cmake_minimum_required(VERSION 3.22)
project(JuceLayoutBoundary VERSION 0.1.0 LANGUAGES C CXX)
set(JUCE_BUILD_HELPER_TOOLS ON CACHE BOOL "" FORCE)
add_subdirectory("''' + juce + '''" JUCE)
if(NOT TARGET juce::juceaide)
  add_executable(juce::juceaide ALIAS juceaide)
endif()
juce_add_console_app(dandrum-drum-machine-demo PRODUCT_NAME "Drums")
target_sources(dandrum-drum-machine-demo PRIVATE empty.cpp)
juce_add_plugin(dandrum-plugin FORMATS Standalone PRODUCT_NAME "Dandrum")
target_sources(dandrum-plugin PRIVATE empty.cpp)
juce_add_plugin(dandrum-filter-slint FORMATS VST3 Standalone PRODUCT_NAME "Dandrum Filter Slint")
target_sources(dandrum-filter-slint PRIVATE empty.cpp)
juce_add_plugin(dandrum-filter-jive FORMATS VST3 Standalone PRODUCT_NAME "Dandrum Filter JIVE")
target_sources(dandrum-filter-jive PRIVATE empty.cpp)
file(GENERATE OUTPUT "${CMAKE_BINARY_DIR}/paths-$<CONFIG>.txt"
  CONTENT "$<TARGET_FILE:dandrum-drum-machine-demo>\\n$<TARGET_FILE:dandrum-plugin_Standalone>\\n$<TARGET_FILE:dandrum-filter-slint_Standalone>\\n$<TARGET_FILE:dandrum-filter-jive_Standalone>\\n")
''')
        (self.root / "empty.cpp").write_text('int main() { return 0; }\n')
        cmake = Path.home() / ".local/bin/cmake"
        cmake = str(cmake) if cmake.is_file() else "cmake"
        env = dict(os.environ)
        if sys.platform.startswith("linux"):
            env["PATH"] = "/usr/bin:/bin:" + env.get("PATH", "")
        build = self.root / "build"
        for config in ["", "Debug", "Release"]:
            subprocess.run([cmake, "-S", str(self.root), "-B", str(build), f"-DCMAKE_BUILD_TYPE={config}"], env=env, check=True)
            observed = (build / f"paths-{config}.txt").read_text().splitlines()
            self.assertEqual(observed, [str(build / "dandrum-drum-machine-demo_artefacts" / config / "dandrum-drum-machine-demo"),
                                        str(build / "dandrum-plugin_artefacts" / config / "Standalone/Dandrum"),
                                        str(build / "dandrum-filter-slint_artefacts" / config / "Standalone/Dandrum Filter Slint"),
                                        str(build / "dandrum-filter-jive_artefacts" / config / "Standalone/Dandrum Filter JIVE")])

    def test_real_git_worktree_and_npm_launch_then_install_failure(self):
        def git(*args):
            subprocess.run(["git", "-C", str(self.root), *args], check=True, capture_output=True)

        (self.root / "src/rust-engine").mkdir(parents=True)
        (self.root / "src/rust-engine/Cargo.toml").write_text('[package]\nname="boundary"\n')
        git("init", "-q")
        git("add", ".")
        git("-c", "user.name=Launcher Test", "-c", "user.email=launcher@example.invalid", "commit", "-qm", "Fixture")
        other = Path(self.temp.name) / "registered worktree with spaces"
        git("worktree", "add", "--detach", str(other))
        package = other / "web/trigger"
        package.mkdir(parents=True)
        manifest = {"name": "dandrum-launcher-boundary", "version": "1.0.0", "private": True, "scripts": {"dev": "node run.cjs"}}
        (package / "package.json").write_text(json.dumps(manifest))
        lock = package / "package-lock.json"
        lock.write_text(json.dumps({"name": manifest["name"], "version": "1.0.0", "lockfileVersion": 3, "packages": {"": {"name": manifest["name"], "version": "1.0.0"}}}))
        (package / "run.cjs").write_text('require("fs").writeFileSync("observed.json", JSON.stringify([process.cwd(), process.argv.slice(2)]));')
        status, output, error = self.invoke("trigger", "--port", "4321", "argument with spaces")
        self.assertEqual(status, 0, error)
        self.assertIn(str(other), output)
        observed = package / "observed.json"
        self.assertEqual(json.loads(observed.read_text()), [str(package), ["--port", "4321", "argument with spaces"]])
        observed.unlink()
        lock.write_text("invalid package lock")
        status, _, error = self.invoke("trigger")
        self.assertNotEqual(status, 0)
        self.assertIn("npm", error)
        self.assertFalse(observed.exists(), "Failed dependency preparation must prevent dev launch")


if __name__ == "__main__":
    unittest.main()
