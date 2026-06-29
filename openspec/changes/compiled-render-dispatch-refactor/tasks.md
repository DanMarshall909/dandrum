## 1. Trait and Shared Dispatch

- [x] 1.1 Define `ModuleInputProvider` trait with methods: `sum_audio_input`, `sum_control_input`, `control_input_or_default`. (Events passed separately — raw path needs `ModuleNode`.)
- [x] 1.2 Implement the trait for `&Routing` (wraps existing raw graph helpers).
- [x] 1.3 Create `CompiledInputProvider` struct wrapping `&CompiledPatch`; implement the trait (wraps existing compiled helpers).
- [x] 1.4 Write `process_module` function that takes `&impl ModuleInputProvider` and `events_in: &[BlockEvent]`, contains the single `match module_type` dispatch.
- [x] 1.5 Write `collect_audio_output` helper extracted from the identical output-extraction block.

## 2. Refactor `process_block`

- [x] 2.1 Replace the inline match dispatch in `process_block` with a call to `process_module` using `&routing as &ModuleInputProvider`.
- [x] 2.2 Replace the inline output-extraction block with `collect_audio_output`.
- [x] 2.3 Verify `cargo test` passes with identical output.

## 3. Refactor `process_block_compiled`

- [x] 3.1 Replace the closure-based match dispatch with a call to `process_module` using a `CompiledInputProvider`.
- [x] 3.2 Replace the inline output-extraction block with `collect_audio_output`.
- [x] 3.3 Verify `cargo test` passes with identical output.

## 4. Refactor `process_block_polyphonic`

- [x] 4.1 Replace the inline match dispatch in the voice phase with `process_module` using `&routing`.
- [x] 4.2 Replace the inline match dispatch in the global phase with `process_module` using `&routing` (same `ModuleInputProvider` impl).
- [x] 4.3 Replace the inline output-extraction block with `collect_audio_output`.
- [x] 4.4 Verify `cargo test` passes with identical output.

## 5. Cleanup

- [x] 5.1 Remove the now-unused `process_node` closure in `process_block_compiled` (already removed with function body replacement).
- [x] 5.2 Remove any input-gathering helpers that are no longer directly called (still used via trait impls — kept).
- [x] 5.3 Verify no dead code warnings (`cargo build` with `-D warnings` passes cleanly).

## 6. Verification

- [x] 6.1 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml` — all tests pass (171/171).
- [x] 6.2 Run `scripts/check-rust-coverage` — passes.
- [x] 6.3 Run `ctest --test-dir build` — passes (100%).
- [x] 6.4 Run `openspec validate compiled-render-dispatch-refactor --strict` — passes.
