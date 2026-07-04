## Why

Dandrum needs built-in control primitives that keep naming consistent with the engine's existing signal model: `audio`, `control`, and `event`.

A reusable `envelope_follower` primitive should convert an audio signal into a control signal for modulation and dynamics workflows. This is similar in spirit to FL Studio's Peak Controller, but the engine-facing name should describe the general DSP role rather than one product's feature name. Dandrum already has an `EnvelopeFollower` utility with peak/RMS detection and attack/release smoothing; this change promotes that concept into a graph primitive.

Dandrum also needs a small `curve_mapper` utility so a generated control signal is not limited to a strictly linear response. Envelope follower output should be usable directly, but it should also be easy to bend into exponential, logarithmic, S-curve, inverted, clipped, or stepped modulation for musical control.

These behaviours belong in Rust primitives rather than Rhai scripting because they are audio-derived, stateful or per-frame control operations that must obey the realtime DSP contract. Scripts may route or configure behaviour, but envelope extraction and control shaping should be implemented as tested primitives.

## What Changes

- Add an `envelope_follower` built-in primitive.
- Accept an audio input and produce a control output representing the smoothed detected envelope level.
- Use attack and release smoothing comparable to a dynamics processor envelope detector.
- Support peak detection first, with RMS mode available if it matches the existing utility cleanly.
- Add a separate `curve_mapper` utility primitive for nonlinear control-signal mapping.
- Keep both primitives deterministic and realtime safe.

## Proposed `envelope_follower` Ports

Inputs:

- `audio_in` (`audio`) — signal to analyse.
- `attack` (`control`) — attack time, normalized or milliseconds depending on existing control conventions.
- `release` (`control`) — release time, normalized or milliseconds depending on existing control conventions.
- `amount` (`control`) — output scaling amount.
- `offset` (`control`) — output offset/baseline.
- `invert` (`control`) — optional inversion for ducking-style output.

Outputs:

- `value` (`control`) — smoothed control signal.

Parameters:

- `mode` (`text`) — one of `peak`, `rms`; default `peak`.

## Proposed `curve_mapper` Ports

Inputs:

- `value` (`control`) — incoming control signal.
- `amount` (`control`) — blend or depth of the mapping effect.
- `bias` (`control`) — pre-map offset or curve centre.
- `scale` (`control`) — output scale.
- `offset` (`control`) — output offset.

Outputs:

- `value` (`control`) — mapped control signal.

Parameters:

- `curve` (`text`) — one of `linear`, `exponential`, `logarithmic`, `s_curve`, `soft_clip`, `hard_clip`, `step`.
- `steps` (`integer`) — number of quantisation steps when `curve: step`.

## Envelope Follower Behaviour

For each frame, `envelope_follower` measures the input level, applies attack when the target envelope rises, applies release when the target envelope falls, applies amount/offset/inversion, and emits a bounded control signal.

The first implementation should prefer deterministic peak detection using the existing `EnvelopeFollower` utility. RMS mode can be exposed if it requires no incompatible behaviour. Stereo linking and sidechain-specific conveniences can be added later if tests prove they are needed.

## Curve Mapper Behaviour

For each frame, `curve_mapper` reads a control input, applies the selected curve, then applies scale and offset. The primitive must clamp or otherwise bound invalid outputs so downstream modules receive finite control values.

The curve mapper exists so dynamics-style control signals, LFOs, envelopes, velocity outputs, and script-generated controls can be made musically useful without embedding curve logic into every primitive.

## Non-Goals

- No scripting implementation in this change.
- No audio-rate waveshaper for audio signals; `curve_mapper` is control-signal only.
- No sidechain compressor replacement; `envelope_follower` outputs control, it does not apply gain reduction itself.
- No UI graph editor work.

## Impact

- **Rust engine crate**: Promote the existing envelope follower concept into a graph primitive and add a control-signal curve mapper primitive.
- **Built-in registry**: Add module definitions, ports, parameter metadata, and examples.
- **Graph processor**: Add dispatch and per-module state as needed.
- **Tests**: Add deterministic render/unit tests for attack, release, inversion, amount/offset, curve mapping, clamping, and finite output.
- **Examples**: Add a ducking/modulation example using `envelope_follower -> curve_mapper -> gain/filter`.
