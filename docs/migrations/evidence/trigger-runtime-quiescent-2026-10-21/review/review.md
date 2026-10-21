# Independent completion review — paired quiescent runtime foundation

Verdict: **PASS** for the exact 20-path candidate and the deterministic five-output finalization envelope. No in-scope source correction remains. This approves a task 4.1 prerequisite, not task 4.1, the editor, or the beta as a whole. The required same-reviewer final precommit addendum must verify the actual 22-path result before normal hooks begin.

## Identity, independence and scope

Candidate manifest SHA-256: `9239b19ccc5cf258f6cf3fad66d2cdcacead41831c1378fff8d0688d46282487`.
Base/current/integration/upstream: `5d332b81adc07117e59f69a528785d194c8da3ff`.
Branch: `lane/trigger-runtime-20261020`; intended upstream: `origin/feat/juce-free-host-adapters`.

I independently reconstructed the manifest byte-for-byte from Git status, stage-zero index entries, blob contents, filesystem modes and hashes. Every staged payload equals its worktree file and calculated Git blob identity. There are 20 changed paths and no outside unstaged/untracked changes or unmerged entries. HEAD, local upstream tracking object and merge base agree. Network freshness remains the root's recorded fetch responsibility and must be checked again unchanged before push.

The two source payloads retain the hashes reviewed preliminarily: runtime.rs `b7a65c05a715d739b5a5dd4ff40a2f244025088a756f584bdfe99f62f71353b8`; runtime_quiescent.rs `c3d23533215e762663c0eb2706ba4ee58ffe83e54a95b79e62ec58f00de3568f`. I reassessed them with the published note-controller integration and combined evidence. The design diff contains only the accepted inactive-runtime section. Tasks and canonical specs/map are unchanged; seven of fourteen tasks remain checked.

Read-only review used the project/shared rules, completion-review, behavior-first TDD, coverage-and-mutation and reflection guidance. I read current proposal, tasks, relevant design, the root decision, tracker scope and source/evidence. I ran no builds, tests, native/browser/network actions or publication commands, changed no repository/index/cache content, and delegated no review decisions. Reviewer evidence was written only under reviewer/round1. The original preliminary record is retained separately.

## Task-to-implementation matrix

| Task or accepted boundary | Implementation and decision |
|---|---|
| Task 4.1 runtime prerequisite: inactive paired service | RuntimeControl::service_quiescent exclusively borrows both endpoints, authenticates their private admission Arc identity first, and returns ForeignEndpoints before any mutation on mismatch. |
| Bounded ownership progress | One retired pop/reclaim and at most one pending pop/adoption; explicit predecessor destruction on the caller; QuiescentReport reports adoption and destruction count. |
| Immediate activation capacity and unchanged Session acceptance | Direct quiescent disposal clears in_flight; service makes no Session publication and creates no artificial retirement requiring a callback. |
| Same graph-adoption editor law | Shared AudioState::adopt_editor_intent clears old gates and records observed onset sequence; the active callback uses its original single intent snapshot. |
| Preservation boundary | Existing scalar values, ordinary FIFO/feedback and public parameter indices are behaviorally checked. Nonzero UI settlement/restore epoch remain the explicitly accepted later-controller observer gap, with only structural preservation inspected here. |
| Tasks 3.3, 4.1 and full beta completion | Remain open. No controller service route, rate/configuration, stopped flush, native CLAP, installed product or DAW acceptance is inferred. |

## Criterion-to-test matrix

Tests below are public integration tests in src/trigger-host/tests/runtime_quiescent.rs, using real prepared graphs and Session acceptance. Existing runtime/editor tests provide the active-path regression baseline.

