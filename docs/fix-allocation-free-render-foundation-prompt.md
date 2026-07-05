# Prompt: Fix Allocation-Free Render Foundation

You are working in `DanMarshall909/dandrum`.

Goal: fix the issues introduced in the allocation-free realtime render foundation work. Do not expand scope beyond making the existing spec/implementation honest, compiling, and aligned with the repo’s `#![deny(dead_code)]` discipline.

## Context

The repo contains OpenSpec change `openspec/changes/eliminate-realtime-callback-allocations`.

Recent work added:

- `src/rust-engine/src/graph_processor/render_plan.rs`
- `src/rust-engine/src/graph_processor/audio_arena.rs`
- `src/rust-engine/src/graph_processor/realtime_allocation_tests.rs`
- task updates marking 1.1, 2.1, 2.2, 3.1, 3.2 complete

Review found the task marking and implementation are premature.

## Required Fixes

### 1. Remove `#![allow(dead_code)]` escape hatches

Do not hide unused infrastructure with module-level `allow(dead_code)`.

The repo intentionally has `#![deny(dead_code)]`.

Either wire the new types into live code enough that they are used, or narrow/remove unused pieces.

Prefer minimal integration over speculative unused code.

### 2. Fix `AudioArena` sizing semantics

The spec/task says the arena is sized by max block size, compiled buffer count, and max voice count.

Current `AudioArena::new` only allocates `frames * buffer_count`.

Either:

1. make `AudioArena` genuinely voice-aware, with storage sized by `frames * buffer_count * max_voices`, and expose APIs that make the voice dimension explicit; or
2. adjust the task/spec wording and task checkbox if the current arena is only mono/global.

Preferred: implement voice-aware storage now, but keep it simple.

### 3. Do not mark tasks complete unless they are actually complete

Reopen task 3.1 if the arena is not fully sized/usable for max voices.

Reopen task 3.2 if default control buffer handling is not truly implemented.

Task checkboxes must reflect reality.

Do not claim live realtime renderer migration has happened unless it has.

### 4. Fix render plan event routing honesty

Current `RenderPlan` creates event queue IDs but does not represent event edges from source event outputs to destination event inputs.

Either add explicit event edges/routing metadata, or leave task 2.2 incomplete.

Preferred: add a minimal `CompiledEventEdge { source: EventQueueId, destination: EventQueueId }` and include it in `RenderStep` or `RenderPlan`, derived from `CompiledPatch::input_port_map`.

### 5. Fix imports and likely compile issues

`builtin_ports` lives under `crate::graph`, not `crate::builtins`.

Check all new files for incorrect imports.

Run:

```bash
cargo fmt
$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml
```

Fix all compile/test failures.

### 6. Improve `AudioArena::add_edge`

It must not allocate. Do not use `.to_vec()` in the realtime path.

The current split-at implementation is acceptable if it compiles and is tested.

Keep the self-edge assertion unless self-routing is valid by design. If valid, handle it explicitly.

### 7. Add or keep tests, but make them honest

Capacity tests are fine for task 1.1.

Do not mark task 1.2 complete unless there is an actual allocation counter/test allocator or equivalent.

Tests should cover:

- arena capacity includes voice count if voice-aware;
- edge summing does not require cloned source buffers;
- render plan event edges are derived for an event connection;
- existing realtime capacity tests still pass.

### 8. Keep scope tight

Do not migrate all modules yet.

Do not redesign DSP functions.

Do not change YAML format.

Do not add speculative structs “for later”.

Every new type/function must either be used now or only exist under tests in a way that satisfies `deny(dead_code)`.

## Expected Final State

- Branch compiles.
- Tests pass.
- No module-level `allow(dead_code)` added to bypass unused code.
- OpenSpec task checkboxes accurately reflect implemented work.
- Render plan and arena are either genuinely integrated enough to avoid dead code, or reduced to only what is currently used.
- Commit message should be focused, for example: `Fix allocation-free render plan foundation`.
