## Why

Patch authors need a compact way to name and organize drum-style trigger pads without turning the engine into a pattern sequencer or signal-chain host. Inspired by Bitwig's pad-oriented Drum Machine workflow, the container should map incoming note/event triggers to named pads and emit the event specified for each pad, while synthesis, sampling, effects, and mixing remain explicit patch signal chains outside the container.

## What Changes

- Add a `drum_machine` event-only module type that expands into ordinary graph event-routing modules before rendering.
- Allow YAML patches to declare named pads with trigger selectors, typically note numbers or event labels, plus per-pad emitted-event configuration.
- Route compatible incoming events to matching pads and emit each pad's configured event on its event output.
- Keep audio generation, sample playback, sequencing, transport, clocking, and mixing outside the container.
- Include diagnostics for invalid pad IDs, duplicate trigger selectors, missing public pad ports, and incompatible routes.

## Capabilities

### New Capabilities
- `drum-machine-container`: Drum-machine event module declaration, trigger selector validation, per-pad event emission, graph expansion, and event routing behavior.

### Modified Capabilities
- `yaml-patch-format`: Allow patch YAML to declare drum-machine event modules with trigger selectors and per-pad emitted-event configuration.
- `modular-routing-graph`: Define graph validation behavior for expanded drum-machine event ports.

## Impact

- Rust engine patch model, YAML parsing, validation, and diagnostics.
- Graph construction/expansion path before offline or realtime processor construction.
- Event routing behavior for named drum-machine pads.
- Example patch fixtures and CLI render acceptance coverage that prove the container emits pad events consumed by an explicitly declared downstream signal chain.
