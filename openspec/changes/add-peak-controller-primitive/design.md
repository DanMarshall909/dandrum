## Overview

Add two control-focused primitives:

1. `peak_controller`: converts an audio input into a smoothed control envelope using attack/decay behaviour similar to a dynamics envelope detector.
2. `control_shaper`: transforms a control signal through a selected nonlinear curve.

Together they allow patches to derive musical modulation from audio energy and then shape that modulation without scripting or bespoke Rust per patch.

## `peak_controller`

`peak_controller` is a stateful Rust primitive. It measures input signal magnitude and produces a per-frame control output.

### Signal Flow

```text
audio_in -> absolute peak detector -> attack/decay smoothing -> amount/offset/invert -> value
```

### Smoothing

The primitive uses separate coefficients for rising and falling envelope movement:

- when target level is greater than current envelope, use `attack`
- when target level is lower than current envelope, use `decay`

The exact coefficient mapping should match the engine's existing dynamics/envelope-follower conventions where possible. If the current dynamics implementation uses normalized control values rather than seconds, this primitive should use the same interpretation for consistency.

### Output

The output is a control-rate buffer with one value per frame. Output values must be finite. The first implementation should clamp to a musically safe range unless an existing control convention requires otherwise.

### Inversion

When `invert` is enabled, a high detected peak should produce a lower output value. This supports ducking and sidechain-style control routing.

## `control_shaper`

`control_shaper` is a control-rate primitive. It takes an incoming control buffer and applies a selected curve per frame.

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

`amount` controls how strongly the shaped value replaces the original value. At `0`, output should equal the input after scale/offset policy is applied. At `1`, output should use the full shaped curve.

## Realtime Safety

Both primitives must be suitable for render-time graph processing:

- no heap allocation during steady-state processing
- no filesystem, logging, locks, or dynamic dispatch that violates existing engine conventions
- deterministic output for identical input, parameters, and render settings
- finite output even for extreme or invalid control input values

## Relationship To Scripting

These primitives are intentionally not part of Rhai scripting. Scripts may route events or controls into them, but audio-derived envelope detection and per-frame control shaping belong in Rust primitives.

## Open Questions

- Should `attack`/`decay` inputs be interpreted as seconds, milliseconds, or normalized values mapped to a safe time range?
- Should `peak_controller` output default to 0..1 only, or allow values above 1 before downstream shaping?
- Should RMS detection be added as a parameter now or deferred until needed?
- Should stereo peak linking be handled by a later stereo variant or by explicit mixers/splitters?