| Criterion | Test and independent outcome |
|---|---|
| Pending adoption, explicit progress, off-audio destruction, repeated idle, immediate second acceptance | quiescent_adoption_destroys_the_predecessor_and_restores_capacity_without_a_render: actual first/second Session revisions, caller deallocation measurement, no residual retirement, fixed signed stereo before/after distinctions. |
| Foreign-pair failure atomicity and two instances | foreign_quiescent_pair_is_rejected_before_either_nontrivial_instance_changes: both pending and already-retired pairs reject without memory change; both pending replacements still render their distinct expected pan and capacity remains occupied until correct service. |
| Real returned audio ownership and retired reclamation | returned_audio_reclaims_a_real_retirement_on_control_and_accepts_another_publication: distinct spawned audio thread renders the swap and returns the owner; original thread destroys retirement, accepts another publication and preserves signed sustained output across idle calls. |
| Held editor notes after adoption | quiescent_adoption_resumes_a_still_held_editor_note_only_on_an_actual_block: heard/unheard held cases resume with literal signed prefix/tail, remain sustained without retrigger, and release to silence. |
| Released/cancelled/reset onsets cannot replay | quiescent_adoption_cannot_replay_a_released_cancelled_or_reset_editor_onset: six distinct prior states produce repeated silence after adoption; a genuine later press sounds and detach silences it. |
| Existing host arbitration | quiescent_adoption_retains_host_pitch_arbitration_until_a_real_host_release: no host voice resurrection, same-pitch editor suppression through host release block, then signed editor resumption on a later untouched block. |
| Queued scalar/notes, applied values, feedback and Session state | quiescent_adoption_preserves_applied_values_feedback_and_queued_scalar_and_note_events: old applied value and offset feedback persist; full FIFO remains full; queued level/pan/remapped note produce fixed signed samples; feedback values, indices and accepted snapshot remain correct. |
| Running callback resource safety | Shared render_events sentinel measures zero allocation and zero net deallocation in real process_events calls, including actual block-boundary swap; caller service separately measures zero allocation and negative retained allocation/bytes. |
| Nonzero UI settlement and restore epoch | Not claimed behaviorally proved. Later controller-owned paired service must supply the public observer evidence; no test-only API is added. |

## Acceptance evidence and test effectiveness

Original baseline coverage precedes source changes. API-absence RED is clearly a missing-symbol compile failure. Retired RED reports zero rather than one reclaimed runtime after a real threaded swap. Editor RED rejects both held silence and released-tap replay with signed samples. The first implementation's array annotation failure and launcher/postprocessing/resource failures are retained separately, never treated as successful verification or behavioral RED. Preservation cases are honestly labeled characterization. The helper extraction follows tests for both adoption paths.

The tests observe stable public behavior, independent fixed PCM values, actual ownership transfer and Session publication. They avoid private counters or fabricated callbacks. A gated sample child makes sustained silence/release meaningful. Exact gains derive from the fixture contract, not the production mapper. The suite's helpers centralize the callback memory sentinel without hiding the outcome oracles. Three comparable timing reruns give approximately 0.057–0.251 seconds per new test; the six-case matrix exceeds 1.5 times the 0.0966-second peer median but earns its cost with distinct transitions and remains below one second. Fast membership with an empty slow selection is reasonable for this bounded suite.

Combined source-bound logs prove 128 ordinary host tests plus two compile-fail doctests, 30 Session tests, formatting and all-target Clippy with warnings denied. Full LLVM runs all 128 ordinary tests. I checked the raw result groups and source bindings, not only the completion summary. Strict scoped OpenSpec, 51 canonical specs, spec-map and both diff checks have successful retained logs whose hashes I verified. The spec map still reports 618 criteria, 379 mapped and 239 ratchet todos; this slice changes no canonical scenario or task checkbox.

Coverage is correctly qualified: all 48 new service/closure/helper regions execute. Reported runtime totals are 97/97 functions, 1180/1186 lines and 1888/1900 regions; merged distinct LCOV coordinates are 1150/1150. Production coordinates cover 1007/1008 regions. The unchanged apply_event error-forwarding region at runtime.rs:932 remains unexecuted under the valid public budget. No blanket owner-100%, branch or MC/DC assertion is accepted. Coverage is execution evidence; the matrix above supplies behavioral assertions.

I independently checked all 328 author, 119 combined and 39 source-free archive members against their inventories and archive digests, plus portable SHA256SUMS. Combined raw profile hashes match the retained archive; its merged profile is byte-identical to the retained reconstructed profile. Eleven objects/raw profiles are bound at consumption time. Direct export equals the retained report for all structures and counters after removing only derived percent presentation and Cargo-added metadata. I independently recomputed the 48/48 new-region result. Replaced shared-cache binaries need not remain available to substantiate those archived consumption-time records; I did not claim to rerun LLVM tools.

