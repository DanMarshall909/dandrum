import json
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
BUNDLE = ROOT / "docs/design-system/advanced-sampler/reference/_ds/dandrum-design-system-3c2eabed-50f8-48be-a226-2fe7d8a388d3/_ds_bundle.js"
HEADER = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">'


class AdvancedSamplerIcons(unittest.TestCase):
    def test_all_guide_icons_keep_exact_imported_geometry(self):
        paths = json.loads(re.search(r"const ICON_PATHS = (\{.*?\n\});", BUNDLE.read_text(), re.S)[1])
        icon = (ROOT / "ui/slint/editor/Icon.slint").read_text()
        for name, geometry in paths.items():
            with self.subTest(name=name):
                self.assertEqual((ROOT / f"ui/slint/editor/assets/{name}.svg").read_text(), HEADER + geometry + "</svg>\n")
                self.assertIn(f'@image-url("assets/{name}.svg")', icon)

    def test_application_aliases_resolve_to_explicit_guide_shapes(self):
        icon = (ROOT / "ui/advanced-sampler/controls/AppIcon.slint").read_text()
        tokens = json.loads((ROOT / "ui/design-system/tokens.json").read_text())["tokens"]
        for alias, guide in {"grid": "tap", "sliders": "settings", "waveform": "one-shot", "scissors": "slice", "route": "pan", "list": "rows-more", "edit": "settings"}.items():
            match = re.search(rf'name == "{alias}" \? Tokens\.([\w-]+)', icon)
            self.assertIsNotNone(match, alias)
            self.assertEqual(tokens[match[1]], {"kind": "text", "value": guide})
        self.assertIn('inherits EditorIcon', icon)
        self.assertIn('resolved-name: SamplerIconNames.resolve(root.name)', icon)
        self.assertNotIn('@image-url', icon)


if __name__ == "__main__":
    unittest.main()
