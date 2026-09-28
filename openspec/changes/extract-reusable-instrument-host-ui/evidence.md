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

## Shared host bridge and controls (tasks 3.1–3.3)

`InstrumentHostWebBridge` now owns the JUCE native bootstrap, registered parameter/note commands, snapshots, browser surface refresh, and shared page/script resources. The editor's browser is declared after the bridge, so C++ member destruction destroys browser-owned callbacks before the bridge they capture. The TB-303 and kick pages load the same `SharedInstrumentUi` script; the TB-303 step/function buttons remain presentation behavior in its own page.

The source-text `Tb303WebUiContractTest.cpp` was removed after executable tests replaced it: `cxx-plugin-editor-bridge` invokes the registered native callbacks and verifies resources and reload behavior; `shared-instrument-ui` executes knob/keyboard/error/update behavior; `tb303-page` executes the shared script with the 303 Sound Lab/presentation page; and `native-function-bootstrap` checks request IDs and completion dispatch. A concurrent kick/303 reload test proved each parameter snapshot is a complete surface; `getPublicParameterSnapshot()` now takes the existing reload mutex on the message thread while copying metadata and values.

An instrumented CMake build in `/tmp/dandrum-host-ui-coverage` and `gcov` measured `InstrumentHostWebBridge.cpp` at **96/96 executable lines (100%)** and `InstrumentDemoConfiguration.cpp` at **10/10 (100%)**. V8 coverage from the shared-control and 303-page VM runs found **zero uncovered nonblank lines** in the shared JavaScript asset. The complete normal build and CTest passed **14/14** after extraction. The out-of-tree coverage CTest command exposed working-directory-dependent example lookup; running the same instrumented binary from the repository root passed and supplied coverage. This asset lookup issue is tracked separately below.

## Asset lookup follow-up (task 3.4)

An instrumented CTest run from `/tmp/dandrum-host-ui-coverage` failed before exercising the bridge: the kick configuration could not find `synthetic-808-kick.yaml` when the process working directory was outside the checkout. The same executable passed from the repository root. Task 3.4 specifies an explicit failing test and a developer-build path resolution fix; installed-plugin asset packaging remains outside this change.

`cxx-plugin-demo-configuration` now changes into a fresh temporary directory, constructs both configured demos there, and checks absolute existing patch/fixture paths, loaded instruments, and public controls. It failed first with `demo patch or fixture did not resolve outside the checkout`. The plugin target now receives the CMake source root and resolves its configured demo assets from that checkout. Both the normal build-tree and `/tmp/dandrum-host-ui-coverage` CTest runs pass `cxx-plugin-demo-configuration` and `cxx-plugin-editor-bridge` (2/2 in each tree), with the latter tests executing from outside the checkout. `gcov` reports `InstrumentDemoConfiguration.cpp` at **12/12 executable lines (100%)** after the change. The temporary directory is removed by the test's working-directory guard on every return path.

## Optional Sound Lab and second demo presentation (tasks 4.1–4.3)

The editor bridge test first failed to compile while expecting a fixture-free editor to have no Sound Lab controller. A second red run found the configured kick page lacked a Sound Lab panel. The editor now constructs `SoundLabController` only for a configured fixture, registers its native actions only in that case, and serves its script, panel, and WAV resources only while the configured source path and instrument ID match the active instrument. The same panel/CSS/JavaScript is composed into the TB-303 and kick page templates; their main appearances stay distinct. A fixture-free kick variant retains parameter and note commands while omitting Sound Lab. Replacing the kick with the TB-303 hides the panel/script and rejects render and match acceptance with a clear mismatch error while the replacement remains audible.

`cxx-plugin-editor-bridge` now renders a kick reference, matches it through the editor bridge, verifies same-generation reference/candidate WAV resources and stale-generation rejection, then accepts the matched YAML from the configured kick source and compares each accepted normalized public value with the match result. The existing `sound-lab-controller` test covers duplicate render requests, conflicting match work, cancellation retaining a best candidate, retryable errors, and result discard. Code inspection confirms render, match, and proposal operations each start a `std::jthread` worker; editor callbacks only initiate these jobs and return, and the realtime callback guard passes. No audio callback path calls the workbench.

