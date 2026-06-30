## 1. YAML Schema And Parsing

- [ ] 1.1 Add failing patch parsing tests for valid `drum_machine` modules with named pads, trigger selectors, optional pad metadata, per-pad emitted-event configuration, and public event port references.
- [ ] 1.2 Extend the Rust patch schema to parse drum-machine pad declarations and emitted-event configuration without changing existing patch files.
- [ ] 1.3 Add failing validation tests proving embedded signal-chain fields such as child modules, internal connections, sample assets, audio outputs, and mix outputs are rejected.
- [ ] 1.4 Add failing validation tests proving embedded sequencing fields such as `pattern`, `patterns`, `steps`, `tempo`, `transport`, and `clock` are rejected.
- [ ] 1.5 Add failing validation tests for unsupported drum-machine pad fields, missing pads, duplicate pad IDs, duplicate trigger selectors, invalid pad ID syntax, and malformed emitted-event configuration.

## 2. Public Event Port Contract

- [ ] 2.1 Add failing graph validation tests proving the standard `events` input resolves as a named typed event input.
- [ ] 2.2 Add failing graph validation tests proving declared pad event inputs and declared pad event outputs resolve as named typed event ports.
- [ ] 2.3 Implement drum-machine public event port derivation from the standard input and declared pad IDs.
- [ ] 2.4 Add failing routing tests proving compatible event routes are accepted and incompatible audio/control routes are rejected with existing type diagnostics.
- [ ] 2.5 Add tests proving references to missing pad ports, `mix`, or undeclared pad audio outputs are rejected.

## 3. Selector Routing And Direct Pad Triggering

- [ ] 3.1 Add failing render tests proving incoming events on `events` route to the pad whose trigger selector matches the event payload.
- [ ] 3.2 Implement deterministic selector routing without changing event timing or inventing unmatched events.
- [ ] 3.3 Add render tests proving non-matching selector events produce no output for that pad.
- [ ] 3.4 Add render tests proving direct pad input events trigger the pad output even when the event payload does not match the pad selector.

## 4. Per-Pad Event Emission

- [ ] 4.1 Add failing render tests proving a triggered pad emits its configured event payload.
- [ ] 4.2 Add failing render tests proving a pad configured to preserve the incoming event emits that event unchanged except for graph routing identity.
- [ ] 4.3 Implement per-pad emitted-event behavior for selector-routed and direct pad inputs.
- [ ] 4.4 Add deterministic render tests proving the same drum-machine patch, render settings, and input events produce identical emitted pad events and downstream audio buffers across repeated renders.

## 5. Event-Only Expansion And Downstream Composition

- [ ] 5.1 Add failing expansion tests proving a valid drum-machine expands into deterministic namespaced event-routing modules only.
- [ ] 5.2 Implement drum-machine expansion before ordinary graph validation and processor construction.
- [ ] 5.3 Add expansion tests proving multiple drum-machine instances do not collide and expanded IDs remain deterministic.
- [ ] 5.4 Add graph-safety tests proving expanded drum machines introduce only explicit event routes and cannot hide implicit audio/control/sampler/mixer behavior.
- [ ] 5.5 Add render tests proving a drum-machine pad output can trigger explicitly declared downstream sampler, script, or generic event-consuming modules.
- [ ] 5.6 Add render tests proving a drum-machine module without downstream audio-generating modules does not produce audio by itself.

## 6. Examples And CLI Acceptance

- [ ] 6.1 Add a minimal drum-machine YAML example patch using external note/event input to trigger named pads.
- [ ] 6.2 Add a minimal drum-machine YAML example patch showing emitted pad events connected to an explicit downstream signal chain.
- [ ] 6.3 Add an end-to-end CLI acceptance test that renders a drum-machine example to a non-empty WAV file produced by downstream modules.
- [ ] 6.4 Document through tests or comments that the drum-machine is a Bitwig-inspired event mapper, not a sequencer, sampler, mixer, signal-chain host, or separate audio engine.

## 7. Verification

- [ ] 7.1 Run Rust unit and acceptance tests with `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [ ] 7.2 Run CMake/CTest verification if build configuration is available: `$HOME/.local/bin/cmake -S . -B build`, `$HOME/.local/bin/cmake --build build`, and `ctest --test-dir build`.
- [ ] 7.3 Run OpenSpec validation for `add-drum-machine-container` and confirm every drum-machine requirement has planned test or implementation evidence.
