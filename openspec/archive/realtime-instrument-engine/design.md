## Context

The Rust engine now owns patch loading, graph validation, graph processing, and offline rendering. The JUCE wrapper can call Rust from an audio callback, but `RustEngineSource` currently protects all engine operations with a `juce::CriticalSection`; MIDI callbacks call into the same locked engine and write note messages to `std::cout`.

The Rust `RealtimeGraphProcessor` is a useful starting point, but it still accepts direct mutable note calls and uses per-render-block dynamic containers such as `Vec` and `HashMap` for event and module outputs. That is acceptable for tests and offline-style rendering, but not enough for an instrument path where callback latency and worst-case behavior matter.

## Goals / Non-Goals

**Goals:**
- Make the JUCE audio callback render from already-prepared engine state without waiting on the MIDI/control callback path.
- Add bounded non-blocking event submission for MIDI and future control events.
- Separate patch loading/preparation from realtime rendering so filesystem, YAML parsing, graph construction, and asset preparation stay off the audio thread.
- Refactor realtime graph rendering toward prepared scratch buffers and explicit max-block handling.
- Add Rust and C++ tests that specify callback-safe behavior before implementation.

**Non-Goals:**
- Shipping a VST, AU, CLAP, or standalone GUI instrument in this change.
- Solving all DSP algorithm realtime-safety issues for every future effect module.
- Replacing JUCE device management or the existing console executable architecture.
- Implementing hard realtime guarantees beyond normal native audio callback discipline.

## Decisions

### Use a bounded SPSC event queue between MIDI/control callbacks and audio rendering

MIDI input is producer-side callback work, and audio rendering is consumer-side callback work. A bounded single-producer/single-consumer queue matches the current JUCE MIDI-to-engine path and gives explicit overflow behavior.

Alternatives considered:
- Keep `CriticalSection`: simple but can block the audio callback behind patch loading, MIDI handling, or destruction.
- Unbounded queue: easier API but can allocate or grow under callback load.
- Direct Rust engine mutation from MIDI callback: preserves current semantics but keeps cross-callback shared mutable state.

### Introduce an explicit realtime engine state split

Patch load and preparation should build an inactive render state off the audio thread. The audio callback should hold or swap to a prepared state and then only drain events and render blocks. Destruction and patch replacement must not free memory currently used by the audio callback.

Alternatives considered:
- Reuse one mutable `DandrumEngine` behind a lock: already exists, but is the core realtime problem.
- Rebuild graph state in the callback on patch changes: violates the no-filesystem/no-allocation callback requirements.

### Keep FFI small and status-oriented

The C boundary should expose event submission status and preparation/render functions with simple POD types. The first implementation can preserve existing functions while adding realtime-specific APIs, then migrate JUCE call sites.

Alternatives considered:
- C++ owns the queue and Rust owns only rendering: workable, but splits event semantics between languages.
- Rust owns all callback integration: would require broader JUCE binding changes and reduce clarity at the C boundary.

### Refactor realtime scratch storage incrementally

First remove callback locks and direct event mutation; then refactor `RealtimeGraphProcessor` internals to reuse prepared buffers. That keeps behavior covered while avoiding a large rewrite of graph processing and effects dispatch.

Alternatives considered:
- Rewrite the whole graph processor around fixed arrays immediately: cleaner final architecture but high risk while reverb/echo and compiled-patch work is active.
- Accept internal block allocations for now: playable experiments may work, but the spec would still leave a known callback hazard.

## Risks / Trade-offs

- Event queue overflow can drop notes under extreme input bursts -> expose dropped-event status and counters so tests and UI can report it.
- Patch replacement can race with rendering -> use a prepared-state handoff where old state remains alive until no callback can access it.
- Refactoring scratch buffers can conflict with active effects work -> keep the first implementation narrow and stage buffer reuse after the event-handoff API is covered.
- Some existing DSP modules allocate internally during construction or parameter changes -> keep construction off the callback and add targeted follow-up tasks for modules that still allocate while processing.

## Migration Plan

1. Add tests for the new realtime event submission API and callback constraints.
2. Implement bounded event handoff in Rust and expose status through FFI.
3. Update JUCE MIDI and audio callback code to use event handoff and avoid console IO/engine locks.
4. Refactor realtime graph rendering to pre-size and reuse scratch buffers for the prepared maximum block size.
5. Keep existing FFI functions as compatibility wrappers until the JUCE path no longer depends on locked direct mutation.
