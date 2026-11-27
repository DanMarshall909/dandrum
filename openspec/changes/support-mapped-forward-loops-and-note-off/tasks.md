## 1. Reproduce and specify mapped playback

- [x] 1.1 Reproduce the suspected forward-loop mismatch with a signed prepared-map render and retain the RED result.
- [x] 1.2 Add mapped loop boundary, crossfade, reverse and rate tests before changing the renderer.

## 2. Render and retire mapped voices

- [x] 2.1 Make mapped forward loops wrap and crossfade with signed waveform tests passing across blocks and rates.
- [x] 2.2 Add explicit one-shot and gated region modes with note-off, retrigger, choke, steal and reset tests before and after implementation.
- [x] 2.3 Validate mode and loop combinations during preparation and protect the active patch from invalid replacement.

## 3. Connect native Trigger

- [x] 3.1 Project and compile accepted playback modes through Rust/C++ native snapshots and the JavaScript document compiler; prove the mode reaches prepared audio.
- [x] 3.2 Prove editor rejection, undo/redo and fresh-instance restore preserve accepted playback settings and signed host audio.
- [x] 3.3 Prove a supported native host renders the mapped result and explains unsupported modes.

## 4. Verify and deliver

- [x] 4.1 Map every new acceptance scenario to meaningful tests, run focused and affected suites, coverage, strict OpenSpec validation and spec coverage, then review refactoring opportunities.
- [ ] 4.2 Obtain independent completion review of the exact candidate, publish under repository gates, and update issue #33 and the canonical handoff with verified evidence.
