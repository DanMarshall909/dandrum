## Overview

Add two control-focused primitives using existing Dandrum nomenclature:

1. `envelope_follower`: converts an audio input into a smoothed control envelope using attack/release behaviour similar to a dynamics envelope detector.
2. `curve_mapper`: transforms a control signal through a selected nonlinear curve.

Together they allow patches to derive musical modulation from audio energy and then map that modulation without scripting or bespoke Rust per patch.

## `envelope_follower`

`envelope_follower` is a stateful Rust primitive. It measures input signal magnitude and produces a per-frame control output.

Dandrum already has an `EnvelopeFollower` utility in `src/rust-engine/src/envelope_follower.rs`; implementation should reuse or adapt that tested utility rather than creating a separate `peak_controller` code path.

### Signal Flow

```text
audio_in -> level detector -> attack/release smoothing -> amount/offset/invert -> value
```

### Smoothing

The primitive uses separate coefficients for rising and falling envelope movement:

- when target level is greater than current envelope, use `attack`
- when target level is lower than current envelope, use `release`

The exact coefficient mapping should match the engine's existing `EnvelopeFollower` conventions where possible. The existing utility currently accepts attack and release times in milliseconds.

### Output

The output is a control-rate buffer with one value per frame. Output values must be finite. The first implementation should clamp to a musically safe range unless an existing control convention requires otherwise.

### Inversion

When `invert` is enabled, a high detected level should produce a lower output value. This supports ducking and sidechain-style control routing.

## `curve_mapper`

`curve_mapper` is a control-rate primitive. It takes an incoming control buffer and applies a selected curve per frame.

### Curves

Initial supported curves:

- `linear`: unchanged except amount/scale/offset
- `exponential`: emphasise higher input values
- `logarithmic`: emphasise lower input values
- `s_curve`: smooth ease-in/ease-out response
- `soft_clip`: saturating curve that remains smooth
- `hard_clip`: clamp to range
- `step`: quantise into a fixed number of levels

### Blend/Amount

`amount` controls how strongly the mapped value replaces the original value. At `0`, output should equal the input after scale/offset policy is applied. At `1`, output should use the full mapped curve.

## Realtime Safety

Both primitives must be suitable for render-time graph processing:

- no heap allocation during steady-state processing
- no filesystem, logging, locks, or dynamic dispatch that violates existing engine conventions
- deterministic output for identical input, parameters, and render settings
- finite output even for extreme or invalid control input values

## Relationship To Scripting

These primitives are intentionally not part of Rhai scripting. Scripts may route events or controls into them, but audio-derived envelope detection and per-frame control mapping belong in Rust primitives.

## Naming Rules

Use these terms consistently in this change:

- **module** for graph building blocks
- **primitive** for built-in Rust implementations
- **composite** for reusable YAML graphs
- **patch** for complete instruments/effects
- **preset** for saved parameter variations
- **cable** for graph connections
- **port** for inputs/outputs
- **control signal** for continuous modulation

Avoid `node`, `processor`, `unit`, `wire`, `pin`, and `CV` in user-facing docs unless discussing an external system.

## Open Questions

- Should `attack`/`release` inputs use milliseconds everywhere, or normalized values mapped to a safe millisecond range?
- Should `envelope_follower` output default to 0..1 only, or allow values above 1 before downstream mapping?
- Should RMS detection be exposed immediately because the existing utility supports it, or deferred until needed?
- Should stereo envelope linking be handled by a later stereo variant or by explicit mixers/splitters?
