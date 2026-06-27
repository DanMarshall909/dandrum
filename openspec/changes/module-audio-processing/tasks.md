## 1. Graph Processor Core

- [x] 1.1 Implement `ModuleProcessor` trait and per-module processor structs in `graph_processor.rs`.
- [x] 1.2 Implement `GraphProcessor` with topological sort and per-block module dispatch.
- [x] 1.3 Implement signal bus routing (audio/control/event) between modules per block.

## 2. Module DSP

- [x] 2.1 Implement oscillator DSP (sawtooth generation from gate event pitch).
- [x] 2.2 Implement ADSR envelope DSP (gate-triggered decay envelope).
- [x] 2.3 Implement VCA/gain DSP (audio × control sample multiply).
- [x] 2.4 Implement audio_output DSP (accumulate to render buffers).
- [x] 2.5 Implement midi_input event emission to signal bus.

## 3. Integration and Verification

- [x] 3.1 Wire `GraphProcessor` into `Engine::render_offline` for non-silent output.
- [x] 3.2 Add a test confirming non-silent audio output from a minimal patch.
- [x] 3.3 Add a 303-style monophonic bassline example patch and validate it renders audibly.
