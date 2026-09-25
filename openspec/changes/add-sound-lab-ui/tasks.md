## 1. Behaviour Contracts

- [x] 1.1 Add failing Rust tests for a sound-fixture render handle that owns one coherent metrics/WAV artifact and preserves errors.
- [x] 1.2 Add a failing C++ controller test for background state transitions, duplicate-request rejection, and a completed 48 kHz acid fixture artifact.
- [x] 1.3 Extend the embedded-page contract test with failing assertions for the Sound Lab action, initial state query, state event, trajectory canvas, and WAV player.

## 2. Rust Workbench Boundary

- [x] 2.1 Implement the opaque sound-fixture render FFI handle and read-only metadata, metric-frame, error, and WAV-byte accessors.
- [x] 2.2 Update the shared C++ bindings and verify null, invalid-path, bounds, and buffer-capacity behavior.

## 3. Background Controller

- [x] 3.1 Implement `SoundLabController` with a `std::jthread`, explicit idle/rendering/ready/error states, immutable completed data, and an atomic generation counter.
- [x] 3.2 Add repository-relative maintained-fixture resolution and build the controller test into CTest.

## 4. Editor Integration

- [x] 4.1 Add native functions for starting a render and reading current state, plus generation-based browser events.
- [x] 4.2 Serve the completed in-memory audition WAV through the JUCE resource provider.
- [x] 4.3 Add the functional Sound Lab panel with clear status, render/retry action, logarithmic centroid plus RMS/peak plot, sample-rate/duration summary, and audio playback.

## 5. Verification

- [x] 5.1 Run focused RED/GREEN tests, the complete Rust suite, the CMake build, and CTest; inspect coverage for changed Rust behavior.
- [x] 5.2 Inject one representative controller/bridge fault and confirm the owning test fails for the intended reason.
- [x] 5.3 Validate the OpenSpec change strictly, sync the accepted spec, map each new scenario to a proving test, and run `scripts/check-spec-coverage`.
- [x] 5.4 Review the resulting UI/resource/thread lifetime diff and commit the verified change as one focused boundary.
