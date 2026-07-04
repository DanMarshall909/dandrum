## Why

Dandrum needs a primitive that converts an audio signal into a control signal for modulation and dynamics workflows. This is similar in spirit to FL Studio's Peak Controller: an incoming audio envelope controls downstream parameters without directly producing audio.

This belongs in Rust rather than Rhai scripting because it is audio-derived, stateful, block/sample-rate sensitive, and must obey the realtime DSP contract. Scripts may route or configure behaviour, but envelope extraction and smoothing should be implemented as a tested primitive.

Dandrum also needs a small control-shaping utility so a generated envelope or CV signal is not limited to a strictly linear response. Peak-controller output should be usable directly, but it should also be easy to bend into exponential, logarithmic, S-curve, inverted, or stepped modulation for musical control.

## What Changes

- Add a `peak_controller` built-in primitive.
- Accept an audio input and produce a control output representing the smoothed peak/envelope level.
- Use attack and decay smoothing comparable to a dynamics processor envelope detector.
- Support shaping parameters suitable for ducking, sidechain modulation, and control automation.
- Add a separate `control_shaper` utility primitive for nonlinear CV shaping.
- Keep both primitives deterministic and realtime safe.

## Proposed `peak_controller` Ports

Inputs:

- `audio_in` (`audio`) — signal to analyse.
- `attack` (`control`) — attack time, normalized or seconds depending on existing control conventions.
- `decay` (`control`) — decay/release time, normalized or seconds depending on existing control conventions.
- `amount` (`control`) — output scaling amount.
- `offset` (`control`) — output offset/baseline.
- `invert` (`control`) — optional inversion for ducking-style output.

Outputs:

- `value` (`control`) — smoothed control signal.

## Proposed `control_shaper` Ports

Inputs:

- `value` (`control`) — incoming control/CV signal.
- `amount` (`control`) — blend or depth of the shaping effect.
- `bias` (`control`) — pre-shape offset or curve centre.
- `scale` (`control`) — output scale.
- `offset` (`control`) — output offset.

Outputs:

- `value` (`control`) — shaped control signal.

Parameters:

- `curve` (`text`) — one of `linear`, `exponential`, `logarithmic`, `s_curve`, `soft_clip`, `hard_clip`, `step`.
- `steps` (`integer`) — number of quantisation steps when `curve: step`.

## Peak Controller Behaviour

For each frame, `peak_controller` measures the input peak level, applies attack when the target envelope rises, applies decay when the target envelope falls, shapes/scales the result, optionally inverts it, and emits a bounded control signal.

The first implementation should prefer a simple deterministic peak follower. RMS mode, curve shape, stereo linking, and sidechain-specific conveniences can be added later if tests prove they are needed.

## Control Shaper Behaviour

For each frame, `control_shaper` reads a control input, applies the selected curve, then applies scale and offset. The primitive must clamp or otherwise bound invalid outputs so downstream modules receive finite control values.

The control shaper exists so dynamics-style control signals, LFOs, envelopes, velocity outputs, and script-generated controls can be made musically useful without embedding curve logic into every primitive.

## Non-Goals

- No scripting implementation in this change.
- No audio-rate waveshaper for audio signals; `control_shaper` is control-rate only.
- No sidechain compressor replacement; `peak_controller` outputs control, it does not apply gain reduction itself.
- No UI graph editor work.

## Impact

- **Rust engine crate**: Add new stateful `peak_controller` primitive and stateless/state-light `control_shaper` primitive.
- **Built-in registry**: Add module definitions, ports, parameter metadata, and examples.
- **Graph processor**: Add dispatch and per-module state as needed.
- **Tests**: Add deterministic render/unit tests for attack, decay, inversion, amount/offset, curve shaping, clamping, and finite output.
- **Examples**: Add a ducking/modulation example using `peak_controller -> control_shaper -> gain/filter`.
