## 1. Characterization And Safety Net

- [x] 1.1 Add or update Rust tests that characterize the current public engine facade, FFI create/destroy/load/render behavior, invalid FFI inputs, and existing fallback synth behavior.
- [x] 1.2 Add or update tests that verify valid patches prepare successfully and invalid schema, graph, asset, or compile failures do not replace live runtime state.
- [x] 1.3 Add or update offline/realtime parity tests for representative graph patches so later compilation/runtime refactors can preserve routing and execution behavior.
- [x] 1.4 Add or update realtime preparation tests that verify configured maximum block size, voice count, event capacity, and scratch capacities are established before rendering.

## 2. Public Facade And FFI Boundary

- [x] 2.1 Move C ABI functions from `lib.rs` into a dedicated FFI module while preserving exported symbol names and existing C++ binding behavior.
- [x] 2.2 Introduce a small safe Rust facade for loading/preparing instruments, rendering audio, submitting events, and querying completion.
- [x] 2.3 Narrow crate root exports so graph processor, runtime helper, and DSP implementation modules are `pub(crate)` unless intentionally exposed.
- [x] 2.4 Update Rust tests, Rust binaries, and JUCE wrapper-facing code to use the facade or FFI boundary rather than private implementation modules.

## 3. Engine Responsibility Split

- [x] 3.1 Split the current `core::Engine` and `synth::DandrumEngine` responsibilities into clearly named headless core/facade, prepared instrument runtime, and fallback/default synth components.
- [x] 3.2 Preserve existing default/fallback audio behavior with tests while removing graph-runtime responsibilities from the fallback synth component.
- [x] 3.3 Ensure CLI and FFI entry points construct the same headless runtime path instead of duplicating preparation logic.

## 4. Prepared Patch Pipeline

- [x] 4.1 Introduce a prepared patch or prepared instrument type that owns validated patch metadata, graph, compiled patch metadata, prepared assets, and preparation diagnostics.
- [x] 4.2 Move patch file loading, schema validation, graph validation, asset preparation, and compilation into explicit pipeline functions with typed error results.
- [x] 4.3 Ensure failed preparation leaves the previous runtime state untouched and reports failure through safe Rust and FFI paths.
- [x] 4.4 Update offline rendering to start from prepared patch data where possible, while preserving existing CLI behavior.

## 5. Compiled Patch As Shared Runtime Contract

- [x] 5.1 Extend `CompiledPatch` to include routing, traversal, scope grouping, MIDI/audio output indices, module kind/config metadata, and port/buffer layout needed by renderers.
- [x] 5.2 Migrate realtime graph processor construction to consume `CompiledPatch` instead of rebuilding routing and traversal from raw `Graph`.
- [x] 5.3 Migrate offline rendering to consume `CompiledPatch` for routing and execution order.
- [x] 5.4 Remove or make private obsolete duplicate routing/traversal helpers after compiled-patch render paths are covered by tests.

## 6. Typed Module Dispatch

- [x] 6.1 Introduce typed module kind/config structures resolved from built-in registry data and patch parameters before rendering.
- [x] 6.2 Migrate module state creation from raw module type string matching to typed module kind/config dispatch.
- [x] 6.3 Migrate render-time module dispatch from raw module type string matching to typed module kind dispatch.
- [x] 6.4 Add tests that unsupported module types fail during preparation or compilation before rendering starts.

## 7. DSP And Module Adapter Boundary

- [ ] 7.1 Identify DSP modules that are currently coupled to graph/runtime concerns and add focused tests proving they can be instantiated without patch or graph setup.
- [ ] 7.2 Move graph input/port translation into module adapter code while keeping DSP algorithms independent from YAML, graph IDs, cables, FFI, CLI, and frontend APIs.
- [ ] 7.3 Reorganize modules only after behavior is covered, preserving existing DSP test coverage and adding coverage for newly extracted modules.

## 8. Realtime Allocation Discipline

- [ ] 8.1 Preallocate realtime event, scratch audio/control/event, module output, and per-voice state capacity during preparation based on maximum block size and voice allocation.
- [ ] 8.2 Update render paths to reuse prepared buffers for repeated blocks no larger than the prepared maximum block size.
- [ ] 8.3 Add tests or checks that repeated realtime renders do not grow scratch capacity or replace runtime state for prepared-size blocks.
- [ ] 8.4 Evaluate whether a small fixed-capacity SPSC/ring-buffer dependency is warranted; document the decision in the design if one is added.

## 9. Verification And Documentation

- [ ] 9.1 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml` and fix any regressions.
- [ ] 9.2 Configure/build with `$HOME/.local/bin/cmake -S . -B build` and `$HOME/.local/bin/cmake --build build`.
- [ ] 9.3 Run `ctest --test-dir build` and fix any regressions.
- [ ] 9.4 Run `openspec validate refactor-rust-engine-architecture --strict` and fix any spec or task validation errors.
- [ ] 9.5 Document any unavoidable verification gaps before marking tasks complete.
