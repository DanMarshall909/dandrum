# Prepared display components

Implementation contract for `KeyMap`, `LayerStack` and `OutputBusses`, supplementing
the preserved [component handoff](reference/handoff/component-specs.md) and
[maintained adaptation rules](README.md). Dimensions are logical pixels at 100%.
These are specifications; the component implementations remain OpenSpec tasks 7.1–7.3.

## Common presentation and lifetime

Use generated semantic tokens from `ui/design-system/tokens.json`. Panels use a
28-pixel header (24 compact), 12-pixel body padding (8 compact), 4-pixel seams,
6-pixel radius and warm brown surfaces. The cream focus outline is 2 pixels with
a 2-pixel gap. Text must be at least 11 pixels, use embedded Barlow families for
labels/titles and JetBrains Mono for values. Allow horizontal/vertical scrolling
inside panels when required; keep controls accessible at 1200×800 and 820×560.

Each display holds an owned prepared document or copied view model. Never retain
pointers into replaced engine storage. A new instrument generation clears
selection, releases editor-owned notes and retires old analysis/telemetry.
Visual state (selection, zoom, collapsed panels) is editor state, not a host
parameter. Every live control identifies its declared parameter and generation.

Empty, unavailable and preparing states are distinct. Missing capability means
"Unavailable for this instrument", not an empty but editable authoring surface.
Do not fabricate mappings, sources, module chains, route feeds or output pairs.

## KeyMap

Reference: [JSX](reference/components/display/KeyMap.jsx) and
[types](reference/components/display/KeyMap.d.ts).

- Axis gutter: 30 pixels, pinned while scrolling. Key/velocity grid: 140 pixels
  high at full size, 100 compact. Keyboard: 56 pixels full, 40 compact. Full-size
  heights match the supplied JSX; compact heights are an explicit adaptation.
- Fit actual prepared keys by default. Zoom is bounded from 1× to 8×; controls
  `−`, `+`, `Fit` are at least 24 pixels in each interactive dimension. MIDI key
  ranges are inclusive; velocity 127 is at the top, 1 at the bottom. Selection
  outlines/keyboard rails use ember; root-key markers require declared root data.
- Das Sampler presents the actual 1–63/64–127 snare split. Group round-robin or
  weighted alternatives with their selection policy. Overlapping rectangles do
  not establish simultaneous layers. Replace the export's unconditional
  "LAYERED"/"all trigger" language with prepared selection semantics.
- Tab focuses zones and zoom controls; Enter/Space selects, then auditions a
  mapped key at a velocity inside the selected zone. The piano supports pointer
  audition and keyboard release. Escape, lost pointer capture, focus loss,
  hide/close and reload release editor notes through the shared session service.
- Prepared mode never changes key, velocity or root bounds: no drag handles,
  resize cursor, arrow nudge, Shift resize or `onChange` structural callback.
  Arrows may navigate selection without altering a zone.
- Local press/hover describes input intent. Playback highlights/alternate and
  cursor indicators require observed host/editor telemetry; hide them with an
  explicit capability state if absent. Remove the reference's timed local flash.

## LayerStack

Reference: [JSX](reference/components/display/LayerStack.jsx) and
[types](reference/components/display/LayerStack.d.ts).

- The header/count line is at least 18 pixels. Source and module blocks are
  38 pixels high, with 8-pixel horizontal connectors. Source width is 190–280
  pixels; module minimum width is 88. Rows scroll horizontally when needed.
  At compact size, details open below the row rather than shrinking labels.
- A source block shows actual source type, identity and prepared summary.
  Selecting it opens copied source/region details and any supported public
  controls. Module blocks appear only when module-chain metadata is supplied.
- Enter/Space opens the focused source/module details; Escape closes them and
  restores focus. Selection is possible even when the data is read-only.
- Add/remove/reorder controls are absent. Gain, bypass, mute, sends and output
  menus are absent or read-only unless individually backed by a public binding.
  Omitting `onChange` is insufficient: the reference mutates local state for
  bypass, level and mute even with `editable=false`. Production must gate those
  handlers and expose no unsupported mutable state.
- Round-robin alternatives are an "Alternatives" list, not simultaneous layers.
  Missing synth/nested-patch/module-chain capability is labeled unavailable.
  Source selection must not create an engine or load an invented module.

## OutputBusses

Reference: [JSX](reference/components/display/OutputBusses.jsx) and
[types](reference/components/display/OutputBusses.d.ts).

- Row labels use at least 11 pixels; rows have 6-pixel vertical/8-pixel horizontal
  padding and a 4-pixel radius. Full layout shows bus, channels, feeds and
  per-channel meters; compact wraps feeds/measurement rows instead of clipping.
  Channel summaries need at least 84 pixels; horizontal meters use 120×4 pixels
  per channel where space permits. Each interactive control has a 24-pixel hit area.
- Enumerate actual named buses and channel counts. A stereo master can say
  "2 channels · L/R"; a mono or six-channel bus must show its actual layout.
  Do not use the export's default 1/2…15/16 channel-options list or assume stereo.
- Feeds appear only when routing metadata exists; otherwise say "Feed details
  unavailable". An absent feed list does not establish "Nothing routed".
- Channels/routes are prepared and read-only. Remove the reference's channel
  menu and local mute/level mutation unless an actual public binding authorizes
  that control. Tab focuses inspection and any supported clip acknowledgements.
- Measurements identify bus, channel and generation. Waiting/unavailable/gapped
  measurements differ from silence. Clip acknowledgements use shared bounded
  commands and affect only the measured channel/generation. Never simulate levels.

## Handoff reconciliation

Keep the reference byte-preserved. The production knob has no pointer and uses
an editable hover/focus/drag value popup, as the current brand and JSX specify.
Use the CSS menu width of 240 pixels as the maintained shared token; the old
C++ reference's 248 is not a second authority. Native parameter bindings use
shared begin/update/end commands and timer-observed values. The original handoff's
attachment examples do not permit audio-originated posting, locks or drawing.
