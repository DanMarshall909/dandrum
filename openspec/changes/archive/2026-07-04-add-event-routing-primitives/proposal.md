## Why

Patch authors need generic event-routing primitives that make musical intent readable without introducing instrument-specific Rust modules. A drum machine and a simple polyphonic synth are useful dogfood targets, but the engine should learn reusable event filtering, routing, and mapping capabilities rather than a `drum_machine` or `poly_synth` primitive.

The target model is a human- and LLM-readable YAML graph where reusable primitives and composites can express standard DAW instruments and effects.

## What Changes

- Add generic event-routing primitives for filtering, routing, and optionally remapping event streams.
- Keep event-routing behavior event-only: no audio generation, samples, sequencing, transport, hidden mixers, or signal-chain ownership.
- Support drum-machine-style pad routing as an example built from generic primitives and composites.
- Support simple polyphonic synth routing as an example built from generic primitives and composites.
- Provide metadata, validation diagnostics, and examples that make the primitives discoverable by humans and future tooling.

## Capabilities

### New Capabilities

- `event-routing-primitives`: Generic event filtering/routing/mapping modules, metadata, validation, deterministic routing, and example coverage.

### Modified Capabilities

- `yaml-patch-format`: Allow patch YAML to declare generic event-routing modules and readable routing rules without instrument-specific container syntax.
- `modular-routing-graph`: Define graph validation behavior for event-only routing modules and their typed event ports.

## Impact

- Rust engine patch model, YAML parsing, validation, built-in module registry, and diagnostics.
- Graph construction and render behavior for event-only modules.
- Example patch fixtures proving drum-machine and simple poly-synth goals are achievable without dedicated instrument primitives.
