## Why

The headless engine can describe and render a modular instrument graph, but playable instruments need more than one simultaneous note. Polyphony is the next foundational behavior because it affects patch format, event scheduling, module instancing, voice lifetime, and deterministic offline rendering.

## What Changes

- Add patch-level voice allocation settings for monophonic and polyphonic instruments.
- Add deterministic MIDI note-on/note-off handling that allocates, releases, and optionally steals voices.
- Process each active voice as an independent voice-local sub-synth with its own module state, routing, modulation, and adjustable output path.
- Allow each voice sub-synth to route through its own voice-local sub-bus and controls before the voices are mixed together.
- Allow different signal families to interact only through explicit conversion, merge, or polymorphic modules rather than implicit mixed-type routing.
- Route per-voice audio/control outputs into explicit mix stages before shared output processing.
- Validate invalid polyphony declarations and unsupported voice/global routing boundaries before rendering.

## Capabilities

### New Capabilities

- `polyphonic-voice-allocation`: Defines patch-declared voice allocation, voice-local sub-synth processing, per-voice sub-buses and controls, explicit signal conversion/merge behavior, note lifetime, deterministic voice stealing, and voice/global routing boundaries.

### Modified Capabilities

- None. No synced base specs exist yet; this change introduces the polyphony contract as a new capability that builds on the headless modular engine behavior.

## Impact

- Rust engine core domain model, patch schema, validation, block scheduler, and render tests.
- Built-in module metadata where modules need voice-scoped or global execution classification.
- YAML examples and diagnostics for valid polyphonic patches and invalid voice/global routing.
