## Why

DJ-style playback of long files has different requirements from preloaded one-shot sampling. Long tracks, stems, backing loops, and DJ decks should not require decoding entire files into memory, but they also cannot do arbitrary disk IO directly from the audio callback.

This change is a separate future capability for realtime-safe streaming from sample files for DJing and long-form playback use cases. It should extend the unified sample source model from advanced sampling rather than creating a competing sampler.

A streaming source is most useful when it returns metadata as well as audio: duration, position, sample rate, channel count, decoded/loaded range, detected tempo, beat grid, cue points, analysis confidence, buffering state, and underrun status.

The primitive should be named generically as `stream_source`, with sample-file streaming as the first source kind. That leaves room for future stream sources such as live inputs, network streams, generated streams, stem streams, or inter-plugin streams without adding parallel primitive families.

## What Changes

- Add a future `sample-streaming` capability for long-file playback using the same sample source/metadata concepts as preloaded sample playback, plus bounded read-ahead buffers.
- Treat preloaded and streaming samples as different `sample_source` implementations with different preparation/buffering contracts, not different sampler families.
- Define streaming primitives around source, metadata, transport, and buffered playback:
  - `stream_source` for bounded realtime streaming from a prepared source, with `kind: sample_file` as the first concrete stream kind.
  - `sample_metadata` reuse/extension for duration, position, tempo, beat grid, cue points, channel layout, buffer state, and analysis status.
  - `stream_transport` or equivalent play/cue/seek control behaviour.
  - optional `stream_loop`/cue-point primitives where justified.
  - preparation-time `beat_analyzer` or equivalent analysis path if automatic beat detection is implemented.
- Support DJ-oriented controls later: play/stop, cue, seek, scrub/cue preview, tempo ratio, pitch bend, loop in/out, gain, pan, and multiple deck outputs.
- Support metadata useful to patches and plugin UI: current position, remaining time, loaded range, underrun state, detected BPM, beat phase, next beat/downbeat, cue markers, and confidence.

## Capabilities

### New Capabilities

- `sample-streaming`: Future capability for DJ-style long-file sample streaming with bounded buffering, unified sample metadata, beat/tempo analysis, and realtime-safe transport controls.

### Modified Capabilities

- `advanced-sampling-options`: Provides the shared sample source, region, metadata, slice, and beat-grid concepts that streaming extends.
- `built-in-modules`: May later include a generic `stream_source` primitive with sample-file streaming as the first supported kind.
- `plugin-integration`: May later need long-file state persistence, deck status display, waveform/metadata display, and safe background preparation.
- `yaml-patch-format`: May later support stream asset declarations, cue points, deck routing, buffer policy, stream kind, and beat-grid metadata.

## Impact

- Requires a separate buffering and background IO contract.
- Requires metadata outputs/status for streaming source state, analysis state, and transport state.
- Requires beat-detection/beat-grid design if automatic analysis is included.
- Requires tests around metadata accuracy, beat-grid preparation, underrun behaviour, seek/cue determinism, block-splitting, transport state, and callback allocation safety.
- Non-goal: do not implement streaming as a separate sampler family or a hidden purpose-specific deck that cannot share metadata/playback concepts with the rest of Dandrum.
