## 1. Event Routing Surface

- [x] 1.1 Add failing parsing tests for generic event-routing module declarations with readable selector configuration.
- [x] 1.2 Define the first event-routing primitives, starting with `event_filter` and adding `event_router` only if repeated filters make dogfood patches unreadable.
- [x] 1.3 Add metadata tests proving routing primitives expose typed event ports, selector parameters, defaults, and examples through capability discovery.
- [x] 1.4 Implement YAML parsing and validation for the selected event-routing primitive declarations without changing existing patch files.

## 2. Event-Only Graph Contract

- [x] 2.1 Add graph validation tests proving event-routing inputs and outputs are typed event ports.
- [x] 2.2 Add routing tests proving compatible event routes are accepted and incompatible audio/control routes are rejected with existing type diagnostics.
- [x] 2.3 Add tests proving event-routing modules do not imply audio, control, sampler, mixer, sequencing, transport, or signal-chain behavior.
- [x] 2.4 Implement graph validation support for the selected event-routing primitives.

## 3. Deterministic Event Behavior

- [x] 3.1 Add render tests proving matching note events pass through `event_filter` without timing changes.
- [x] 3.2 Add render tests proving non-matching events produce no output from `event_filter`.
- [x] 3.3 Add render tests proving event routing is deterministic across repeated renders with identical inputs.
- [x] 3.4 Implement event-routing render behavior without heap allocation on realtime render paths.

## 4. Dogfood Examples

- [x] 4.1 Add tests proving a drum-machine-style patch can route kick, snare, and hat notes to explicit downstream voice composites without a `drum_machine` primitive.
- [x] 4.2 Add tests proving a simple polyphonic synth patch can consume note events through generic routing and explicit voice/synth graph behavior without a `poly_synth` primitive.
- [x] 4.3 Add readable YAML examples for the drum-machine and simple poly-synth dogfood targets.
- [x] 4.4 Add CLI acceptance coverage proving the dogfood examples render deterministic non-empty WAV output.

## 5. Verification

- [x] 5.1 Run Rust unit and acceptance tests with `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [x] 5.2 Run CMake/CTest verification if build configuration is available: `$HOME/.local/bin/cmake -S . -B build`, `$HOME/.local/bin/cmake --build build`, and `ctest --test-dir build`.
- [x] 5.3 Run OpenSpec validation for `add-event-routing-primitives` and confirm every event-routing requirement has planned test or implementation evidence.
