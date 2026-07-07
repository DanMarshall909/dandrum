## Why

Dandrum is currently a headless-first Rust instrument engine with a JUCE console wrapper. The next product step is a DAW plugin that can load authored Dandrum instruments, expose their playable control surface, and render safely inside AU/VST3 hosts.

The plugin should remain a reliable runtime surface during normal DAW use: it loads a prepared instrument definition and exposes generic JUCE controls for the declared public surface. Authoring/editing the instrument definition itself is explicitly out of scope for the plugin runtime UI (see `add-yaml-editor` for the external-edit/auto-reload companion capability).

DAW hosts also expect plugin parameters to remain stable for automation, project recall, and preset management. Allowing live graph edits or changing the parameter layout implicitly while audio is running would create avoidable host-compatibility and realtime-safety problems.

The important runtime split is: YAML describes the immutable instrument definition currently running in the plugin, while declared public parameter values remain mutable for live tweaking, presets, automation, and state recall. Changing parameter values must not edit or reload YAML. Changing YAML requires an explicit reload operation that mutes audio, stops the current DSP, compiles the replacement DSP, starts it, and reconciles presets/parameter values against the new instrument surface.

## What Changes

- Add a JUCE AU/VST3 plugin target beside the existing JUCE console app.
- Treat YAML instrument definitions as immutable while they are the currently loaded/running plugin instrument.
- Load or reload instrument definitions only through explicit off-audio-thread preparation.
- Treat `preset_surface.parameters` as the mutable runtime control surface declared by the immutable YAML definition.
- Generate generic JUCE plugin controls from the loaded instrument's declared `preset_surface.parameters`.
- Apply public parameter value changes to prepared runtime parameter state without mutating the YAML definition.
- Preserve compatible preset values across an instrument reload where parameter IDs still exist.
- Initialise newly introduced parameters to YAML-declared default values after an instrument reload.
- Keep the normal plugin UI generic: knobs/sliders, preset selection, status/errors, and load/reload actions.
- Keep realtime callback constraints strict: no locks, no allocation, no file I/O, no YAML parsing, no sample loading, and no logging in the audio callback.
- Add plugin state persistence for the loaded instrument identity/content and current parameter values.
- Add sample-accurate MIDI handoff from JUCE `MidiBuffer` into the Rust engine.

## Out of Scope

- No inline graph editing inside `processBlock` or while the active DSP graph is rendering.
- No embedded web UI for the plugin v1.
- No live mutation of the running YAML graph during audio rendering.
- No dynamic parameter-layout changes without an explicit instrument reload/replacement operation.
- No YAML rewrite/edit operation for ordinary plugin parameter changes.
- No sample-accurate public parameter automation in the first mutable-parameter slice unless explicitly added later.
- No multichannel output beyond stereo unless required by a later change.
- No new DSP module types as part of the plugin shell work.
- Instrument authoring/editing UI is a separate capability — see `add-yaml-editor` (external edit + file-watch auto-reload) rather than an embedded editor here.
- Initial 808/909-style drum voice content and reference parameter seeding is a separate capability — see `add-drum-voice-authoring`.

## Impact

- Adds a new product boundary: Dandrum Plugin as a DAW runtime.
- Keeps Dandrum Engine as the Rust DSP/runtime boundary.
- Requires explicit mute/stop/compile/start orchestration for instrument reload/replacement.
- Requires FFI expansion for sample-accurate MIDI, parameter updates, state/error reporting, and prepared instrument loading.
- Requires a retained immutable loaded-instrument definition plus mutable public parameter state in the Rust engine.
- Requires preset reconciliation when a new instrument version changes the public parameter surface.
- Requires CMake/JUCE target changes and C++ plugin processor/editor implementation.
- Requires tests for plugin construction, realtime-safe processing, state round-tripping, parameter mapping, MIDI timing, and preset reconciliation.
- `add-yaml-editor` and `add-drum-voice-authoring` both depend on the immutable-instrument-replacement mechanism this change establishes (section 3).
