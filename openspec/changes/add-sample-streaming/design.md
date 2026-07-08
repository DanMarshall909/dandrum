## Context

Sample streaming is for DJ-style and long-form playback: full tracks, long loops, stems, backing audio, and deck-like workflows. It should not be implemented as a flag on `sample_player` because it needs a different contract: background IO, decode buffering, seek/cue state, underrun policy, and session restore behaviour.

## Goals / Non-Goals

**Goals:**

- Stream long sample files without decoding the whole file into memory.
- Keep audio callback behaviour bounded and allocation-free.
- Support transport-like controls suitable for DJing.
- Preserve deterministic state transitions for play/stop/cue/seek events.
- Make underrun behaviour explicit and testable.

**Non-Goals:**

- Replace preloaded `sample_player` for drum hits and short regions.
- Implement full DJ software in the first slice.
- Perform blocking file IO, decoding, allocation, or filesystem discovery in the audio callback.

## Candidate Primitive Surfaces

### `sample_stream_source`

Inputs:

- `play` (`event` or control) — starts playback.
- `stop` (`event` or control) — stops playback.
- `seek` (`event` or control) — moves playback position.
- `rate` (`control`) — playback-rate ratio.
- `level` (`control`) — linear gain.

Outputs:

- stereo audio outputs according to project convention.
- optional `position` control output.
- optional `underrun` status output.

Static parameters:

- `stream_asset` — prepared stream asset ID.
- `buffer_size_ms` — bounded read-ahead buffer size.
- `underrun_mode` — `silence`, `hold`, or `stop` where supported.

### `stream_transport`

A separate transport primitive may be useful if multiple streaming modules need to share play/cue/seek state.

## Realtime Contract

The audio callback may read from prepared ring buffers and update bounded transport state. It must not perform blocking IO, allocate, decode arbitrary packets, wait on locks, or log.

Background workers may decode and fill buffers according to a bounded policy. Failure/underrun state must be visible to the engine/plugin off the audio thread.

## Open Questions

- Whether streaming belongs in the plugin/host layer, Rust engine, or a split model.
- Whether tempo-sync and beat grids belong in this spec or a later DJ-deck spec.
- How much cue/loop metadata should be YAML-authored versus runtime state.
