## Context

Sample streaming is for DJ-style and long-form playback: full tracks, long loops, stems, backing audio, and deck-like workflows. It should not be a separate sampler family because the rest of Dandrum also needs sample source metadata, regions, slices, cue points, and beat grids.

Streaming should therefore extend the unified sample source model:

```text
sample_source
   -> source metadata / analysis metadata
   -> playback or streaming primitive
   -> transport / selection / slicing / effects
```

A preloaded source and a streaming source have different memory and IO contracts, but they should expose compatible metadata wherever possible.

## Goals / Non-Goals

**Goals:**

- Stream long sample files without decoding the whole file into memory.
- Keep audio callback behaviour bounded and allocation-free.
- Support transport-like controls suitable for DJing.
- Preserve deterministic state transitions for play/stop/cue/seek events.
- Make underrun behaviour explicit and testable.
- Expose useful source and transport metadata to the graph/plugin.
- Support explicit beat-grid metadata and future preparation-time beat detection.
- Keep streaming aligned with `sample_source`, `sample_metadata`, regions, slices, and cue points from the advanced sampling model.

**Non-Goals:**

- Replace preloaded `sample_player` for drum hits and short regions.
- Implement full DJ software in the first slice.
- Perform blocking file IO, decoding, allocation, beat detection, or filesystem discovery in the audio callback.
- Introduce a second, incompatible sample metadata model.

## Decisions

### Streaming is a source implementation, not a new sampler type

`sample_stream_source` should represent a file-backed sample source with bounded buffering. It should expose the same general metadata shape as preloaded sources plus streaming-specific state such as buffered range, current decode position, underrun status, and background worker state.

A future `sample_player` may consume source windows from preloaded sources, while `sample_stream_source` renders continuous transport playback. They share source identity, metadata, cue points, beat-grid data, and analysis state.

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

## Candidate Primitive Surfaces

### `sample_stream_source`

Inputs:

- `play` (`event` or control) — starts playback.
- `stop` (`event` or control) — stops playback.
- `cue` (`event`, optional) — moves to configured cue position and optionally stops.
- `seek` (`event` or control) — moves playback position.
- `rate` (`control`) — playback-rate ratio.
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
- `next_beat_frames` (`control`, optional when beat grid is known).
- `next_downbeat_frames` (`control`, optional when beat grid is known).

Static parameters:

- `source` or `stream_asset` — prepared stream-capable sample source ID.
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
- `transport_state`.

### `beat_analyzer`

This may be a preparation/background service rather than a realtime graph module. If exposed as a module, it must represent prepared/offline analysis state, not audio-callback analysis.

Outputs/metadata:

- `tempo_bpm`,
- `beat_grid`,
- `downbeats`,
- `confidence`,
- `provenance` such as `declared`, `detected`, or `missing`.

### `stream_transport`

A separate transport primitive may be useful if multiple streaming modules need to share play/cue/seek state.

Inputs:

- `play`, `stop`, `cue`, `seek`, `rate`, `loop_enable`.

Outputs:

- transport state,
- playhead position,
- normalized position,
- beat phase where a beat grid is attached.

## Realtime Contract

The audio callback may read from prepared ring buffers and update bounded transport state. It must not perform blocking IO, allocate, decode arbitrary packets, wait on locks, run beat detection, or log.

Background workers may decode and fill buffers according to a bounded policy. Failure/underrun state must be visible to the engine/plugin off the audio thread. Publishing newly available analysis metadata must use a safe handoff that does not mutate graph structure during rendering.

## Validation Rules

- Streaming sources must resolve to supported long-file formats or fail preparation.
- Buffer size and read-ahead policy must be bounded.
- Cue points, loop points, and beat-grid markers must be inside source duration.
- Beat detection metadata must include confidence and provenance.
- Runtime transport state must not be persisted as immutable source metadata unless explicitly saved as plugin/session state.
- Missing or low-confidence beat analysis must be represented explicitly.

## Open Questions

- Whether streaming belongs fully in the Rust engine, fully in the plugin/host layer, or a split model.
- Whether tempo sync and beat grids belong here or need a later DJ-deck spec. Current leaning: beat-grid metadata belongs here; full DJ deck sync can be later.
- How much cue/loop metadata should be YAML-authored versus runtime/plugin state.
- Whether `sample_stream_source` should output beat metadata directly or whether all metadata should go through `sample_metadata`.
