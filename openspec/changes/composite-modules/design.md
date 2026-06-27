## Context

The engine currently parses patch YAML into assets, modules, ports, connections, and render settings. Built-in modules are registered in Rust, while `script` modules can declare ad hoc ports. This is enough for small patches, but larger instruments need reusable subsystems such as sample voice chains, drum cells, or modulation blocks without turning every subsystem into a Rust built-in.

Composite modules should stay a patch/YAML feature. The engine should parse YAML module definitions, validate them, and expand instances into the existing graph model before rendering. The graph processor should continue to process ordinary modules and cables rather than learning a second nested execution model.

## Goals / Non-Goals

**Goals:**

- Define reusable composite module types in YAML.
- Give each composite explicit public typed input and output ports.
- Allow a composite to contain existing built-ins, scripts, and other non-recursive composites.
- Bind public ports to internal module ports through explicit YAML mappings.
- Bind instance parameters/assets to internal module parameters/assets through explicit YAML mappings.
- Validate composite definitions before graph expansion with clear diagnostics.
- Expand composite instances into a flat graph that existing validation and processors can use.

**Non-Goals:**

- Adding a Rust plugin API for user-defined modules.
- Adding a new runtime scheduler for nested graphs.
- Hiding implicit mixers or implicit feedback boundaries inside composites.
- Supporting recursive or dynamically generated module definitions.
- Changing the pure signal-generator contract for oscillator or sampler modules.

## Decisions

- Composite module definitions are YAML, not Rust built-ins.
  - Rationale: The user-facing need is patch composition. YAML keeps subsystems inspectable, shareable, and versionable with patches.
  - Alternative considered: Register composites from Rust code. That would solve reuse but would not let patch authors define their own subsystems.

- Use explicit `module_definitions` in patch YAML, plus optional included YAML libraries later.
  - Rationale: In-patch definitions are the smallest complete feature. Includes can reuse the same schema without changing expansion semantics.
  - Alternative considered: Put definitions under `assets`. That overloads asset loading with graph schema concerns.

- Expand composite instances into a flat graph before processing.
  - Rationale: Existing graph validation already handles typed ports, many-to-one routing, and feedback boundaries. Reusing it avoids a nested processor path.
  - Alternative considered: Execute composites as nested processors. That adds scheduling and buffering complexity before it is needed.

- Public ports are declared separately from internal ports and mapped explicitly.
  - Rationale: This preserves abstraction boundaries while making the exposed subsystem contract clear.
  - Alternative considered: Auto-expose every unconnected internal port. That is convenient but leaks internals and makes refactoring definitions breaking.

- Composite instances may override declared parameters and asset bindings only through definition-declared public parameters.
  - Rationale: This keeps instances from reaching through the abstraction boundary into arbitrary internal module fields.
  - Alternative considered: Allow `module.parameter` paths from instances. That is flexible but couples instances to internals.

- Recursive composite definitions are invalid.
  - Rationale: Static expansion needs an acyclic definition graph and predictable diagnostics.
  - Alternative considered: Allow recursion with runtime limits. That is surprising for audio graphs and not needed for subsystems.

## Risks / Trade-offs

- YAML schema could become verbose -> Keep the first schema explicit and add sugar only after tests show repeated boilerplate.
- Expansion diagnostics can reference generated internal IDs that users did not write -> Preserve source composite/instance/module IDs in diagnostics.
- Flattening can make graph IDs long -> Use deterministic namespacing such as `voice1::env` internally while keeping user diagnostics readable.
- Includes may introduce path/security concerns -> Start with in-patch definitions; add includes later with path resolution tests.
- Composite abstraction could hide invalid feedback or implicit many-to-one routing -> Expand then run the same graph validation rules, and validate public mappings before expansion.
