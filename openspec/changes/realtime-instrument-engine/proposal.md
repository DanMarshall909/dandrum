## Why

The engine can render offline and JUCE can call into Rust, but the current instrument path still takes locks and performs non-audio work from callbacks. To use this as an actual playable instrument, the realtime path must have a prepared render state, bounded event handoff, and callback behavior that avoids blocking and unbounded allocation.

## What Changes

- Add a realtime instrument runtime boundary that can be prepared off the audio thread, receive MIDI/events through bounded non-blocking APIs, and render audio blocks from prebuilt state.
- Replace the JUCE callback shared-engine lock path with a non-blocking handoff model for MIDI and control events.
- Define realtime callback constraints for no filesystem access, no patch parsing, no logging, no mutex waits, and no unbounded allocation during audio rendering.
- Add tests and smoke coverage proving event delivery, dropped-event diagnostics, render determinism, and no callback-time blocking primitives in the JUCE audio/MIDI callback path.

## Capabilities

### New Capabilities
- `realtime-instrument-runtime`: Prepared realtime render state, bounded event queues, callback-safe render behavior, and JUCE integration expectations for playable instrument use.

### Modified Capabilities
- `headless-engine`: Clarify that the block processing model is shared by offline and realtime rendering entry points.

## Impact

- Rust engine core and FFI: `src/rust-engine/src/synth.rs`, `src/rust-engine/src/graph_processor.rs`, `src/rust-engine/src/lib.rs`.
- JUCE wrapper: `src/juce-wrapper/RustEngineSource.*`, `src/juce-wrapper/MidiToRustEngine.*`, and C++ FFI smoke coverage.
- Tests: Rust unit tests around realtime event queues/rendering and C++/CTest checks for callback-safe API usage.
