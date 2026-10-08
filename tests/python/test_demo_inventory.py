"""Catch catalog drift when independently launchable Dandrum demos change."""
import json
import re
import shutil
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def inventory_errors(root):
    demos = json.loads((root / "scripts/demos.json").read_text())
    errors = []
    names = [demo["name"] for demo in demos]
    if len(set(names)) != len(names):
        errors.append("duplicate demo names")
    targets = {demo["target"].removesuffix("_Standalone"): demo for demo in demos if demo["kind"] == "native"}
    for kind, body in re.findall(r"^[ \t]*juce_add_(plugin|console_app|gui_app)\((.*?)\)", (root / "CMakeLists.txt").read_text(), re.S | re.M):
        target = body.split()[0]
        if kind == "plugin" and not re.search(r'\bFORMATS\b[^"]*\bStandalone\b', body):
            continue
        if target not in targets:
            errors.append(f"unregistered native demo: {target}")
            continue
        product = re.search(r'PRODUCT_NAME\s+"([^"]+)"', body).group(1)
        expected = f"{target}_artefacts/Standalone/{product}" if kind == "plugin" else f"{target}_artefacts/{product if kind == 'gui_app' else target}"
        for demo in demos:
            if demo.get("target", "").removesuffix("_Standalone") == target and demo["artifact"] != expected:
                errors.append(f"wrong artifact for {demo['name']}: expected {expected}")
    # Plain CMake UI executables need catalog entries too. Test/smoke programs
    # whose declared sources live under tests/ are verification targets, not demos.
    for body in re.findall(r"^[ \t]*add_executable\((.*?)\)", (root / "CMakeLists.txt").read_text(), re.S | re.M):
        if not body.split():
            continue
        target = body.split()[0]
        if re.search(r"\b(?:ALIAS|IMPORTED)\b|(?:^|[\s/])tests/", body):
            continue
        if target not in targets:
            errors.append(f"unregistered native demo: {target}")
    sources = {demo["source"] for demo in demos if demo["kind"] == "web"}
    for manifest in sorted((root / "web").glob("*/package.json")):
        if "dev" in json.loads(manifest.read_text()).get("scripts", {}):
            source = manifest.relative_to(root).as_posix()
            if source not in sources:
                errors.append(f"unregistered browser demo: {source}")
    return errors


