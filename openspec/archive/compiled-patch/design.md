## Context

The engine's `graph_processor.rs` currently performs topological sorting, routing construction, and per-module state initialisation inside `render_offline` and `render_offline_polyphonic`. These are pure-function-of-patch computations: for a given `Graph` and `RenderSettings`, the execution order and routing never change. Repeating them on every render call wastes CPU and prevents the render path from being realtime-safe (allocation, hash-map lookups).

The `Graph` type in `graph.rs` already holds validated modules and cables. Ports are stored as named strings; routing is built as `HashMap<String, Vec<(usize, String)>>` — string keys resolved at render time.

## Goals / Non-Goals

**Goals:**
- Define a `CompiledPatch` data structure that holds a fully resolved execution plan.
- Implement a `compile` function that consumes `Graph` + `RenderSettings` and produces `CompiledPatch`.
- Move topological sort and routing construction into the compilation step.
- Store node execution order as a flat `Vec<usize>` (module indices, not string IDs).
- Store routing as index-based references: `(src_module_idx, src_port_idx)` → `(dst_module_idx, dst_port_idx)`.
- Separate voice-scoped and global-scoped nodes into distinct execution lists, with global nodes compiled before voice nodes in the final execution order.
- Reduce allocation pressure by pre-sizing compile-time collections where graph sizes are known and by storing the final compiled routing in `Vec`-based structures instead of hash maps or string-keyed routing tables.
- Cover each behavioural guarantee with a focused Rust unit test.
- Keep the compile-only path completely independent of the existing render path initially.

**Non-Goals:**
- Rewriting `graph_processor.rs` render internals to consume `CompiledPatch` — that is a follow-up task.
- Adding multithreading or parallel compilation.
- Changing the `PatchDocument` YAML format.
- Changing the C FFI or JUCE integration.
- Adding new dependencies.
- Removing any existing code.
- Guaranteeing zero allocations during compilation. Compilation is not realtime code; the goal is to keep the compiled representation allocation-light and render-friendly.

## Decisions

### 1. Standalone `compile` function vs method on `Engine`

**Decision**: A free function `compiled_patch::compile(graph, render_settings)` that returns `Result<CompiledPatch, CompileError>`.

**Rationale**: The function has no dependencies on `Engine` state (the engine is currently stateless for rendering). A standalone function is more testable and composable. `Engine::compile` can be a thin wrapper later if needed.

### 2. Compiled types and their representations

**`CompiledPatch`** — the top-level compiled artefact:
- `nodes: Vec<CompiledNode>` — pre-resolved node data
- `execution_order: Vec<usize>` — global nodes first, then voice nodes; each group preserves dependency order
- `voice_node_indices: Vec<usize>` — subset of execution_order for voice-scoped nodes
- `global_node_indices: Vec<usize>` — subset of execution_order for global-scoped nodes
- `render_settings: RenderSettings` — copied for convenience

**`CompiledNode`** — a node with pre-resolved port references:
- `id: ModuleId` — stable ID preserved for diagnostics and module-type dispatch
- `module_type: String` — preserved for state creation dispatch
- `execution_scope: ExecutionScope`
- `input_port_map: Vec<Vec<CompiledPortRef>>` — one Vec per input port, with zero or more resolved source references
- `output_port_map: Vec<usize>` — one per output port, maps port index to local index

**`ExecutionStep`** — initially just `usize` (node index). Could become a richer type later; start with a type alias for clarity.

**`CompiledPortRef`** — a resolved connection target:
- `module_index: usize` — index into the `CompiledPatch::nodes` array
- `port_index: usize` — index into the source node's output ports

`CompiledVoiceTemplate` is intentionally deferred until render/polyphony migration needs it. This first pass records `voice_node_indices` and places voice nodes at the end of the compiled execution order.

### 3. Compilation error handling

**Decision**: A `CompileError` enum with variants for:
- `MissingPort { module_id: String, port_name: String }`
- `CycleDetected` — if topological sort fails to include all nodes
- `UnknownModuleType { module_type: String }` (already validated at Graph level, kept for defensive completeness)

Normal callers should pass a validated `Graph`, but compilation still defensively rejects invalid graphs. That keeps `CompiledPatch` construction safe even in unit tests and future internal call sites.

### 4. Topological sort placement

**Decision**: Implement a compile-local topological sort in `compiled_patch.rs` first. Do not move `graph_processor.rs` internals until the render path actually consumes `CompiledPatch`.

**Rationale**: The sort is a compile-time concern, but moving private render code now creates churn without behaviour change. Keeping the new sorter local makes this change easier to review.

### 5. Voice nodes compile at the end

**Decision**: Build a full dependency-valid topological order, then partition it into `global_node_indices` and `voice_node_indices`. The final `execution_order` is `global_node_indices` followed by `voice_node_indices`.

**Rationale**: Direct voice-to-global routing is invalid in the graph model, so placing global nodes before voice nodes preserves legal dependencies while making the global/voice split explicit and easy for the eventual render path to consume.

### 6. Port resolution strategy

Input ports are resolved to `Vec<CompiledPortRef>` by walking cables. For each cable:
- Find source module index and output port index (by name).
- Find destination module index and input port index (by name).
- Push `CompiledPortRef { module_index: src, port_index: out_port_idx }` to the destination's input map at the resolved port index.

This eliminates all string-based lookups at render time — the render loop iterates `execution_order`, processes each node by index, and reads pre-resolved port references.

### 7. Allocation reduction strategy

Compiled data should use `Vec` storage sized from known graph counts:
- `nodes` capacity = `graph.modules().len()`
- routing capacity = per-node input/output counts
- source-reference capacity can be filled by walking cables once

Temporary compile-time lookup maps are acceptable if they keep the implementation small, but the resulting `CompiledPatch` should not store hash maps or string-keyed routing tables.

## Risks / Trade-offs

- **[Risk] Adding a new type layer increases code surface** → Mitigated by keeping the first version minimal and behaviour-preserving. No existing code is modified beyond adding the new module and a re-export.
- **[Risk] Duplicate logic during transition** → Acceptable. The compile and render paths will both compute topological sort and routing until the render path is migrated. This is a deliberate incremental strategy.
- **[Risk] CompiledPatch may need to change when voice/polyphony design evolves** → Mitigated by deferring `CompiledVoiceTemplate` and only committing to voice node ordering plus `voice_node_indices` in this pass.
- **[Trade-off] Index-based references are fragile if node order changes** → The `CompiledPatch` is an opaque internal type produced by compilation and consumed immediately during rendering of a single patch. It is never serialised or cached across patch loads, so index stability is guaranteed within a single compile→render lifecycle.
