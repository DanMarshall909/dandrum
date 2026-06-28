## Why

The engine currently rebuilds routing tables, topological order, port lookups, and module state every call to `render_offline`. This means the audio callback (or offline renderer) repeatedly resolves string-based port IDs and re-derives an execution order that never changes for a given patch. Moving these one-time computations into an explicit compile phase is the first step toward realtime-safe rendering and sets the architectural foundation for all future engine work (composite modules, step sequencer, etc.).

## What Changes

- Introduce a `CompiledPatch` type that holds the resolved, validated execution plan derived from a `Graph` and render settings.
- Add a `compile` function on `Engine` (or a new `Compiler` struct) that accepts a `Graph` + `RenderSettings` and returns a `CompiledPatch` or a compilation error.
- Define `CompiledNode`, `ExecutionStep`, `CompiledPortRef`, and `CompiledVoiceTemplate` types that the render loop can consume directly without string lookups or graph traversal.
- Move topological sorting and routing construction out of `graph_processor::render_offline` into the compilation step.
- Keep existing rendering working unchanged; wire `CompiledPatch` into `render_offline` only if it can be done cleanly as a drop-in replacement.
- Add focused unit tests for compilation behaviour (dependency order, port validation, voice/global separation, ID preservation).

## Capabilities

### New Capabilities
- `compiled-patch`: The compile phase that transforms a validated `Graph` into a ready-to-execute `CompiledPatch` with resolved node indexes, port references, topological order, and voice/global separation.

### Modified Capabilities

- *(none)*

## Impact

- New module `src/rust-engine/src/compiled_patch.rs` with public types and a `compile` function.
- `graph_processor.rs`: render internals will gradually consume `CompiledPatch` instead of raw `Graph` — initially an opt-in path alongside existing code.
- `core.rs`: `Engine::compile` or equivalent API added.
- `lib.rs`: new module exported.
- Tests in `compiled_patch.rs` (unit) plus potential integration test updates.
- **No** C API or JUCE wrapper changes yet.
- **No** new dependencies.
