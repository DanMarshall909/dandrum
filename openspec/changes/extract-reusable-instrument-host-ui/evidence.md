# Verification evidence

## Baseline before behavior changes (task 1.1)

Branch base: `06d3b92` (`agent/fix-plugin-preparation-lifecycle`).

- Focused CTest command: `ctest --test-dir build -R 'cxx-plugin-construction|cxx-plugin-host-parameter-surface|sound-lab-controller|tb303-web-ui-contract' --output-on-failure`: 4/4 pass.
- Fixture command: `cargo run --manifest-path src/rust-engine/Cargo.toml --bin dandrum-sound-workbench -- render examples/sound-design/tb303-acid-poc.yaml --output-wav /tmp/dandrum-host-ui-evidence/baseline.wav --output-metrics /tmp/dandrum-host-ui-evidence/baseline.csv`: pass.
- WAV: 2,112,044 bytes, SHA-256 `1ac813cea4ba929141af90b713a37378ad253ffbe2c1e08e32d130abd567b077`.
- Metrics CSV: 195,902 bytes, SHA-256 `338a1cbfe4ee8faf7a08fa6f863e82fe9777320b75f870a494bae68dd249f824`.

Coverage inspection: `PluginConstructionTest.cpp` covers fixed slots, editor MIDI queue overflow, public parameter updates, presets, state restore, reload success/failure, and prepared-engine behavior. `PluginHostParameterSurfaceTest.cpp` covers fixed host slot count and changing the active public surface. `SoundLabControllerTest.cpp` covers rendering, duplicate rejection, invalid fixtures, matching, cancellation, and result discard. `Tb303WebUiContractTest.cpp` only searches HTML and C++ source text, so it cannot prove WebView commands, page behavior, or generation-scoped resources. The C++ targets have no instrumented coverage configuration; task 1.2 adds behavior tests before bridge movement, and task 3.3 will inspect coverage of extracted modules. Rust line coverage is gated separately by `scripts/check-rust-coverage`.

## Native bridge characterization (task 1.2)

`cxx-plugin-editor-bridge`: pass. It invokes the editor's native handlers and checks metadata snapshots, a parameter change reaching the same fixed host slot and listener, missing/unknown arguments, note-on/off admission and full-queue rejection, replacement-surface generation consumption, current page serving, Sound Lab render/duplicate response, and current-versus-stale generation WAV serving. The browser refresh check proves the generation transition and new snapshot; a visible browser navigation is not asserted in the headless CTest runner.

## Host boundary corrections (task 1.3)

Two new direct-Rust comparison tests in `cxx-plugin-construction` were red before the production fix:

- Fresh default 808: first sample host `0`, Rust `0.999942`; the visible host slot had the authored default, but the APVTS raw value used by the audio callback did not.
- TB-303 host MIDI note-on velocity 100 at frame 7: host sample `-0.000962368`, direct Rust sample `-0.000132931`; JUCE `getVelocity()` was multiplied by 127 a second time.

After changing slot initialization to notify the host/APVTS listener and forwarding JUCE's byte velocity directly, `cxx-plugin-construction` passes. The tests compare the full first stereo block against a separate Rust engine, including frame offset and all authored/default or explicit public values.

The complete CMake build passed, including plugin formats and the new bridge test. Complete CTest passed 11/11 (14.05 seconds). `openspec validate extract-reusable-instrument-host-ui --strict` passed. The build printed nonfatal Linux display authorization warnings while generating VST3 module information; artifacts and tests completed successfully.

## Coherent demo configuration and second instrument (tasks 2.1–2.3)

The fresh TB-303 test failed first because the existing processor opened the kick. The extended configuration test then failed to compile before the configuration API existed. After implementation, the shipped default loads the TB-303 patch and metadata; a kick-configured instance loads the 808 patch, keeps the same fixed 64 slot objects, and restores saved kick state into a TB-303-configured host. A failed reload retains that restored instrument and audible output. Existing kick-specific tests now construct the kick configuration explicitly.

The kick fixture renders one second at 48 kHz through Sound Lab. Its standalone render produced 48,000 stereo frames, PCM peak `0.999969`, RMS `0.114103`, WAV SHA-256 `0bdb56efbc12a580033bc873d8730a5a7dd6b7b126b0b886abb41f69fb07f433`, and metrics SHA-256 `b68d8c5d6a8017d5a18c07bcb8cefdf3fa90ca5470574d591076a94dbbbfe42c`. The bridge test failed with the old 303 fixture path, then passed after selecting the fixture from configuration. The new kick page is visually distinct and uses metadata-driven controls and keyboard from a shared JavaScript asset. Its control, update, note, and error behavior passes the Node VM test. A headless Chrome preview at 1280×900 was inspected: six labeled controls and the playable keyboard fit without overlap.

The complete CMake build and CTest passed 13/13 after this boundary. The shared script is currently used by the kick page; task 3.2 moves the TB-303 page onto the same script and removes its duplicate inline code. The optional Sound Lab UI/composition and active-instrument mismatch checks remain tasks 4.1–4.3.
