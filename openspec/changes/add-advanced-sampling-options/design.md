## Context

Dandrum's sampling path should follow the same architectural rule as the rest of the engine: patch structure and assets are prepared up front, while the realtime render path performs bounded DSP work only. Advanced sampling is valuable because it lets a small DSP core cover a wide range of instruments: drum machines, layered kits, sliced breaks, pitched one-shots, acoustic-style velocity maps, and hybrid sample+synth patches.

The engine already has a module graph, typed ports, patch preparation, sample loading concepts, module-library direction, and plugin constraints. This spec extends those concepts rather than introducing a separate sampler subsystem.

This change covers the first practical sampling layer only: drum-machine sampling, explicit breakbeat slicing, and modest chromatic sample playback. Full workstation sampling, creative/granular/time-stretch sampling, and DJ-style streaming are intentionally separate specs.

## Goals / Non-Goals

**Goals:**

- Provide reusable sampling primitives suitable for drum-machine, break-slicer, and modest chromatic/instrument patches.
- Support prepared sample assets with regions, explicit slices, simple loops, root note, gain, pan, and playback metadata.
- Support sample maps with key ranges, velocity ranges, round-robin/probability alternates, and deterministic selection.
- Support one-shot, gated, simple-looped, reversed, pitched, sliced, and choke-group playback.
- Keep all decoding, validation, metadata preparation, preparation-time resampling where needed, and allocation off the audio thread.
- Make sample-based patches easy for LLMs to author by using explicit YAML declarations and clear diagnostics.
- Preserve deterministic rendering across identical patches, sample assets, seeds, and render settings.
- Build around small reusable primitives that can be composed into larger modules through the module library.

**Non-Goals:**

- Implement a full DAW sampler, Kontakt clone, SFZ importer, articulation engine, keyswitch system, or sample editor.
- Implement disk streaming or DJ deck-style sample playback in this change.
- Implement realtime time-stretching, pitch-shifting with formant preservation, granular synthesis, or spectral resynthesis.
- Bundle proprietary samples or copy commercial sample-library mappings.
- Build a custom plugin waveform/slice editor in this change.

## Decisions

### Treat sample regions and maps as prepared assets

Patch YAML should declare sample assets, sample regions, explicit slices, and sample maps under the asset/preparation layer, not as ad hoc strings hidden inside module parameters. Preparation resolves files, validates regions, decodes audio into engine-owned sample buffers, converts channel layouts where required, and builds lookup structures for render-time selection.

The render path should not read files, decode formats, scan slices, allocate vectors, build maps, or recover from malformed metadata.

### Keep the primitive set small

The first useful set should be:

- `sample_player` — plays a prepared sample region as one-shot, gated, simple-looped, reversed, or pitched audio.
- `sample_zone_selector` — selects a prepared region/zone from key, velocity, round-robin, probability, and seed.
- `sample_map_player` — optional convenience wrapper if it remains a thin composition of zone selection plus sample playback.
- `sample_slicer` — plays one explicit slice from a prepared slice table, suitable for chopped breaks and rhythmic one-shots.
- `voice_choke` — stops, fades, or releases previous voices in the same exclusive group.

If existing primitives already cover part of this behaviour, the implementation should extend/reuse them instead of adding duplicate names.

### Split selection from playback

Selection and playback should be separable where practical:

```text
note event + velocity
   -> sample_zone_selector selects prepared region/zone
   -> sample_player voice renders region playback
   -> envelope/filter/modulation/effects modules shape the result
   -> audio_output
```

A `sample_map_player` may exist as a convenience module, but the underlying implementation should still be factored as selector + player + voice/choke behaviour. This avoids a single expanding sampler module that gradually absorbs unrelated workstation-sampler features.

### Make selection deterministic

Round-robin and probability selection must be deterministic for identical render inputs. Each prepared selector/player instance should own seeded selection state. Re-rendering the same event stream with the same seed should produce the same selected zones.

