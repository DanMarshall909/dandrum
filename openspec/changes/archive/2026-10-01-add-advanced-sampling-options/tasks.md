## 1. Specification And Validation Surface

- [x] 1.1 Define the `advanced-sampling-options` capability and its sample asset/map requirements for drum-machine, break-slicer, and modest chromatic sampling only.
- [x] 1.2 Add or extend built-in module registry entries for `sample_player`, `sample_zone_selector`, optional thin `sample_map_player`, `sample_slicer`, and `voice_choke` behaviour.
- [x] 1.3 Extend YAML/schema validation to accept sample assets, regions, simple loops, explicit slices, sample maps, zones, and choke groups.
- [x] 1.4 Add structured diagnostics for missing files, unsupported decode formats, malformed regions, invalid loop points, invalid velocity/key ranges, invalid voice limits, and unsupported interpolation/choke modes.
- [x] 1.5 Confirm naming follows the project module terminology and does not reintroduce composite-specific user-facing names.
- [x] 1.6 Reject or defer workstation-sampler, creative/granular/time-stretch, and streaming-specific declarations to their separate specs.

## 2. Sample Asset Preparation

- [x] 2.1 Resolve sample paths relative to the patch/module package root using the existing asset resolution rules.
- [x] 2.2 Decode supported sample formats off the audio thread into engine-owned buffers.
- [x] 2.3 Validate region start/end frames, root note, gain, pan, reverse, fades, simple loop points, and loop crossfades.
- [x] 2.4 Prepare sample maps into deterministic render-time lookup structures.
- [x] 2.5 Prepare slice tables from explicit metadata; defer transient auto-detection unless it can be done entirely during preparation.
- [x] 2.6 Allocate all voice state, scratch buffers, and lookup tables required for steady-state rendering.

## 3. Primitive Playback Rendering

- [x] 3.1 Implement or extend `sample_player` one-shot region playback.
- [x] 3.2 Implement gated playback where release/stop behaviour is externally observable.
- [x] 3.3 Implement simple looped playback with validated loop start/end and optional crossfade.
- [x] 3.4 Implement pitch-ratio playback using deterministic interpolation.
- [x] 3.5 Implement reverse playback from prepared region metadata.
- [x] 3.6 Implement fade-in and fade-out at region boundaries.
- [x] 3.7 Verify oversized block splitting produces identical output to equivalent smaller blocks.

## 4. Primitive Zone Selection

- [x] 4.1 Implement key-range selection from incoming note events.
- [x] 4.2 Implement velocity-range selection from incoming note events.
- [x] 4.3 Implement deterministic round-robin selection per group.
- [x] 4.4 Implement deterministic weighted/probability selection where enabled.
- [x] 4.5 Implement per-zone gain, pan, pitch offset, and region override metadata.
- [x] 4.6 Ensure selection is independent of hashmap iteration order, filesystem order, wall-clock time, and audio block size.
- [x] 4.7 Decide whether `sample_zone_selector` is exposed directly now or kept internal behind a thin `sample_map_player` until structured events are ready.

## 5. Voice And Choke Behaviour

- [x] 5.1 Implement bounded `max_voices` handling for sample playback modules.
- [x] 5.2 Implement configured voice stealing: `oldest`, `quietest`, or `reject_new` where supported.
- [x] 5.3 Implement exclusive/choke groups for mutually exclusive articulations.
- [x] 5.4 Make choke behaviour sample-accurate within a block using event frame offsets.
- [x] 5.5 Support `cut`, `fade`, or `release` choke modes where implemented; reject unsupported modes during preparation.

## 6. Slice Playback

- [x] 6.1 Implement explicit slice-table playback by numeric slice index.
- [x] 6.2 Support sequential and deterministic random slice selection only if required by examples.
- [x] 6.3 Add a chopped-break example using explicit slice metadata.
- [x] 6.4 Defer tempo-sync/time-stretch behaviour to a separate creative or streaming spec.

## 7. Patch And Preset Examples

- [x] 7.1 Add a minimal one-shot sample patch.
- [x] 7.2 Add a layered electronic drum patch using velocity layers and round-robin alternates.
- [x] 7.3 Add an open/closed hi-hat patch proving choke groups.
- [x] 7.4 Add a modest chromatic sample playback patch using root note and pitch ratio.
- [x] 7.5 Add a sliced break patch using explicit slice metadata.
- [x] 7.6 Add preset surfaces exposing musical controls without exposing internal module IDs.

## 8. Verification

- [x] 8.1 Add registry tests proving each sampling primitive exposes the expected ports and parameters.
- [x] 8.2 Add preparation tests proving valid sample assets/maps are accepted.
- [x] 8.3 Add preparation tests proving malformed files, regions, loops, zones, and choke declarations fail with structured diagnostics.
- [x] 8.4 Add render tests proving one-shot, gated, simple-looped, reverse, pitch-ratio, fade, and crossfade behaviour.
- [x] 8.5 Add selection tests proving velocity/key matching, round-robin order, weighted random determinism, and block-size independence.
- [x] 8.6 Add choke tests proving sample-accurate mutually exclusive playback.
- [x] 8.7 Add slice tests proving trigger/index behaviour.
- [x] 8.8 Add tests or instrumentation proving the steady-state render path performs no heap allocation.
- [x] 8.9 Run `openspec validate add-advanced-sampling-options --strict` and fix validation errors.

## 9. Sampler VST3 Example And External Modulation

- [x] 9.1 Add a drum sampler patch with named public pitch, start, level, pan, and variation controls that affect rendered audio while it runs.
- [x] 9.2 Provide a sampler-focused VST3 example that loads the drum patch and its redistributable reference samples by default.
- [x] 9.3 Prove JUCE host parameter automation reaches the corresponding Rust sampler controls and changes rendered audio without replacing the instrument.
- [x] 9.4 Document MIDI drum-note mapping, host modulation setup, sample asset location, and the preparation-only choices that require reload.
- [x] 9.5 Play prepared samples at the correct pitch and duration across common host sample rates without file IO, decoding, or allocation on the audio thread.
- [x] 9.6 Expose per-pad host controls for kick, snare, closed hat, and open hat while retaining shared kit voice and choke behavior.
- [x] 9.7 Verify the prepared instrument owns typed sample identities, decoded rate and bounds, loops, slices, and map metadata after source input is released; document the reload lifetime boundary for future UI access while deferring visualization analysis and browser transport.
