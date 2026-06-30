## Why

The engine now has a compile-only `CompiledPatch`, but rendering still derives topological order and string-keyed routing from `Graph` at render time. A parity-render path proves the compiled execution plan can drive audio output without changing behavior before we migrate the primary renderer.

## What Changes

- Add a compiled offline render entry point that accepts `CompiledPatch`, input events, and required sampler assets.
- Keep the existing raw `Graph` render path available and behaviorally unchanged.
- Reuse existing graph processor module behavior where possible while replacing runtime `topological_sort` and `build_routing` use with `CompiledPatch::execution_order` and `CompiledNode::input_port_map`.
- Add render parity tests that compare old `Graph` rendering and new `CompiledPatch` rendering for identical output.
- Keep the YAML patch format, JUCE/FFI surface, dependencies, and threading model unchanged.

## Capabilities

### New Capabilities

- `compiled-patch-render-parity`: Rendering from a `CompiledPatch` produces the same offline audio as rendering from the source `Graph` for supported patch paths.

### Modified Capabilities

- None.

## Impact

- `src/rust-engine/src/graph_processor.rs`: add a compiled render entry point and shared render internals as needed.
- `src/rust-engine/src/compiled_patch.rs`: consume the existing compiled execution plan and resolved port references; adjust getters only if necessary.
- Rust unit tests: add parity tests for oscillator/output, supported MIDI/ADSR/gain voice behavior, sampler rendering when simple asset setup is available, and polyphonic rendering only if it does not require broad redesign.
- No C API, JUCE wrapper, YAML schema, dependency, or multithreading changes.
