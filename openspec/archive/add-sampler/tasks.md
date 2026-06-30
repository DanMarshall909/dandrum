## 1. Built-In Module Contract

- [x] 1.1 Add failing tests that require the built-in registry to expose a pure `sampler` signal-generator module with `trigger` event input, `rate` and playback-control inputs, and `audio` audio output ports.
- [x] 1.2 Register the built-in `sampler` module and shared port constants needed by graph validation.
- [x] 1.3 Add routing validation tests proving event-to-sampler-trigger, control-to-sampler-rate/control, and sampler-to-audio-output connections are accepted.

## 2. Sampler Asset Validation

- [x] 2.1 Add failing patch validation tests for sampler `asset` parameter success, missing parameter, missing asset ID, and non-sample asset kind.
- [x] 2.2 Implement sampler asset-reference validation with diagnostics that identify the module ID and offending asset configuration.

## 3. Sample Loading

- [x] 3.1 Add tests for loading readable PCM WAV sample assets, missing files, unsupported files, and sample-rate mismatches.
- [x] 3.2 Implement sample asset loading before rendering without doing file I/O in per-frame processing.
- [x] 3.3 Return clear render-preparation diagnostics for unreadable, unsupported, or incompatible sample assets.

## 4. Sampler Processing

- [x] 4.1 Add render tests proving trigger events start sample playback, trigger payload velocity does not scale sampler output, routed rate control controls pitch/playback speed, later triggers replace monophonic playback, and output becomes silent after sample completion.
- [x] 4.2 Add render tests for start-position and loop playback controls.
- [x] 4.3 Implement sampler module state and audio generation in the graph processor.
- [x] 4.4 Ensure offline and realtime graph processor construction handles sampler modules without panics.

## 5. Determinism And CLI Acceptance

- [x] 5.1 Add a deterministic sampler render test that renders the same patch, sample, settings, control signals, and events twice and compares buffers.
- [x] 5.2 Add a minimal sampler YAML example and deterministic sample fixture for tests/examples.
- [x] 5.3 Add an end-to-end CLI acceptance test that renders the sampler example to a non-empty WAV file.

## 6. Future Polyphony Design Guardrails

- [x] 6.1 Document in code comments or tests that sampler playback is intentionally monophonic until generic per-voice bus support exists.
- [x] 6.2 Add tests or examples showing MIDI note-to-rate and velocity gain remain patchable through upstream/downstream modules rather than sampler-internal policy.

## 7. Verification

- [x] 7.1 Run Rust unit and acceptance tests with `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [x] 7.2 Run CMake/CTest verification if build configuration is available: `$HOME/.local/bin/cmake -S . -B build`, `$HOME/.local/bin/cmake --build build`, and `ctest --test-dir build`.
- [x] 7.3 Run OpenSpec validation for `add-sampler` and confirm every sampler requirement has test or implementation evidence.
