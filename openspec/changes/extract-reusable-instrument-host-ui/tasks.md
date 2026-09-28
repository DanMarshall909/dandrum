## 1. Characterize Existing Behavior

- [x] 1.1 Run focused processor, Sound Lab, and WebView tests; inspect coverage and record the deterministic TB-303 fixture PCM/metrics fingerprint before editing behavior.
- [x] 1.2 Add behavior tests for parameter set/get and host notification, unknown IDs, note-on/off and queue overflow, browser surface refresh, and Sound Lab generation-scoped resources before moving bridge code.
- [x] 1.3 Add failing host-boundary tests for authored public defaults after the first audio block and unchanged JUCE MIDI velocity, repair both paths, and verify current 808 behavior before selecting the TB-303 demo (`plugin-integration`: host-boundary defaults and velocity scenarios).

## 2. Coherent Demo Configuration

- [x] 2.1 Add failing tests for fresh TB-303 configuration, second instrument configuration, fixed automation-slot identity, saved-state restoration, and failed-reload recovery (`instrument-demo-configuration`: TB-303, second demo, restore scenarios).
- [x] 2.2 Introduce immutable demo configuration and select the TB-303 patch, fixture, match source, title, and UI assets for the shipped plugin; keep the common `DandrumAudioProcessor` host.
- [x] 2.3 Add the second instrument fixture and distinct appearance; prove that its configuration loads, renders, and exposes metadata-driven controls without host code duplication (`instrument-demo-configuration`: second demo scenario).

## 3. Shared WebView Controls And Bridge

- [x] 3.1 Extract native bootstrap, parameter get/set, note-on/off, browser events, and shared resource serving into a reusable host bridge while preserving callback lifetime and error behavior (`plugin-integration`: public controls, playable notes scenarios).
- [x] 3.2 Extract JavaScript native-call/error handling, metadata-driven knobs, surface updates, and playable keyboard into shared assets; retain demo-specific HTML/CSS and presentation-only TB-303 steps (`plugin-integration`: public controls scenario).
- [x] 3.3 Replace moved source-text assertions with executable bridge/page assertions, inspect changed-code coverage, and reach full coverage for newly extracted modules.
- [ ] 3.4 Add a failing out-of-tree working-directory test and resolve both configured demo patches and fixtures in developer builds without relying on the process directory (`instrument-demo-configuration`: unrelated working directory scenario).

## 4. Optional Sound Lab Composition

- [ ] 4.1 Add failing tests for configured and absent Sound Lab, fixture/active-instrument mismatch, generation-scoped WAV resources, and match acceptance from the configured source (`instrument-demo-configuration`: optional/mismatch scenarios; `sound-lab-ui`: render and acceptance scenarios).
- [ ] 4.2 Compose Sound Lab commands, events, presentation, and resources only when configured; route render/match/accept paths through the configuration and reject mismatched active instruments.
- [ ] 4.3 Reuse the existing `SoundLabController` and verify that render, match, cancellation, and proposal jobs remain off the audio and editor message threads (`sound-lab-ui`: render, bridge, and match scenarios).

## 5. Verification And Delivery

- [ ] 5.1 Compare the post-change TB-303 fixture PCM/metrics fingerprint to baseline; verify parameter updates, state restoration, automation-slot identity, second-instrument loading, and failed-reload recovery.
- [ ] 5.2 Run the complete native build/CTest and Rust tests, inspect coverage of each extracted module, and run the realtime callback guard; address any behavior-relevant gaps.
- [ ] 5.3 Validate this OpenSpec change strictly, sync accepted delta specs, map every touched scenario to a proving test in `spec-tests.map`, review fingerprints, and run `scripts/check-spec-coverage`.
- [ ] 5.4 Inspect the final status/diff/log, obtain the required independent completion review, and commit each verified material boundary separately.
