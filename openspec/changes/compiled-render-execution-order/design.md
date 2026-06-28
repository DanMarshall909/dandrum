## Context

`CompiledPatch` stores three ordering views:
- `execution_order`: globals-first, then voices (intentional, per spec)
- `global_node_indices`: global-scoped node indices in dependency order
- `voice_node_indices`: voice-scoped node indices in dependency order

`process_block_compiled` blindly iterates `execution_order`. When a voice-scoped module produces audio consumed by a global module (e.g., oscillator → mixer → output), the global runs first and gets default/silent input because the voice hasn't computed yet.

The raw polyphonic path (`process_block_polyphonic`, `graph_processor.rs:1034-1278`) already solves this by separating topology into `voice_seq` and `global_seq`, processing voices first with per-voice accumulation, then processing globals against the accumulated map. The compiled path has the same information already stored (`voice_node_indices`, `global_node_indices`) but doesn't use it correctly.

## Goals / Non-Goals

**Goals:**
- Fix compiled offline rendering so voice producers are processed before global consumers.
- Keep `CompiledPatch::execution_order` field, `global_node_indices`, `voice_node_indices` unchanged.
- Keep `compile()` unchanged — globals-first remains the spec.
- Remove the public `execution_order()` getter so external code cannot accidentally flat-iterate globals-first order. Renderers must use `voice_node_indices()` / `global_node_indices()`.
- Add a parity test that fails before the fix and passes after.
- Add a guard test proving `execution_order` stays globals-first after the fix.
- Keep all existing parity and unit tests passing.

**Non-Goals:**
- Do not change the `CompiledPatch` data structure's private fields or `compile()` function.
- Do not change raw `Graph` rendering paths (`process_block`, `process_block_polyphonic`).
- Do not change JUCE, FFI, YAML, dependencies, CMake, or threading.
- Do not implement polyphonic compiled rendering — this change is for the existing monophonic compiled path only.
- Do not change the spec for `execution_order`.

## Decisions

### Decision: Keep `execution_order` globals-first, fix renderer

**Decision**: Keep `CompiledPatch::execution_order` as globals-first (unchanged spec). Fix `process_block_compiled` to use a two-phase iteration over the already-separated `voice_node_indices` and `global_node_indices`.

**Rationale**: The scope ordering is intentional metadata that documents the voice/global boundary. Changing it to pure dependency order would lose this signal and require re-spec. The renderer should be correct regardless of execution order's arrangement — it has the information it needs via the separate index lists.

Alternatives considered:
- Change `execution_order` to pure dependency order: rejected — loses scope metadata, breaks spec, and doesn't actually solve the problem (polyphonic rendering still needs two-phase).
- Keep flat iteration but reorder modules in graph: rejected — would violate scope semantics.

### Decision: Two-phase voice-then-global in `process_block_compiled`

**Decision**: `process_block_compiled` will:
1. Seed external/MIDI events into `all_outputs` (as today).
2. Iterate `compiled.voice_node_indices()` in order, storing each module's outputs in `all_outputs`.
3. Iterate `compiled.global_node_indices()` in order, with each module reading from `all_outputs` (including voice outputs that were accumulated in step 2).
4. Read the final `audio_output` module's outputs from `all_outputs` (as today).

This matches the proven pattern in `process_block_polyphonic` but without per-voice state arrays or voice allocator concerns (single voice/no polyphony in the compiled path).

**Rationale**: The raw monophonic `process_block` uses a single flat iteration of `topo_order` — but that `topo_order` is pure dependency order, not globals-first. The compiled path's `execution_order` is different (globals-first), so the compiled path cannot safely flat-iterate. Two-phase is the minimal correct change.

### Decision: Remove public `execution_order()` getter

**Decision**: Remove the `pub fn execution_order()` getter from `CompiledPatch`. The field remains as a private struct member for internal bookkeeping, but no code outside `compiled_patch.rs` can iterate the flat globals-first order.

**Rationale**: The getter had one external caller — `process_block_compiled` at `graph_processor.rs:468` — which was the exact line that caused the bug. Removing the getter makes this class of bug a compile error forever: any future render path must explicitly choose `voice_node_indices()` or `global_node_indices()`, forcing correct scope ordering at the call site.

Alternatives considered:
- Rename to `scope_ordered_indices()`: rejected — convention is not enforcement.
- Return a non-iterable wrapper: rejected — adds complexity for a single getter that should not exist.

Tests in `compiled_patch.rs` access the field directly (same module, private field visibility permits this).

### Decision: Use existing `voice_node_indices` / `global_node_indices`

**Decision**: Reuse the already-computed scope-split lists rather than recomputing or filtering `execution_order` at render time.

**Rationale**: These lists are already stored in `CompiledPatch` for exactly this purpose. Filtering `execution_order` at render time would be redundant and slower.

### Decision: No spec-level changes

**Decision**: No new capability spec file is needed. The behavioral change (voice→global routing in compiled path) is a renderer bug fix, not a new spec requirement. All existing spec-level invariants — globals-first `execution_order`, voice/global separation, port validation, render settings preservation — remain unchanged.

**Rationale**: The spec describes what `compile()` guarantees. That contract is unchanged. The renderer was simply not using the data correctly.

## Risks / Trade-offs

- **[Risk] Removing the `execution_order()` getter limits diagnostic inspection** → Mitigation: the order is trivially reconstructable as `global_node_indices ++ voice_node_indices`. The struct's `Debug` impl can also show it.
- **[Risk] Two-phase adds a second loop over modules** → Mitigation: this is a constant-factor change (one extra iteration over the same total modules) and doesn't affect order of module processing within each scope.
- **[Risk] Existing parity tests may mask edge cases** → Mitigation: the new voice→global parity test explicitly exercises the bug scenario. All three existing parity tests must still pass (they use all-global graphs).
- **[Risk] The monophonic compiled path doesn't have per-voice state** → Mitigation: the compiled path only processes one voice (no `voice_idx` dimension), so accumulation is simply storing into the shared `all_outputs` map — no summing needed.
