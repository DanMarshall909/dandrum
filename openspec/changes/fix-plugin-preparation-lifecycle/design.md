## Context

`DandrumAudioProcessor` loads the default instrument in its constructor. `DandrumEngine::prepare_realtime` currently updates only engine fields and the fallback synth; a loaded graph retains its original sample rate and capacity. Replacement engines instead prepare before loading, exposing the order dependency. The native probe found zero difference at 44.1 kHz but RMS differences of 0.209627 at 48 kHz and 0.175614 at 96 kHz.

## Goals / Non-Goals

**Goals:** Make preparation order independent and repeatable, preserving parameter values and slot identities. Cover both Rust loading entry points and the JUCE lifecycle. Keep runtime rebuilding outside rendering.

**Non-Goals:** DSP changes, continuity of sounding notes across host preparation, public surface changes, asset discovery, YAML parsing changes, shared host/UI extraction, publication.

## Decisions

- Reuse validated in-memory graph and sampler assets to build fresh runtime state at preparation. Do not reread files or add per-Primitive sample-rate setters; rebuilding consistently resets all rate-dependent state, buffers and queues.
- Clone the existing compiled parameter surface when rebuilding runtime state. This preserves values and slot indices without recompilation or a second value store. Include non-default parameter values in audible assertions.
- Apply current host settings to runtime construction so the legacy instrument graph's buffers and event queues use coherent settings. Kernel graph preparation is a separate API and is outside this plugin lifecycle change.
- Preparation starts a fresh processing session even when settings repeat. Voices, phase, delay state and queued notes reset; the fallback synth already follows this convention.
- Use deterministic Rust audio tests with an independent frequency/sample oracle and event bursts to expose stale rate/capacity. Use native plugin audio/state tests to prove cross-boundary compatibility; keep existing fixture renders unchanged.

## Risks / Trade-offs

- Retaining preparation inputs costs memory → keep graph, sampler assets and voice-allocation settings inside the runtime owner. Compiled resource handles share existing reference-counted storage; legacy sampler assets currently require an extra clone. Avoid a separate parameter model.
- Rebuilding resets sounding notes → explicitly specify preparation as a fresh session, consistent with host setup outside processing.
- Runtime reconstruction could lose values → assert old handles remain valid and audible non-default values survive repeated preparation.
- Larger block capacity affects event storage → exercise more events than the previous capacity and verify late events audibly; retain realtime allocation tests.

## Migration Plan

No API or state format migration. Commit locally after tests and independent review; do not push or merge. Rollback is a source revert.

## Open Questions

None blocking implementation; exact retention location will follow the smallest cohesive runtime boundary found in tests.
