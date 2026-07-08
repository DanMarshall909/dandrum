## Context

Sample streaming is for DJ-style and long-form playback: full tracks, long loops, stems, backing audio, and deck-like workflows. It should not be a separate sampler family because the rest of Dandrum also needs sample source metadata, regions, slices, cue points, and beat grids.

Streaming should therefore extend the unified sample source model:

```text
sample_source
   -> source metadata / analysis metadata
   -> stream_source or playback primitive
   -> transport / selection / slicing / effects
```

A preloaded source and a streaming source have different memory and IO contracts, but they should expose compatible metadata wherever possible.

The realtime primitive should be named `stream_source`, not `sample_stream_source`. Sample-file streaming is the first stream kind, but the primitive should be general enough to later support other bounded realtime streams without duplicating transport, buffering, metadata, or status concepts.

## Goals / Non-Goals

**Goals:**

- Stream long sample files without decoding the whole file into memory.
- Keep audio callback behaviour bounded and allocation-free.
- Support transport-like controls suitable for DJing.
- Preserve deterministic state transitions for play/stop/cue/seek events.
- Make underrun behaviour explicit and testable.
- Expose useful source and transport metadata to the graph/plugin.
- Support explicit beat-grid metadata and future preparation-time beat detection.
- Support independent user intent for target BPM and pitch shift even when v1 only implements pitch-linked rate playback.
- Keep streaming aligned with `sample_source`, `sample_metadata`, regions, slices, and cue points from the advanced sampling model.
- Keep the streaming primitive generic enough for future stream kinds such as live input, network input, generated streams, stem streams, or inter-plugin streams where the realtime contract matches.

**Non-Goals:**

- Replace preloaded `sample_player` for drum hits and short regions.
- Implement full DJ software in the first slice.
- Perform blocking file IO, decoding, allocation, beat detection, or filesystem discovery in the audio callback.
- Introduce a second, incompatible sample metadata model.
- Implement pitch-preserved time-stretch in v1. The API should reserve the control shape, but unsupported independent tempo/pitch rendering must fail validation or degrade explicitly.
- Support arbitrary non-audio streaming protocols in the first implementation.

## Decisions

### Streaming is a source implementation, not a new sampler type

`stream_source` should represent a bounded realtime stream from a prepared source. The first concrete stream kind should be sample-file backed streaming. It should expose the same general metadata shape as preloaded sources plus streaming-specific state such as buffered range, current decode position, underrun status, and background worker state.

A future `sample_player` may consume source windows from preloaded sources, while `stream_source` renders continuous transport playback. They share source identity, metadata, cue points, beat-grid data, and analysis state where the source kind supports those concepts.

### `stream_source` is generic, but source kinds are explicit

The primitive name should stay generic, while source-specific behaviour is selected through prepared source kind/type metadata. For sample-file streaming, the source kind might be:

```yaml
assets:
  sample_sources:
    - id: track_01
      kind: sample_file
      path: tracks/track_01.wav
```

Future stream kinds may be added only when they can honour the realtime contract and expose a coherent metadata surface. The implementation should not add protocol-specific stream primitives unless their behaviour cannot share the common transport/buffer/status model.

### Streaming source metadata is part of the feature

A streaming source should expose metadata that is useful for DJing, patch authoring, and UI display:

- source duration in frames/seconds,
- sample rate,
- channel count/layout,
- current playback position,
- normalized position,
- remaining time,
- loaded/buffered frame range,
- buffer fill level,
- underrun state/count,
- transport state,
- detected or declared tempo,
- beat phase,
- nearest/next beat,
- nearest/next downbeat,
- cue points,
- loop points,
- analysis confidence,
- analysis status/error diagnostics.

Metadata should be stable where it describes the source and updated through bounded state where it describes transport/buffering.

### Beat detection is preparation/background analysis

Beat detection must not run in the audio callback. The streaming spec should support:

1. explicit beat-grid metadata declared in YAML or sidecar metadata,
2. preparation-time beat detection for files where bounded analysis is acceptable,
3. background beat analysis that updates inactive/prepared source metadata off the audio thread, if the engine/plugin can publish the result safely.

The graph should be able to distinguish declared beat grids, detected beat grids, missing beat grids, and low-confidence analysis.

### Transport is separate from source analysis