The Node page test runs the extracted Sound Lab JavaScript with the common control script and covers render, match, proposal, acceptance, audio, plots, and events. It caught a proposing-status fallthrough, fixed with a return before the idle presentation reset. V8 reports **97/97 nonblank executable script lines covered**. Instrumented C++ tests report `InstrumentHostWebBridge.cpp` **96/96 lines (100%)** and `InstrumentDemoConfiguration.cpp` **12/12 (100%)**. The complete CMake build and CTest pass **14/14**. Headless browser previews of the kick with its Sound Lab panel at 1280 and 760 pixels wide were visually inspected; the panel, controls, and keyboard fit without horizontal overlap.

## Fixture and host invariants (task 5.1)

The post-change TB-303 fixture WAV and metrics CSV hashes exactly match the baseline: `1ac813cea4ba929141af90b713a37378ad253ffbe2c1e08e32d130abd567b077` and `338a1cbfe4ee8faf7a08fa6f863e82fe9777320b75f870a494bae68dd249f824`. The complete CTest run includes host parameter/gesture notification, authored defaults, MIDI velocity and offsets, fixed 64 automation slot identity, cross-demo state restoration, failed-reload recovery, second-instrument loading, and the runtime callback guard.

## Full verification and specification sync (tasks 5.2–5.3)

The final CMake build passed for the shared plugin, standalone binary, VST3 bundle, and test executables. `ctest --test-dir build --output-on-failure` passed **15/15** in 13.76 seconds, including Rust engine tests, the realtime callback guard, the Sound Lab controller, editor bridge, demo configuration, executable JavaScript pages, and the spec-map checker. `scripts/check-rust-coverage` passed its strict engine-file policy; cargo-llvm-cov printed a nonfatal mismatched-data warning after all tests passed. The out-of-tree instrumented CTest pass and `gcov` measured the extracted bridge at **96/96 C++ lines** and demo configuration at **12/12 lines**. V8 measured the shared Sound Lab script at **97/97 nonblank executable lines** and earlier measured the shared control script with no uncovered nonblank lines.

The `instrument-demo-configuration` baseline spec was added; `plugin-integration` gained authored-default/velocity and metadata-control contracts while removing the obsolete native-JUCE-only requirement; and `sound-lab-ui` now describes configured fixtures and acceptance sources. Three changed Sound Lab scenario fingerprints were reviewed against the controller, editor bridge, and page tests before re-affirmation. A new red/green Python test enabled truthful `js:` test references in `spec-tests.map`; removed source-text test references were replaced with executable evidence. `scripts/check-spec-coverage` passes with **422** criteria, **57** mapped to tests, and **365** remaining ratchet todos; the unchanged legacy runtime-information scenario retains a todo. The change validates strictly, and all **34** main specs validate strictly.

## Independent review repair (task 5.4)

The first independent review returned **FAIL** on three verification gaps. The editor bridge test now invokes callbacks from the editor's composed `WebBrowserComponent::Options`, asserts all shared and configured Sound Lab registrations, and asserts the Sound Lab commands are absent in a fixture-free editor. It also exercises match rejection after the configured kick is replaced by the TB-303. The page test now asserts exact score text, six separate plotted comparison series with distinct multi-point trajectories, a cancelled best result that remains audible and acceptable, error display and retry, and graph proposal presentation. Separate in-memory mutation probes confirmed this test rejects a wrong score, an empty comparison plot, and a disabled cancelled-result acceptance path.

The CMake configuration now requires Node.js so these page and bridge checks cannot silently disappear. A fresh configure with a nonexistent `DANDRUM_NODE_EXECUTABLE` succeeded before that change and fails with the required-node diagnostic after it. The README names the dependency. The targeted C++ bridge test and Node page test pass after the repair. Complete build, CTest, and strict OpenSpec validation were rerun for the second review candidate; retained logs and exact commands are in the second review packet.
