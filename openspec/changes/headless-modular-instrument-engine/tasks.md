## 1. Project Foundation

- [x] 1.1 Choose the initial implementation language, package layout, and test framework for the headless engine.
- [x] 1.2 Create the engine core package without GUI, plugin, or CLI coupling.
- [ ] 1.3 Add a minimal CLI entry point for loading patches, inspecting validation results, and invoking offline renders.
- [x] 1.4 Add CI-ready test commands for unit and acceptance tests.

## 2. YAML Patch Format

- [ ] 2.1 Define the YAML patch schema for metadata, render settings, assets, modules, ports, and connections.
- [ ] 2.2 Implement YAML patch loading with clear parse errors for invalid YAML or unsupported file formats.
- [ ] 2.3 Implement schema validation for duplicate module IDs, missing required fields, and malformed connection references.
- [ ] 2.4 Add patch-format tests covering valid YAML, invalid YAML, duplicate IDs, missing modules, and script custom ports.

## 3. Modular Routing Graph

- [ ] 3.1 Implement core domain types for modules, ports, signal types, cables, and graphs.
- [ ] 3.2 Implement graph construction from validated patch declarations.
- [ ] 3.3 Implement port existence, direction, and signal compatibility validation.
- [ ] 3.4 Implement validation for unsupported implicit many-to-one routing.
- [ ] 3.5 Add routing graph tests for valid routes, missing ports, wrong direction, incompatible types, and many-to-one errors.

## 4. VCA and Control Routing

- [ ] 4.1 Define VCA/control signal types and compatibility rules.
- [ ] 4.2 Represent modulatable destinations such as gain, pitch, cutoff, pan, and envelope parameters as ports.
- [ ] 4.3 Implement explicit control mixer or summing module behavior.
- [ ] 4.4 Add tests proving any compatible VCA/control output can connect to any compatible VCA/control input.

## 5. Built-In Modules

- [ ] 5.1 Implement the built-in module registry.
- [ ] 5.2 Add MIDI/event input and audio output module definitions.
- [ ] 5.3 Add oscillator or sample player, gain/VCA, audio mixer, control mixer, ADSR, LFO, and simple filter module definitions.
- [ ] 5.4 Add one-sample audio delay, block delay, and control delay module definitions with feedback-boundary metadata.
- [ ] 5.5 Add tests that inspect built-in module ports, signal types, directions, and feedback-boundary declarations.

## 6. Script Modules

- [ ] 6.1 Select and integrate the initial script runtime or define an internal script abstraction if runtime selection is deferred.
- [ ] 6.2 Implement script module loading with declared input and output ports from the YAML patch.
- [ ] 6.3 Implement bounded script processing for events and control values.
- [ ] 6.4 Implement module-local script state between processing calls.
- [ ] 6.5 Add tests for script routing, event-to-control transformation, state retention, and prevention of recursive graph execution.

## 7. Feedback Routing

- [ ] 7.1 Implement graph cycle detection with diagnostics that include the cycle path.
- [ ] 7.2 Implement audio feedback validation requiring an explicit audio delay boundary.
- [ ] 7.3 Implement control feedback validation requiring a control delay, smoothing stage, or tick/block boundary.
- [ ] 7.4 Implement event and script feedback scheduling to a future tick or block.
- [ ] 7.5 Add tests for valid delayed feedback and invalid instantaneous audio/control feedback.

## 8. Offline Rendering

- [ ] 8.1 Implement the block scheduler used by offline rendering.
- [ ] 8.2 Implement input event sequencing into the graph scheduler.
- [ ] 8.3 Implement WAV file output for rendered audio.
- [ ] 8.4 Add deterministic render tests using the same patch, assets, settings, and events twice.
- [ ] 8.5 Add an end-to-end CLI acceptance test that renders a simple YAML patch to WAV.

## 9. Documentation and Examples

- [ ] 9.1 Add a minimal valid YAML patch example for event input to oscillator or sample player to VCA to output.
- [ ] 9.2 Add a YAML patch example showing LFO/envelope VCA routing through a control mixer.
- [ ] 9.3 Add a YAML patch example showing valid feedback through a delay boundary.
- [ ] 9.4 Document invalid feedback and many-to-one routing diagnostics.

## 10. Final Verification

- [ ] 10.1 Run all unit and acceptance tests.
- [ ] 10.2 Run OpenSpec validation for the change.
- [ ] 10.3 Confirm all acceptance criteria are represented by tests or documented implementation evidence.
