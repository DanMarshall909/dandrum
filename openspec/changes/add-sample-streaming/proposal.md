## Why

DJ-style playback of long files has different requirements from prepared one-shot sampling. Long tracks, stems, backing loops, and DJ decks should not require decoding entire files into memory, but they also cannot do arbitrary disk IO directly from the audio callback.

This change is a separate future capability for realtime-safe streaming from sample files for DJing and long-form playback use cases.

## What Changes

- Add a future `sample-streaming` capability for long-file playback using prepared metadata and bounded read-ahead buffers.
- Keep streaming separate from `sample_player`, which remains a preloaded region playback primitive.
- Define primitives around streaming sources/decks rather than workstation sampling:
  - `sample_stream_source` for buffered file-backed audio playback.
  - `stream_transport` or equivalent play/cue/seek control behaviour.
  - optional `stream_loop`/cue-point primitives where justified.
- Support DJ-oriented controls later: play/stop, cue, seek, scrub/cue preview, tempo ratio, pitch bend, loop in/out, gain, pan, and multiple deck outputs.

## Capabilities

### New Capabilities

- `sample-streaming`: Future capability for DJ-style long-file sample streaming with bounded buffering, prepared metadata, and realtime-safe transport controls.

### Modified Capabilities

- `advanced-sampling-options`: Remains focused on preloaded sample regions, maps, explicit slices, and choke groups.
- `plugin-integration`: May later need long-file state persistence, deck status display, and safe background preparation.
- `yaml-patch-format`: May later support stream asset declarations, cue points, deck routing, and buffer policy.

## Impact

- Requires a separate buffering and background IO contract.
- Requires tests around underrun behaviour, seek/cue determinism, block-splitting, transport state, and callback allocation safety.
- Non-goal: do not implement streaming as a hidden mode of the base `sample_player` primitive.
