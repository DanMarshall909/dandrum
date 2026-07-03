## Why

Provide usable example drum kit patches that demonstrate how existing modules (oscillator, sampler, filter, ADSR, etc.)
can be composed to create acoustic and electronic drum sounds. A noise generator module is needed for hi-hat and snare
sounds that require broadband excitation.

## What Changes

- Add a `noise` built-in module type that generates white/pink/brownian noise as a pure signal source (no gate, no event
  input)
- Add a `note_to_control` built-in module that extracts velocity from NoteOn events and emits it as a control signal
- Add a `multiply` built-in module that computes `out = a × b` for control signals
- Expose existing `delay_line` DSP utility as a built-in module type so patch authors can build custom
  delay/flanger/chorus patches
- Expose existing `envelope_follower` DSP utility as a built-in module type so patch authors can build custom
  compressor/ducking/tremolo patches
- Create composite module definitions for reusable drum building blocks (`velocity_vca`, `impulse_tone`,
  `impulse_noise`, `impulse_layer`)
- Create an example drum kit YAML patch that wires MIDI input into voice composites through a shared output bus
- Optionally add effect composite definitions (reverb send, compressor) for drum bus processing
- No built-in module is velocity-aware — velocity is composed through `note_to_control` + `multiply` at the composite
  level

## Capabilities

### New Capabilities

- `noise-generator`: A pure-signal noise source module supporting white, pink, and brownian noise with amplitude and
  colour control inputs
- `note-to-control`: Extracts velocity from NoteOn events as a control signal; enables velocity-sensitive patches
  without changing ADSR or VCA
- `multiply`: Pure math module computing `out = a × b` for control signals; enables envelope × velocity composition
- `built-in-primitives`: Expose existing low-level DSP utilities (`delay_line`, `envelope_follower`) as patch-level
  module types so compound effects (echo, dynamics processor) become YAML-composable rather than monolithic built-ins
- `drum-kit-patches`: Example YAML patch files and composite module definitions demonstrating velocity-aware drum voice
  construction and multi-voice drum kit wiring

### Modified Capabilities

- *(none — only adding new modules and example patches)*

## Impact

- `src/rust-engine/src/builtins/module_types.rs` — add `NOISE`, `NOTE_TO_CONTROL`, `MULTIPLY`, `DELAY_LINE`,
  `ENVELOPE_FOLLOWER` constants
- `src/rust-engine/src/builtins/module_kind.rs` — add `Noise`, `NoteToControl`, `Multiply`, `DelayLine`,
  `EnvelopeFollower` variants
- `src/rust-engine/src/builtins.rs` — add definitions and register new modules
- `src/rust-engine/src/noise.rs` — new module with `NoiseGenerator` struct, DSP, and unit tests
- `src/rust-engine/src/note_to_control.rs` — new module extracting velocity from events as control signal
- `src/rust-engine/src/multiply.rs` — new module implementing control multiplier
- `src/rust-engine/src/delay_line.rs` — expose as built-in module type (already exists as internal utility)
- `src/rust-engine/src/envelope_follower.rs` — expose as built-in module type (already exists as internal utility;
  rename from `envelope_detector`)
- `src/rust-engine/src/lib.rs` — export new modules
- `src/rust-engine/src/graph_processor/processing.rs` — add `process_noise()`, `process_note_to_control()`,
  `process_multiply()`, `process_delay_line()`, `process_envelope_follower()`
- `src/rust-engine/src/graph_processor/dispatch.rs` — add dispatch arms for new module kinds
- `examples/patches/` — new YAML files for composites and drum kit patch
- No changes to ADSR, VCA/gain, echo, dynamics_processor, or other existing modules — backward compatible
