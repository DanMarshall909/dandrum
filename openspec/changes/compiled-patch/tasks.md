## 1. Module and Type Definitions

- [ ] 1.1 Create `compiled_patch.rs` with public type scaffolding: `CompiledPatch`, `CompiledNode`, `CompiledPortRef`
- [ ] 1.2 Add `ExecutionStep` type alias and `CompileError` enum (variants: `MissingPort`, `CycleDetected`, `UnknownModuleType`)
- [ ] 1.3 Define `CompiledNode::input_port_map` as `Vec<Vec<CompiledPortRef>>` and avoid storing string-keyed routing maps in `CompiledPatch`
- [ ] 1.4 Register `compiled_patch` module in `lib.rs`
- [ ] 1.5 Add getter methods for `CompiledPatch` fields (nodes, execution_order, voice_node_indices, global_node_indices, render_settings)

## 2. Compile Function

- [ ] 2.1 Implement `compile(graph: &Graph, render_settings: &RenderSettings) -> Result<CompiledPatch, CompileError>` as a free function
- [ ] 2.2 Implement compile-local topological sort (Kahn's algorithm) in `compiled_patch.rs` without moving `graph_processor.rs` internals
- [ ] 2.3 Build `execution_order` from topological sort; detect cycles and return `CompileError::CycleDetected`
- [ ] 2.4 Separate sorted nodes into `global_node_indices` and `voice_node_indices`; set final `execution_order` to globals first, voices at the end
- [ ] 2.5 Copy `RenderSettings` into `CompiledPatch`

## 3. Port Resolution

- [ ] 3.1 Build per-node `input_port_map: Vec<Vec<CompiledPortRef>>` by walking cables and resolving port names to indices
- [ ] 3.2 Validate that every cable source/destination port name exists on its module; return `CompileError::MissingPort` on mismatch
- [ ] 3.3 Store `output_port_map: Vec<usize>` per compiled node (maps output port index to local position)
- [ ] 3.4 Preserve `ModuleId` and `module_type` from source `ModuleNode` in each `CompiledNode`
- [ ] 3.5 Pre-size compile-time vectors from graph module, port, and cable counts where straightforward

## 4. Tests

- [ ] 4.1 Test: nodes are compiled in dependency order (linear chain)
- [ ] 4.2 Test: disconnected modules all appear exactly once in execution order
- [ ] 4.3 Test: graph with cycle returns `CycleDetected`
- [ ] 4.4 Test: unknown source port returns `MissingPort`
- [ ] 4.5 Test: unknown destination port returns `MissingPort`
- [ ] 4.6 Test: valid ports compile successfully with correct `CompiledPortRef` values
- [ ] 4.7 Test: voice-scoped nodes appear only in `voice_node_indices`
- [ ] 4.8 Test: global-scoped nodes appear only in `global_node_indices`
- [ ] 4.9 Test: mixed voice/global graph separates correctly and voice nodes are at the end of `execution_order`
- [ ] 4.10 Test: compiled patch preserves all `ModuleId` values
- [ ] 4.11 Test: `render_settings` are preserved in `CompiledPatch`
- [ ] 4.12 Test: compiled routing uses Vec-based `CompiledPortRef` collections rather than string-keyed maps

## 5. Verification

- [ ] 5.1 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`
- [ ] 5.2 Run `openspec validate compiled-patch --strict`
