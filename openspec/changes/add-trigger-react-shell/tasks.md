## 1. Source and foundation

- [x] 1.1 Preserve supplied source/provenance, capture all 24 states and responsive reference, and import the React primitives/tokens/fonts without the Design Components runtime.
- [x] 1.2 Specify and implement EngineAdapter, Empty/Felt Kit data, editor store and reversible command/gesture undo with owning tests.
- [x] 1.3 Convert canonical frame and split props-only React components; verify four geometries, tree/browser/macros/inspector/history and current-model bindings.

## 2. Workspaces and editing

- [x] 2.1 Implement Sample region/loop/fade/value/history editing, reordering, source replacement and visual audition.
- [x] 2.2 Implement Slices split/merge/delete/map and asynchronous transient/pitch/loop/loudness jobs, progress/cancel/failure preservation.
- [x] 2.3 Implement Mapping neighbour/free-edge/whole-zone editing, roots, clipboard/delete alternatives, drag/drop and all four multi-file policies.
- [x] 2.4 Implement Layers selector modes, candidate reorder/mute/solo/weights and velocity crossfade/cycle feedback.
- [x] 2.5 Implement Voice template/modules/policy/envelope editors, then Modulation sources/routes/grouping/previews and Macro destinations/rename/MIDI learn.
- [x] 2.6 Implement Routing inheritance/sends/graph and Effects chain add/move/bypass controls.
- [x] 2.7 Finish every context menu, control precision/value/reset/popup, keyboard and drag interaction; make unsupported actions explicitly disabled with a reason.

## 3. Mock runtime and review artifacts

- [x] 3.1 Implement separate 30Hz silent voices/playheads/meters/selector/modulation/host telemetry, complete input cleanup and docked filtered event log/failure switches.
- [x] 3.2 Finish preset browser/new/reload/loading/missing/unsupported/drop recovery and prove all 24 states are reachable through actual actions.
- [x] 3.3 Implement every isolated-component catalog/state/prop/width/background story and final engine contract, updated platform document and complete feature audit.

## 4. Verification and handoff

- [x] 4.1 Run complete owning tests, coverage diagnostics, type/static/build and specification checks; inspect interactions and matching four-size visual comparisons, fixing P0/P1/P2 findings.
- [x] 4.2 Independently review the exact complete candidate, retain evidence, commit the verified shell and leave the preview available for the requested phase-3 review.

Evidence: `docs/trigger/feature-audit.md`, `docs/trigger/verification.md`,
`docs/trigger/evidence/final/`. Task 4.2 is verified by the independent exact-candidate PASS and its
reviewed finalization envelope, retained at `/tmp/dandrum-trigger-final-review/`.
The shell remains available for phase-3 human review; this change is not archived.
