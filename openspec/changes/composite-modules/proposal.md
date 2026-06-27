## Why

Patches can currently connect built-in modules, but larger instruments need reusable subsystems made from those same modules without moving behavior into Rust code. YAML-defined composite modules let patches name a subsystem, expose custom typed ports, and instantiate it like any other module while preserving the engine's graph validation rules.

## What Changes

- Add YAML syntax for declaring composite module definitions inside patch documents or included YAML module libraries.
- Allow patches to instantiate a YAML-defined module type with custom public inputs, outputs, parameters, assets, and an internal graph of existing modules.
- Validate composite module boundaries, internal routes, parameter bindings, asset references, and port type compatibility before rendering.
- Expand composite modules into the engine graph for processing while keeping CLI, GUI, plugin, and realtime front ends unaware of the internal structure.
- Reject invalid composites such as recursive definitions, hidden implicit many-to-one routing, and internal feedback that lacks explicit delay or future scheduling boundaries.

## Capabilities

### New Capabilities

- `yaml-composite-modules`: YAML-defined reusable module/subsystem definitions with explicit public typed ports and internal modular graphs.

### Modified Capabilities

None.

## Impact

- Rust patch schema and YAML parsing.
- Graph construction and validation.
- Parameter and asset binding rules.
- Offline and realtime graph processor construction through expanded graphs.
- Examples and tests for reusable YAML subsystems.
