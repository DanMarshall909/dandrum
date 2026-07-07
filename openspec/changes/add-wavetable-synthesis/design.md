## Context

Dandrum's preferred direction is to keep DSP code small, reusable, highly tested, and composable. A Nord/Virus-like synth voice should therefore not be a single monolithic `virus_lead` or `nord_pad` module. It should be assembled from general primitives and composites.

Wavetable synthesis is a good fit for that model because the expensive/unsafe work can happen during preparation: load table data, validate table dimensions, normalize levels, precompute interpolation/bandlimiting data, and allocate scratch buffers. The realtime render path can then perform bounded phase advancement and table lookup only.

## Goals / Non-Goals

**Goals:**

- Provide a product-neutral `wavetable_oscillator` primitive suitable for bright digital and virtual-analogue synth sounds.
- Add the small set of missing reusable primitives needed to build wide, moving, aggressive, or glassy synth patches.
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

### Add primitives only where composition would otherwise be awkward

The supporting primitive set should stay narrow and behaviour-driven. Add a primitive when it is reusable, realtime-stateful, and awkward to express as YAML composition without either excessive graph size or hidden state.

The first supporting set should be:

- `unison_voice` — renders or fans out a small stack of detuned/spread oscillator voices where doing so as explicit repeated modules would be noisy and CPU-sensitive.
- `stereo_pan` — equal-power mono-to-stereo placement for width and unison spread.
- `chorus` — short modulated delay for wide synth ensemble tone.
- `phase_distortion` — phase-domain waveshaping useful for digital edge, pulse-like motion, and aggressive timbres.
- `oscillator_sync` — phase-reset/sync utility where direct sync is not built into a specific oscillator.
- `ring_modulator` — audio-rate multiplication for metallic and bell-like sidebands.
- `sample_and_hold` — stepped random or sampled modulation.
- `slew_limiter` — control-signal smoothing for click-free modulation and classic glide-style movement.

If existing primitives already cover one of these behaviours, the implementation task should reuse/extend the existing primitive instead of adding a duplicate module type.

### Separate oscillator, unison, filter, and effects

The Virus/Nord-like character should come from patch composition:

```text
note events
   -> note_to_control
   -> wavetable_oscillator x N / unison_voice composite
   -> mixer
   -> filter
   -> VCA controlled by envelope
   -> stereo_pan / chorus / delay / reverb
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

### Supporting Primitive Surfaces

#### `unison_voice`

Inputs:

- `frequency` (`control`)
- `pitch_ratio` (`control`, optional)
- `position` (`control`, optional, passed to the oscillator source where supported)
- `gate` or `phase_reset` (`event`, optional)

Outputs:

- `audio_left` (`audio`)
- `audio_right` (`audio`)

Static parameters:

- `voice_count`
- `detune_cents`
- `spread`
- `level_compensation`
- `phase_mode`
- source oscillator reference or nested/composite source strategy, depending on the final graph model

#### `stereo_pan`

Inputs:

- `audio_in` (`audio`)
- `pan` (`control`)
- `width` (`control`, optional)

Outputs:

- `audio_left` (`audio`)
- `audio_right` (`audio`)

#### `chorus`

Inputs:

- `audio_in` (`audio`)
- `rate_hz` (`control`)
- `depth_ms` (`control`)
- `mix` (`control`)

Outputs:

- `audio_left` (`audio`)
- `audio_right` (`audio`)

Static parameters:

- `base_delay_ms`
- `feedback`
- `phase_offset`
- `max_delay_ms`

#### `phase_distortion`

Inputs:

- `phase` (`control`) or `audio_in` (`audio`), depending on selected mode
- `amount` (`control`)
- `symmetry` (`control`, optional)

Outputs:

- `phase_out` (`control`) or `audio_out` (`audio`)

#### `oscillator_sync`

Inputs:

- `master_phase` (`control`) or `master_trigger` (`event`)
- `slave_phase` (`control`) or `frequency` (`control`)

Outputs:

- `reset` (`event`) or `phase_out` (`control`)

#### `ring_modulator`

Inputs:

- `carrier` (`audio`)
- `modulator` (`audio`)
- `mix` (`control`, optional)

Outputs:

- `audio_out` (`audio`)

#### `sample_and_hold`

Inputs:

- `signal_in` (`control`)
- `trigger` (`event`)

Outputs:

- `control_out` (`control`)

Static parameters:

- `hold_mode`
- `seed` where random mode is supported

#### `slew_limiter`

Inputs:

- `control_in` (`control`)

Outputs:

- `control_out` (`control`)

Static parameters:

- `rise_ms`
- `fall_ms`
- `curve`

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
- `osc.sync_amount`
- `osc.phase_distortion_amount`
- `filter.cutoff_hz`
- `filter.resonance`
- `amp.attack_ms`
- `amp.decay_ms`
- `amp.sustain`
- `amp.release_ms`
- `mod.sample_hold_depth`
- `mod.slew_ms`
- `fx.chorus_mix`
- `fx.delay_mix`
- `fx.reverb_mix`

## Risks / Trade-offs

- Wavetable aliasing is easy to get wrong → Start with tested octave-bank bandlimiting or clearly mark naive mode as low-quality/debug only.
- Asset formats can sprawl → Keep v1 narrow and validation-heavy.
- Smooth modulation can hide state bugs → Test position sweeps and phase continuity directly.
- Unison can become CPU-heavy → Express unison as a composite first where possible, then introduce/optimize a primitive only if profiling shows it is needed.
- Supporting primitives can duplicate existing modules → Reuse existing modules where the behaviour already exists and only add missing primitives.
- Branded references can create false expectations → Use product names only as inspiration in design notes, never as compatibility claims.

## Migration Plan

1. Add OpenSpec coverage for wavetable assets, `wavetable_oscillator`, and supporting primitives.
2. Add module registry metadata for each new primitive with typed ports and parameter declarations.
3. Add validation for wavetable asset references and unsupported/invalid asset formats.
4. Implement deterministic wavetable preparation off the audio thread.
5. Implement realtime-safe oscillator rendering with no steady-state allocation.
6. Implement supporting primitives in priority order: `stereo_pan`, `slew_limiter`, `sample_and_hold`, `ring_modulator`, `phase_distortion`, `chorus`, `oscillator_sync`, `unison_voice`.
7. Add example patches/presets for wide lead, evolving pad, pluck, and bass sounds.
8. Add spectral/aliasing comparison tests or fixtures once the basic render behaviour is stable.