Random/probability behaviour should never depend on hashmap iteration order, thread timing, wall-clock time, filesystem order, or audio block size.

### Prefer preloaded samples for this capability

This capability should preload decoded sample buffers into memory during preparation. Disk streaming is deliberately out of scope because it has a different realtime contract, buffering model, failure mode, and plugin/session portability concern.

DJ-style long-file streaming is valuable, but it belongs in the separate sample-streaming spec rather than being hidden inside `sample_player`.

### Define loops and fades as region metadata

Loop points, loop crossfade, fade-in, fade-out, reverse playback, and start/end frames belong to the prepared region metadata. The realtime player reads this prepared metadata and performs bounded interpolation/crossfade work.

Invalid loop points are hard validation failures. Unsupported crossfade modes or interpolation modes are hard validation failures, not silent fallbacks.

Looping in this capability is modest region looping for sampled instruments and sustained textures. It is not host-synced DJ looping, warped looping, or time-stretched phrase playback.

### Choke groups are voice-management behaviour

Open/closed hi-hats and mutually exclusive articulations should be represented as exclusive/choke groups. Triggering a voice in a group should stop or fade existing voices in the same group according to a configured mode.

Choke behaviour must be sample-accurate within the current block: an event at frame `N` affects voices from frame `N` onward, not at the beginning of the block unless the event offset is zero.

## Proposed YAML Shape

The exact schema can evolve during implementation, but the intent is:

```yaml
assets:
  samples:
    - id: amen_break
      path: samples/amen.wav
      regions:
        - id: full
          start_frame: 0
          end_frame: 88200
          root_note: 60
          gain_db: -3
          loop:
            mode: off
        - id: slice_01
          start_frame: 0
          end_frame: 5512
          fade_out_ms: 2

  sample_maps:
    - id: acoustic_kick_map
      selection_seed: 12345
      zones:
        - region: kick_soft
          key_range: [36, 36]
          velocity_range: [1, 70]
          round_robin_group: kick_soft
        - region: kick_hard_1
          key_range: [36, 36]
          velocity_range: [71, 127]
          round_robin_group: kick_hard
        - region: kick_hard_2
          key_range: [36, 36]
          velocity_range: [71, 127]
          round_robin_group: kick_hard
```

## Proposed Primitive Surfaces

### `sample_player`

Inputs:

- `trigger` (`event`) — starts playback of the configured region.
- `gate` (`control` or `event`, optional) — starts/stops gated playback where supported.
- `pitch_ratio` (`control`, optional) — multiplicative playback-rate modulation.
- `start_offset` (`control`, optional) — normalized offset within the prepared region.
- `level` (`control`, optional) — linear playback gain.
- `pan` (`control`, optional) — normalized pan where stereo output is generated by the module.

Outputs:

- `audio_out` (`audio`) for mono playback or `left`/`right` (`audio`) for stereo-capable playback, matching existing project conventions.
- `playing` (`control`, optional) — non-zero while active, useful for diagnostics or modulation.

Static parameters:

- `region` — prepared sample region ID.
- `mode` — `one_shot`, `gated`, or `looped`.
- `interpolation` — `nearest`, `linear`, or `cubic` where supported.
- `reverse` — boolean.
- `max_voices` — bounded voice count for polyphonic triggering.
- `voice_steal` — `oldest`, `quietest`, or `reject_new`.
- `choke_group` — optional exclusive group ID.
- `choke_mode` — `cut`, `fade`, or `release` where supported.

### `sample_zone_selector`

Inputs:

- `note` (`event` or note-event stream) — incoming note event with note number and velocity.
- `variation` (`control`, optional) — selects between deterministic variation/probability behaviours where supported.

Outputs:

- `selected_region` (`event` or structured control/event output) — selected prepared region plus per-zone playback modifiers.

Static parameters:

