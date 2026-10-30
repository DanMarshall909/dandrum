Scope revision, 22 October 2026: the first beta is CLAP-only. Existing task identities are retained. Former task 1.4 (VST3 candidate qualification), tasks 3.1–3.3 (VST3), 4.1–4.3 (standalone/devices) and 6.3 (standalone Save/Load) are deferred, not completed. Their later scope is recorded in `future-products.md`. No VST3 or standalone implementation is authorized by this active increment.

## 1. Qualification And Verification Ownership

- [x] 1.1 Record the published CLAP baseline, current spec/owner evidence, compiler/SDK/device/DAW availability, licences, disk/target sizes and measured resource/transport budgets; distinguish verified facts from candidate support.
- [x] 1.2 Specify and implement the maintained normal-hook policy for the recorded mutation exclusion before changed-engine publication, preserving ordinary tests/strict coverage and proving the intended changed-file behavior without bypassing hooks.
- [x] 1.3 Specify and verify owned disposable-run cleanup after success, failure, cancellation and recovery, retaining compact evidence/reusable caches and proving exact worker/path ownership and bytes reclaimed.

Root owns integration/Git/tracker/shared caches. Task 1.3's cleanup runner/tests and RED/GREEN evidence passed independent review and were normally published at `c112ed10`; preserve that verified boundary and its reusable caches. Independent ready work may use existing isolated lanes with explicit handback and one heavy build at a time. Every behavior change follows RED/GREEN/focused coverage/refactor, normal checks, independent material-boundary review and focused publication. Mutation remains excluded.

## 2. Precise Format-Neutral Host Clocks

- [x] 2.1 Specify and implement precise supported realtime clock/configuration/preparation behavior, preserving integer support and proving fractional sample timing, independent instances and off-audio replacement/reclamation through real engine owners.
- [x] 2.2 Connect precise clocks through session/controller/CLAP reconfiguration; prove invalid-candidate rollback, existing state/parameter behavior and the exact formerly failing official fractional-rate case without rounding or exclusions.

## 5. Selected Samples And Interactive Patch Publication

- [x] 5.1 Specify and implement bounded immutable sample snapshot/catalog ownership using the existing decoder; prove real metadata/content identity, original-file replacement/removal, isolation, memory bounds and off-audio lifetime/reclamation.
- [x] 5.2 Extend the versioned bounded native protocol and controller admission/status for selected resources and full-patch work; prove version 1 compatibility, invalid/stale/conflicting/busy results and exact unadmitted versus admitted ownership.
- [x] 5.3 Publish interactive full-patch edits through native history while preserving public identities/current physical values; prove compatible-default handling, true layout rejection, empty-patch silence, once-only import and native undo/redo with signed PCM.
- [x] 5.4 Integrate actual native file selection with the production frontend; prove real decoded metadata, cancel/invalid/stale-view cleanup and valid normal single-file mapping instead of accidental kit overlap or mock acceptance.
- [x] 5.5 Join native sample import/mapping/status/recovery/Undo through real editor destruction/recreation and an independent instance; retain actual selected-waveform oracles and no late callback/resource leak.

The clause-to-evidence completion audit is recorded in `docs/migrations/evidence/trigger-selected-foundation-completion-2026-10-29/README.md`. These component tasks do not close installed/native-input/continuous-audio/DAW acceptance or portable state. Existing coverage gaps and the engine strict-Clippy baseline failure remain explicit.

## 6. Portable Patch Sample And Parameter State

- [x] 6.1 Specify and implement bounded self-contained versioned state framing/content validation, retaining version 2 packaged-kit reading and proving malformed/corrupt/oversized/short-reading rollback before publication.
- [x] 6.2 Connect portable state through the existing CLAP projection; prove fresh instances restore selected PCM, logical patch/resource identity and named physical values after original instances/files disappear, without shared source/cache fallback.

Fresh CLAP restoration evidence: `docs/migrations/evidence/trigger-clap-fresh-state-2026-10-30/README.md`.

## 7. Installed Native And Named-DAW Acceptance

- [ ] 7.1 Build/install the intended beta artifact set outside the checkout with pinned notices and complete resource identities; prove source-hidden offline CLAP execution and no JUCE build/runtime/linkage requirement.
- [ ] 7.2 Exercise installed CLAP with actual native input and host audio/MIDI callbacks during selected-sample mapping, public edits and complete editor loss/recreation; prove retained history, signed playback, callback safety and independent instances at their real boundaries.
- [ ] 7.3 Qualify and run the named DAW acceptance on installed CLAP instances: balanced gestures, timed host automation/feedback, editor-absent servicing, two independently edited instances and saved-project fresh selected-sample/parameter restoration.

## 8. CLAP Beta Completion

- [ ] 8.1 Complete current-source owner coverage/refactoring and all Rust/JavaScript/native/build/resource/dependency/licence/official-validator checks, retaining unsupported capabilities and unavoidable external gaps honestly with portable evidence.
- [ ] 8.2 Independently audit every retained CLAP beta requirement against actual artifacts/native/DAW evidence, earn scenario mappings and strict canonical validation, normally publish verified release boundaries and remove only owned disposable work; mark the CLAP beta complete only when all required outcomes are proven.
