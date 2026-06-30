## Context

The Rust engine crate currently contains the right major concepts, but their boundaries are loose. `lib.rs` exports most modules publicly and also contains the C ABI. `core::Engine` covers headless/offline concerns, while `synth::DandrumEngine` combines a hardcoded fallback synth with graph runtime behavior. Patch loading, schema validation, graph validation, asset preparation, compilation, state allocation, and rendering are split across modules but do not yet form one explicit pipeline.

Accepted specs already require a frontend-independent headless engine, named typed routing ports, explicit feedback boundaries, script modules, built-in modules, VCA control routing, and YAML patch format behavior. This change keeps those external capabilities intact and makes the Rust architecture easier to extend for filters, echo/reverb, drum-machine containers, and future realtime work.

## Goals / Non-Goals

**Goals:**

- Define a small public Rust facade and move C ABI functions into a dedicated FFI boundary.
- Separate frontend/CLI IO from headless engine preparation and rendering.
- Make patch preparation a clear pipeline from YAML document to validated graph to compiled patch to runtime.
- Make `CompiledPatch` the shared handoff for offline and realtime rendering.
- Replace raw string runtime dispatch with typed module kinds/configuration resolved before rendering.
- Keep DSP algorithms independent from patch YAML, graph routing, FFI, and CLI concerns.
- Preallocate realtime rendering state, event capacity, and scratch buffers during preparation.
- Preserve existing patch semantics and externally observable audio behavior while refactoring.

**Non-Goals:**

- Do not change the YAML patch format unless another capability change explicitly requires it.
- Do not add new instrument modules as part of this architecture refactor.
- Do not redesign JUCE device IO or plugin/frontend behavior.
- Do not require a new external dependency unless the implementation demonstrates a clear realtime-safety or simplicity benefit.
- Do not remove the existing C ABI without a tested replacement path for the JUCE wrapper.

## Decisions

### Use layered crate modules with a narrow facade

The crate root will expose intentional API surfaces and keep implementation modules `pub(crate)` where possible. The target dependency direction is:

```text
frontend adapters / ffi / cli
        |
        v
public engine facade
        |
        v
patch preparation -> graph validation -> compilation
        |
        v
runtime execution
        |
        v
dsp algorithms
```

Alternative considered: keep the flat module surface and document conventions. That is cheaper initially, but it allows new features to couple directly to internals and makes future refactors risky.

### Treat FFI as a thin adapter

The C ABI should live in a dedicated module and convert raw pointers, paths, buffers, and status codes into calls on safe Rust APIs. Patch loading through FFI may still perform file IO, but IO and unsafe pointer handling should not be mixed with graph/runtime internals.

Alternative considered: keep FFI functions in `lib.rs`. This works mechanically, but it makes the crate root both public API and unsafe adapter code, which obscures ownership and error boundaries.

### Split engine naming by responsibility

The implementation should distinguish the headless core/facade, prepared instrument runtime, and any temporary fallback/default synth. Names should communicate responsibility rather than historical origin.

Alternative considered: continue evolving `DandrumEngine` as the main type. That avoids churn, but it keeps the fallback synth and graph runtime coupled.

### Compile once, render many

Validation and graph analysis should produce a `CompiledPatch` that contains the stable routing, execution order, scope grouping, module kind/configuration, port maps, and output buffer layout needed by renderers. Offline and realtime paths should render from this compiled representation rather than rebuilding routing and traversal state independently.

Alternative considered: keep separate realtime/offline preparation paths. That can be optimized locally, but it makes behavior parity harder to prove.

### Resolve module kinds before runtime dispatch

Patch strings and parameter text should be resolved into typed module kinds/configuration before rendering. Runtime dispatch may still use enums rather than trait objects for predictable control flow, but it should not branch on raw module type strings during audio processing.

Alternative considered: use one trait object per module. Trait objects can work well, but enum dispatch keeps state ownership explicit and may be easier to test while the built-in module set is still changing quickly.

### Keep DSP algorithms below module adapters

DSP structs such as filters, delay lines, dynamics processors, convolution, saturator, echo, and reverb should remain reusable signal-processing code. Module adapters should translate graph inputs, parameters, and events into calls on those DSP structs.

Alternative considered: let each DSP type understand graph ports directly. That reduces adapter code, but it couples reusable DSP to patch/runtime concerns.

### Preallocate realtime resources

Realtime render calls should reuse event buffers, scratch audio/control/event storage, module states, and output buffers allocated during preparation. Temporary allocations in non-realtime preparation, validation, and offline rendering are acceptable unless they are on a shared realtime render path.

Alternative considered: optimize allocations later. The current proof is acceptable, but establishing the boundary now prevents new render-path allocations from spreading.

## Risks / Trade-offs

- Broad refactor can hide behavior regressions -> Preserve behavior with focused tests before each move, plus offline/realtime parity and FFI smoke tests.
- Public API narrowing may break internal tests or binaries -> Migrate call sites in small steps and keep intentional facade exports.
- Typed module dispatch may duplicate existing registry data during migration -> Use an intermediate conversion layer and remove duplicate string dispatch after parity tests pass.
- Compiled patch buffer layout may constrain future routing features -> Keep compilation data explicit and extendable rather than optimizing prematurely.
- Realtime allocation guarantees are easy to weaken accidentally -> Add tests/acceptance checks around prepared capacities and render-path reuse where feasible.

## Migration Plan

1. Add characterization tests for current public facade, FFI loading/rendering, offline render determinism, and realtime prepared-capacity behavior.
2. Move FFI functions behind a dedicated module while preserving exported symbol names.
3. Narrow `lib.rs` exports to the intended facade and update internal module visibility.
4. Split or rename engine/runtime/fallback synth responsibilities without changing audio behavior.
5. Extend compilation to carry the routing, traversal, scope, module kind/config, and buffer metadata needed by renderers.
6. Migrate offline and realtime rendering to consume `CompiledPatch`.
7. Introduce typed module dispatch and module state creation from compiled module kinds.
8. Move DSP/module adapter boundaries into clearer modules if useful after behavior is covered.
9. Remove obsolete duplicate routing/traversal/string-dispatch code.
10. Run Rust tests, CMake/CTest, and OpenSpec validation before marking implementation tasks complete.

Rollback is straightforward while the refactor is done in small commit-sized steps: keep the existing tests passing at each step, and revert only the most recent focused change if a migration step proves wrong.

## Open Questions

- Should the temporary hardcoded fallback synth remain as a named test/default instrument, or should it be converted into a normal patch once graph loading is mature enough?
- Should typed module dispatch use one central `ModuleKind` enum, per-family enums, or a registry-owned factory API?
- Should realtime event queue implementation remain homegrown initially or adopt a small well-tested SPSC/ring-buffer crate?
