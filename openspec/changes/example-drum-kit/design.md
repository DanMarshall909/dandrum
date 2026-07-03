## Context

The engine supports YAML-defined composite modules and the platform change provides the primitive contracts needed for
drum-voice composition. Existing patches demonstrate individual effects (echo, reverb) and simple voices
(oscillator+envelope+VCA) but nothing that assembles a full multi-voice percussion instrument. Composing drum voices from
platform primitives closes the gap between toy examples and playable instrument patches.

## Goals / Non-Goals

**Goals:**

- Consume `noise`, `impulse`, `note_to_control`, `gain`, `multiply`, and generic event-routing primitives as
  platform-provided capabilities
- Create composite module definitions for reusable building blocks (`velocity_vca`, `impulse_tone`, `impulse_noise`,
  `impulse_layer`)
- Create an example drum kit YAML patch wiring MIDI input through generic event routing into all voices and then through
  a shared output bus
- Each composite works as a standalone voice and within the kit patch
- All new code has unit tests; example patches are renderable by the CLI

**Non-Goals:**

- Not changing existing module port contracts (ADSR, VCA, echo, dynamics_processor, etc. remain pure)
- Not adding, removing, or deprecating built-in module types
- Not adding a step sequencer, pattern sequencer, or groove box UI
- Not adding sample loading for drum hits (existing sampler module already supports this)
- Not building velocity into any built-in module — velocity is a composite-level concern

## Design Philosophy

Modules are small, single-responsibility, and composable — like UNIX pipes. No built-in module knows about velocity,
note numbers, or drum semantics. Higher-level behaviour emerges from wiring:

- `note_to_control` extracts velocity from events → control signal
- `gain` scales audio by a control signal, so two gain stages can apply envelope and velocity
- `multiply` remains an audio-rate product primitive
- Composite modules bundle these primitives into reusable voice architectures

This means the same building blocks can be rearranged for synthesisers, effects, or other instruments without any module
knowing about drum-specific concepts.

## Decisions

### Platform Primitive Dependency

**Decision:** This change does not define or implement built-in primitives. It uses the primitive contracts owned by
`declarative-instrument-platform`, including deterministic noise, impulse triggering, note-to-control conversion, gain,
and audio-rate multiplication.

**Rationale:** Keeping primitive contracts in one platform change avoids conflicting port, parameter, determinism, and
dispatch requirements. Drum-kit work can then focus on proving those primitives through musical composites and examples.

### Velocity Composition via note_to_control + gain

**Decision:** Velocity scaling is handled entirely in composites, not in any built-in module.

**Rationale:**

- ADSR stays a pure envelope generator (no velocity distraction)
- VCA/gain stays a pure audio scaler (no envelope-or-velocity awareness)
- `note_to_control` is the single point where events become control signals — trivial to test, trivial to understand
- Two `gain` stages compose into `velocity_vca`: events + envelope + audio → velocity-scaled audio

### Drum Voice Composites

**Decision:** Define reusable composite modules that compose sound sources with `velocity_vca`:

- `velocity_vca`: events + envelope + audio → velocity-scaled audio output
- `impulse_tone`: oscillator + ADSR + velocity VCA pattern (for tuned percussion: kick, tom)
- `impulse_noise`: noise + filter + ADSR + velocity VCA pattern (for noise percussion: hi-hat, rim click)
- `impulse_layer`: oscillator + noise + filter + ADSR + velocity VCA pattern (for layered percussion: snare)

**Rationale:**

- No new Rust code needed for the voices themselves — just wiring existing modules and composites
- Each composite can be tested standalone
- The kit patch just picks the right composite per MIDI note

### Drum Kit Patch Structure

**Decision:** A single kit patch routes MIDI events to individual drum voice composites. Each voice receives all events
and filters by assigned note.

**Rationale:**

- The engine's MIDI input module emits events on all note numbers; generic event-routing modules route assigned notes to
  explicit voice composites
- Drum voices route through summing audio mixers into the master output
- This mirrors how hardware and software drum synths organise their voice architecture

## Risks / Trade-offs

- **[Primitive contract drift]** Drum-kit examples may accidentally assume ports or behaviour that the platform primitives
  do not provide -> Keep examples aligned with the accepted platform primitive metadata and tests.
- **[MIDI note routing]** Drum-kit readability depends on generic event-routing primitives; if those primitives are not
  ready, this example should wait rather than hiding note filtering inside drum voices
- **[Composite depth]** Nested composites (velocity_vca inside impulse_tone inside drum kit) increase preparation
  complexity, so these examples inline the velocity VCA pattern inside each impulse composite.
- **[Future effect examples]** Custom flanger, chorus, ducking, or sidechain examples may need `delay_line` or
  `envelope_follower` as public built-ins -> Propose those separately with primitive-gate justification instead of
  adding them through this drum-kit example change.
