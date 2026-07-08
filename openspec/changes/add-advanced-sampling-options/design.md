## Context

Dandrum's sampling path should follow the same architectural rule as the rest of the engine: patch structure and assets are prepared up front, while the realtime render path performs bounded DSP work only. Advanced sampling is valuable because it lets a small DSP core cover a wide range of instruments: drum machines, layered kits, sliced breaks, pitched one-shots, acoustic-style velocity maps, and hybrid sample+synth patches.

The engine already has a module graph, typed ports, patch preparation, sample loading concepts, module-library direction, and plugin constraints. This spec extends those concepts rather than introducing a separate sampler subsystem.

This change covers the first practical sampling layer only: drum-machine sampling, explicit breakbeat slicing, and modest chromatic sample playback. Full workstation sampling, creative/granular/time-stretch sampling, and DJ-style streaming are intentionally separate specs, but they should extend the same sample source/metadata/playback model rather than introducing competing sampler modules.

## Goals / Non-Goals

**Goals:**

- Provide reusable sampling primitives suitable for drum-machine, break-slicer, and modest chromatic/instrument patches.
- Support prepared sample sources with metadata, regions, explicit slices, simple loops, root note, gain, pan, and playback metadata.
- Support metadata outputs for duration, sample rate, channel count, region length, root note, slice markers, cue points, detected tempo/beat grid where available, and analysis confidence.
- Support sample maps with key ranges, velocity ranges, round-robin/probability alternates, and deterministic selection.
- Support one-shot, gated, simple-looped, reversed, pitched, sliced, and choke-group playback.
- Keep all decoding, validation, metadata preparation, preparation-time resampling where needed, optional beat/tempo analysis, and allocation off the audio thread.
- Make sample-based patches easy for LLMs to author by using explicit YAML declarations and clear diagnostics.
- Preserve deterministic rendering across identical patches, sample assets, seeds, and render settings.
- Build around small reusable primitives that can be composed into larger modules through the module library.
- Ensure later streaming, workstation, and creative sampling features reuse the same source/metadata concepts where possible.

**Non-Goals:**

- Implement a full DAW sampler, Kontakt clone, SFZ importer, articulation engine, keyswitch system, or sample editor.
- Implement disk streaming or DJ deck-style sample playback in this change.
- Implement realtime time-stretching, pitch-shifting with formant preservation, granular synthesis, or spectral resynthesis.
- Bundle proprietary samples or copy commercial sample-library mappings.
- Build a custom plugin waveform/slice editor in this change.

## Decisions

### Use a unified sample source model

Dandrum should not have one sampler for drums, another for slicing, another for chromatic playback, and another unrelated streaming deck. The shared concept should be a `sample_source`: an audio source plus prepared metadata. Short preloaded samples and future long streaming files are different source implementations, not different musical sampler concepts.

A source can expose regions, slices, cue points, duration, channel layout, sample rate, optional detected tempo, optional beat grid, optional downbeat markers, and optional analysis confidence. Playback, slicing, zone selection, and transport primitives should consume that common source/metadata model.

### Treat sample sources, regions, maps, and analysis as prepared assets

Patch YAML should declare sample sources, sample regions, explicit slices, cue points, beat metadata, and sample maps under the asset/preparation layer, not as ad hoc strings hidden inside module parameters. Preparation resolves files, validates regions, decodes audio into engine-owned sample buffers for preloaded sources, converts channel layouts where required, and builds lookup structures for render-time selection.

The render path should not read files, decode formats, scan slices, run beat detection, allocate vectors, build maps, or recover from malformed metadata.

### Keep the primitive set small

The first useful set should be:

- `sample_source` — prepared source identity and metadata, not necessarily an audio-rendering module.
- `sample_metadata` — exposes prepared source metadata as control/event values where the graph needs it.
- `sample_player` — plays a prepared sample source region as one-shot, gated, simple-looped, reversed, or pitched audio.
- `sample_zone_selector` — selects a prepared region/zone from key, velocity, round-robin, probability, and seed.
- `sample_map_player` — optional convenience wrapper if it remains a thin composition of zone selection plus sample playback.
- `sample_slicer` — plays one explicit slice from a prepared slice table, suitable for chopped breaks and rhythmic one-shots.
- `voice_choke` — stops, fades, or releases previous voices in the same exclusive group.

If existing primitives already cover part of this behaviour, the implementation should extend/reuse them instead of adding duplicate names.

### Split source, metadata, selection, playback, and voice handling

The graph should be able to compose source metadata, selection, playback, and voice management separately where practical:

```text
sample_source metadata
   -> sample_metadata exposes duration / tempo / beat grid / slice count where needed

note event + velocity
   -> sample_zone_selector selects prepared region/zone
   -> sample_player voice renders region playback
   -> envelope/filter/modulation/effects modules shape the result
   -> audio_output
```

A `sample_map_player` may exist as a convenience module, but the underlying implementation should still be factored as selector + player + voice/choke behaviour. This avoids a single expanding sampler module that gradually absorbs unrelated workstation-sampler features.

### Metadata should be useful, not decorative

Source metadata should support real patch behaviour:

- duration in frames/seconds,
- sample rate,
- channel count/layout,
- root note where known,
- region length,
- slice count,
- cue point positions,
- detected tempo where available,
- beat grid/downbeat positions where available,
- analysis confidence,
- analysis status/error diagnostics.

