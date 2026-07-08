## Why

Dandrum has basic sample loading/playback support, but practical drum machines and hybrid instruments need more than a single file fired at a fixed pitch. To build credible acoustic kits, chopped breaks, layered electronic hits, velocity-sensitive instruments, and LLM-authored sample patches, the engine needs a richer but still realtime-safe sampling model.

The goal is not to turn Dandrum into a full DAW sampler. The goal is to provide a small set of prepared sampling primitives that can be composed with the existing graph system, envelopes, filters, modulation, effects, module library, and preset surface.

## What Changes

- Add an `advanced-sampling-options` capability covering prepared sample assets, sample regions, sample maps, zone selection, velocity layers, round-robin alternates, choke groups, slicing, looping, and deterministic playback behaviour.
- Extend the YAML patch format so sample assets can declare reusable sample maps and regions instead of only isolated file references.
- Add or extend sampling primitives around a reusable `sample_player` core:
  - `sample_player` for one-shot and pitched playback of a prepared sample region.
  - `sample_map_player` for velocity/key/round-robin selection from a prepared sample map.
  - `sample_slicer` for triggerable slice playback from a prepared break/loop asset.
  - `sample_choke` or equivalent voice-group behaviour for closed/open hats and mutually exclusive sample voices.
- Support region-level playback options: start/end frames, start offset modulation, gain, pan, pitch ratio, root note, loop mode, loop start/end, loop crossfade, reverse, interpolation mode, fade-in, fade-out, and envelope handoff.
- Support sample-map selection options: key ranges, velocity ranges, probability, round-robin group, exclusive/choke group, per-zone gain/pan/pitch offsets, and deterministic seeded selection.
- Keep expensive work off the audio thread: file IO, decoding, resampling for preparation, validation, region indexing, slice detection/import, and buffer allocation happen before rendering.
- Add example patches for layered drums, acoustic-style velocity kits, sliced break playback, chromatic sample playback, and open/closed hi-hat choking.

## Capabilities

### New Capabilities

- `advanced-sampling-options`: Defines prepared sample assets, sample regions, sample maps, zone selection, velocity layers, round-robin alternates, slicing, looping, choke groups, and realtime-safe playback requirements.

### Modified Capabilities

- `built-in-modules`: The built-in module registry will include the sample playback primitives and any required voice/choke helpers.
- `yaml-patch-format`: Patch assets may declare sample maps, regions, zones, slices, and per-region playback metadata.
- `instrument-presets`: Presets may expose public controls such as sample map choice, layer mix, start offset, pitch, decay, choke group selection, loop mode, slice index, and variation amount without exposing internal module IDs.
- `plugin-integration`: The plugin parameter surface should expose advanced sampling controls from the preset surface like any other instrument while preserving the realtime callback contract.
- `module-library`: Bundled modules may use advanced sampling primitives to provide reusable drum-kit, break-slicer, and chromatic-sampler building blocks.

## Impact

- Engine: asset preparation needs to understand sample maps, regions, slices, looping metadata, deterministic selection state, and voice/choke ownership.
- Rendering: sampling modules must be deterministic, block-splitting safe, and free of steady-state heap allocation.
- Validation: malformed regions, invalid loop points, missing files, unsupported formats, overlapping/ambiguous zones, and invalid choke/group declarations must produce structured diagnostics before rendering.
- Tests: add behaviour-first tests for selection, velocity/key mapping, round-robin determinism, loop boundaries, slice triggering, reverse playback, choke groups, and no render-time allocation.
- Examples: add small sample-based patches using freely redistributable or synthetic reference assets only.
- Non-goal: this change does not require proprietary sample libraries, Kontakt/SFZ import, disk streaming, granular synthesis, spectral resynthesis, or a custom waveform editor UI.
