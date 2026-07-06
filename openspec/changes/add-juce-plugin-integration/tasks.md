## 0. Reference Review

- [x] 0.1 Review `https://github.com/nberr/juce-template` before implementation.
- [x] 0.2 Identify reusable JUCE plugin structure, CMake, parameter, preset, state, and UI component patterns.
- [x] 0.3 Explicitly exclude template features that conflict with Dandrum v1 scope, including embedded React UI, website preset sharing, registration/licensing flows, and unrelated DSP architecture.
- [x] 0.4 Capture any adopted patterns in the implementation notes or relevant code comments before applying them.

## 1. Plugin Target

- [x] 1.1 Add a `dandrum-plugin` JUCE target with VST3, AU, and Standalone formats.
- [x] 1.2 Keep the existing console app (`dandrum-drum-machine-demo`) as the smoke-test/debug harness.
- [x] 1.3 Add `src/juce-plugin/PluginProcessor.*` and `src/juce-plugin/PluginEditor.*`.
- [x] 1.4 Link the plugin target to the existing Rust static library import.
- [x] 1.5 Add a minimal plugin construction test or host-smoke test where practical.

## 2. Plugin Processor

- [x] 2.1 Implement a `DandrumAudioProcessor` that owns the plugin lifecycle.
- [x] 2.2 Implement `prepareToPlay` to prepare the Rust engine with sample rate and max block size.
- [x] 2.3 Implement `processBlock` with no locks, allocation, file I/O, YAML parsing, sample loading, graph compilation, or logging.
- [x] 2.4 Clear unused output channels deterministically.
- [x] 2.5 Keep stereo rendering as the v1 output model.

## 3. Immutable Instrument Loading

- [ ] 3.1 Introduce an explicit plugin concept of a loaded immutable instrument definition.
- [ ] 3.2 Ensure YAML graph/instrument definition loading happens only off the audio thread.
- [ ] 3.3 Implement safe prepared-engine replacement for explicit load/reload actions.
- [ ] 3.4 Ensure changing the instrument definition is a full reload/replacement, not an in-place graph mutation.
- [ ] 3.5 Preserve current sample rate and max block size when preparing replacement engines.
- [ ] 3.6 Retain the loaded YAML definition as immutable engine state after successful load.
- [ ] 3.7 Ensure plugin parameter changes never rewrite, mutate, or reload the YAML definition.

## 4. Generic JUCE Controls

- [ ] 4.1 Read public control metadata from `preset_surface.parameters` after instrument load.
- [ ] 4.2 Create generic JUCE knobs/sliders for every declared public parameter.
- [ ] 4.3 Use declared labels, defaults, ranges, and display metadata where available.
- [ ] 4.4 Keep parameter IDs stable for the lifetime of the loaded instrument instance.
- [ ] 4.5 Do not expose graph editing or YAML editing in the plugin UI.
- [ ] 4.6 Do not embed a web UI in the v1 plugin editor.

## 5. Parameter Bridge

- [ ] 5.1 Add Rust engine APIs for setting public parameter values by stable ID or prepared index.
- [ ] 5.2 Add C FFI for plugin parameter updates.
- [ ] 5.3 Avoid string lookup on the audio thread where practical by preparing parameter handles/indices.
- [ ] 5.4 Apply parameter values to the active graph without reallocating in the callback.
- [ ] 5.5 Add tests proving parameter values update the mapped instrument targets.
- [ ] 5.6 Add `LoadedInstrument`, `InstrumentDefinition`, and `InstrumentParameterState` so immutable definition state and mutable value state are explicit.
- [ ] 5.7 Resolve `preset_surface.parameters[*].maps_to` into prepared public parameter bindings at instrument load time.
- [ ] 5.8 Initialise mutable parameter state from YAML defaults when the instrument is loaded.
- [ ] 5.9 Apply live public parameter changes to mutable runtime state without editing or reparsing YAML.
- [ ] 5.10 Use block-boundary parameter application for v1; defer sample-accurate public parameter automation unless separately specified.

## 6. Presets

- [ ] 6.1 Treat presets as value changes for the currently loaded instrument surface.
- [ ] 6.2 Reject presets that do not match the loaded instrument identity/schema version.
- [ ] 6.3 Ensure presets cannot mutate graph, routing, render, scheduling, script, feedback, or undeclared module structure.
- [ ] 6.4 Add plugin/editor support for selecting/loading compatible presets.
- [ ] 6.5 Apply compatible presets to mutable runtime parameter state, not the immutable YAML definition.

## 7. Sample-Accurate MIDI

- [x] 7.1 Add FFI methods for `note_on_at` and `note_off_at` with frame offsets.
- [x] 7.2 Store incoming MIDI as bounded pending `BlockEvent` values in Rust.
- [x] 7.3 Decode JUCE `MidiBuffer` sample offsets in `processBlock`.
- [x] 7.4 Forward note on/off events to Rust with their block-local frame offsets.
- [x] 7.5 Add tests proving non-zero MIDI frame offsets are preserved.

## 8. State Persistence

- [ ] 8.1 Implement plugin state schema versioning.
- [ ] 8.2 Store enough instrument information to restore projects without relying only on absolute file paths.
- [ ] 8.3 Store current public parameter values.
- [ ] 8.4 Store compatible preset identity/content where applicable.
- [ ] 8.5 Restore plugin state by preparing a replacement engine off the audio thread.
- [ ] 8.6 Report restore failures clearly in the editor/status surface.
- [ ] 8.7 Restore immutable instrument definition first, then apply saved mutable public parameter values.

## 9. Error and Status Reporting

- [ ] 9.1 Add FFI for querying last load/prepare error off the audio thread.
- [ ] 9.2 Preserve structured Rust load/validation/compile/sample-preparation errors for display.
- [ ] 9.3 Display current instrument status and error messages in the JUCE editor.
- [ ] 9.4 Expose diagnostic counters such as dropped MIDI events where useful.

## 10. Tests and Safety Checks

- [x] 10.1 Add Rust tests for sample-accurate event submission.
- [ ] 10.2 Add Rust tests for public parameter mapping.
- [x] 10.3 Add C++/CTest coverage for FFI plugin-facing operations.
- [x] 10.4 Add plugin processor tests where JUCE test infrastructure allows.
- [x] 10.5 Extend realtime callback safety checks to include plugin `processBlock` sources.
- [x] 10.6 Verify existing Rust and CMake/CTest suites still pass.
- [ ] 10.7 Add tests proving public parameter changes do not mutate/reload YAML.
- [ ] 10.8 Add tests proving unknown public parameters are rejected.
- [ ] 10.9 Add tests proving block-boundary public parameter updates affect mapped runtime targets.

## 11. Documentation

- [ ] 11.1 Document the plugin/runtime versus authoring split.
- [ ] 11.2 Document immutable instrument definition semantics.
- [ ] 11.3 Document generic controls generated from `preset_surface.parameters`.
- [ ] 11.4 Document reload/recreate behaviour for changing instruments.
- [ ] 11.5 Document plugin state persistence expectations.
- [ ] 11.6 Document that public parameter values are mutable runtime state owned by the loaded plugin instance.
