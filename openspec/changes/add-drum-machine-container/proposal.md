## Why

Patch authors need event-transformer modules that reshape musical events before those events reach sound-generating signal chains. The drum machine is the first member of that family: inspired by Bitwig's pad-oriented Drum Machine workflow, it maps incoming note/event triggers to named pads and emits the event specified for each pad, without becoming a pattern sequencer or signal-chain host.

## What Changes

- Add a `drum_machine` event-only module type that expands into ordinary graph event-routing modules before rendering.
- Allow YAML patches to declare named pads with trigger selectors, typically note numbers or event labels, plus per-pad emitted-event configuration.
- Route compatible incoming events to matching pads and emit each pad's configured event on its event output.
- Establish the event-transformer shape that future modules such as event delays, transposers, arpeggiators, and other event processors can follow.
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
- Event-transformer routing behavior for named drum-machine pads.
- Example patch fixtures and CLI render acceptance coverage that prove the container emits pad events consumed by an explicitly declared downstream signal chain.
