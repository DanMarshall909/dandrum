## Why

The headless engine has graph routing, validation, and scheduler infrastructure but produces silence. Adding minimal DSP processing to the built-in modules turns the routing graph into an actual instrument that can generate audio.

## What Changes

- Add a `ModuleProcessor` trait and DSP implementations for oscillator, ADSR, VCA/gain, and audio_output modules.
- Add a `GraphProcessor` that topologically sorts the module graph and dispatches per-block processing with signal routing between modules.
- Wire `GraphProcessor` into `Engine::render_offline` so that YAML patches produce audible WAV output.
- Replace the existing fixed-architecture `synth.rs` playback in the JUCE side with graph-based processing (deferred to a follow-up).
- Remove the `audio_delay_one_sample` and `block_delay` DSP from scope; delay modules keep their definition and validation metadata but no audio processing yet.

## Capabilities

### New Capabilities
- `graph-processor`: Topological sort of modules respecting delay-boundary cycle breaks, per-block module dispatch, and audio/control/event bus routing.
- `module-dsp`: DSP implementations for oscillator (sawtooth/square), ADSR envelope, VCA/gain, and audio_output modules integrated into the graph processor.

### Modified Capabilities
- `headless-engine`: `Engine::render_offline` produces audio instead of silence.
- `built-in-modules`: Module definitions are extended with processor implementations.

## Impact

- Adds `src/rust-engine/src/graph_processor.rs` for processing logic.
- Adds DSP logic to module types; each processor is a small struct with per-instance state (phase, envelope level, etc).
- `Engine::render_offline` signature is unchanged; callers get real audio.
- Existing validation and scheduling tests continue to pass; new DSP unit tests and an end-to-end audible render test are added.
