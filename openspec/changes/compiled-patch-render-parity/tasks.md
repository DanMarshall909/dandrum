## 1. Parity Test Coverage

- [ ] 1.1 Add a failing parity test that renders an oscillator-to-output patch through the raw `Graph` path and the compiled path and asserts identical left/right buffers.
- [ ] 1.2 Add a failing parity test for a supported MIDI-driven ADSR/gain voice patch, comparing raw and compiled output buffers.
- [ ] 1.3 Add a failing parity test for a supported sampler patch using the same prepared sampler assets in both render paths.
- [ ] 1.4 Investigate whether existing polyphonic rendering can be driven by `CompiledPatch` without redesign; add a parity test only if it stays narrow, otherwise document the deferral in the test or task notes.

## 2. Compiled Render Entry Point

- [ ] 2.1 Add `render_offline_compiled` or equivalent public function in `graph_processor.rs` accepting `&CompiledPatch`, events, and sampler assets.
- [ ] 2.2 Keep existing `render_offline` and `render_offline_with_sampler_assets` signatures and behavior unchanged.
- [ ] 2.3 Ensure JUCE/FFI entry points remain untouched.

## 3. Compiled Routing Consumption

- [ ] 3.1 Add compiled-path input helpers that read sources from `CompiledNode::input_port_map` instead of string-keyed routing maps.
- [ ] 3.2 Use `CompiledPatch::execution_order` instead of calling runtime `topological_sort` in the compiled path.
- [ ] 3.3 Reuse existing module processing behavior where practical, changing only the routing/order source for compiled rendering.
- [ ] 3.4 Preserve sampler asset lookup behavior in the compiled path.

## 4. Scope Guards

- [ ] 4.1 Confirm the YAML patch format is unchanged.
- [ ] 4.2 Confirm no dependencies or multithreading are added.
- [ ] 4.3 Confirm the compiled path does not convert `CompiledPatch` back into string-keyed runtime routing maps.

## 5. Verification

- [ ] 5.1 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [ ] 5.2 Run `scripts/check-rust-coverage`.
- [ ] 5.3 Run `ctest --test-dir build`.
- [ ] 5.4 Run `openspec validate compiled-patch-render-parity --strict`.
