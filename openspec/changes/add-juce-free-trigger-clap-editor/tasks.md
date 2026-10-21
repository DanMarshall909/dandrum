## 1. Baseline And Build Boundary

- [x] 1.1 Record free space, reusable build sizes, pinned Clack/Wry/WebKitGTK toolchain and licence/dependency scope; characterize the current headless CLAP, Trigger transport and session coverage before changing behavior.
- [x] 1.2 Add failing absent-JUCE/source-lookup and copied-resource build tests, then scaffold the format-neutral Trigger host crate and opt-in editor build without changing the two existing CLAP fixtures.

## 2. Format-Neutral Trigger Host

- [x] 2.1 Specify and implement deterministic packaged-resource resolution plus a session owner for the real Advanced Drum Kit; prove copied external resources, known signed MIDI-36 output and invalid/missing-resource preservation.
- [x] 2.2 Specify and implement capacity-one runtime activation/reclamation and bounded parameter/event handoffs; prove off-audio construction/destruction, block-boundary swaps, backpressure, reset and allocation/lock-free callback safety.
- [x] 2.3 Specify and implement the versioned bounded editor protocol with attachment/request identities, explicit capability errors and malformed/oversized/stale/conflicting input rejection that preserves accepted state.
- [x] 2.4 Specify and implement public parameter gesture/feedback and version-2 atomic Trigger state; prove balanced automation, nonzero-offset host feedback, fresh-instance document/resource/value restore and complete invalid-state rollback without structural journal pollution.

## 3. Production React And Wry Editor

- [x] 3.1 Refactor the Trigger native adapter onto the versioned transport selector with behavior-first JavaScript tests; retain the silent browser preview and all existing JUCE comparison behavior.
- [ ] 3.2 Package the production Trigger bundle/fonts and implement the Wry Linux-X11 child owner on the CLAP main thread; prove actual parenting, local-only resource loading, fixed-size acceptance, resize rejection, show/hide and create/order/API failure cleanup.
- [ ] 3.3 Connect the required session snapshot, mapping/job recovery, native undo, notes and parameter methods; prove complete WebView destruction releases notes/gestures/callbacks exactly once while admitted work and accepted history survive reconnect.

Task 3.2 remains open through task 4.2's actual CLAP thread/API/order proof. Its verified Wry library facets permit isolated task 3.3 snapshot and frontend work in parallel. Task 3.3 includes explicit optional-service metadata, omission compatibility and honest unavailable presentation as specified in the design; required session/notes/parameter/mapping/undo behavior remains native. Neither lane alone completes 3.3 or the editor acceptance gate.

## 4. Actual CLAP Product Boundary

- [x] 4.1 Add the deliberate Trigger identity to the dynamically loaded release binary with session-backed audio/MIDI/parameters and copied resources; prove descriptor stability, sample-offset signed playback, reset/reactivation and no JUCE/source-tree dependency.
- [ ] 4.2 Implement the CLAP GUI and host-parameter event extensions over the shared host/editor core; drive a rendered Trigger control and host feedback through an actual X11 parent without a private event loop.
- [ ] 4.3 Extend dynamic-host acceptance for lost acknowledgement, destroy/recreate/undo, invalid GUI/state paths and two independently edited/rendering instances; retain official validator results and every unsupported/skipped capability explicitly.

## 5. Verification And Publication

- [ ] 5.1 Complete focused owner coverage, changed-code refactoring, dependency/link/resource audits and required Rust/JavaScript/native/build/callback-safety checks; preserve reusable caches and retain source-bound evidence. Mutation remains excluded by recorded user direction.
- [ ] 5.2 Independently review each material boundary, map only scenarios genuinely proved by the final tests, pass strict scoped/canonical/spec-coverage gates and normally publish focused commits. Keep VST3, standalone devices, named DAW, installed release packaging and non-Linux support open for the next increment.
