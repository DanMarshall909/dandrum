# Issue #24 implementation evidence

Scope: [JUCE replacement](https://github.com/DanMarshall909/dandrum/issues/24), retaining the existing Rust engine and canonical React UI. The [investigation](../investigations/juce-replacement-investigation.md) and its scope correction are integrated in main at `68dd6553a2f07e71e2fe09ee1d7094d518559937`. No legacy JUCE state/project/ID compatibility is required.

This document records implementation evidence as slices land. An independent engine build is a prerequisite; it does not meet the full issue definition of done.

| Issue stage | Current evidence | Remaining required result |
| --- | --- | --- |
| J0 inventory and baseline | Investigation has 25 responsibility rows, source/resource receipt and proposed platform/candidate matrix. Existing signed native ABI baseline passes; sample-offset onset characterized. | Confirm release/licence/driver scope and capture the remaining signed-render/resource/performance comparisons. No native DAW/device baseline is claimed. |
| J1 shared boundary | Independent configure/build and absent-JUCE/browser fixture pass; retained JUCE target inventory passes. All 18 engine-only CTest checks passed, including 1,088 Rust tests. | Publish the verified independent engine build; agree #23's minimum owned session/SDK/EditorHost contracts, adapt comparison consumer and prove independent resources/protocol. |
| J2 concrete feasibility | Pinned Clack upstream/example contracts inspected; no Dandrum replacement plugin exists yet. | Actual CLAP host/validator signed audio/events/state/lifecycle/instances; parented production React, one real gesture/feedback, lifecycle failure cases; early VST3 and second native OS evidence. |
| J3 replacement state and identity | Backwards-compatibility exclusions are agreed. | Deliberate replacement IDs, bounded versioned state, fresh-instance state/automation/assets round trips and invalid-state recovery. |
| J4 services and session | Existing non-JUCE services and remaining JUCE FFT/window/watch owners are inventoried. | Migrate covered services; consume #23's single session/journal; destructive-editor/undo and lost-acknowledgement proofs, reader/reclamation and job recovery. |
| J5 formats and devices | Existing AU/VST3/Standalone declarations remain the scope until explicitly revised. | Required actual format validators/DAWs and standalone drivers/device recovery/settings/chrome/MIDI. |
| J6 hardening and cutover | Working JUCE comparison source and reusable builds are preserved. | Cross-cutting realtime/event/reconfigure/browser/input/security/resource matrix; installed offline package, licence/link/build audits with JUCE absent, maintained launchers/CI/specs and verified fallback/cutover. |

Gate A requires J0-J2 native evidence; Gate B requires J3-J4 state/session evidence; Gate C requires J5-J6 formats/devices/packaging. All three remain open. Linux/WSLg checks cannot prove native Windows, macOS or physical-device compatibility. Product-scope/licence/ASIO questions are pending; no release support is silently dropped.

## Build prerequisite receipt

- Baseline: current C++ signed-gain/reset ABI test compiled and passed against the retained Rust archive; this is distinguished from a fresh engine build.
- Red: the new absent-JUCE/absent-browser build guard and six conflicting-option cases failed against the unchanged root configuration (three methods, seven failures). Default/explicit-disabled engine-only selections still reached JUCE as expected.
- Green: fresh absent-JUCE/web/ui fixture built every independent native target and passed signed stereo/reset and literal note-offset waveform tests (three guard methods, 71.496 seconds). Retained native-only JUCE comparison configure/target/test inventory passed (one method, 2.164 seconds). README root engine-only configure/build and all 18 CTest checks passed in 106.56 seconds, including 1,088 Rust tests, both signed native tests, retirement/safety guards and independent service tests. The build-boundary check also passed through CTest (28.62 seconds). Final review and whitespace pass; publication is the remaining receipt action. No native DAW/device/other-OS or complete J1 claim is made.
- Acceptance: three reviewed scenarios synced and genuinely mapped; strict canonical OpenSpec passes 48/48 plus the scoped change; coverage map passes 598 scenarios / 359 mapped / 239 unchanged pre-existing todo entries.
- Raw task-local logs: `/tmp/issue24-j1-logs/` and `build/engine-only-regression/`. These local audit paths are not portable release artifacts. The selected build regression uses a fresh source/configuration snapshot and retains the task-owned Cargo cache; no primary build or vendored source was removed.
- Mutation testing remains excluded by the user's instruction. No DSP implementation change or numeric performance improvement is claimed.
