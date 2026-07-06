## Why

Some users will want to use their own plugin library, such as a FabFilter effect, inside a Dandrum patch rather than only the built-in modules. A host-backed plugin module would let Dandrum keep owning the graph and scheduling while delegating external plugin execution to a dedicated boundary.

## What Changes

- Add a plugin-host module capability that can load and run user-selected external plugins inside the engine graph.
- Keep plugin hosting at the edge of the runtime rather than mixing plugin SDK concerns into built-in DSP modules.
- Expose hosted plugins as normal graph modules with typed audio/control/event ports where supported.
- Treat plugin discovery, load failure, state save/restore, and latency as explicit host behavior.

## Capabilities

### New Capabilities

- `plugin-host-module`: Defines how a patch can host a user-selected external plugin instance as a graph module.

### Modified Capabilities

- `rust-engine-architecture`: The runtime boundary and preparation pipeline will need to account for hosted plugin instances, their state, and their render-time constraints.
- `built-in-modules`: The module registry and validation surface will need to distinguish hosted plugin modules from built-in DSP modules.

## Impact

- Rust engine preparation, graph validation, and runtime dispatch.
- JUCE wrapper/plugin loading and audio-device integration.
- Patch syntax or metadata for hosted plugin references.
- Save/restore and latency reporting for hosted plugin instances.
