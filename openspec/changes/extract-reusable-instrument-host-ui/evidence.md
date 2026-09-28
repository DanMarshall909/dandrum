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
