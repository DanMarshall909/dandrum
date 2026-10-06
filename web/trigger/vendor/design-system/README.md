This module contains the compiled React primitives supplied in
`Advanced Sampler App Design.zip`. The immutable input remains in
`docs/trigger/reference/_ds/dandrum-design-system-3c2eabed-50f8-48be-a226-2fe7d8a388d3/_ds_bundle.js`.

`primitives.mjs` adds an ESM import/export boundary around the received primitive
sections. It excludes the Design Components runtime and unrelated SamplerApp.
The immutable input has not been modified. Authored wrappers provide precision,
unit parsing, gesture cleanup, numeric display precision and in-editor menus.

Compatibility changes in this compiled copy:
- Toggle's switch receives its visible label as an accessible name.
- KeyMap accepts current controlled zones after initial mount.
- LayerStack module containers expose `data-module-id` for selection/drag identity.
- LayerStack typed parameters parse their actual units, including kHz; parameter
  IDs identify context/modulation targets.
- `installControlAdapters` directs controls inside composed received widgets to
  the same authored Knob/NumericField/Toggle/MenuButton wrappers used elsewhere.
  Original drawing exports remain available to those wrappers.

The user's 200-line component preference applies to authored components; this
received compiled library retains its original structure. Styling uses the
supplied tokens plus named swatches for the source-specific velocity diagram and
window border/shadow. Fonts retain their OFL notices and provenance.
