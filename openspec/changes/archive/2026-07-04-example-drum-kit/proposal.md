## Why

Provide usable dogfood drum kit patches that demonstrate how generic platform primitives and existing modules
(event_filter/event routing, oscillator, sampler, filter, ADSR, noise, note_to_control, gain, multiply, etc.) can be composed
to create acoustic and electronic drum-machine-style instruments. This change consumes primitive module contracts rather
than adding or redefining built-ins.

## What Changes

- Depend on the `noise`, `impulse`, `note_to_control`, `gain`, and `multiply` module contracts supplied by
  `declarative-instrument-platform`.
- Depend on generic event-routing primitives from `add-event-routing-primitives` for note-to-voice routing.
- Create composite module definitions for reusable drum building blocks (`velocity_vca`, `impulse_tone`,
  `impulse_noise`, `impulse_layer`)
- Create an example drum kit YAML patch that wires MIDI input through generic event routing into voice composites and a
  shared output bus
- Optionally add effect composite definitions (reverb send, compressor) for drum bus processing
- No sound-generation built-in module is velocity-aware — velocity is composed through `note_to_control` + `gain` stages
  at the composite level
- No `drum_machine`, `drum_pad`, or drum-specific Rust primitive is in scope; the drum kit is a dogfood target for the
  generic system.

## Capabilities

### New Capabilities

- `drum-kit-patches`: Example YAML patch files and composite module definitions demonstrating velocity-aware drum voice
  construction and multi-voice drum kit wiring through generic event routing

### Modified Capabilities

- `noise` runs in voice scope so noise voices participate in per-voice envelopes and voice allocation.
- Composite-internal audio/control mixers run in voice scope so layered composites can mix sources before crossing the
  explicit global output mixer boundary.

## Impact

- `examples/patches/` — new YAML files for composites and drum kit patch
- `src/rust-engine/src/` — scope handling updates for voice-local noise and composite-internal mixers
