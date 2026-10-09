# INTERACTIONS — actions, state transitions, gestures, keys

Status column: **D** = behaviour defined by the prototype; **Q-n** = open question in `IMPLEMENTATION.md` § 5; **R** = requires an engine capability that does not exist yet (see `INTEGRATION.md`).

## 1. Global

| Input | Result | Status |
|---|---|---|
| Ctrl+Z / Ctrl+Shift+Z, Ctrl+Y, header Undo/Redo | Undo/redo last edit; tooltip names it; drags within 600 ms merge | D, Q-2 (what is undoable once edits are prepared) |
| Ctrl+1…8 | Switch page | D |
| Esc | Close menu / popover / cancel drag / clear to parent selection | D |
| Delete, Ctrl+D | Delete / duplicate selection (context-dependent) | D, R |
| Middle-click or Alt-click on a control | Reset to default | D |
| Right-click on a control | Control context menu | D |
| Space | Audition selection | D |
| Size switch Min/Def/Exp | Re-layout to 820×560 / 1200×800 / 1600×1000 | D (host resize behaviour: Q-6) |
| Performance view / Open editor | Swap to/from Perform view (860×610) | D, Q-6 |
| "Why did this play?" | Open Diagnostics panel | D, R |

## 2. Header — Patch

| Input | Result | Status |
|---|---|---|
| Click Patch box | Open patch browser popover under the box | D |
| Right-click Patch box | Menu: Import definition…, Export definition…, Copy definition to clipboard, Reload Patch | D, Q-4 (wording "YAML") |
| Browser: type in search | Filter list by name, type, contents | D |
| Browser: category | Filter (All, Drums, Keys, Hybrid, Synth, User, Favourites) | D, Q-3 (categories source) |
| Browser: click row | Select | D |
| Browser: double-click row, Enter, or Load | Load patch; close; status bar "Loaded …" | D → reload transaction |
| Browser: star | Toggle favourite (stored where? Q-3) | D, Q-3 |
| Browser: Import | File picker (.yaml/.yml); report zones found; stage for Reload Patch | D (mock), R |
| Browser: Export | Download current definition as `<patch>.yaml` | D (mock content), R |
| Browser: Copy | Copy definition text | D (mock content) |
| Browser: Cancel, ×, Esc, outside click | Close without loading | D |
| Reload Patch | Commit prepared edits through one reload; failure keeps active patch and shows error | D (states 26), existing repo behaviour |

## 3. Page rail and tree

| Input | Result | Status |
|---|---|---|
| Rail item | Switch page; current item highlighted | D |
| Tree row click | Select node; inspector shows it; page may change (sample → Sample page) | D |
| Tree row right-click | Open, Audition, Rename…, Duplicate, Delete | D, R |
| Source row click | Preview (progress bar on row, playhead on waveform) | D, R (audition) |
| Drag source → tree node / pad / zone | Adds layer / alternate / pad; eligible targets outlined; status bar names result | D, R, Q-1 |
| Drag modulator → control | Adds route at +25 % (Alt: −25 %) | D, R |
| Modulator category chevron | Expand/collapse | D |

## 4. Play drawer

| Input | Result | Status |
|---|---|---|
| Click drawer bar title | Open/close; closed shows summary | D |
| Drag top edge | Resize height (limits in DESIGN § 5); drag below half minimum closes | D |
| Double-click top edge | Reset to standard height and open | D |
| Section icons (Macros / Pads / Keyboard) | Show/hide section; opens the drawer; all hidden shows a hint | D |
| PC keys icon | Toggle computer-keyboard playing; status note; letters appear on keys | D, Q-5 |
| Octave − / + (or Z / X when PC keys on) | Shift base note ±12 (0–108); keyboard scrolls to it; leaves keyboard Fit | D |
| Drag macros divider | Resize macro area (knob columns reflow); double-click resets | D |
| Drag pads/keys divider | Resize pads area | D |
| Pad click / press | `onTrigger(note, velocity)` — velocity from click position (top = 127); pad lights with level | D → noteOn |
| Pad select | Select pad (inspector) | D |
| Pad zoom − / + | Size −/+ 8 px (32–160), from Fit's start note | D |
| Pad size label | Return to Fit | D |
| Fit, then −, then + | Returns to the identical Fit layout | D (regression case) |
| Key press | `noteOn(note, vel)`; velocity from vertical position (40–127); key lights; `noteOff` on release | D → noteOn/noteOff |
| Keyboard − / + | Key width −/+ 4 px (4–48) | D |
| Keyboard Fit label | Fit mapped range to visible width; scroll to lowest mapped note | D |
| Keyboard scrollbar track | Click/drag scrolls | D |
| Hover unmapped key | Tooltip "… · not mapped"; still plays (silent) | D |

