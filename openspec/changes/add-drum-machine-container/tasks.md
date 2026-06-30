## 1. YAML Schema And Parsing

- [ ] 1.1 Add failing patch parsing tests for valid `drum_machine` modules with named pads, trigger selectors, optional pad metadata, optional pad child chains, and public event port references.
- [ ] 1.2 Extend the Rust patch schema to parse drum-machine pad declarations and child-chain declarations without changing existing patch files.
- [ ] 1.3 Add failing validation tests proving embedded sequencing fields such as `pattern`, `patterns`, `steps`, `tempo`, `transport`, and `clock` are rejected.
- [ ] 1.4 Add failing validation tests for unsupported drum-machine pad fields, missing pads, duplicate pad IDs, duplicate trigger selectors, and invalid pad ID syntax.

## 2. Public Event Port Contract

- [ ] 2.1 Add failing graph validation tests proving declared pad event inputs and declared pad event outputs resolve as named typed ports.
- [ ] 2.2 Implement drum-machine public event port derivation from declared pad IDs.
- [ ] 2.3 Add failing routing tests proving compatible event routes are accepted and incompatible audio/control routes are rejected with existing type diagnostics.
- [ ] 2.4 Add tests proving references to missing pad ports, `mix`, or undeclared pad audio outputs are rejected.

## 3. Selector Routing And Direct Pad Triggering

- [ ] 3.1 Add failing render tests proving incoming container trigger events route to the pad whose trigger selector matches the event payload.
- [ ] 3.2 Implement deterministic selector routing without changing event timing or inventing new events.
- [ ] 3.3 Add render tests proving non-matching selector events produce no output for that pad.
- [ ] 3.4 Add render tests proving direct pad input events trigger the pad output even when the event payload does not match the pad selector.

## 4. Container And Pad-Chain Expansion

- [ ] 4.1 Add failing expansion tests proving a valid drum-machine expands into deterministic namespaced event-routing modules.
- [ ] 4.2 Add failing expansion tests proving declared pad child-chain modules and routes expand under deterministic namespaced module IDs.
- [ ] 4.3 Implement drum-machine expansion before ordinary graph validation and processor construction.
- [ ] 4.4 Add expansion tests proving multiple drum-machine instances do not collide and expanded IDs remain deterministic.
- [ ] 4.5 Add graph-safety tests proving expanded drum machines introduce only explicit routes and cannot hide invalid child-chain routes or feedback cycles.

## 5. Downstream Composition

- [ ] 5.1 Add render tests proving a drum-machine pad output can trigger a downstream sampler.
- [ ] 5.2 Add render tests proving a drum-machine pad output can trigger a compatible script or generic event-consuming module.
- [ ] 5.3 Add render tests proving a pad child chain receives its pad trigger event.
- [ ] 5.4 Add render tests proving a drum-machine container without child-chain or downstream audio-generating modules does not produce audio by itself.
- [ ] 5.5 Document through tests or comments that the drum-machine is a Bitwig-inspired pad/chain trigger container, not a sequencer, sampler, mixer, or separate audio engine.

## 6. Examples And CLI Acceptance

- [ ] 6.1 Add a minimal drum-machine YAML example patch using external note/event input to trigger named pads.
- [ ] 6.2 Add a minimal drum-machine YAML example patch showing a pad child chain that contains ordinary downstream modules.
- [ ] 6.3 Add an end-to-end CLI acceptance test that renders a drum-machine example to a non-empty WAV file produced by downstream modules.
- [ ] 6.4 Ensure examples keep sequencing and sound generation visibly outside the drum-machine container core.

## 7. Verification

- [ ] 7.1 Run Rust unit and acceptance tests with `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [ ] 7.2 Run CMake/CTest verification if build configuration is available: `$HOME/.local/bin/cmake -S . -B build`, `$HOME/.local/bin/cmake --build build`, and `ctest --test-dir build`.
- [ ] 7.3 Run OpenSpec validation for `add-drum-machine-container` and confirm every drum-machine requirement has planned test or implementation evidence.
