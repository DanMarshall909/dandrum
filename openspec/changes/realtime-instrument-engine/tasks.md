## 1. Callback Safety Tests

- [x] 1.1 Add a Rust test specifying bounded realtime MIDI/event submission and dropped-event status.
- [x] 1.2 Add a Rust test proving queued note events render the same audio behavior as direct note calls.
- [x] 1.3 Add a C++ or CI-ready test that rejects `CriticalSection` locking in the JUCE audio callback path.
- [x] 1.4 Add a C++ or CI-ready test that rejects console IO in the MIDI callback path.

## 2. Realtime Event Handoff

- [x] 2.1 Define a small realtime event type and bounded queue in the Rust engine layer.
- [x] 2.2 Add non-blocking event submission APIs that return accepted or dropped status.
- [x] 2.3 Drain queued events at the start of realtime audio rendering without blocking.
- [x] 2.4 Expose realtime event submission status through the C FFI.

## 3. JUCE Callback Integration

- [x] 3.1 Update `RustEngineSource` so `getNextAudioBlock` does not acquire the shared engine lock.
- [x] 3.2 Update MIDI note handling to enqueue events instead of mutating the engine under the shared lock.
- [x] 3.3 Remove callback-time console logging from MIDI input handling.
- [x] 3.4 Keep patch loading, sample preparation, and engine replacement outside the audio callback.

## 4. Prepared Realtime Scratch State

- [ ] 4.1 Add a prepared maximum block size to the realtime graph processor setup path.
- [ ] 4.2 Replace steady-state realtime render `Vec` and `HashMap` allocation with reusable scratch storage for module outputs and event routing.
- [ ] 4.3 Define and test oversized callback block handling by splitting or explicit fallback.
- [ ] 4.4 Add deterministic realtime render tests across repeated engines with the same queued events and block sequence.

## 5. Verification and Documentation

- [ ] 5.1 Document the realtime callback contract in the relevant README or engine notes.
- [ ] 5.2 Run Rust unit tests for the engine crate.
- [ ] 5.3 Run CMake/CTest for the JUCE wrapper and FFI smoke coverage.
- [ ] 5.4 Run `openspec validate realtime-instrument-engine --strict`.