The source-free release result is a valid qualified composite, not a fresh unfiltered rerun. The first command's stale 15-test unit executable is explicitly excluded. Its normal library and ten integration targets were freshly compiled, supplying 113 current integration tests and two doctests. A timestamp-only correction to the copied runtime input forced a fresh default-release --lib compilation and 15 current unit tests. Raw compiler lines, distinct old/new binary digests and fingerprints support that correction. I verified all 592 source input digests against the current integrated worktree and the supplemental raw 15-test result. Root bytes/timestamps and 590 unrelated fixture inputs are preserved. This supports Cargo/Rust dependency independence only, not new CMake/native/CLAP/browser/DAW/installed readiness.

## Architecture, operational safety and alternatives

Identity is checked before reclaim or adoption. Both endpoints are uniquely borrowed and non-cloneable, so safe callers cannot simultaneously process that RuntimeAudio. Destruction is explicitly on the caller; API documentation assigns that caller to control. The work is finite: one retired owner, one pending owner, a fixed 128-entry intent snapshot and reconciliation. No unsafe code, mutex, browser, file lookup, clock dependency, new crate or DSP behavior is introduced. Active audio still reserves retirement space before consuming a candidate. Direct service restores capacity only after predecessor destruction; rejected foreign pairs cannot consume either instance's owner.

Shared helper extraction is the useful simplification: it keeps the adoption law common while preserving the active callback snapshot moment. A larger ownership abstraction or new configuration/controller route would increase this slice's scope without demonstrated benefit. No further required refactor was identified.

Resource-stopped RAM baseline is disclosed as NO_REPORT. Final targets/profiles/TMPDIR resolve to existing physical-disk locations and retained disk/memory floors are satisfied. Root's cache handback receipt records no active Rust builders and separate release-cache ownership. No cleanup, relocation or weakened threshold is approved. Mutation is explicitly excluded by the recorded user direction, not reported pending or passed.

## Publication soundness and finite envelope

The reviewed external one-off manifest, validation, curator, apply-envelope, exact-tree-transfer, normal-publication, sync and recording helpers are proportionate operational scripts. The curator's first schema-key assertion failed before writing candidate output; the preserved preimage and corrected script do not constitute a product behavior RED. Manifest reconstruction and archive checks above independently verify their important outputs.

Only five deterministic metadata outputs are approved: exact copies of this report and verdict into portable review/, the prescribed receipt.review object, the single prescribed progress sentence replacement, and regenerated SHA256SUMS. Source, tests, design, raw archives, task checkboxes and specs/map cannot change. approved-envelope.json identifies each resulting byte sequence by length, SHA-256 and Git blob and predicts the exact final manifest hash; two added files make 22 paths.

The same reviewer must next compare the actual final manifest/tree and named validations with this envelope before normal hooks. The publication preflight must reject an empty integration tree before invoking a hook, then accept the exact staged transferred tree. Transfer is allowed only into the clean integration worktree at the recorded base and must reproduce every blob/mode and full tree. Normal commit hooks, clean-lane/exact-staged fast-forwards, unchanged-upstream fetch and normal push remain required. No force, main merge, hook bypass or cache deletion is approved. A failed gate, source/content drift, upstream movement or unexpected generated file invalidates this PASS and requires reassessment. The final addendum is an exact-state verification gate, not permission for new semantic changes.

## Reusable lessons and remaining scope

No actionable in-scope finding remains. The timestamp freshness incident is a useful narrow evidence lesson: content copies that preserve an older mtime can fool a checksum-disabled Cargo freshness decision. The evidence record already owns the concrete cause and correction. A general standing rule change is not required or authorized here; broad cache invalidation would waste reusable builds. No new guidance was written.

The next controller route must prove nonzero UI settlement/restore epoch preservation and connect actual inactive servicing. Rate/configuration, stopped flush, CLAP lifecycle and GUI scheduling, native editor destruction, VST3, standalone devices, named DAW and installed offline acceptance remain open. This PASS makes none of those product-readiness claims.
