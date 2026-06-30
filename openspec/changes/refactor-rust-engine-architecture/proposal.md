## Why

The Rust engine has grown from a proof-of-concept into a modular instrument runtime, but its architectural boundaries are still soft: the crate root exposes most internals, the C ABI mixes loading/validation/runtime concerns, and realtime/offline paths still duplicate graph preparation work. Tightening these boundaries now will make future module, container, and realtime work easier to test without changing the patch authoring model.

## What Changes

- Introduce explicit Rust engine layers for public facade, FFI boundary, patch preparation, graph compilation, runtime execution, DSP algorithms, and frontend/CLI adapters.
- Narrow the public Rust API so implementation modules are crate-private unless intentionally exposed.
- Move C ABI functions out of the crate root into an FFI boundary that delegates to safe Rust APIs.
- Split the current engine concepts into clearly named headless core, prepared instrument/runtime, and any temporary fallback/default synth behavior.
- Make compiled patch data the shared handoff between validation/preparation and both offline and realtime rendering.
- Replace runtime string dispatch for built-in modules with typed module kinds/configuration resolved during preparation or compilation.
- Move toward allocation-free realtime rendering by preallocating scratch buffers and event capacity during prepare/load.
- Preserve existing patch YAML semantics and externally observable audio behavior unless a later capability change explicitly says otherwise.

## Capabilities

### New Capabilities
- `rust-engine-architecture`: Defines architectural contracts for the Rust engine crate, including public/private API boundaries, FFI delegation, prepared/compiled patch handoff, typed module dispatch, shared offline/realtime execution, and realtime allocation expectations.

### Modified Capabilities
- None.

## Impact

- Affected Rust files: `src/rust-engine/src/lib.rs`, `src/rust-engine/src/core.rs`, `src/rust-engine/src/synth.rs`, `src/rust-engine/src/compiled_patch.rs`, `src/rust-engine/src/graph_processor/**`, `src/rust-engine/src/builtins.rs`, DSP modules, CLI entry points, and FFI-facing tests.
- Affected C++/C ABI consumers: JUCE wrapper bindings should continue to call the same stable exported functions unless this change explicitly updates them with tests.
- Testing impact: requires focused Rust unit tests around each architectural boundary, existing offline/realtime render parity tests where applicable, FFI smoke coverage, and normal CMake/CTest verification before tasks are marked complete.
- No new third-party dependencies are required by default; a fixed-capacity realtime queue dependency may be considered only if it materially improves safety and keeps the CMake/Cargo build simple.
