## Why

The headless engine can validate and render modular patches, but it does not yet provide a concrete sample source for playing user-provided audio assets from routed events and controls. Adding a sampler makes the engine useful for drum and one-shot playback while exercising asset loading, event scheduling, pitch control, and deterministic offline rendering.

## What Changes

- Add a built-in sampler module that loads named sample assets from YAML patch declarations.
- Allow routed event input to trigger monophonic sample playback, with MIDI note controlling pitch and amplitude/velocity handled by downstream modules.
- Treat the sampler as an audio source with explicit controls for playback behavior such as start position and loop bounds.
- Validate sampler configuration, missing assets, unsupported sample formats, and invalid sampler port connections with clear diagnostics.
- Render sampler output deterministically in offline renders.
- Add examples and tests for a minimal event-to-sampler-to-output patch.

## Capabilities

### New Capabilities

- `sampler-module`: Built-in sample-source behavior, asset validation, event-triggered monophonic playback, MIDI-note pitch control, playback controls, and deterministic audio output.

### Modified Capabilities

None.

## Impact

- Rust engine core module registry, patch validation, asset loading, graph processing, and offline renderer.
- YAML patch examples and diagnostics documentation.
- Unit and acceptance tests for sampler validation and rendering.
