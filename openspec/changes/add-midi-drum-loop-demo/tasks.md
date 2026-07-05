## 1. Integration Characterization And Test Coverage

- [x] 1.1 Add or identify test coverage proving the drum container patch or preset loads, validates, and renders non-empty audio from kick, snare, and hat MIDI note events.
- [x] 1.2 Add failing integration coverage for the built-in drum loop event schedule proving it loads the selected drum container asset, prepares the engine, submits MIDI events, and produces non-empty rendered output.

## 2. Drum Loop Scheduling

- [x] 2.1 Define the fixed four-beat drum loop pattern with kick, snare, and hat note events using notes supported by the drum container patch or preset.
- [x] 2.2 Keep the loop scheduler reusable by the demo command and tests without depending on audio-device initialization.

## 3. User-Facing Integration Demo Command

- [x] 3.1 Add a `dandrum-beep` integration demo option that loads the drum container patch or preset and starts the MIDI drum loop.
- [x] 3.2 Route scheduled drum loop events through the existing note on/off engine path rather than adding drum-specific render shortcuts.
- [x] 3.3 Ensure the demo path exercises the existing audio-device setup, `RustEngineSource`, MIDI event queue, and render callback path where local playback is available.
- [x] 3.4 Allow the running loop to stop cleanly on user interrupt.

## 4. Verification

- [x] 4.1 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml` and fix regressions.
- [x] 4.2 Configure/build with `$HOME/.local/bin/cmake -S . -B build` and `$HOME/.local/bin/cmake --build build` if the local environment supports it.
- [x] 4.3 Run `ctest --test-dir build` if the CMake build is available.
- [x] 4.4 Manually run the drum loop demo long enough to confirm the patch loads and audible playback starts, or document any local audio-device limitation.
- [x] 4.5 Run `openspec validate add-midi-drum-loop-demo --strict` and fix validation errors.
