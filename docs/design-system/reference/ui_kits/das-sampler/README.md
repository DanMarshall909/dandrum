# Das Sampler UI kit

Interactive HTML recreation of the proposed Das Sampler editor (design target, not shipped).

- `index.html` — main 1200 × 800. Click pads (height = velocity), switch regions, drag knobs, right-click a knob (or focus + M) for the modulation menu, Assign modulation… → pick a source → click highlighted controls → Esc.
- `compact.html` — 820 × 560 compact layout.
- `modulation-menu.html` — menu open on Pitch ratio while the demo pattern plays.
- `slices.html` — sliced-break patch with slice index.
- `missing-asset.html` — missing sample banner + Reload Patch.
- `reuse.html` — same components on example synth / effect panels.

Mockup controls under the window (demo pattern, host automation, missing sample, patch type) are not part of the plugin.

Files: `data.js` (mock patch), `Header.jsx`, `PadGrid.jsx`, `WaveView.jsx`, `LiveControls.jsx` (+ modulation menu builder), `PadDetails.jsx`, `App.jsx`.