Transport controls should not be mixed into file decoding or beat analysis. A `stream_transport` primitive or internal transport component should own play/stop/cue/seek/rate state. Source analysis owns static metadata. Buffering owns IO/decode state.

Separating these makes it easier to reuse metadata and analysis for slicers, sample players, and future creative sampling.

### Tempo intent and pitch intent are separate controls

Dandrum should allow users/modules to set target BPM and pitch independently, even before pitch-preserved rendering exists. The control model should distinguish:

- source tempo metadata, usually `source_bpm`,
- target tempo, usually `target_bpm` from host/deck/manual control,
- tempo ratio, usually `target_bpm / source_bpm`,
- pitch shift, in semitones or pitch ratio,
- manual deck rate/pitch-fader adjustment,
- temporary nudge/bend adjustment.

For v1 rate playback, tempo and pitch are physically linked by resampling. The engine may still accept separate intent values, but the selected tempo mode determines whether they can both be honoured.

Supported/expected modes:

- `free` — source plays at original speed and pitch unless manual pitch/rate is applied.
- `rate` — playback-rate ratio changes tempo and pitch together.
- `beat_locked_rate` — derives playback-rate ratio from `target_bpm / source_bpm`; pitch changes with tempo like a turntable/CDJ pitch fader.
- `stretch` — future mode where target BPM and pitch shift can be honoured independently by a time-stretch/pitch-shift primitive.

Until `stretch` is implemented, independent BPM+pitch requests must not silently pretend to preserve both. They should either:

1. fail validation when `tempo_mode: stretch` is requested but unsupported,
2. run in `beat_locked_rate` with an explicit diagnostic that pitch follows rate,
3. run in `rate` plus an additional pitch-shift primitive only if a supported pitch-shift path exists.

### Effective v1 rate calculation

For pitch-linked tempo matching:

```text
tempo_ratio = target_bpm / source_bpm
pitch_ratio_from_semitones = 2^(pitch_shift_semitones / 12)
effective_rate = manual_rate * tempo_ratio * nudge_ratio
```

In `rate` and `beat_locked_rate` v1 modes, `pitch_shift_semitones` is either rejected, ignored with diagnostics, or applied only if an explicit downstream pitch-shift primitive exists. The raw streaming cursor rate should not claim to preserve pitch independently.

For future `stretch` mode:

```text
tempo_ratio = target_bpm / source_bpm
pitch_ratio = 2^(pitch_shift_semitones / 12)
```

The streaming/creative sampling path will use tempo ratio and pitch ratio as separate DSP controls.

## Candidate Primitive Surfaces

### `stream_source`

Inputs:

- `play` (`event` or control) — starts playback.
- `stop` (`event` or control) — stops playback.
- `cue` (`event`, optional) — moves to configured cue position and optionally stops.
- `seek` (`event` or control) — moves playback position.
- `rate` (`control`) — final playback cursor rate used by v1 pitch-linked rendering.
- `tempo_ratio` (`control`, optional) — target tempo ratio before conversion to cursor rate.
- `pitch_ratio` (`control`, optional future extension) — independent pitch ratio for future stretch/pitch-shift mode.
- `pitch_shift_semitones` (`control`, optional future extension) — independent pitch shift intent.
- `level` (`control`) — linear gain.
- `loop_enable` (`control`, optional).
- `loop_start` / `loop_end` (`control` or prepared cue references, optional).

Outputs:

- stereo audio outputs according to project convention.
- `position_frames` (`control`, optional).
- `position_seconds` (`control`, optional).
- `position_normalized` (`control`, optional).
- `remaining_seconds` (`control`, optional).
- `buffer_fill` (`control`, optional).
- `underrun` (`control`, optional).
- `beat_phase` (`control`, optional when beat grid is known).
- `tempo_bpm` (`control`, optional when known).
- `effective_rate` (`control`, optional).
- `effective_pitch_ratio` (`control`, optional, reports pitch-linked result in v1 modes).
- `pitch_preserved` (`control`, optional boolean/status output).
- `next_beat_frames` (`control`, optional when beat grid is known).
- `next_downbeat_frames` (`control`, optional when beat grid is known).

Static parameters:

