## Context

The Rust headless engine already loads YAML patches, validates modular graphs, registers built-in modules, schedules input events, renders offline audio, and writes WAV files. Patch documents already include `assets`, with `sample` as an asset kind, but no built-in runtime module consumes those sample assets as a reusable signal source.

The sampler should remain in the engine core and must not depend on JUCE, the console wrapper, GUI, plugin code, or realtime device I/O. It should work through the same patch, graph, event, and render paths used by the existing oscillator/VCA examples. It should not own velocity gain, piano grouping, or generic polyphonic voice routing; those are graph-level concerns handled by other modules and future generic bus support.

## Goals / Non-Goals

**Goals:**

- Add a built-in `sampler` module that behaves as a pure audio source with routed event/control inputs and audio output.
- Validate that each sampler references a declared sample asset and that the asset can be loaded before rendering.
- Play a loaded sample monophonically when routed events trigger it, using routed control signals to control playback rate/pitch.
- Expose sampler controls for playback start and loop behavior as ports/parameters so the sample source composes with the graph like any other signal source.
- Produce deterministic offline render output for the same patch, assets, settings, and events.
- Add a minimal sampler patch example and tests covering validation and rendering.

**Non-Goals:**

- Velocity scaling, amplitude envelopes, piano key grouping, and trigger selection inside the sampler; these belong in routing/control modules around the sampler.
- Generic bus infrastructure and polyphonic voice allocation. The initial sampler is monophonic, but its control contract must not block a future per-voice bus design shared by other modules.
- Time-stretching, multi-sample key zones, choke groups, streaming, or high-quality interpolation beyond direct sample playback.
- Adding new GUI, plugin, or JUCE device behavior.
- Supporting every audio file format. The initial implementation can support the engine's WAV path only.

## Decisions

- Use a built-in module type named `sampler`.
  - Rationale: This matches the existing module registry pattern and keeps patches concise.
  - Alternative considered: Treat sampling as a script module. That would blur asset loading and bounded DSP behavior that should be engine-owned.

- Define sampler ports as `trigger` event input, control inputs for playback rate/position/loop behavior, and `audio` audio output.
  - Rationale: The sampler is a pure signal generator. Triggering and playback-rate control are generic typed inputs, while MIDI note, velocity, and grouping semantics stay in upstream conversion/routing modules.
  - Alternative considered: Use `gate` plus MIDI note payload inspection. That would repeat the oscillator's current MIDI-coupling mistake and make the sampler less modular.

- Keep velocity scaling out of the sampler.
  - Rationale: Velocity is amplitude/control information and should be patchable through VCA, envelope, or future bus-controlled modules rather than hidden inside sample playback.
  - Alternative considered: Scale sample amplitude directly from `NoteOn` velocity. That is convenient for simple drums but makes the sampler less modular.

- Use a routed `rate` control input to determine playback speed/pitch.
  - Rationale: Note-to-pitch mapping is a reusable conversion concern, not sampler behavior. This keeps samplers and oscillators in the same pure signal-generator category.
  - Alternative considered: Let sampler inspect MIDI note payloads directly. That would hide policy inside the sampler and prevent generic MIDI-to-control modules from composing with other generators.

- Implement monophonic playback first while preserving a future generic bus design for polyphony.
  - Rationale: The current engine has monophonic processing paths; a generic per-voice bus should apply to samplers and future modules rather than be sampler-specific.
  - Alternative considered: Add sampler-only polyphony now. That would create special voice-routing semantics that other modules could not reuse.

- Reference the sample asset through a module parameter named `asset` containing the asset ID.
  - Rationale: Patch assets are already top-level declarations; module parameters are the current extension point for per-module configuration.
  - Alternative considered: Add a first-class `asset_id` field to module declarations. That would require broader patch schema changes for one module type.

- Load sample data during graph/render preparation, not inside per-frame processing.
  - Rationale: File I/O and decode errors should happen before DSP, preserving deterministic processing and clearer diagnostics.
  - Alternative considered: Lazy-load on first trigger. That could hide configuration failures until render time and would be unsuitable for realtime use.

- Support PCM WAV sample assets initially.
  - Rationale: The project already owns WAV output code and tests can generate small deterministic WAV fixtures without adding a decoder dependency.
  - Alternative considered: Add a general audio decoding crate immediately. That increases dependency surface before sampler behavior is proven.

## Risks / Trade-offs

- Unsupported sample formats can frustrate users expecting arbitrary audio files -> Return explicit diagnostics that identify the asset ID, path, and supported format.
- Sample-rate mismatches can make playback speed unexpected -> Start with same-rate playback and diagnose unsupported mismatches, then add resampling later if needed.
- Large samples loaded fully into memory can be expensive -> Accept for initial one-shot sampler; document streaming as out of scope.
- Current graph processor has module-type-specific match logic -> Keep the sampler change minimal there, but tests should cover both offline and realtime processor construction paths where applicable.
- Monophonic playback can feel limiting -> Document that generic per-voice buses are the intended path to polyphony and avoid sampler-specific polyphonic shortcuts.
