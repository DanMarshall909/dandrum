# Dandrum design system

This is the current design reference for Dandrum's native JUCE and WebView editors.
**Das Sampler** is the first product view. The supplied screens describe the visual
target; production behaviour comes from prepared instrument metadata and public
host parameters.

The user-supplied `Dandrum Design System (1).zip` was imported on 2 October 2026.
Its 161 files in [reference/](reference/) are preserved byte for byte.
[provenance.json](provenance.json) records the archive SHA-256, integrated base
commit and each file's SHA-256. Original export notes, including `github.md`,
describe the export's historical source rather than current integration status.
The bundled `SKILL.md` is a reference file, not an installed agent skill; bundled
tooling is retained as supplied and was not executed during import.

## Start here

- [Brand, interaction and component overview](reference/readme.md)
- [JUCE implementation guide](reference/handoff/juce-implementation.md)
- [Native component specifications](reference/handoff/component-specs.md)
- [Prepared KeyMap, LayerStack and OutputBusses specifications](native-display-specs.md)
- [Maintained production tokens and font provenance](../../ui/design-system/README.md)
- [CSS tokens](reference/tokens/colors.css) and [C++ token reference](reference/handoff/DandrumTokens.h)
- [Asset manifest](reference/handoff/asset-manifest.md) and [SVG icons](reference/assets/icons/manifest.json)
- [Main sampler](reference/ui_kits/das-sampler/index.html), [compact sampler](reference/ui_kits/das-sampler/compact.html), [slices](reference/ui_kits/das-sampler/slices.html), [modulation menu](reference/ui_kits/das-sampler/modulation-menu.html), [missing sample](reference/ui_kits/das-sampler/missing-asset.html) and [component reuse](reference/ui_kits/das-sampler/reuse.html)
- [Key map](reference/components/display/keymap.card.html), [layers and routing](reference/components/display/routing.card.html)
- [Maintained design brief](../sampler-juce-ui-design-prompt.md) and [renderer-independent UI plan](../../openspec/changes/add-renderer-independent-plugin-ui/design.md)

## Visual direction

| Role | Design rule |
| --- | --- |
| Surfaces | Warm brown ramp, `#130F0C` wells through `#524437` raised controls; flat fills |
| Text | Cream `#F2E6D3`; secondary `#CBB9A0`; Barlow Semi Condensed for labels |
| Selection and primary action | Ember `#E08A4E`; existing `vermilion` token names remain compatible |
| Live values | JetBrains Mono; knob values appear in an editable hover/focus/drag popup |
| Knobs | Pointer-free cap, 270-degree value arc, origin tick, at most two outer modulation rings |
| Modulation | Teal/lilac/lime/pink with A/B/C/D and distinct shapes; blue identifies host automation |
| Spacing | 2/4/6/8/12/16/24/32 logical pixels; 12 inside panels, 8 between controls |
| Shape and focus | Radii 2/4/6; 2-pixel cream focus ring with a 2-pixel gap |
| Layout | 1200 x 800 main, 820 x 560 compact; collapsible panels and prepared-detail rollouts |
| Feedback | Dynamic waveforms, activity, meters and markers; no baked labels, textures or background imagery |

For example, a prepared snare zone displays `vel 1–63`; its live level can display
`−3.0 dB`. The zone bounds require external patch editing and Reload Patch, while
level changes use the declared host parameter.

## Adaptation rules

These rules resolve differences between the illustrative export and the maintained
engine contract. They take precedence over suggestions in the preserved reference.

- Display the loaded kit's snare split at **1–63 / 64–127**, rather than the demo's
  1–95 / 96–127. Retain actual shared and per-pad control scopes.
- KeyMap shows actual key/velocity ranges and declared zone-selection semantics.
  Overlap or alternates do not establish simultaneous synth/sample/patch layering.
- LayerStack and OutputBusses show supported prepared sources, module chains,
  named host buses and channel counts. Hide or explain unavailable capabilities;
  do not fabricate the export's fixed output pairs or module chains.
- Zone bounds, module order, structural routing, voice limits and choke policy
  remain prepared settings. Disable structural edit callbacks; enable a live
  control only when it has a valid public parameter binding.
- In-plugin modulation assignment remains a proposed interaction. Host automation
  is DAW-owned; the editor cannot claim to inspect or edit arbitrary DAW sources.
- Use the pointer-free knob described by the current brand rules. The original
  native handoff still mentions a pointer and permanent value labels in places.
- Audio callbacks publish bounded data; they never draw, post messages, allocate,
  lock or wait for analysis. Use the shared typed services and timer-observed
  parameter binding in the UI plan when adapting the handoff's attachment examples.

## Reference and production packaging

The supplied HTML previews use CDN React/ReactDOM, runtime Babel and Google Fonts.
They need network access and a local HTTP server for browser-loaded JSX:

```bash
python3 -m http.server 8080 --directory docs/design-system/reference
# Open http://localhost:8080/ui_kits/das-sampler/index.html
```

The reference export includes no font binaries or licenses. Pinned unmodified
font files and license notices are now retained under
[ui/design-system/fonts](../../ui/design-system/fonts/); production renderers
still need to package them locally with compiled executable assets. The imported
CSS/C++ sheets remain reference outputs. The maintained production generator and
source live in [ui/design-system](../../ui/design-system/README.md), and the
[prepared display specifications](native-display-specs.md) supplement the
original native handoff. Renderer adaptations and offline runtime verification
remain implementation tasks in the UI plan.

This import was checked for ZIP integrity, byte identity and local HTML asset
links. It does not establish rendered preview quality, native build success or
plugin runtime behaviour.
