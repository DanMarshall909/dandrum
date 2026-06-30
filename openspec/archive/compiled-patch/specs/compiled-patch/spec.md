## ADDED Requirements

### Requirement: Compilation produces a valid execution order

The compile function SHALL produce a `CompiledPatch` whose `execution_order` lists every module node index exactly once. Global-scoped nodes SHALL appear before voice-scoped nodes, and dependencies SHALL appear before dependents within each legal scope ordering.

#### Scenario: Nodes are ordered after their dependencies
- **WHEN** a graph has module A connected to module B connected to module C
- **THEN** A appears before B before C in `execution_order`

#### Scenario: Graph with no connections includes every node
- **WHEN** a graph has disconnected modules X, Y, Z
- **THEN** all modules appear exactly once in `execution_order`

#### Scenario: Voice nodes compile at the end
- **WHEN** a graph has global-scoped and voice-scoped modules
- **THEN** every global-scoped module appears before every voice-scoped module in `execution_order`

#### Scenario: Cycle detection rejects invalid graphs
- **WHEN** a graph contains a cycle
- **THEN** compilation returns `CompileError::CycleDetected`

### Requirement: Compilation validates port references

The compile function SHALL verify that every cable source and destination port exists on the connected module. If a referenced port is missing, compilation SHALL fail with a descriptive error.

#### Scenario: Unknown source port is rejected
- **WHEN** a cable references a source port that the source module does not declare
- **THEN** compilation returns `CompileError::MissingPort`

#### Scenario: Unknown destination port is rejected
- **WHEN** a cable references a destination port that the destination module does not declare
- **THEN** compilation returns `CompileError::MissingPort`

#### Scenario: Valid ports compile successfully
- **WHEN** all cable ports match declared ports on their respective modules
- **THEN** compilation succeeds

### Requirement: Voice and global nodes are separated into distinct lists

The `CompiledPatch` SHALL separate `voice_node_indices` and `global_node_indices` according to each node's `ExecutionScope`.

#### Scenario: Voice-scoped nodes appear only in voice list
- **WHEN** a graph contains voice-scoped modules
- **THEN** those module indices appear in `voice_node_indices` and NOT in `global_node_indices`

#### Scenario: Global-scoped nodes appear only in global list
- **WHEN** a graph contains global-scoped modules
- **THEN** those module indices appear in `global_node_indices` and NOT in `voice_node_indices`

#### Scenario: Mixed graph separates correctly
- **WHEN** a graph has both voice-scoped and global-scoped modules
- **THEN** `voice_node_indices` contains only voice nodes and `global_node_indices` contains only global nodes

#### Scenario: Voice list follows final execution order
- **WHEN** a graph has multiple voice-scoped modules
- **THEN** `voice_node_indices` matches the voice-scoped suffix of `execution_order`

### Requirement: Compiled patch preserves module IDs

Every `CompiledNode` in the `CompiledPatch` SHALL retain the original `ModuleId` from the source graph.

#### Scenario: Module IDs match after compilation
- **WHEN** a graph has modules with specific IDs
- **THEN** each `CompiledNode::id` matches the corresponding `ModuleNode::id` from the input graph

### Requirement: Compiled port references are index-based

Input port connections in `CompiledNode::input_port_map` SHALL use `CompiledPortRef` values that reference source nodes by index and output port by index, not by string name.

#### Scenario: Port reference resolves to correct source
- **WHEN** a cable connects module A output port `audio` to module B input port `audio_in`
- **THEN** `CompiledNode` for B has an `input_port_map` entry at the `audio_in` port index containing `CompiledPortRef` with A's module index and the index of A's `audio` output port

#### Scenario: Compiled routing stores no string-keyed maps
- **WHEN** a graph is compiled successfully
- **THEN** the resulting `CompiledPatch` exposes routing as Vec-based `CompiledPortRef` collections, not string-keyed lookup tables

### Requirement: Render settings are preserved

The `CompiledPatch` SHALL store a copy of the `RenderSettings` used during compilation.

#### Scenario: Render settings accessible from compiled patch
- **WHEN** a graph is compiled with specific render settings
- **THEN** `CompiledPatch::render_settings` matches the input settings
