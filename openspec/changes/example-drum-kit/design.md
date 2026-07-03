## Context

The engine has 22 built-in module types and supports YAML-defined composite modules, but lacks a noise source and has no
complete drum kit example patches. Existing patches demonstrate individual effects (echo, reverb) and simple voices (
oscillator+envelope+VCA) but nothing that assembles a full multi-voice percussion instrument. Adding a noise generator
and composing drum voices from existing primitives closes the gap between toy examples and playable instrument patches.

## Goals / Non-Goals

**Goals:**

- Add a `noise` built-in module that generates white, pink, and brownian noise as a pure signal source
- Add `note_to_control` and `multiply` built-in modules to enable velocity composition at the patch level
- Expose existing `delay_line` and `envelope_follower` DSP utilities as built-in module types for patch-level
  composition
- Create composite module definitions for reusable building blocks (`velocity_vca`, `impulse_tone`, `impulse_noise`,
  `impulse_layer`)
- Create an example drum kit YAML patch wiring all voices through a shared output bus
- Each composite works as a standalone voice and within the kit patch
- All new code has unit tests; example patches are renderable by the CLI

**Non-Goals:**

- Not changing existing module behaviour or port contracts (ADSR, VCA, echo, dynamics_processor, etc. remain pure)
- Not removing or deprecating existing compound modules — adding primitives alongside them
- Not adding a step sequencer, pattern sequencer, or groove box UI
- Not adding sample loading for drum hits (existing sampler module already supports this)
- Not building velocity into any built-in module — velocity is a composite-level concern

## Design Philosophy

Modules are small, single-responsibility, and composable — like UNIX pipes. No built-in module knows about velocity,
note numbers, or drum semantics. Higher-level behaviour emerges from wiring:

- `note_to_control` extracts velocity from events → control signal
- `multiply` multiplies two control signals (e.g., envelope × velocity)
- `gain` scales audio by a control signal
- `delay_line` writes audio into a buffer, reads back after N samples
- `envelope_follower` tracks signal amplitude with configurable attack/release
- Composite modules bundle these primitives into reusable voice architectures

This means the same building blocks can be rearranged for synthesisers, effects, or other instruments without any module
knowing about drum-specific concepts.

## Decisions

### Noise Generator Architecture

**Decision:** Implement a single `NoiseGenerator` struct supporting three noise colours via an enum, with stateless
per-sample generation.

**Rationale:**

- White noise: `rand::random::<f32>() * 2.0 - 1.0` per sample — trivial, no state
- Pink noise: Voss-McCartney algorithm with 16 octave generators — simple 1/f spectrum without FFT
- Brownian noise: random walk clamped to [-1, 1] — single state variable
- Using an enum rather than trait objects keeps dispatch predictable and avoids heap allocation in the realtime path
- No external dependency needed: `rand` is already available via the Rust ecosystem or we use a minimal LCG

### Noise Module Ports

**Decision:** Audio output only, with control inputs for amplitude and colour. No gate, no event input, no velocity
awareness.

**Rationale:**

- Pure signal source — other modules (ADSR, VCA, filter) shape the noise
- Colour select allows switching between white (hi-hat), pink (snare texture), brownian (kick texture) without different
  module types
- Consistent with UNIX philosophy: do one thing, do it well

### Velocity Composition via note_to_control + multiply

**Decision:** Velocity scaling is handled entirely in composites, not in any built-in module.

**Rationale:**

- ADSR stays a pure envelope generator (no velocity distraction)
- VCA/gain stays a pure audio scaler (no envelope-or-velocity awareness)
- `note_to_control` is the single point where events become control signals — trivial to test, trivial to understand
- `multiply` is a pure math function — no state, no side effects
- Together they compose into `velocity_vca`: events + envelope + audio → velocity-scaled audio

### Decomposition of Compound Modules

**Decision:** Expose existing internal DSP utilities (`delay_line`, `envelope_follower`) as built-in module types
alongside the existing compound modules (echo, dynamics_processor).

**Rationale:**

- `delay_line` is already a clean read/write primitive used internally by echo and reverb — exposing it unlocks flanger,
  chorus, comb filter, and custom echo patches without any new DSP code
- `envelope_follower` is already a clean level-tracking primitive used internally by dynamics_processor — exposing it
  unlocks ducking, sidechain, tremolo, and custom compressor patches
- Existing compound modules remain unchanged and fully supported — this is additive, not breaking
- Patch authors can now build effects at the YAML level that previously required new Rust code

**Ports for delay_line:**

- `audio_in` (audio), `delay_samples` (control) → `audio_out` (audio)

**Relationship to existing delay types:** The engine already has `audio_delay_one_sample`, `block_delay` (multi-sample),
and `control_delay`. The `delay_line` module differs by supporting fractional-sample delays via linear interpolation and
by exposing `delay_samples` as a runtime control input rather than a compile-time parameter. Long term, `delay_line`
could supersede the fixed delay types, but for now they coexist.

**Ports for envelope_follower:**

- `audio_in` (audio), `attack` (control), `release` (control) → `envelope` (control)

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

- **[Noise colour quality]** Pink noise via Voss-McCartney is approximate; true 1/f requires more generators — we accept
  the approximation for a first implementation; users who need precision can use the sampler with a noise sample
- **[Noise determinism]** `rand::random` varies across Rust versions — we may want a seeded LCG later; for now,
  non-deterministic noise is acceptable for a proof-of-concept drum kit
- **[MIDI note routing]** The engine has no built-in note-to-trigger demux module, so each drum voice receives all MIDI
  events — voices must filter by note number internally; if script modules support note filtering this can be addressed
  in the patch
- **[Composite depth]** Nested composites (velocity_vca inside impulse_tone inside drum kit) increase preparation
  complexity — but the existing composite implementation already handles arbitrary nesting, so this is not new risk
- **[Primitive proliferation]** Exposing delay_line and envelope_follower as module types creates a larger built-in
  surface — but the DSP code already exists and is tested, so the risk is limited to module registration and dispatch
  correctness
