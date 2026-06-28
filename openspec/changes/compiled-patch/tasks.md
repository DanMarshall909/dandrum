## 1. Module and Type Definitions

- [x] 1.1 Create `compiled_patch.rs` with public type scaffolding: `CompiledPatch`, `CompiledNode`, `CompiledPortRef`
- [x] 1.2 Add `ExecutionStep` type alias and `CompileError` enum (variants: `MissingPort`, `CycleDetected`, `UnknownModuleType`)
- [x] 1.3 Define `CompiledNode::input_port_map` as `Vec<Vec<CompiledPortRef>>` and avoid storing string-keyed routing maps in `CompiledPatch`
- [x] 1.4 Register `compiled_patch` module in `lib.rs`
- [x] 1.5 Add getter methods for `CompiledPatch` fields (nodes, execution_order, voice_node_indices, global_node_indices, render_settings)

## 2. Compile Function

- [x] 2.1 Implement `compile(graph: &Graph, render_settings: &RenderSettings) -> Result<CompiledPatch, CompileError>` as a free function
- [x] 2.2 Implement compile-local topological sort (Kahn's algorithm) in `compiled_patch.rs` without moving `graph_processor.rs` internals
- [x] 2.3 Build `execution_order` from topological sort; detect cycles and return `CompileError::CycleDetected`
- [x] 2.4 Separate sorted nodes into `global_node_indices` and `voice_node_indices`; set final `execution_order` to globals first, voices at the end
- [x] 2.5 Copy `RenderSettings` into `CompiledPatch`

## 3. Port Resolution

- [x] 3.1 Build per-node `input_port_map: Vec<Vec<CompiledPortRef>>` by walking cables and resolving port names to indices
- [x] 3.2 Validate that every cable source/destination port name exists on its module; return `CompileError::MissingPort` on mismatch
- [x] 3.3 Store `output_port_map: Vec<usize>` per compiled node (maps output port index to local position)
- [x] 3.4 Preserve `ModuleId` and `module_type` from source `ModuleNode` in each `CompiledNode`
- [x] 3.5 Pre-size compile-time vectors from graph module, port, and cable counts where straightforward

## 4. Tests

- [x] 4.1 Test: nodes are compiled in dependency order (linear chain)
- [x] 4.2 Test: disconnected modules all appear exactly once in execution order
- [x] 4.3 Test: graph with cycle returns `CycleDetected`
- [x] 4.4 Test: unknown source port returns `MissingPort`
- [x] 4.5 Test: unknown destination port returns `MissingPort`
- [x] 4.6 Test: valid ports compile successfully with correct `CompiledPortRef` values
- [x] 4.7 Test: voice-scoped nodes appear only in `voice_node_indices`
- [x] 4.8 Test: global-scoped nodes appear only in `global_node_indices`
- [x] 4.9 Test: mixed voice/global graph separates correctly and voice nodes are at the end of `execution_order`
- [x] 4.10 Test: compiled patch preserves all `ModuleId` values
- [x] 4.11 Test: `render_settings` are preserved in `CompiledPatch`
- [x] 4.12 Test: compiled routing uses Vec-based `CompiledPortRef` collections rather than string-keyed maps

## 5. Verification

- [x] 5.1 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`
- [x] 5.2 Run `openspec validate compiled-patch --strict`
