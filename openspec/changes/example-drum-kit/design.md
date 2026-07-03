## Context

The engine supports YAML-defined composite modules and the platform change provides the primitive contracts needed for
drum-voice composition. Existing patches demonstrate individual effects (echo, reverb) and simple voices
(oscillator+envelope+VCA) but nothing that assembles a full multi-voice percussion instrument. Composing drum voices from
platform primitives closes the gap between toy examples and playable instrument patches.

## Goals / Non-Goals

**Goals:**

- Consume `noise`, `impulse`, `note_to_control`, and `multiply` as platform-provided primitives
- Create composite module definitions for reusable building blocks (`velocity_vca`, `impulse_tone`, `impulse_noise`,
  `impulse_layer`)
- Create an example drum kit YAML patch wiring all voices through a shared output bus
- Each composite works as a standalone voice and within the kit patch
- All new code has unit tests; example patches are renderable by the CLI

**Non-Goals:**

- Not changing existing module behaviour or port contracts (ADSR, VCA, echo, dynamics_processor, etc. remain pure)
- Not adding, removing, or deprecating built-in module types
- Not adding a step sequencer, pattern sequencer, or groove box UI
- Not adding sample loading for drum hits (existing sampler module already supports this)
- Not building velocity into any built-in module — velocity is a composite-level concern

## Design Philosophy

Modules are small, single-responsibility, and composable — like UNIX pipes. No built-in module knows about velocity,
note numbers, or drum semantics. Higher-level behaviour emerges from wiring:

- `note_to_control` extracts velocity from events → control signal
- `multiply` multiplies two control signals (e.g., envelope × velocity)
- `gain` scales audio by a control signal
- Composite modules bundle these primitives into reusable voice architectures

This means the same building blocks can be rearranged for synthesisers, effects, or other instruments without any module
knowing about drum-specific concepts.

## Decisions

### Platform Primitive Dependency

**Decision:** This change does not define or implement built-in primitives. It uses the primitive contracts owned by
`declarative-instrument-platform`, including deterministic noise, impulse triggering, note-to-control conversion, and
audio/control multiplication.

**Rationale:** Keeping primitive contracts in one platform change avoids conflicting port, parameter, determinism, and
dispatch requirements. Drum-kit work can then focus on proving those primitives through musical composites and examples.

### Velocity Composition via note_to_control + multiply

**Decision:** Velocity scaling is handled entirely in composites, not in any built-in module.

**Rationale:**

- ADSR stays a pure envelope generator (no velocity distraction)
- VCA/gain stays a pure audio scaler (no envelope-or-velocity awareness)
- `note_to_control` is the single point where events become control signals — trivial to test, trivial to understand
- `multiply` is a pure math function — no state, no side effects
- Together they compose into `velocity_vca`: events + envelope + audio → velocity-scaled audio

### Drum Voice Composites

**Decision:** Define reusable composite modules that compose sound sources with `velocity_vca`:

- `velocity_vca`: events + envelope + audio → velocity-scaled audio output
- `impulse_tone`: oscillator + ADSR + `velocity_vca` (for tuned percussion: kick, tom)
- `impulse_noise`: noise + filter + ADSR + `velocity_vca` (for noise percussion: hi-hat, rim click)
- `impulse_layer`: oscillator + noise + filter + ADSR + `velocity_vca` (for layered percussion: snare)

**Rationale:**

- No new Rust code needed for the voices themselves — just wiring existing modules and composites
- Each composite can be tested standalone
- The kit patch just picks the right composite per MIDI note

### Drum Kit Patch Structure

**Decision:** A single kit patch routes MIDI events to individual drum voice composites. Each voice receives all events
and filters by assigned note.

**Rationale:**

- The engine's MIDI input module emits events on all note numbers; each voice subscribes to its assigned note
- Drum voices route through summing audio mixers into the master output
- This mirrors how hardware and software drum synths organise their voice architecture

## Risks / Trade-offs

- **[Primitive contract drift]** Drum-kit examples may accidentally assume ports or behaviour that the platform primitives
  do not provide -> Keep examples aligned with the accepted platform primitive metadata and tests.
- **[MIDI note routing]** The engine has no built-in note-to-trigger demux module, so each drum voice receives all MIDI
  events — voices must filter by note number internally; if script modules support note filtering this can be addressed
  in the patch
- **[Composite depth]** Nested composites (velocity_vca inside impulse_tone inside drum kit) increase preparation
  complexity — but the existing composite implementation already handles arbitrary nesting, so this is not new risk
- **[Future effect examples]** Custom flanger, chorus, ducking, or sidechain examples may need `delay_line` or
  `envelope_follower` as public built-ins -> Propose those separately with primitive-gate justification instead of
  adding them through this drum-kit example change.