- `source` or `stream_asset` — prepared stream-capable source ID.
- `kind` — concrete source kind such as `sample_file`.
- `tempo_mode` — `free`, `rate`, `beat_locked_rate`, or future `stretch`.
- `source_bpm_policy` — `metadata`, `manual`, or `required` where supported.
- `buffer_size_ms` — bounded read-ahead buffer size.
- `underrun_mode` — `silence`, `hold`, or `stop` where supported.
- `metadata_policy` — whether unavailable metadata emits missing values, diagnostics, or configured defaults.

### `sample_metadata`

The same metadata primitive from advanced sampling should be reused or extended for streaming sources. It should not be duplicated as `stream_metadata` unless the general metadata primitive becomes impossible to keep clean.

Additional streaming outputs may include:

- `buffer_start_frames`,
- `buffer_end_frames`,
- `buffer_fill`,
- `analysis_status`,
- `analysis_confidence`,
- `transport_state`,
- `stream_kind` where useful.

### `beat_analyzer`

This may be a preparation/background service rather than a realtime graph module. If exposed as a module, it must represent prepared/offline analysis state, not audio-callback analysis.

Outputs/metadata:

- `tempo_bpm`,
- `beat_grid`,
- `downbeats`,
- `confidence`,
- `provenance` such as `declared`, `detected`, or `missing`.

### `stream_transport`

A separate transport primitive may be useful if multiple streaming modules need to share play/cue/seek state and convert musical tempo intent into cursor rate.

Inputs:

- `play`, `stop`, `cue`, `seek`, `loop_enable`.
- `source_bpm`.
- `target_bpm`.
- `manual_rate`.
- `nudge_ratio`.
- `pitch_shift_semitones`.
- `pitch_ratio`.

Outputs:

- transport state,
- playhead position,
- normalized position,
- `tempo_ratio`,
- `effective_rate`,
- `pitch_ratio`,
- `pitch_preserved`,
- beat phase where a beat grid is attached.

Static parameters:

- `tempo_mode` — `free`, `rate`, `beat_locked_rate`, or future `stretch`.
- `unsupported_pitch_mode` — `error`, `diagnostic_and_rate`, or equivalent explicit fallback policy.

## Realtime Contract

The audio callback may read from prepared ring buffers and update bounded transport state. It must not perform blocking IO, allocate, decode arbitrary packets, wait on locks, run beat detection, run unbounded time-stretch analysis, or log.

Background workers may decode and fill buffers according to a bounded policy. Failure/underrun state must be visible to the engine/plugin off the audio thread. Publishing newly available analysis metadata must use a safe handoff that does not mutate graph structure during rendering.

Pitch-preserved `stretch` mode requires a separate bounded DSP contract before implementation. The streaming API may reserve tempo/pitch controls before that mode is supported.

## Validation Rules

- Stream sources must resolve to supported stream-capable source kinds or fail preparation.
- `kind: sample_file` sources must resolve to supported long-file formats or fail preparation.
- Buffer size and read-ahead policy must be bounded.
- Cue points, loop points, and beat-grid markers must be inside source duration where the source has finite duration.
- Beat detection metadata must include confidence and provenance.
- Runtime transport state must not be persisted as immutable source metadata unless explicitly saved as plugin/session state.
- Missing or low-confidence beat analysis must be represented explicitly.
- `beat_locked_rate` requires source BPM from metadata or manual override.
- `stretch` requires a supported pitch-preserving stretch implementation; otherwise it must fail validation or use an explicitly configured fallback policy.
- Independent `target_bpm` and `pitch_shift` controls must not silently imply pitch preservation in `rate` or `beat_locked_rate` mode.

## Open Questions

- Whether streaming belongs fully in the Rust engine, fully in the plugin/host layer, or a split model.
- Whether tempo sync and beat grids belong here or need a later DJ-deck spec. Current leaning: beat-grid metadata belongs here; full DJ deck sync can be later.
- How much cue/loop metadata should be YAML-authored versus runtime/plugin state.
- Whether `stream_source` should output beat metadata directly or whether all metadata should go through `sample_metadata`.
- Whether future pitch preservation is implemented as `stream_source tempo_mode: stretch`, a separate `time_stretch_stream` primitive, or a creative-sampling primitive inserted after streaming decode.
- Which future source kinds are valid for `stream_source` versus needing their own specialised primitive.
