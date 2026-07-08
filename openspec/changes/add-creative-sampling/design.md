## Context

Creative sampling is a separate future track from primitive prepared sampling. It may use the same asset preparation and sample buffers, but its render-time DSP is different enough to require a separate contract.

## Candidate Primitive Families

- `time_stretch_player` — plays a sample region at a target duration or tempo ratio.
- `pitch_shift_player` — changes pitch without simple playback-rate coupling where implemented.
- `granular_player` — emits grains from a prepared sample region with bounded grain count.
- `sample_scrubber` — exposes playback position as a controllable signal.
- `sample_freeze` — holds a window/buffer for drone or texture behaviour.

Names are placeholders. The implementation should add the narrowest primitives needed for externally useful behaviour.

## Design Principle

Do not hide these behaviours inside `sample_player`. Base `sample_player` should remain a predictable region playback primitive. Creative sampling should compose with envelopes, filters, modulation, and effects like everything else.

## Non-Goals For Now

- No implementation in the advanced sampling v1 work.
- No arbitrary proprietary algorithm clone.
- No unbounded grain allocation or dynamic file IO in the audio callback.
