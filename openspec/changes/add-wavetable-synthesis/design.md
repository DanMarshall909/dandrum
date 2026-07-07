## Context

Dandrum's preferred direction is to keep DSP code small, reusable, highly tested, and composable. A Nord/Virus-like synth voice should therefore not be a single monolithic `virus_lead` or `nord_pad` module. It should be assembled from general primitives and composites.

Wavetable synthesis is a good fit for that model because the expensive/unsafe work can happen during preparation: load table data, validate table dimensions, normalize levels, precompute interpolation/bandlimiting data, and allocate scratch buffers. The realtime render path can then perform bounded phase advancement and table lookup only.

## Goals / Non-Goals

**Goals:**

- Provide a product-neutral `wavetable_oscillator` primitive suitable for bright digital and virtual-analogue synth sounds.
- Support smooth wavetable position morphing, pitch modulation, and audio-rate rendering.
- Support prepared, validated wavetable assets referenced from YAML patches.
- Make unison and wide stacked sounds easy to express through reusable composites or patch patterns.
- Keep all loading, validation, allocation, and table preparation off the audio thread.
- Give LLM-authored patches an obvious path to classic lead, pad, pluck, bass, and evolving timbre examples.

**Non-Goals:**

- Emulate Nord Lead, Access Virus, or any other commercial synth at circuit/firmware level.
- Copy proprietary factory wavetables, waveforms, preset names, or parameter values.
- Add a bespoke branded synth voice module.
- Build a custom plugin UI for wavetable editing in this slice.
- Support arbitrary user wavetable editors or sample-analysis tools in the first step.

## Decisions

### Use a general `wavetable_oscillator` primitive

The module type should be `wavetable_oscillator` because it describes the reusable behaviour and follows the project nomenclature. Product-specific names should stay out of user-facing docs and module IDs.

Alternative considered: add `virus_oscillator`, `nord_oscillator`, or a full `virtual_analog_synth` module. That would make patch authoring easy at first, but it would bake a style preset into the engine and make the DSP harder to reuse and test.

### Treat wavetables as prepared assets

Patch YAML should reference a wavetable asset by ID. Preparation should resolve the asset, validate it, normalize it where required, and convert it into a render-ready representation before realtime rendering begins.

The render path should not read files, parse metadata, allocate vectors, resample tables, or choose fallback assets.

### Prefer bandlimited table banks over naive playback

The oscillator should avoid obvious aliasing by selecting prepared bandlimited table data appropriate to the current pitch where available. The first implementation may use a simple octave-bank strategy, but the spec should leave room for better preparation later.

Alternative considered: read one high-resolution table at every pitch. That is simpler but produces poor results in high notes and makes the synth sound obviously unfinished.

### Separate oscillator, unison, filter, and effects

The Virus/Nord-like character should come from patch composition:

```text
note events
   -> note_to_control
   -> wavetable_oscillator x N / unison composite
   -> mixer
   -> filter
   -> VCA controlled by envelope
   -> chorus / delay / reverb
   -> audio_output
```

This keeps the oscillator primitive narrow and lets the same module serve basses, pads, leads, FM-style layers, and drum transients.

### Start with deterministic table morphing

Wavetable position should be a normalized control value from `0.0` to `1.0`. The oscillator should map that to neighbouring frames and interpolate deterministically. Modulating position should be smoothed or interpolated in a way that avoids frame-to-frame clicks.

## Proposed Module Surface

### `wavetable_oscillator`

Inputs:

- `frequency` (`control`) — oscillator frequency in Hz.
- `pitch_ratio` (`control`, optional) — multiplicative pitch modulation, if the engine supports this alongside frequency.
- `position` (`control`) — normalized wavetable frame position from `0.0` to `1.0`.
- `phase_reset` (`event`, optional) — note/trigger event that can reset phase according to mode.
- `sync` (`event` or `control`, optional future extension) — hard-sync input if added later.

Outputs:

- `audio_out` (`audio`) — rendered oscillator signal.
- `phase` (`control`, optional future extension) — normalized phase for modulation/debugging where useful.

Static parameters:

- `wavetable` — asset ID or built-in wavetable name.
- `initial_phase` — normalized phase offset.
- `level` — output level scalar.
- `interpolation` — `none`, `linear`, or `cubic` where supported.
- `phase_reset_mode` — `free`, `note`, or `trigger` where supported.
- `anti_aliasing` — `off`, `octave_banks`, or future modes.

## Wavetable Asset Model

A wavetable asset should resolve to a prepared model with:

- frame count greater than zero.
- sample count per frame greater than zero and consistent across frames after preparation.
- deterministic normalization policy.
- explicit interpolation support.
- optional bandlimited banks per frame.
- stable diagnostic IDs for asset and frame validation errors.

Initial supported input formats can be deliberately narrow. The first slice may support only engine-owned YAML/JSON metadata plus raw/CSV/text float arrays, or a small built-in library, as long as unsupported assets fail before render with clear diagnostics.

## Preset Surface Guidance

Example patches should expose musically useful public controls rather than internal graph detail:

- `osc.wavetable`
- `osc.position`
- `osc.position_mod_depth`
- `osc.detune`
- `osc.spread`
- `filter.cutoff_hz`
- `filter.resonance`
- `amp.attack_ms`
- `amp.decay_ms`
- `amp.sustain`
- `amp.release_ms`
- `fx.chorus_mix`
- `fx.delay_mix`
- `fx.reverb_mix`

## Risks / Trade-offs

- Wavetable aliasing is easy to get wrong → Start with tested octave-bank bandlimiting or clearly mark naive mode as low-quality/debug only.
- Asset formats can sprawl → Keep v1 narrow and validation-heavy.
- Smooth modulation can hide state bugs → Test position sweeps and phase continuity directly.
- Unison can become CPU-heavy → Express unison as a composite first, then introduce a primitive only if profiling shows it is needed.
- Branded references can create false expectations → Use product names only as inspiration in design notes, never as compatibility claims.

## Migration Plan

1. Add OpenSpec coverage for wavetable assets and `wavetable_oscillator` behaviour.
2. Add module registry metadata for `wavetable_oscillator` with typed ports and parameter declarations.
3. Add validation for wavetable asset references and unsupported/invalid asset formats.
4. Implement deterministic wavetable preparation off the audio thread.
5. Implement realtime-safe oscillator rendering with no steady-state allocation.
6. Add example patches/presets for wide lead, evolving pad, pluck, and bass sounds.
7. Add spectral/aliasing comparison tests or fixtures once the basic render behaviour is stable.
