## 1. Reconcile Spec With Existing Engine

- [x] 1.1 Document the current YAML patch shape, including `metadata`, `render`, `assets`, `module_definitions`, `modules`, `connections`, and `voice_allocation`.
- [x] 1.2 Confirm this change preserves existing inline `module_definitions` semantics and does not introduce a conflicting `type: composite` / `composite_id` model.
- [x] 1.3 Add tests proving existing composite expansion still works after this change.
- [x] 1.4 Add tests proving expanded composite module IDs remain deterministic and namespaced by instance ID.
- [x] 1.5 Add tests proving graph validation still runs against the expanded graph.

## 2. Structured Diagnostics Foundation

- [x] 2.1 Define a structured diagnostic record with stable error code, severity, message, optional YAML source range, optional module ID, optional port name, expected value/type, actual value/type, and suggested fix.
- [x] 2.2 Add error code namespaces: `loading.*`, `validation.*`, `graph.*`, `script.*`, `render.*`.
- [x] 2.3 Convert graph validation diagnostics to structured diagnostics without losing the existing human-readable display text.
- [x] 2.4 Add diagnostics collection API for loading, patch validation, graph construction, and render preparation.
- [x] 2.5 Add tests proving multiple diagnostics are collected instead of failing on only the first error.
- [x] 2.6 Source-location tracking: `serde_yaml` does not expose structured line/column locations during deserialization. The `Diagnostic` and `SourceLocation` types are defined to accept locations when available, and `PatchLoadError::to_diagnostic()` carries file path info. Full YAML field-level source mapping would require a custom YAML visitor or switching to a YAML parser that exposes locations; this is documented for future tooling work.

## 3. Module Parameter Metadata

- [x] 3.1 Define parameter metadata model: name, value type, default, optional range, optional unit, optional enum values, and realtime note.
- [x] 3.2 Add parameter metadata for existing oscillator, gain, ADSR, filter, sampler, saturator, dynamics, echo, reverb, splitter, and spectral processor modules.
- [x] 3.3 Add tests proving metadata can be queried without constructing an audio renderer.
- [x] 3.4 Add validation tests proving unknown or invalid parameter values produce structured diagnostics where metadata exists.

## 4. Minimal New Primitives

- [x] 4.1 Implement `noise` primitive with deterministic seeded white noise output.
- [x] 4.2 Implement `impulse` primitive with event trigger input and sample-accurate one-sample click output.
- [x] 4.3 Implement `multiply` primitive for audio/control multiplication needed by modulation and VCA-style composites.
- [x] 4.4 Implement `note_to_control` primitive that converts note events to frequency, pitch ratio/CV, gate/trigger, and normalized velocity control outputs.
- [x] 4.5 Register the new primitives in the built-in module registry with typed ports and parameter metadata.
- [x] 4.6 Add deterministic render tests for each new primitive.
- [x] 4.7 Decision: Reduce acceptance examples to current oscillator behaviour (saw wave only) rather than adding waveform support now. The saw oscillator can be used for the 808 kick body with pitch envelope, and filter/noise can shape it further. Waveform selection is a candidate for a future parameter metadata extension once a clear acceptance example requires it.

## 5. Script Module Constraints

- [ ] 5.1 Define the supported script language/runtime surface for the first implementation.
- [ ] 5.2 Parse and validate scripts off the audio thread before graph render preparation.
- [ ] 5.3 Reject filesystem, network, blocking, and nondeterministic APIs at validation time where possible.
- [ ] 5.4 Enforce bounded execution cost during render-time script execution.
- [ ] 5.5 Prevent audio-rate output ports on script modules in the initial implementation.
- [ ] 5.6 Report script validation/runtime failures through structured diagnostics.
- [ ] 5.7 Add deterministic tests for event/control script behaviour.

## 6. Composite Hardening

- [x] 6.1 Add tests for composite parameter exposure using existing `module_definitions.parameters` bindings.
- [x] 6.2 Add tests for composite asset bindings using existing `module_definitions.asset_bindings`.
- [x] 6.3 Add diagnostics that map expanded graph failures back to the composite instance and internal module path where possible.
- [x] 6.4 Add optional external composite library loading only after inline composite behaviour is fully covered.
- [x] 6.5 Add maximum composite nesting-depth validation if external or recursive composite loading is introduced.

## 7. YAML Format Extensions

- [x] 7.1 Extend existing `assets` validation with missing-asset diagnostics and supported asset kind checks.
- [x] 7.2 Add patch-level parameter bindings only if they do not conflict with existing module-level parameters or composite bindings.
- [x] 7.3 Add preset support as named parameter sets applied to an existing patch/composite without changing graph semantics.
- [x] 7.4 Ensure existing patches remain valid unless a separate migration spec explicitly changes the schema.

## 8. Drum Machine Event Mapper Alignment

- [ ] 8.1 Keep the drum machine event-only: no samples, synthesis chains, audio outputs, mixer, sequencer, tempo, clock, probability, or transport.
- [ ] 8.2 Align any pad configuration with the existing `add-drum-machine-container` change or explicitly supersede that change.
- [ ] 8.3 Add tests proving pad event outputs trigger explicitly declared downstream voice composites.
- [ ] 8.4 Add tests proving a drum machine without downstream audio modules produces no audio by itself.

## 9. Acceptance Examples

- [ ] 9.1 Build the first acceptance example: synthetic 808-style kick from primitives/composites, not a dedicated Rust kick module.
- [ ] 9.2 Verify the 808 kick loads and renders deterministically.
- [ ] 9.3 Add synthetic snare example after `noise`, `multiply`, and envelope/control routing are proven.
- [ ] 9.4 Add closed/open hi-hat examples after filter/noise/envelope routing is proven.
- [ ] 9.5 Add subtractive synth voice example only after oscillator waveform support is explicitly implemented or the example is adjusted to current oscillator capability.
- [ ] 9.6 Add sampler voice example using the existing sampler module and explicit pitch/amp control routing.
- [ ] 9.7 Add effects rack example using existing effect modules.
- [ ] 9.8 Add script event/control mapping example after script constraints are implemented.
- [ ] 9.9 Add drum-machine-to-voice-composite example after event mapper behaviour is implemented.

## 10. Capability Discovery

- [x] 10.1 Implement module type enumeration API after module metadata exists.
- [x] 10.2 Implement port metadata query using the built-in registry and composite metadata.
- [x] 10.3 Implement parameter metadata query using the metadata model from section 3.
- [x] 10.4 Include module category: primitive, composite, script, preset, or future tooling.
- [x] 10.5 Include realtime notes where relevant.
- [x] 10.6 Keep discovery separate from audio rendering and add tests proving discovery does not construct or block the render path.

## 11. Verification

- [ ] 11.1 Run Rust tests: `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [ ] 11.2 Run CMake/CTest verification if local build dependencies are available.
- [ ] 11.3 Run OpenSpec validation for `declarative-instrument-platform`.
- [ ] 11.4 Update task checkboxes only after related tests and validation pass, or document any verification gap.