PC keys mapping (only while on, ignored in text inputs and with Ctrl/Alt/Meta): A W S E D F T G Y H U J K = 13 semitones from the base note; Z / X = octave. Key repeat ignored.

## 5. Perform view

| Input | Result | Status |
|---|---|---|
| Patch list row click | Load immediately (one click, for live use) | D |
| Search, star | As editor browser (shared state) | D |
| Pads, keyboard, zoom, Fit, PC keys, octave | As Play drawer (same shared components and zoom state) | D |
| Macro knobs | As macros elsewhere | D, Q-8 |

## 6. Controls (everywhere)

| Input | Result | Status |
|---|---|---|
| Knob vertical drag | 200 px = full range; Shift fine ×0.1; gesture begin/update/end | D |
| Knob double-click / Enter | Type value | D |
| ↑/↓, PgUp/PgDn, Home/End | Step, ×10, min/max | D |
| Hover 300 ms / focus | Value popup: value, peak/trough, modulation sources | D, R (modulation list) |
| Control context menu | Modulation ▸ (Add ▸, routes ▸ Edit/Invert/Bypass/Remove), Learn MIDI, Map to macro…, Reset to default | D, R |
| Macro context menu | Go to destination (outlines control 1.8 s), Add destination…, Learn MIDI, Rename…, Reset | D, R, Q-8 |
| Filter response drag | x → cutoff (20 Hz–20 kHz log), y → resonance; knobs and headings follow | D, R |
| Envelope corner drag | Moves attack/decay/sustain/release; knobs follow | D, R |
| Region fields | Start / End / Fade in / Fade out numeric entry; wedges show fades | D, R (prepared) |

## 7. Pages

| Page | Interactions | Status |
|---|---|---|
| Sample | Play mode (Once/Gated/Loop/Ping-pong/Sustain loop), reverse, audition, click waveform to preview, region fields, loop fields + suggestions | D, R (prepared edits) |
| Slices | Detect transients, Even/Grid, count, add/delete/clear markers, prev/next, sensitivity, Apply/Discard, Map sequentially, Export MIDI… | D, R |
| Mapping | KeyMap: edge drag pushes neighbour; Shift = free edge (overlap = crossfade, gap = silent); move zone; Ctrl+C/V, Ctrl+Shift+V (paste as layer), Delete / Shift+Delete; Map by root / Velocity layers / Sequential | D, R, Q-1 |
| Layers | Mode bar (Stack/Velocity/Round robin/Alternate/Random/Weighted); velocity crossfade handles; width, curve, No crossfade; candidate reorder, mute, solo; Restart cycle | D, R |
| Voice | Chain chips (Edit template…), section knobs, envelope editors, voice policy (Poly/Mono/Legato, limit, steal, same note, exclusive group, glide), filter response | D, R |
| Modulation | Group by Source/Destination, Add route, row select, amount drag, On toggle, group "+ Destination", modulator shape/knobs | D, R |
| Routing | Output bus menu per row (Inherit), send knobs per FX bus, Show graph | D, R |
| Effects | LayerStack: add/move/bypass modules per chain | D, R |
| Overview | Inheritance view; override markers; reset overrides | D, Q-9 |
| Diagnostics | Last hit trace; idle state | D, R |
| Empty / drop | Browse samples…, Load multiple…; multi-file drop shows four mapping choices; Esc cancels | D, R |
| Missing / unsupported | Locate file…, Search folder…, Replace with… | D, R |

## 8. State transitions

- Draft lifecycle (structural edits): clean → **pending** (banner "Changes pending · Reload Patch") → preparing → ready | failed (previous patch keeps playing, error shown). Live parameters never enter the draft.
- Asset: loading (progress) → loaded | missing | unsupported.
- Analysis job: idle → running (progress, editing stays enabled) → done (Apply/Discard) | failed (regions unchanged).
- Drawer: open/closed × height × sections; persisted per editor session (Q-6).
- Fit/zoom: Fit ↔ manual; manual remembers its start note until Fit is pressed.
