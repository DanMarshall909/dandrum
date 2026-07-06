## Why

Dandrum is currently a headless-first Rust instrument engine with a JUCE console wrapper. The next product step is a DAW plugin that can load authored Dandrum instruments, expose their playable control surface, and render safely inside AU/VST3 hosts.

The plugin must not become the instrument authoring environment. Editing YAML graphs, module routing, assets, and instrument definitions belongs in the CLI or a future standalone authoring UI. The plugin should be a reliable runtime that loads a prepared instrument definition and exposes generic JUCE controls for the declared public surface.

DAW hosts also expect plugin parameters to remain stable for automation, project recall, and preset management. Allowing live graph edits or changing the parameter layout after a plugin instance has been loaded would create avoidable host-compatibility and realtime-safety problems.

The important runtime split is: YAML describes the immutable instrument definition, while declared public parameter values remain mutable for live tweaking, presets, automation, and state recall. Changing parameter values must not edit or reload YAML. Changing YAML requires an explicit instrument reload/replacement.

## What Changes

- Add a JUCE AU/VST3 plugin target beside the existing JUCE console app.
- Treat YAML instrument definitions as immutable for the lifetime of a loaded plugin instrument.
- Load or reload instrument definitions only through explicit off-audio-thread preparation.
- Treat `preset_surface.parameters` as the mutable runtime control surface declared by the immutable YAML definition.
- Generate generic JUCE plugin controls from the loaded instrument's declared `preset_surface.parameters`.
- Apply public parameter value changes to prepared runtime parameter state without mutating the YAML definition.
- Keep the plugin UI generic: knobs/sliders, preset selection, status/errors, and reload/load actions.
- Keep instrument authoring outside the plugin, using the CLI or a future standalone editor.
- Preserve realtime callback constraints: no locks, no allocation, no file I/O, no YAML parsing, no sample loading, and no logging in the audio callback.
- Add plugin state persistence for the loaded instrument identity/content and current parameter values.
- Add sample-accurate MIDI handoff from JUCE `MidiBuffer` into the Rust engine.

## Out of Scope

- No graph editor inside the DAW plugin.
- No embedded web UI for the plugin v1.
- No live mutation of the YAML graph during audio rendering.
- No dynamic parameter-layout changes after the instrument is loaded.
- No YAML rewrite/edit operation for plugin parameter changes.
- No sample-accurate public parameter automation in the first mutable-parameter slice unless explicitly added later.
- No multichannel output beyond stereo unless required by a later change.
- No new DSP module types as part of the plugin shell work.

## Impact

- Adds a new product boundary: Dandrum Plugin as a DAW runtime.
- Keeps Dandrum Engine as the Rust DSP/runtime boundary.
- Leaves Dandrum instrument authoring to the CLI or a future dedicated editor.
- Requires FFI expansion for sample-accurate MIDI, parameter updates, state/error reporting, and prepared instrument loading.
- Requires a retained immutable loaded-instrument definition plus mutable public parameter state in the Rust engine.
- Requires CMake/JUCE target changes and C++ plugin processor/editor implementation.
- Requires tests for plugin construction, realtime-safe processing, state round-tripping, parameter mapping, and MIDI timing.
