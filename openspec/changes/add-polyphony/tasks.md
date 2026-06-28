## 1. Patch Schema And Validation

- [x] 1.1 Add tests for missing `voice_allocation` defaulting to one voice with stealing disabled.
- [x] 1.2 Add tests for loading positive polyphonic `max_voices` and stealing policy from YAML.
- [x] 1.3 Add tests for rejecting zero or invalid voice limits with clear diagnostics.
- [x] 1.4 Implement patch domain types, YAML loading, defaults, and validation for voice allocation.

## 2. Voice Allocation Core

- [x] 2.1 Add allocator tests for overlapping note-on events allocating independent voice slots.
- [x] 2.2 Add allocator tests for note-off releasing only the matching active voice.
- [x] 2.3 Add allocator tests for repeated same-note note-on events allocating separate voices when capacity remains.
- [x] 2.4 Add allocator tests for oldest-active stealing and no-steal full-capacity behavior.
- [x] 2.5 Implement deterministic voice slot state, allocation, release, and stealing policy.

## 3. Voice Sub-Synth Scope And Routing Validation

- [x] 3.1 Add tests for built-in module execution scope metadata for voice-scoped and global modules.
- [x] 3.2 Add tests for a voice-local sub-synth using voice-scoped mixer, VCA/control, and output-shaping modules before global mixing.
- [x] 3.3 Add tests rejecting unsupported voice-to-global routing into non-mixing single-source inputs.
- [x] 3.4 Add tests accepting explicit same-type mixing and explicit event/control/audio conversion modules.
- [x] 3.5 Add tests rejecting implicit mixed-type signal routing into modules that do not declare conversion, merge, or polymorphic behavior.
- [x] 3.6 Implement graph preparation metadata for voice/global module scope, voice-to-global boundary validation, and explicit mixed-signal interaction validation.

## 4. Polyphonic Rendering

- [x] 4.1 Add render tests proving overlapping sampler notes mix instead of replacing earlier playback.
- [x] 4.2 Add render tests proving note-off releases only the matching voice envelope.
- [x] 4.3 Add render tests proving each voice has an independent voice-local sub-synth with independently applied control/VCA processing before final global mix.
- [x] 4.4 Implement per-voice module state instances and active-voice block processing in the offline renderer.
- [x] 4.5 Integrate voice allocation with realtime graph processor note-on/note-off handling without adding frontend coupling.

## 5. Determinism, Examples, And Documentation

- [ ] 5.1 Add deterministic render tests for repeated polyphonic renders with and without voice stealing.
- [ ] 5.2 Add a minimal polyphonic YAML patch example.
- [ ] 5.3 Add a YAML patch example with a per-voice sub-synth before global output mixing.
- [ ] 5.4 Document invalid voice allocation and invalid voice-to-global routing diagnostics.

## 6. Verification

- [ ] 6.1 Run Rust unit tests for the engine crate.
- [ ] 6.2 Run CMake/CTest verification for CLI acceptance coverage when feasible.
- [ ] 6.3 Run OpenSpec validation for `add-polyphony`.
- [ ] 6.4 Confirm all `polyphonic-voice-allocation` scenarios are covered by tests or documented implementation evidence.