class DemoInventoryTest(unittest.TestCase):
    def test_repository_demo_inventory_is_registered(self):
        self.assertEqual(inventory_errors(ROOT), [], "Update scripts/demos.json when demos change")

    def test_filter_spikes_register_isolated_native_only_standalone_demos(self):
        demos = {demo["name"]: demo for demo in json.loads((ROOT / "scripts/demos.json").read_text())}
        for name, product in [("filter-slint", "Dandrum Filter Slint"), ("filter-jive", "Dandrum Filter JIVE")]:
            with self.subTest(name=name):
                self.assertIn(name, demos)
                demo = demos[name]
                target = f"dandrum-{name}"
                self.assertEqual(demo["kind"], "native")
                self.assertEqual(demo["source"], "spikes/filter-ui/README.md")
                self.assertEqual(demo["target"], f"{target}_Standalone")
                self.assertEqual(demo["artifact"], f"{target}_artefacts/Standalone/{product}")
                self.assertTrue(demo["nativeOnly"])
                self.assertEqual(demo["buildDirectory"], "build/filter-ui-spikes")
                self.assertEqual(demo["cmakeOptions"], ["DANDRUM_BUILD_FILTER_UI_SPIKES=ON", "CMAKE_BUILD_TYPE=Release"])

    def test_trigger_slint_registers_silent_isolated_native_gui_preview(self):
        demos = {demo["name"]: demo for demo in json.loads((ROOT / "scripts/demos.json").read_text())}
        self.assertIn("trigger-slint", demos)
        demo = demos["trigger-slint"]
        self.assertIn("silent macro preview", demo["description"])
        self.assertEqual(demo["kind"], "native")
        self.assertEqual(demo["source"], "spikes/trigger-slint/README.md")
        self.assertEqual(demo["target"], "dandrum-trigger-slint")
        self.assertEqual(demo["artifact"], "dandrum-trigger-slint_artefacts/Dandrum Trigger Slint")
        self.assertTrue(demo["nativeOnly"])
        self.assertEqual(demo["buildDirectory"], "build/filter-ui-spikes")
        self.assertEqual(demo["cmakeOptions"], ["DANDRUM_BUILD_TRIGGER_SLINT=ON", "CMAKE_BUILD_TYPE=Release"])

    def test_slint_library_registers_a_silent_isolated_plain_cmake_catalog(self):
        demos = {demo["name"]: demo for demo in json.loads((ROOT / "scripts/demos.json").read_text())}
        self.assertIn("slint-library", demos)
        demo = demos["slint-library"]
        self.assertIn("silent UI catalog", demo["description"])
        self.assertEqual(demo["kind"], "native")
        self.assertEqual(demo["source"], "ui/slint/README.md")
        self.assertEqual(demo["target"], "dandrum-slint-catalog")
        self.assertEqual(demo["artifact"], "slint-catalog/dandrum-slint-catalog")
        self.assertTrue(demo["nativeOnly"])
        self.assertEqual(demo["buildDirectory"], "build/slint-library")
        self.assertEqual(demo["cmakeOptions"], ["DANDRUM_SLINT_LIBRARY_ONLY=ON", "CMAKE_BUILD_TYPE=Release"])
        self.assertIsNotNone(re.search(r"add_executable\(dandrum-slint-catalog\)", (ROOT / "CMakeLists.txt").read_text()), "Root CMake must declare the catalog target")

    def test_guard_names_missing_native_and_browser_demos_and_wrong_artifacts(self):
        with tempfile.TemporaryDirectory(prefix="dandrum inventory ") as temporary:
            root = Path(temporary)
            (root / "scripts").mkdir()
            shutil.copy(ROOT / "scripts/demos.json", root / "scripts/demos.json")
            shutil.copy(ROOT / "CMakeLists.txt", root / "CMakeLists.txt")
            self.assertEqual(inventory_errors(root), [])
            original = (root / "CMakeLists.txt").read_text()
            (root / "CMakeLists.txt").write_text(original + '\nadd_executable(new-native-demo main.cpp)\n')
            self.assertEqual(inventory_errors(root), ["unregistered native demo: new-native-demo"])
            (root / "CMakeLists.txt").write_text(original + '\nadd_executable(new-unit-test tests/cpp/new.cpp)\n')
            self.assertEqual(inventory_errors(root), [])
            (root / "CMakeLists.txt").write_text(original + '\njuce_add_console_app(new-demo PRODUCT_NAME "New Demo")\n')
            self.assertEqual(inventory_errors(root), ["unregistered native demo: new-demo"])
            (root / "CMakeLists.txt").write_text(original + '\njuce_add_plugin(new-plugin\n FORMATS\n  VST3 Standalone\n PRODUCT_NAME "New Plugin")\n')
            self.assertEqual(inventory_errors(root), ["unregistered native demo: new-plugin"])
            (root / "CMakeLists.txt").write_text(original + '\njuce_add_gui_app(new-gui PRODUCT_NAME "New GUI")\n')
            self.assertEqual(inventory_errors(root), ["unregistered native demo: new-gui"])
            gui = original
            if not re.search(r'juce_add_gui_app\(\s*dandrum-trigger-slint\s', original):
                gui += '\njuce_add_gui_app(dandrum-trigger-slint PRODUCT_NAME "Dandrum Trigger Slint")\n'
            (root / "CMakeLists.txt").write_text(gui)
            self.assertEqual(inventory_errors(root), [])
            catalog = root / "scripts/demos.json"
            demos = json.loads(catalog.read_text())
            next(demo for demo in demos if demo["name"] == "trigger-slint")["artifact"] = "stale/gui"
            catalog.write_text(json.dumps(demos))
            self.assertEqual(inventory_errors(root), ["wrong artifact for trigger-slint: expected dandrum-trigger-slint_artefacts/Dandrum Trigger Slint"])
            shutil.copy(ROOT / "scripts/demos.json", catalog)
            (root / "CMakeLists.txt").write_text(original)
            package = root / "web/new-preview/package.json"
            package.parent.mkdir(parents=True)
            package.write_text('{"scripts":{"dev":"vite"}}')
            self.assertEqual(inventory_errors(root), ["unregistered browser demo: web/new-preview/package.json"])
            package.unlink()
            demos = json.loads((root / "scripts/demos.json").read_text())
            demos[0]["artifact"] = "stale/path"
            (root / "scripts/demos.json").write_text(json.dumps(demos))
            self.assertEqual(inventory_errors(root), ["wrong artifact for tb303: expected dandrum-plugin_artefacts/Standalone/Dandrum"])


if __name__ == "__main__":
    unittest.main()
