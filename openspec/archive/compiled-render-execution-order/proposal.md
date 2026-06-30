## Why

`CompiledPatch::execution_order` intentionally stores global nodes before voice nodes (per spec). But `process_block_compiled` iterates this order directly — when a voice-scoped oscillator feeds a global `audio_mixer` or `audio_output`, the global consumer runs first and reads silence from the not-yet-computed voice producer. This is a real bug in the compiled render path: voice→global routing produces silent output.

The raw `process_block_polyphonic` path already handles this correctly with a two-phase voice-then-global approach, proving the fix pattern. The compiled path needs the same semantics without changing the `CompiledPatch` data model.

## What Changes

- Change `process_block_compiled` in `graph_processor.rs` from a single flat iteration of `execution_order` to a two-phase process: voice nodes first (accumulate outputs), then global nodes (consume accumulated outputs).
- Keep `CompiledPatch::execution_order`, `global_node_indices`, and `voice_node_indices` unchanged.
- Keep `compile()` unchanged — globals-first `execution_order` remains the spec.
- Add a parity test for voice-scoped oscillator → global mixer → global output that catches the current bug.
- Add a guard test that `execution_order` stays globals-first even after the fix.
- Keep all existing parity tests passing.
- Do not change raw `Graph` rendering, JUCE, FFI, YAML, dependencies, CMake, or threading.
- Do not implement polyphonic compiled rendering here.

## Capabilities

### New Capabilities
- `compiled-render-execution-order`: Voice-scoped modules routed to global modules produce correct output in compiled offline rendering.

### Modified Capabilities

None — the `CompiledPatch` spec (`execution_order` globals-first, voice/global separation) is unchanged. This is a renderer bug fix, not a spec change.

## Impact

- `src/rust-engine/src/graph_processor.rs`: modify `process_block_compiled` only; add new parity tests in the compiled render parity section.
- No C API, JUCE wrapper, YAML schema, dependency, or multithreading changes.
