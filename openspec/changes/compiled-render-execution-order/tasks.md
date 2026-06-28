## 1. Bug Reproduction Test

- [ ] 1.1 Add a failing parity test `compiled_render_matches_raw_for_voice_to_global_patch` that:
  - Creates a graph with a voice-scoped oscillator → global `audio_mixer` → global `audio_output`.
  - Renders via the raw path (`render_offline_with_sampler_assets`) and compiled path (`render_offline_compiled`).
  - Asserts identical left/right buffers.
  - Confirms the test fails before the production change (voice oscillator produces silence in compiled path).

## 2. Guard Test: Execution Order Invariant

- [ ] 2.1 Add a test `compiled_execution_order_remains_globals_first_after_fix` that:
  - Creates the same voice→global graph.
  - Asserts `execution_order` has all global indices before any voice index.
  - Asserts `global_node_indices` and `voice_node_indices` are correct.
  - Passes both before and after the production change.

## 3. Remove Public `execution_order()` Getter

- [ ] 3.1 Remove `pub fn execution_order()` from `CompiledPatch`. The field stays private.
- [ ] 3.2 Update `compiled_patch.rs` tests to access `compiled.execution_order` directly (field access, same module).

## 4. Fix `process_block_compiled`

- [ ] 4.1 Split `process_block_compiled` iteration into two phases:
  - Phase 1: iterate `compiled.voice_node_indices()` in order, processing each voice module and storing outputs in `all_outputs`.
  - Phase 2: iterate `compiled.global_node_indices()` in order, processing each global module reading from `all_outputs`.
- [ ] 4.2 Keep `midi_input` seeding and `audio_output` extraction at the correct points (seed before phase 1, extract after phase 2).
- [ ] 4.3 Keep existing module processing dispatch unchanged (`process_oscillator`, `process_vca`, etc.).
- [ ] 4.4 Keep existing input-gathering helpers unchanged (`compiled_gather_event_inputs`, `compiled_control_input_or_default`, etc.).

## 4. Verify Parity

- [ ] 4.1 Confirm new voice→global parity test passes.
- [ ] 4.2 Confirm guard test passes.
- [ ] 4.3 Confirm all three existing parity tests still pass.
- [ ] 4.4 Confirm all `compiled_patch.rs` unit tests still pass.

## 5. Scope Guards

- [ ] 5.1 Confirm `compile()` and `CompiledPatch` data structure are unchanged.
- [ ] 5.2 Confirm raw `Graph` rendering functions are unchanged.
- [ ] 5.3 Confirm no JUCE, FFI, YAML, dependencies, CMake, or threading changes.
- [ ] 5.4 Confirm no polyphonic compiled rendering is implemented.

## 6. Verification

- [ ] 6.1 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [ ] 6.2 Run `scripts/check-rust-coverage`.
- [ ] 6.3 Run `ctest --test-dir build`.
- [ ] 6.4 Run `openspec validate compiled-render-execution-order --strict`.
