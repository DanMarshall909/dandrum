## Why

Provide usable example drum kit patches that demonstrate how platform primitives and existing modules (oscillator,
sampler, filter, ADSR, noise, note_to_control, multiply, etc.) can be composed to create acoustic and electronic drum
sounds. This change consumes the primitive module contracts established by `declarative-instrument-platform` rather than
adding or redefining built-ins.

## What Changes

- Depend on the `noise`, `impulse`, `note_to_control`, and `multiply` module contracts supplied by
  `declarative-instrument-platform`.
- Create composite module definitions for reusable drum building blocks (`velocity_vca`, `impulse_tone`,
  `impulse_noise`, `impulse_layer`)
- Create an example drum kit YAML patch that wires MIDI input into voice composites through a shared output bus
- Optionally add effect composite definitions (reverb send, compressor) for drum bus processing
- No sound-generation built-in module is velocity-aware — velocity is composed through `note_to_control` + `multiply` at the composite
  level

## Capabilities

### New Capabilities

- `drum-kit-patches`: Example YAML patch files and composite module definitions demonstrating velocity-aware drum voice
  construction and multi-voice drum kit wiring

### Modified Capabilities

- *(none — this change adds example patches and composites only)*

## Impact

- `examples/patches/` — new YAML files for composites and drum kit patch
- No changes to built-in module registration or DSP implementation are in scope for this change; missing primitives must
  be implemented through their owning platform or primitive changes first.
