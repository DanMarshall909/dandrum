## 1. Voice retrigger reset

- [ ] 1.1 Identify which voice-scoped modules carry resettable state (start with `filter`
      and `envelope_follower`; audit the rest of the `ExecutionScope::Voice` modules) and
      confirm each exposes a `reset()` that fully clears that state.
- [ ] 1.2 Add a reset entry point on `PerModuleState` that dispatches `reset()` to the
      stateful variants and is a no-op for stateless ones.
- [ ] 1.3 Call the per-voice reset when a voice is allocated for a note-on (voice-allocation
      path in `realtime_graph_processor`), before the note renders.
- [ ] 1.4 Behaviour test: a retriggered high-resonance filter voice starts without the
      previous note's decaying tail; confirm global effect tails are untouched by note
      activity.

## 2. Engine reset / panic

- [ ] 2.1 Add a reset entry point that dispatches `reset()` to global (non-voice-scoped)
      effect state as well (`reverb`, `echo`, `dynamics`, `convolution`, `spectral`,
      `saturator`, `frequency_splitter`).
- [ ] 2.2 Add an engine-level `reset()` on the realtime graph processor / facade that stops
      all active voices and cascades the reset to every module's state.
- [ ] 2.3 Behaviour test: impulse-excited reverb renders silence after engine reset; active
      voices become inactive after reset.

## 3. FFI + cleanup

- [ ] 3.1 Export an additive FFI symbol for host-driven reset/panic; confirm existing
      `dandrum_*` symbols are unchanged. Smoke-test it.
- [ ] 3.2 Remove the `#[allow(dead_code)]` allowances on `reset()` methods now that they are
      reachable; confirm the crate builds under `#![deny(dead_code)]`.
- [ ] 3.3 Update docs to describe voice retrigger reset and the engine reset/panic entry point.
