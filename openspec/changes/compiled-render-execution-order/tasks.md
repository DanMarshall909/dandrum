## 1. Bug Reproduction Test

- [x] 1.1 Add a parity test `compiled_render_matches_raw_for_voice_to_global_patch` that:
  - Creates a graph with a voice-scoped oscillator → global `audio_mixer` → global `audio_output`.
  - Renders via the raw path (`render_offline_with_sampler_assets`) and compiled path (`render_offline_compiled`).
  - Asserts identical left/right buffers.
  - Fails before the production change, passes after.

## 2. Guard Test: Execution Order Invariant

- [x] 2.1 Add a test `compiled_execution_order_remains_globals_first` that:
  - Creates the same voice→global graph.
  - Asserts `execution_order` has all global indices before any voice index.
  - Asserts `global_node_indices` and `voice_node_indices` are correct.
  - Passes both before and after the production change.

## 3. Fix `process_block_compiled`

- [x] 3.1 Add doc comment to `CompiledPatch::execution_order()`: scope-ordered metadata, not render order.
- [x] 3.2 Split `process_block_compiled` iteration into two phases:
  - Phase 1: iterate `compiled.voice_node_indices()` in order, processing each voice module and storing outputs in `all_outputs`.
  - Phase 2: iterate `compiled.global_node_indices()` in order, processing each global module reading from `all_outputs`.
- [x] 3.3 Keep `midi_input` seeding and `audio_output` extraction at the correct points (seed before phase 1, extract after phase 2).
- [x] 3.4 Keep existing module processing dispatch unchanged (`process_oscillator`, `process_vca`, etc.).
- [x] 3.5 Keep existing input-gathering helpers unchanged (`compiled_gather_event_inputs`, `compiled_control_input_or_default`, etc.).

## 4. Verify Parity

- [x] 4.1 Confirm new voice→global parity test passes.
- [x] 4.2 Confirm guard test passes.
- [x] 4.3 Confirm all three existing parity tests still pass.
- [x] 4.4 Confirm all `compiled_patch.rs` unit tests still pass.

## 5. Scope Guards

- [x] 5.1 Confirm `compile()` and `CompiledPatch` data structure are unchanged.
- [x] 5.2 Confirm raw `Graph` rendering functions are unchanged.
- [x] 5.3 Confirm no JUCE, FFI, YAML, dependencies, CMake, or threading changes.
- [x] 5.4 Confirm no polyphonic compiled rendering is implemented.

## 6. Verification

- [x] 6.1 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml` — 171 pass.
- [x] 6.2 Run `scripts/check-rust-coverage` — passes.
- [x] 6.3 Run `ctest --test-dir build` — passes.
- [x] 6.4 Run `openspec validate compiled-render-execution-order --strict` — passes.