Metadata values must be prepared off the audio thread and stable for the loaded source. Where analysis is unavailable or low confidence, the graph should receive explicit absence/diagnostic state rather than fake values.

### Beat detection is preparation-time analysis

Beat detection is valuable for slicing, DJ-style streaming, tempo-aware triggering, and future creative sampling, but it must not run in the audio callback. This spec may define the metadata shape and support explicit beat-grid metadata. Automatic beat detection may be introduced as a preparation-time analysis step if it is deterministic, bounded, testable, and reports confidence.

For v1, explicit beat-grid/slice metadata can be supported before automatic detection. Automatic detection should not block the primitive playback work.

### Make selection deterministic

Round-robin and probability selection must be deterministic for identical render inputs. Each prepared selector/player instance should own seeded selection state. Re-rendering the same event stream with the same seed should produce the same selected zones.

Random/probability behaviour should never depend on hashmap iteration order, thread timing, wall-clock time, filesystem order, or audio block size.

### Prefer preloaded sources for this capability, but keep the source contract streaming-compatible

This capability should preload decoded sample buffers into memory during preparation. Disk streaming is deliberately out of scope because it has a different realtime contract, buffering model, failure mode, and plugin/session portability concern.

However, the asset model should not make streaming a separate sampler family. A future streaming source should implement the same source metadata contract and feed compatible transport/playback primitives where possible.

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
  sample_sources:
    - id: amen_break
      path: samples/amen.wav
      analysis:
        tempo_bpm: 136
        confidence: 0.92
        beat_grid:
          unit: frames
          downbeats: [0, 44100]
          beats: [0, 11025, 22050, 33075, 44100]
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
      cues:
        - id: first_downbeat
          frame: 0

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

### `sample_metadata`

Inputs:

- `source` — static prepared sample source ID, or equivalent module parameter.

Outputs:

- `duration_frames` (`control`, optional).
- `duration_seconds` (`control`, optional).
- `sample_rate` (`control`, optional).
- `channel_count` (`control`, optional).
- `tempo_bpm` (`control`, optional when known).
- `beat_count` (`control`, optional when known).
- `slice_count` (`control`, optional when known).
- `analysis_confidence` (`control`, optional when known).

Static parameters:

- `source` — prepared sample source ID.
- `missing_value` — configured value for unavailable numeric metadata where a control output must emit something.

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
- `position` (`control`, optional) — current playback position inside the region/source where useful.

Static parameters:

- `source` — prepared sample source ID.
- `region` — prepared sample region ID or inline source window.
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
- `slice_position` (`control`, optional) — current position within the slice.

Static parameters:

- `source` — prepared sample source ID.
- `slice_table` — explicit slice table ID or source slice metadata.
- `selection_mode` — `index`, `sequential`, `random_weighted`, or `midi_note` where supported.
- `sync_mode` — `free` for this capability. Tempo-sync/time-stretch belongs in a later creative/streaming spec, but the source metadata should already be able to carry beat-grid information.

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

- Sample source IDs, region IDs, sample-map IDs, zone IDs, cue IDs, beat-grid IDs, and choke group IDs must be stable strings.
- Missing files are hard preparation errors.
- Unsupported decode formats are hard preparation errors.
- Regions must have valid start/end frame ranges after decode.
- Loop ranges must be inside the region and must have enough frames for the selected interpolation/crossfade mode.
- Explicit beat-grid, cue, and slice markers must be inside the source duration.
- Detected beat/tempo metadata must include confidence and provenance when generated automatically.
- Velocity ranges must be inside `1..=127`.
- MIDI key ranges must be inside `0..=127`.
- Zone selection ties must be deterministic. Ambiguous overlaps are allowed only when an explicit selection mode resolves them.
- `max_voices` must be bounded and validated before rendering.
- Choke groups must not require graph mutation during rendering.
- Streaming-specific buffering settings are rejected in this capability and belong to the sample-streaming spec, but source metadata shapes should stay compatible.
- Workstation-sampler articulation/key-switch/release-trigger settings are rejected in this capability and belong to a workstation-sampling spec.
- Granular/time-stretch settings are rejected in this capability and belong to a creative-sampling spec.

## Testing Strategy

- Asset tests prove files/sources/regions/maps/metadata are accepted or rejected before rendering.
- Metadata tests prove duration, sample rate, channel count, slice counts, cue points, and explicit beat-grid metadata are prepared deterministically.
- Registry tests prove sampling primitives expose the expected ports and static parameters.
- Render tests prove one-shot playback, pitched playback, reverse playback, loop boundaries, fades, and crossfades.
- Selection tests prove velocity/key matching, round-robin order, weighted random determinism, and block-size independence.
- Choke tests prove open/closed hat style behaviour with sample-accurate offsets.
- Allocation tests or instrumentation prove steady-state rendering performs no heap allocation.

## Open Questions

- Whether `sample_player` should emit mono, stereo, or channel-count-matched outputs in v1. Prefer explicit mono/stereo surfaces that match current engine conventions.
- Whether `sample_zone_selector` should produce a structured event type immediately or be introduced behind `sample_map_player` until the event model is mature enough.
- Whether automatic beat detection belongs in the first implementation or whether v1 should accept explicit beat-grid metadata only.
- Whether metadata outputs should be control ports, structured event outputs, or preparation-time query data available to the plugin/editor only.
