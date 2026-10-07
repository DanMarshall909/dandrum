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
    for kind, body in re.findall(r"^[ \t]*juce_add_(plugin|console_app)\((.*?)\)", (root / "CMakeLists.txt").read_text(), re.S | re.M):
        target = body.split()[0]
        if kind == "plugin" and not re.search(r'\bFORMATS\b[^"]*\bStandalone\b', body):
            continue
        if target not in targets:
            errors.append(f"unregistered native demo: {target}")
            continue
        product = re.search(r'PRODUCT_NAME\s+"([^"]+)"', body).group(1)
        expected = f"{target}_artefacts/Standalone/{product}" if kind == "plugin" else f"{target}_artefacts/{target}"
        for demo in demos:
            if demo.get("target", "").removesuffix("_Standalone") == target and demo["artifact"] != expected:
                errors.append(f"wrong artifact for {demo['name']}: expected {expected}")
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

    def test_guard_names_missing_native_and_browser_demos_and_wrong_artifacts(self):
        with tempfile.TemporaryDirectory(prefix="dandrum inventory ") as temporary:
            root = Path(temporary)
            (root / "scripts").mkdir()
            shutil.copy(ROOT / "scripts/demos.json", root / "scripts/demos.json")
            shutil.copy(ROOT / "CMakeLists.txt", root / "CMakeLists.txt")
            self.assertEqual(inventory_errors(root), [])
            original = (root / "CMakeLists.txt").read_text()
            (root / "CMakeLists.txt").write_text(original + '\njuce_add_console_app(new-demo PRODUCT_NAME "New Demo")\n')
            self.assertEqual(inventory_errors(root), ["unregistered native demo: new-demo"])
            (root / "CMakeLists.txt").write_text(original + '\njuce_add_plugin(new-plugin\n FORMATS\n  VST3 Standalone\n PRODUCT_NAME "New Plugin")\n')
            self.assertEqual(inventory_errors(root), ["unregistered native demo: new-plugin"])
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
