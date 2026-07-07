## Why

Dandrum can already express subtractive and drum-oriented patches, but it does not yet have a first-class wavetable synthesis capability for bright digital oscillator movement, smooth table morphing, and wide stacked synth voices.

The target sound family is inspired by classic virtual-analogue and wavetable hardware such as Nord Lead and Access Virus instruments, but the implementation should remain product-neutral: Dandrum should provide reusable primitives and patch patterns rather than a branded emulation.

A wavetable capability gives LLM-authored patches a practical path to supersaw-like stacks, evolving digital timbres, metallic motion, vowel/formant-style sweeps, and modern hybrid synth layers without requiring a large number of bespoke oscillator modules.

## What Changes

- Add a realtime-safe `wavetable_oscillator` primitive with deterministic table lookup, phase accumulation, pitch input, wavetable position/morph input, and typed audio/control ports.
- Add the extra primitives needed to build Nord/Virus-like patches as composites: `unison_voice`, `stereo_pan`, `chorus`, `phase_distortion`, `oscillator_sync`, `ring_modulator`, `sample_and_hold`, and `slew_limiter` where they are not already available.
- Define a prepared wavetable asset model so table data is loaded, validated, normalized, and bandlimited off the audio thread.
- Support smooth wavetable position modulation across frames without zippering or discontinuities.
- Support unison/stacked oscillator patterns suitable for wide virtual-analogue sounds without baking a branded synth voice into the engine.
- Keep full synth voices as patches/composites built from primitives: wavetable oscillator, filter, envelopes, VCA, modulation, stereo spread, chorus/delay/reverb, and parameter surface.
- Add example patches/presets that demonstrate classic bright trance/lead/pad motion while avoiding direct copying of proprietary factory presets.

## Capabilities

### New Capabilities

- `wavetable-synthesis`: Defines wavetable assets, a wavetable oscillator primitive, supporting synth primitives, morphing behaviour, modulation routing, unison use, and Nord/Virus-inspired patch patterns.

### Modified Capabilities

- `built-in-modules`: The built-in module registry will include the `wavetable_oscillator` primitive plus the supporting primitives needed for practical wavetable synth patches.
- `yaml-patch-format`: Patch assets need to describe wavetable resources and module parameters need to reference them deterministically.
- `instrument-presets`: Presets should expose public controls such as wavetable selection, position, detune, spread, filter cutoff, envelope amounts, modulation depth, and effects mix without exposing internal module IDs.
- `plugin-integration`: The plugin parameter surface should be able to expose wavetable synth controls from the preset surface like any other instrument.

## Impact

- Requires Rust DSP implementation for the oscillator, supporting primitives, and any helper preparation code.
- Requires schema and validation coverage for wavetable assets, supported formats, table size, frame count, and fallback behaviour.
- Requires tests proving deterministic rendering, interpolation, pitch behaviour, morph behaviour, invalid asset rejection, and no audio-thread allocation after preparation.
- Requires behaviour-first tests for each new primitive so the engine gains reusable building blocks rather than implementation-only coverage.
- Does not require a branded Nord Lead, Access Virus, or other commercial synthesizer emulation.
- Does not require plugin UI special cases beyond the existing generic parameter surface.
