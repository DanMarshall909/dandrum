## 1. Specification And Validation Surface

- [ ] 1.1 Define the `advanced-sampling-options` capability and its sample asset/map requirements.
- [ ] 1.2 Add or extend built-in module registry entries for `sample_player`, `sample_map_player`, `sample_slicer`, and voice/choke behaviour.
- [ ] 1.3 Extend YAML/schema validation to accept sample assets, regions, loops, slices, sample maps, zones, and choke groups.
- [ ] 1.4 Add structured diagnostics for missing files, unsupported decode formats, malformed regions, invalid loop points, invalid velocity/key ranges, invalid voice limits, and unsupported interpolation/choke modes.
- [ ] 1.5 Confirm naming follows the project module terminology and does not reintroduce composite-specific user-facing names.

## 2. Sample Asset Preparation

- [ ] 2.1 Resolve sample paths relative to the patch/module package root using the existing asset resolution rules.
- [ ] 2.2 Decode supported sample formats off the audio thread into engine-owned buffers.
- [ ] 2.3 Validate region start/end frames, root note, gain, pan, reverse, fades, loop points, and loop crossfades.
- [ ] 2.4 Prepare sample maps into deterministic render-time lookup structures.
- [ ] 2.5 Prepare slice tables from explicit metadata; defer transient auto-detection unless it can be done entirely during preparation.
- [ ] 2.6 Allocate all voice state, scratch buffers, and lookup tables required for steady-state rendering.

## 3. Sample Playback Rendering

- [ ] 3.1 Implement or extend one-shot region playback.
- [ ] 3.2 Implement gated playback where release/stop behaviour is externally observable.
- [ ] 3.3 Implement looped playback with validated loop start/end and optional crossfade.
- [ ] 3.4 Implement pitch-ratio playback using deterministic interpolation.
- [ ] 3.5 Implement reverse playback from prepared region metadata.
- [ ] 3.6 Implement fade-in and fade-out at region boundaries.
- [ ] 3.7 Verify oversized block splitting produces identical output to equivalent smaller blocks.

## 4. Sample Map Selection

- [ ] 4.1 Implement key-range selection from incoming note events.
- [ ] 4.2 Implement velocity-range selection from incoming note events.
- [ ] 4.3 Implement deterministic round-robin selection per group.
- [ ] 4.4 Implement deterministic weighted/probability selection where enabled.
- [ ] 4.5 Implement per-zone gain, pan, pitch offset, and region override metadata.
- [ ] 4.6 Ensure selection is independent of hashmap iteration order, filesystem order, wall-clock time, and audio block size.

## 5. Voice And Choke Behaviour

- [ ] 5.1 Implement bounded `max_voices` handling for sample playback modules.
- [ ] 5.2 Implement configured voice stealing: `oldest`, `quietest`, or `reject_new` where supported.
- [ ] 5.3 Implement exclusive/choke groups for mutually exclusive articulations.
- [ ] 5.4 Make choke behaviour sample-accurate within a block using event frame offsets.
- [ ] 5.5 Support `cut`, `fade`, or `release` choke modes where implemented; reject unsupported modes during preparation.

## 6. Slice Playback

- [ ] 6.1 Implement explicit slice-table playback by numeric slice index.
- [ ] 6.2 Support sequential and deterministic random slice selection only if required by examples.
- [ ] 6.3 Add a chopped-break example using explicit slice metadata.
- [ ] 6.4 Defer tempo-sync/time-stretch behaviour unless a separate spec adds a realtime-safe contract.

## 7. Patch And Preset Examples

- [ ] 7.1 Add a minimal one-shot sample patch.
- [ ] 7.2 Add a layered electronic drum patch using velocity layers and round-robin alternates.
- [ ] 7.3 Add an open/closed hi-hat patch proving choke groups.
- [ ] 7.4 Add a chromatic sample playback patch using root note and pitch ratio.
- [ ] 7.5 Add a sliced break patch using explicit slice metadata.
- [ ] 7.6 Add preset surfaces exposing musical controls without exposing internal module IDs.

## 8. Verification

- [ ] 8.1 Add registry tests proving each sampling primitive exposes the expected ports and parameters.
- [ ] 8.2 Add preparation tests proving valid sample assets/maps are accepted.
- [ ] 8.3 Add preparation tests proving malformed files, regions, loops, zones, and choke declarations fail with structured diagnostics.
- [ ] 8.4 Add render tests proving one-shot, gated, looped, reverse, pitch-ratio, fade, and crossfade behaviour.
- [ ] 8.5 Add selection tests proving velocity/key matching, round-robin order, weighted random determinism, and block-size independence.
- [ ] 8.6 Add choke tests proving sample-accurate mutually exclusive playback.
- [ ] 8.7 Add slice tests proving trigger/index behaviour.
- [ ] 8.8 Add tests or instrumentation proving the steady-state render path performs no heap allocation.
- [ ] 8.9 Run `openspec validate add-advanced-sampling-options --strict` and fix validation errors.
