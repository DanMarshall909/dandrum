"""The opt-in Trigger CLAP product installs with its sibling resources and notices."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class TriggerClapInstallTest(unittest.TestCase):
    def test_install_places_exact_plugin_resources_and_notices(self):
        with tempfile.TemporaryDirectory(prefix="trigger-clap-install-") as scratch:
            root = Path(scratch)
            bundle = root / "bundle"
            bundle.mkdir()
            for name in ("index.html", "app.js", "app.css"):
                (bundle / name).write_bytes(("fixture:" + name).encode())
            build = root / "build"
            configured = subprocess.run(
                ["cmake", "-S", str(ROOT), "-B", str(build),
                 "-DDANDRUM_ENGINE_ONLY=ON", "-DDANDRUM_CLAP_PROOF=ON",
                 "-DDANDRUM_TRIGGER_CLAP_EDITOR=ON",
                 f"-DDANDRUM_TRIGGER_EDITOR_BUNDLE_DIR={bundle}"],
                capture_output=True, text=True,
            )
            self.assertEqual(configured.returncode, 0, configured.stdout + configured.stderr)
            product = build / "clap/DandrumTrigger.clap"
            product.parent.mkdir(parents=True, exist_ok=True)
            product.write_bytes(b"fixture-clap-binary")
            resources = build / "clap/DandrumTrigger.resources"
            (resources / "editor").mkdir(parents=True)
            (resources / "editor/app.js").write_bytes(b"fixture-editor-script")
            (resources / "instrument").mkdir()
            (resources / "instrument/advanced-drum-kit.yaml").write_bytes(b"fixture-patch")
            prefix = root / "installed"
            installed = subprocess.run(
                ["cmake", "--install", str(build), "--prefix", str(prefix),
                 "--component", "trigger-clap"],
                capture_output=True, text=True,
            )
            self.assertEqual(installed.returncode, 0, installed.stdout + installed.stderr)
            self.assertEqual((prefix / "lib/clap/DandrumTrigger.clap").read_bytes(),
                             product.read_bytes())
            self.assertEqual((prefix / "lib/clap/DandrumTrigger.resources/editor/app.js").read_bytes(),
                             b"fixture-editor-script")
            self.assertEqual((prefix / "lib/clap/DandrumTrigger.resources/instrument/advanced-drum-kit.yaml").read_bytes(),
                             b"fixture-patch")
            notices = prefix / "share/doc/dandrum-trigger/THIRD_PARTY_NOTICES.json"
            self.assertTrue(notices.is_file(), "installed product must carry pinned notices")

    def test_notices_pin_the_current_runtime_dependency_and_font_inputs(self):
        notices = json.loads((ROOT / "docs/migrations/trigger-clap-beta-notices.json").read_text())
        digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
        self.assertEqual(notices["inputs"]["cargo_lock_sha256"],
                         digest(ROOT / "src/clap-adapter/Cargo.lock"))
        self.assertEqual(notices["inputs"]["package_lock_sha256"],
                         digest(ROOT / "web/trigger/package-lock.json"))
        tree = subprocess.run(
            ["cargo", "tree", "--offline", "--locked", "--manifest-path",
             str(ROOT / "src/clap-adapter/Cargo.toml"), "--features", "trigger-editor",
             "--edges", "normal,build", "--target", "x86_64-unknown-linux-gnu",
             "--prefix", "none"], capture_output=True, text=True,
        )
        self.assertEqual(tree.returncode, 0, tree.stdout + tree.stderr)
        actual_rust = {tuple(match.groups()) for line in tree.stdout.splitlines()
                       if (match := re.match(r"^([^ ]+) v([^ ]+)", line))}
        pinned_rust = {(item["name"], item["version"])
                       for item in notices["rust_packages"]}
        self.assertEqual(pinned_rust, actual_rust)
        for item in notices["rust_packages"]:
            if item["source"] != "first-party":
                self.assertTrue(item["license_declaration"])
        npm = json.loads((ROOT / "web/trigger/package-lock.json").read_text())
        actual_js = {(path.removeprefix("node_modules/"), item["version"])
                     for path, item in npm["packages"].items()
                     if path and not item.get("dev")}
        pinned_js = {(item["name"], item["version"])
                     for item in notices["javascript_runtime_packages"]}
        self.assertEqual(pinned_js, actual_js)
        for item in notices["bundled_font_notices"]:
            self.assertEqual(item["sha256"],
                             digest(ROOT / "web/trigger/public" / item["path"].removeprefix("editor/")))


if __name__ == "__main__":
    unittest.main()