- `sample_map` — prepared sample map ID.
- `selection_mode` — `first_match`, `round_robin`, `random_weighted`, or `round_robin_then_random` where supported.
- `selection_seed` — deterministic seed overriding/augmenting the sample-map seed.

### `sample_map_player`

`sample_map_player` MAY exist as a convenience module only when it does not become a dumping ground for full sampler features. Its behaviour should be equivalent to composing `sample_zone_selector`, `sample_player`, and voice/choke handling.

Inputs:

- `note` (`event` or note-event stream) — incoming note event with note number and velocity.
- `pitch_ratio` (`control`, optional) — additional pitch modulation.
- `level` (`control`, optional) — global gain.
- `variation` (`control`, optional).

Outputs:

- `audio_out` or stereo audio outputs according to module convention.

Static parameters:

- `sample_map` — prepared sample map ID.
- `selection_mode`, `selection_seed`, `max_voices`, `voice_steal`, `choke_group`, `choke_mode`.

### `sample_slicer`

Inputs:

- `trigger` (`event`) — starts the selected slice.
- `slice_index` (`control`) — selected slice number or normalized position.
- `pitch_ratio` (`control`, optional).
- `level` (`control`, optional).

Outputs:

- `audio_out` or stereo audio outputs according to module convention.

Static parameters:

- `sample` — prepared sample ID.
- `slice_table` — explicit slice table ID or inline region/slice metadata.
- `selection_mode` — `index`, `sequential`, `random_weighted`, or `midi_note` where supported.
- `sync_mode` — `free` for this capability. Tempo-sync/time-stretch belongs in a later creative/streaming spec.

### `voice_choke`

Inputs:

- `trigger` or selected voice event.
- Optional audio/control inputs if implemented as an explicit graph module rather than voice-manager behaviour.

Outputs:

- Choked voice event/audio according to implementation shape.

Static parameters:

- `group` — exclusive group ID.
- `mode` — `cut`, `fade`, or `release` where supported.
- `fade_ms` — bounded fade duration for `fade` mode.

## Validation Rules

- Sample IDs, region IDs, sample-map IDs, zone IDs, and choke group IDs must be stable strings.
- Missing files are hard preparation errors.
- Unsupported decode formats are hard preparation errors.
- Regions must have valid start/end frame ranges after decode.
- Loop ranges must be inside the region and must have enough frames for the selected interpolation/crossfade mode.
- Velocity ranges must be inside `1..=127`.
- MIDI key ranges must be inside `0..=127`.
- Zone selection ties must be deterministic. Ambiguous overlaps are allowed only when an explicit selection mode resolves them.
- `max_voices` must be bounded and validated before rendering.
- Choke groups must not require graph mutation during rendering.
- Streaming-specific settings are rejected in this capability and belong to the sample-streaming spec.
- Workstation-sampler articulation/key-switch/release-trigger settings are rejected in this capability and belong to a workstation-sampling spec.
- Granular/time-stretch settings are rejected in this capability and belong to a creative-sampling spec.

## Testing Strategy

- Asset tests prove files/regions/maps are accepted or rejected before rendering.
- Registry tests prove sampling primitives expose the expected ports and static parameters.
- Render tests prove one-shot playback, pitched playback, reverse playback, loop boundaries, fades, and crossfades.
- Selection tests prove velocity/key matching, round-robin order, weighted random determinism, and block-size independence.
- Choke tests prove open/closed hat style behaviour with sample-accurate offsets.
- Allocation tests or instrumentation prove steady-state rendering performs no heap allocation.

## Open Questions

- Whether `sample_player` should emit mono, stereo, or channel-count-matched outputs in v1. Prefer explicit mono/stereo surfaces that match current engine conventions.
- Whether `sample_zone_selector` should produce a structured event type immediately or be introduced behind `sample_map_player` until the event model is mature enough.
- Whether slice detection is imported only from explicit metadata in v1, or whether simple transient detection is prepared offline. Prefer explicit metadata first.
