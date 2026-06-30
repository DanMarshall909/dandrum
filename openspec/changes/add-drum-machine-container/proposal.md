## Why

Patch authors need a compact way to name and organize drum-style trigger pads without turning the engine into a pattern sequencer. Inspired by Bitwig's Drum Machine, the container should route incoming note/event triggers to specific pad chains while leaving sequencing, synthesis, sampling, effects, and mixing policy to ordinary modules.

## What Changes

- Add a `drum_machine` container module type that expands into ordinary graph trigger-routing modules and pad child chains before rendering.
- Allow YAML patches to declare named pads with trigger selectors, typically note numbers or event labels, plus optional pad metadata.
- Route compatible incoming events to the matching pad chain input and expose each pad's event output for external routing.
- Allow each pad to host an explicit child module chain that receives the pad trigger event.
- Keep audio generation, sample playback, sequencing, transport, clocking, and mixing outside the container.
- Include diagnostics for invalid pad IDs, duplicate trigger selectors, missing public pad ports, and incompatible routes.

## Capabilities

### New Capabilities
- `drum-machine-container`: Drum-machine pad container declaration, trigger selector validation, graph expansion, and event routing behavior.

### Modified Capabilities
- `yaml-patch-format`: Allow patch YAML to declare drum-machine pad containers with trigger selectors and optional pad child chains.
- `modular-routing-graph`: Define graph validation behavior for expanded drum-machine pad event ports and child-chain routes.

## Impact

- Rust engine patch model, YAML parsing, validation, and diagnostics.
- Graph construction/expansion path before offline or realtime processor construction.
- Event routing behavior for named drum-machine pads.
- Example patch fixtures and CLI render acceptance coverage that prove the container routes triggers to pad chains rather than sequencing them itself.
