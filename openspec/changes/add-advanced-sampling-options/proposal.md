## Why

Dandrum has basic sample loading/playback support, but practical drum machines and hybrid instruments need more than a single file fired at a fixed pitch. To build credible drum kits, chopped breaks, layered electronic hits, modest chromatic instruments, velocity-sensitive instruments, and LLM-authored sample patches, the engine needs a richer but still realtime-safe sampling model.

The goal is not to turn Dandrum into a full DAW sampler. The goal is to provide a small set of prepared sampling primitives that can be composed with the existing graph system, envelopes, filters, modulation, effects, module library, and preset surface.

The design should be as unified as possible: Dandrum should not grow multiple similar samplers for drum hits, sliced breaks, chromatic playback, workstation instruments, and DJ streaming. Those use cases should share source, region, metadata, playback, selection, transport, and analysis primitives where the realtime contracts are compatible.

This change is deliberately limited to the first three sampling families:

1. drum-machine sampling,
2. breakbeat/slice sampling,
3. modest chromatic/instrument sampling.

Full workstation sampling, creative/granular/time-stretch sampling, and DJ-style sample streaming are separate specs only because they add extra requirements. They should extend the same unified sample model rather than define competing sampler modules.

## What Changes

- Add an `advanced-sampling-options` capability covering prepared sample assets, sample sources, sample regions, sample maps, zone selection, velocity layers, round-robin alternates, choke groups, explicit slicing, modest looping, metadata outputs, and deterministic playback behaviour.
- Extend the YAML patch format so sample assets can declare reusable sample sources, maps, regions, slices, and analysis metadata instead of only isolated file references.
- Build around small primitives rather than purpose-specific sampler modules:
  - `sample_source`/prepared source metadata for file-backed audio and reusable analysis metadata.
  - `sample_region`/prepared region metadata for bounded ranges inside a source.
  - `sample_metadata` or source metadata outputs for duration, sample rate, channel count, region length, root note, detected tempo where available, beat grid where available, cue points, slice markers, and analysis confidence.
  - `sample_player` for one-shot, gated, looped, reversed, and pitched playback of a prepared sample region/source window.
  - `sample_zone_selector` or equivalent selection primitive for key/velocity/round-robin/probability region selection.
  - `sample_map_player` as a convenience module only if it composes the selector and player behaviours without hiding unrelated workstation-sampler features.
  - `sample_slicer` for triggerable playback of explicit slices from a prepared source.
  - `voice_choke` or equivalent voice-group primitive for closed/open hats and mutually exclusive sample voices.
- Support region-level playback options: start/end frames, start offset modulation, gain, pan, pitch ratio, root note, simple loop mode, loop start/end, loop crossfade where implemented, reverse, interpolation mode, fade-in, fade-out, and envelope handoff.
- Support sample-map selection options: key ranges, velocity ranges, probability, round-robin group, exclusive/choke group, per-zone gain/pan/pitch offsets, and deterministic seeded selection.
- Keep expensive work off the audio thread: file IO, decoding, preparation-time resampling where needed, validation, region indexing, explicit slice metadata import, beat/tempo analysis where implemented, and buffer allocation happen before rendering.
- Add example patches for layered drums, acoustic-style velocity kits, sliced break playback, modest chromatic sample playback, and open/closed hi-hat choking.

## Capabilities

### New Capabilities

- `advanced-sampling-options`: Defines prepared sample assets, sample sources, sample regions, metadata outputs, sample maps, zone selection, velocity layers, round-robin alternates, explicit slicing, modest looping, choke groups, and realtime-safe playback requirements.

### Modified Capabilities

- `built-in-modules`: The built-in module registry will include the sample source/playback primitives and any required selector/voice/choke/metadata helpers.
- `yaml-patch-format`: Patch assets may declare sample sources, maps, regions, zones, slices, cue points, beat-grid metadata, and per-region playback metadata.
- `instrument-presets`: Presets may expose public controls such as sample source choice, sample map choice, layer mix, start offset, pitch, decay, choke group selection, loop mode, slice index, and variation amount without exposing internal module IDs.
- `plugin-integration`: The plugin parameter surface should expose advanced sampling controls from the preset surface like any other instrument while preserving the realtime callback contract.
- `module-library`: Bundled modules may compose advanced sampling primitives into reusable drum-kit, break-slicer, and chromatic-sampler building blocks.

## Impact

- Engine: asset preparation needs to understand sample sources, sample maps, regions, slices, looping metadata, analysis metadata, deterministic selection state, and voice/choke ownership.
- Rendering: sampling primitives must be deterministic, block-splitting safe, and free of steady-state heap allocation.
- Validation: malformed regions, invalid loop points, missing files, unsupported formats, overlapping/ambiguous zones, invalid analysis metadata, and invalid choke/group declarations must produce structured diagnostics before rendering.
- Tests: add behaviour-first tests for source metadata, selection, velocity/key mapping, round-robin determinism, loop boundaries, slice triggering, reverse playback, choke groups, and no render-time allocation.
- Examples: add small sample-based patches using freely redistributable or synthetic reference assets only.
- Non-goal: this change does not require proprietary sample libraries, Kontakt/SFZ import, full workstation sampler features, disk streaming, DJ deck streaming, realtime time-stretching, granular synthesis, spectral resynthesis, or a custom waveform editor UI.
