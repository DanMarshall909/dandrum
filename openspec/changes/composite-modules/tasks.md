## 1. YAML Schema And Parsing

- [ ] 1.1 Add failing tests for patch YAML with `module_definitions` containing public inputs, public outputs, internal modules, internal connections, parameters, and asset bindings.
- [ ] 1.2 Extend the Rust patch schema to parse YAML composite module definitions without changing existing patch files.
- [ ] 1.3 Add tests that duplicate composite type names and malformed public/internal port references produce clear schema diagnostics.

## 2. Composite Definition Validation

- [ ] 2.1 Add failing validation tests for public input/output mappings with compatible and incompatible signal types.
- [ ] 2.2 Implement validation that public composite inputs map only to internal inputs and public composite outputs map only from internal outputs.
- [ ] 2.3 Add validation tests for undeclared instance parameters, declared parameter bindings, and declared asset bindings.
- [ ] 2.4 Implement parameter and asset binding validation with diagnostics identifying the composite definition and instance.

## 3. Graph Expansion

- [ ] 3.1 Add failing tests proving a valid composite instance expands into deterministic namespaced internal module IDs and cables.
- [ ] 3.2 Implement composite expansion before ordinary graph validation and processor construction.
- [ ] 3.3 Add tests proving two instances of the same composite do not collide and produce deterministic expanded IDs.
- [ ] 3.4 Ensure expanded composites preserve source diagnostics that mention the user-written composite instance and internal module path.

## 4. Graph Safety And Recursion

- [ ] 4.1 Add tests proving composites cannot hide implicit many-to-one routes without an explicit mixer.
- [ ] 4.2 Add tests proving composites cannot hide instantaneous audio/control feedback without explicit delay or future scheduling boundaries.
- [ ] 4.3 Add tests for direct and indirect recursive composite definitions.
- [ ] 4.4 Implement recursive definition detection before graph expansion.

## 5. Rendering Integration

- [ ] 5.1 Add render tests proving a composite wrapping oscillator/gain/audio routing renders the same output as the equivalent flat graph.
- [ ] 5.2 Add render tests proving a composite can wrap a sampler signal-generator while exposing only generic trigger/rate/control ports.
- [ ] 5.3 Ensure offline and realtime graph processor construction receive only expanded ordinary graph nodes and do not require nested processing logic.

## 6. Examples And Documentation Evidence

- [ ] 6.1 Add a minimal YAML example defining and instantiating a composite `drum_voice` subsystem.
- [ ] 6.2 Add a YAML example showing a composite exposes custom public inputs while hiding its internal modules.
- [ ] 6.3 Document through tests or comments that composites are YAML-defined subsystems, not Rust built-ins.

## 7. Verification

- [ ] 7.1 Run Rust unit and acceptance tests with `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [ ] 7.2 Run CMake/CTest verification if build configuration is available: `$HOME/.local/bin/cmake -S . -B build`, `$HOME/.local/bin/cmake --build build`, and `ctest --test-dir build`.
- [ ] 7.3 Run OpenSpec validation for `composite-modules` and confirm every YAML composite module requirement has test or implementation evidence.
