## Context

The engine currently routes, validates, and schedules patches but fills output buffers with silence. Module definitions exist as port/shape metadata only. The existing `synth.rs` is a fixed-architecture additive voice engine not connected to the graph.

This design adds a graph-level processor that dispatches per-module DSP in topological order, reads/writes to per-block signal buses, and accumulates final output.

## Goals / Non-Goals

**Goals:**
- Process any validated graph into audio per-block, respecting topological order and delay-boundary cycle breaks.
- Implement minimal DSP for: oscillator (sawtooth), ADSR (simple decay envelope), VCA/gain, audio_output.
- Route signals: audio as `Vec<f32>`, control as `f32`, events as `Vec<ScriptEvent>`.
- Wire into `Engine::render_offline` so YAML patches produce audible WAV output.
- Keep module processor state per-instance (phase, envelope level, delay buffer).
- Add a test that confirms non-silent audio output from a simple patch.

**Non-Goals:**
- No filter DSP (defer to later; the 303's resonant filter is its character but saw+VCA+envelope is enough to prove the system works).
- No delay/echo DSP (defer; modules keep their definitions and validation metadata).
- No LFO DSP (defer; not needed for a basic monophonic bassline).
- No JUCE integration change (the fixed `synth.rs` voice engine stays for JUCE playback; graph processing is offline-only for now).
- No polyphony (monophonic only for MVP).

## Decisions

### Decision: Graph processor as a new module, not part of core.rs

`graph_processor.rs` contains `GraphProcessor`, `ModuleProcessor` trait, and per-module processor implementations. This keeps DSP logic separated from routing/validation infrastructure.

Alternatives considered:
- Inline in `core.rs`: fewer files but bloats the core module with DSP detail.
- Per-module files (`osc.rs`, `adsr.rs`, etc.): premature modularity for an MVP with 4 module types.

### Decision: Signal bus uses send buffers per port

For each block, the processor writes each module's output into per-port send buffers: `HashMap<String, Vec<f32>>` for audio, `HashMap<String, f32>` for control, `HashMap<String, Vec<ScriptEvent>>` for events. Downstream modules read from the bus by port reference. This is simple and correct for acyclic graphs.

Alternatives considered:
- Directly connecting module outputs to inputs via the cable list: more efficient but couples dispatch order to allocation order.
- Pull-based: each module reads what it needs from upstream. Cleaner but requires recursion or pre-computed dependency order.

### Decision: Topological sort on every render

Compute processing order by DFS topological sort of the module graph at processor creation time. This is O(V+E) and trivial for the MVP. Delay-boundary modules are treated as graph cut points: the sort stops at them and starts a new topological region.

Alternatives considered:
- Cache the sort: unnecessary optimization for MVP.
- Event-driven dispatch: over-engineering for a block-based offline renderer.

### Decision: Oscillator pitch from gate events, not control port

The oscillator receives gate events but pitch travels through the event itself (MIDI note number). When the oscillator receives a `NoteOn` gate event, it extracts the note number and converts to Hz. The `pitch` control port is ignored for MVP.

Alternatives considered:
- Requiring an explicit pitch control connection: more architecturally pure but breaks all existing example patches and adds complexity for no musical benefit at MVP stage.

### Decision: ADSR simplifies to envelope following gate

The ADSR processor watches for gate events. `NoteOn` triggers attack/decay/sustain phases; `NoteOff` triggers release. Attack/decay/release times and sustain level are fixed constants for MVP (can be parameterized later).

### Decision: 303-style character deferred beyond MVP

A proper 303 emulation needs: resonant low-pass filter with accent, slide/portamento, distortion. None of these are in the MVP scope. The MVP proves the graph processor works with a sawtooth oscillator + simple envelope + VCA.

## Risks / Trade-offs

- Fixed ADSR parameters (no modulation yet) means patches can't shape envelopes per-voice. Acceptable for MVP; parameterization is a natural follow-up.
- Ignoring the pitch control port breaks the contract that *all* modulatable parameters go through named ports. This is a deliberate MVP shortcut that should be resolved by adding a `note_to_pitch` converter module or making the oscillator prefer the pitch control port when connected.
- Graph processor is O(V*block_size) per module; fine for MVP but may need optimization for dense graphs at realtime block sizes.
- No delay DSP means the tune-with-delay example still won't produce echo until the effects task.
