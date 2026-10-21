# Independent completion review: persistent render reconfiguration

**PASS** for the frozen 18-path component candidate and the exact administrative envelope in `approved-envelope.json`. No in-scope correction remains. This is not completion of OpenSpec task 4.1, the CLAP facade, or the full beta. Publication remains subject to the same reviewer's actual-final-tree check and the named normal-hook gates.

The review started without the implementer's conversation. It inspected actual staged/worktree bytes, surrounding implementations, executable assertions, raw source-bound evidence, and publication helpers. It followed project/shared rules, the agentic-delivery completion-review reference, coverage-and-mutation guidance, and reflection guidance. No repository/index/cache/build mutation, test execution, native host launch, network access, deletion, or delegation was performed. Only this external reviewer directory was written. OpenSpec list/status/apply were read with telemetry disabled. Network preflight was inapplicable under the explicit no-network review scope.

## Identity and scope

| Identity | Independently checked value |
|---|---|
| Candidate worktree / branch | `/tmp/dandrum-lanes/runtime`, `lane/trigger-runtime-20261020` |
| Base, HEAD, intended upstream, integration base | `301641faa72e57837df3d49c6c4aa6563882d6c4` |
| Intended upstream ref | `origin/feat/juce-free-host-adapters` |
| Serialized manifest SHA256 | `24bd25ae143ee0c5f5dbd8d605233829dd1dd7f8db736edd6e6debe4f4f48862` |
| Complete candidate index tree, computed without write-tree | `ab7e48774fdc751f981299c804fcfb64d2c0ab4f` |
| Candidate paths | 18, all regular files, canonical Git mode 100644, stage 0 |
| Unmerged / unstaged / untracked | None |
| Primary worktree | Clean at `68dd6553a2f07e71e2fe09ee1d7094d518559937` |

Each changed path's index and worktree content, SHA256, Git blob hash, mode, and status was reconstructed and compared to the serialized manifest. Bytewise path ordering and JSON serialization reproduce the manifest digest. The lane has no configured tracking branch; the explicit intended upstream is recorded in the packet, live tracker and prefreeze integration receipt and resolves to the same object as HEAD. No integration was needed. The recorded successful prefreeze fetch and its empty-log hash were inspected; the reviewer did not independently contact the remote. An unchanged-upstream fetch remains mandatory before normal push.

Current OpenSpec list/apply reports 7/14 tasks, with task 4.1 and all remaining product acceptance tasks open. All contextFiles returned by apply were read. The accepted design section defines only controller-owned paired service plus atomic returned-audio render reconfiguration. The five source/test paths, design addition, progress note and eleven portable evidence files comprise the complete candidate. No canonical specification, task checkbox, scenario fingerprint, dependency, native FFI implementation, or vendored code changes.

## Task-to-implementation matrix

| Accepted component responsibility | Actual implementation | Assessment |
|---|---|---|
| Owner-authorized Session preparation | `src/instrument-session/src/lib.rs::prepare_render_configuration_owned` changes visibility and documents retained owner/revision; existing prepare/commit implementation is reused | Intentional public owner API, no new editor authority or Session semantics |
| Controller route to paired quiescent service | `src/trigger-host/src/parameters.rs::ParameterController::service_quiescent` forwards to the private runtime pair | No job collection, note expiry, scalar settlement or document-generation policy added |
| Ordered preflight and explicit no-op | `ParameterController::reconfigure_quiescent`, `runtime.rs::validate_endpoint`, `configuration`, `activation_available` | Foreign identity, bounds, Busy, then complete equality; no work consumed on rejection or equality |
| Prepare with retained document and context | Named mono-control/stereo-output buses from accepted metadata; owned Session candidate with new rate/block settings | Matches initial host bus convention; accepted YAML and aliases remain unchanged |
| Atomic runtime installation | `runtime.rs::QuiescentControl::activate` and `QuiescentLease::publish`, existing Session commit | All construction/probing/layout/event-capacity failures precede Session mutation; publication is infallible |
| Retain persistent ownership and policy | Replace only current processor and control configuration; reuse `adopt_editor_intent` | Queues, slots, atomics, epoch, scalars, controller identities/history/gestures and note-policy records retain their owners |
| Regression and failure evidence | New host `tests/render_reconfiguration.rs` and Session `tests/render_configuration.rs`, existing owner suites | Ten public host tests and three Session characterizations; existing Session foreign/stale/activation rejection coverage retained |
| Design and readiness reporting | Accepted design section; migration progress; portable receipt, freezes, completion/coverage/reuse records, archive and inventory | Claims confined to this component; full product gates expressly deferred |
| Completion/publication discipline | Hash-bound packet plus five exact metadata outputs and same-reviewer final gate | No source, tests, requirements, raw evidence or task status can change inside the envelope |

## Criterion-to-test matrix

Host test names below are in `src/trigger-host/tests/render_reconfiguration.rs`; Session names are in `src/instrument-session/tests/render_configuration.rs` unless stated otherwise.

| Criterion | Executable assertion / evidence |
|---|---|
| Paired service preserves nonzero settlement/epoch and does not service structural jobs | `paired_controller_service_preserves_nonzero_restore_epoch_and_ui_settlement_without_servicing_jobs`: actual restore, revision 2 distinct from epoch 1, settled sequence 1, queued sequence 2 and subsequent sequence 3, retained feedback offsets/values, ready job still uncommitted, signed mapped output |
| Foreign endpoint precedes invalid bounds and Busy; instances unaffected | `foreign_identity_and_invalid_bounds_precede_busy_without_consuming_structural_work`: rate 0, block 0 and 16385, both crossed pairs, queued job survives, independent DSP continuation has known signed samples |
| Queued, preparing, ready, pending-runtime and retired-runtime ownership remain Busy even for equality | `structural_and_runtime_ownership_busy_requires_separate_service_even_for_equal_configuration`: each state asserted, preparation subsequently taken, separate publication and reclamation required before Unchanged |
| A completed real worker result is not implicitly collected | `completed_worker_result_remains_busy_and_publishes_once_only_through_separate_job_service`: wake originates on another thread; reconfiguration/service preserve running job; explicit service publishes once; new-rate audio and later Idle verified |
| Equal settings preserve active DSP and do not touch resources | `equal_configuration_continues_real_dsp_without_resource_io_allocation_or_destruction`: delete sample after initial frames; equality succeeds with zero allocation/deallocation; subsequent frames continue the known waveform |
| Changed rate/block settings are coherent with subsequent Session work | `changed_configuration_publishes_session_and_runtime_together_preserving_history_epoch_and_automation`: Changed result, 32-frame rendering, 33-frame rejection/zeroing, independently derived 44.1 kHz samples and continuation, real later undo publishes using the same new settings |
| Authoring state, outcomes, request identities, descriptors, gestures/Ends, feedback, values and cancellation cause survive | Same changed-configuration test: revision 4/history 1+1, recovery outcome, next native request, generation 5, settled cancelled sequence 2 versus queued 3, feedback sequence/offsets, exact Begin/physical Value/End queue, next sequence 4 and actual undo are asserted |
| Retained event capacity rejects unsupported shrink atomically | `retained_event_capacity_rejects_shrink_before_commit_and_keeps_old_dsp_and_queued_value`: E=65 at block 130 rejects 128 and 16, snapshot/value/automation preserved, queued scalar subsequently applied to known continuation, old 130-frame rendering remains valid |
| Missing/corrupt resources preserve loaded runtime and old settings | `missing_or_corrupt_resources_leave_the_loaded_runtime_and_configuration_accepted`: both faults return Session PreparationFailed, snapshot/value equality, known active DSP continuation, old configuration remains Unchanged |
| Fresh DSP resumes only live editor intent; authentication, press generation, watermark and activity survive | `changed_configuration_resumes_only_live_editor_intent_and_retains_press_generation_and_activity`: gated sample prefix/tail, held/released/detached/reset cases, exact same admitted onset rejected as ReplayedEditorNote, foreign release rejected, original 1999/2000 ms expiry boundary preserved, new press/release remains usable |
| Actual ownership return, control destruction and host pitch arbitration | `returned_audio_configuration_preserves_host_pitch_arbitration_and_destroys_old_owners_on_control`: audio moves to a real thread and returns before reconfiguration, control allocation with negative live-allocation delta, later callback remains allocation/destruction-free, host-held pitch blocks editor resumption until real host release |
| Session Busy and resource edges characterized before visibility change | `render_preparation_while_a_native_job_is_pending_preserves_accepted_audio`, `failed_render_resource_preparation_preserves_the_loaded_instrument`, `render_commit_cannot_consume_or_publish_over_an_admitted_native_job`: exact errors, pending request retained, snapshot unchanged and signed loaded audio |
| Existing Session candidate ownership/revision and activation rollback | Existing `tests/session.rs::render_reconfiguration_preserves_accepted_document_history_and_failure_rollback` covers foreign/stale candidate rejection, successful history-preserving commit and rejecting activation host |
| Ordinary queued-note FIFO retained | Existing runtime-quiescent tests remain in focused/full host runs; inspection confirms the new operation never replaces or drains the event endpoint. No fictitious controller note-injection seam was added |

Every render helper measures actual `process_events` callbacks and requires a zero `AllocationInfo`. The signed audio checks are independent of the implementation under review: the packaged WAV is PCM16 mono at 48,000 Hz; directly decoded frames begin -16384, -358, 1554, 4137, and frames 31..33 are 11539, 13385, 19805. These reproduce the asserted 48 kHz continuation, 160/147 fractional positions at 44.1 kHz, and later continuation interpolation. Multi-frame, offset, gain and post-undo checks prevent a common-first-sample or two-wrong-paths comparison from passing. The gated fixture proves lifecycle release against a sustained signed tail.

## Acceptance evidence and test effectiveness

All 807 archive members independently match the complete `author-artifacts.json` byte counts and SHA256 inventory; the outer archive hash is `a0dba83ad6bdfb3b9ce3e19d1fc9b0ff97cd2a02bea6eb17ce0f2b9186fe1f08`. Every portable SHA256SUMS entry matches. Gate receipts bind command, working directory, base HEAD, source hashes, outcome, timing and resource observations. Final host source hashes match current files. All nine explicitly reused Session inputs match; the sole recorded final-Session-run/current-source difference is the later host-test-only watermark assertion, outside that owner. The 21 baseline reuse inputs also match the base commit exactly.

The raw history contains genuine absent-public-API REDs (E0624/E0599) and five changed-configuration assertion failures against the temporary Unchanged implementation. Before visibility changed, the three Session Busy/resource/commit-Busy edges were characterized and run with the full 33-test owner suite. Final GREEN is separate from the disclosed corrections to normalized-versus-physical automation, supported undo, press generation, invalid-block zeroing, control allocation accounting, formatting and command-launch setup. The later worker/oracle and same-request watermark assertions are honestly recorded as immediate-PASS strengthening, not fabricated REDs.

| Final gate | Retained outcome and elapsed wall time |
|---|---|
| Host ordinary tests | 138 tests plus 2 compile-fail doctests, 3.018 s |
| Host owning LLVM run | All 138 tests, 29.141 s |
| Host fmt / all-target Clippy -D warnings | Both pass, approximately 0.504 / 0.505 s |
| Session ordinary tests | 33 tests, 5.531 s |
| Session owning LLVM run | All 33 tests, 8.040 s |
| Session fmt / all-target Clippy -D warnings | Both pass, approximately 0.505 / 1.008 s |
| Strict scoped OpenSpec / all canonical specs / spec map | Pass; 51 canonical specs; map 618 scenarios, 379 tested and 239 explicit todo entries |
| Cached and unstaged whitespace checks | Pass |

The new final host test executable completes its ten tests in 0.18 s ordinary / 0.17 s instrumented; the new Session group completes in 0.04 s. Real-worker waits have a bounded failure timeout, not a timed sleep used as a success oracle. Tests use public outcomes and retained ownership, with helpers localized to their test files. No test-only production authority, dependency or extraction was introduced. Full owner runs supplement focused runs. There is no measured per-test outlier requiring new lane policy in this bounded change; existing suite-wide lane/performance work is not claimed complete.

Twelve actual host executable paths and three Session executable paths in verbose run logs match the proof's objects and recorded binary hashes. The 84 host and 80 Session fingerprint files hash correctly. Raw-profile hashes match their archived bytes. Independently comparing archived original merged profiles to retained remerged profiles gives exact byte equality for the appropriate owner: host `77324cd55b85953ad8a3f42328e9bf2ae705a31a55605e06d4b48f7f885a1547`, Session `9d27017b701ed3339355be1a15580e38e5564f3e59737df09f186c4306350a10`. Direct-export and Cargo-export paths, segments, functions, regions and integer counters compare exactly after removing only derived percentages and Cargo metadata. The disclosed percentage differences are floating presentation differences around 1.42e-14. The reviewer inspected these retained outputs and proof procedure, rather than rerunning compilers or LLVM commands.

Coverage is correctly qualified. Runtime owner lines are 1222/1228 and regions 1932/1945; parameter owner lines 876/897 and regions 1310/1352; Session lib lines 672/676 and regions 819/826. Session native_ffi gaps remain separate and unchanged. The new runtime selected regions are 45/46; controller selected regions 66/68. The uncovered constructor propagation exit and two Session-error arm regions are necessary defenses under broader APIs, but the same-owner exclusive transaction has already validated bounds, retained accepted YAML/layout, and unchanged owner/revision/idle state. Resource failure before commit and actual capacity rejection are publicly executed and asserted. No claim of 100% overall, branch, MC/DC or mutation evidence is made; mutation is explicitly excluded by the user. These disclosed limits do not hide a missing accepted behavior assertion.

## Architecture, operational safety, and alternatives

The implementation maintains the dependency direction: Trigger host depends on Session and engine; neither engine nor Session gains GUI/CLAP/JUCE ownership. Exclusive mutable controller/audio borrows make the returned-audio boundary explicit. Private Arc identity prevents mixing independently valid endpoints. The controller remains control-thread-affine, while RuntimeAudio can cross real thread boundaries. The worker owns preparation, not authority to mutate the accepted session.

The preflight sequence cannot reclaim, wait for, cancel, or collect work. The equal branch precedes candidate preparation. Changed preparation retains the accepted document root and macro roots, reruns normal resource preparation, and derives equivalent named buses. `QuiescentControl::activate` constructs and probes a complete PreparedRuntime, checks retained layout and E <= budget/2, then creates a lease. Existing Session commit changes accepted preparation/context/settings only after activation returns successfully. Lease publication changes control configuration and current DSP, applies the existing editor-adoption law and drops the previous processor on control. No queue, slot, scalar atomic, epoch or controller-policy replacement occurs. Accepted state replacement and its predecessor destruction also stay on control. Safe Rust ownership and non-panicking ordinary publication operations supply the transaction boundary; allocation failure/process abort is not represented as a recoverable host error.

A simpler-looking endpoint reconstruction would discard persistent queues/policy and break the specified identity contract. A fake render to service ownership would consume events and advance DSP. A post-commit fallible runtime install would break rollback. A second public candidate accessor or injected failure seam would expand authority solely for tests. The small ActivationHost/Lease adapter reuses the existing Session transaction and editor adoption helper and is the clearer in-scope choice. Shared preflight helpers beyond the small endpoint check would add indirection without removing a materially repeated policy. No further extraction is justified by this change.

There is no new secret, persistence format, unsafe block, external capability or dependency to audit. Actual CLAP activation/retry, float-to-integer host conversion, stopped flush, outbound automation, source-free release/CMake/native/browser/DAW/installed acceptance remain explicit later gates. Historical product binaries are not treated as current-source evidence. Resource receipts retain disk outputs and the fixed disk/memory floors; caches are handed back, preserved, and not part of cleanup authority.

## Reusable lessons and publication soundness

No new standing guidance change is required. The useful lesson already appears in this candidate's qualification history: whole-owner coverage must be bound to executed binaries/source and independent observable values; numeric coverage alone does not establish a preserved nonzero epoch/watermark or correct resampling. Proposed owner would be the existing coverage/evidence workflow; that workflow already states this rule, so adding another rule would duplicate it and increase maintenance cost. Disposition: deduplicated, no new guidance or memory written.

The packet's exact five-output envelope is sound: add this review and verdict under portable `review/`, replace only receipt.review, replace one exact progress sentence, and recompute portable SHA256SUMS in bytewise order excluding itself. No raw source/test/design evidence changes or task/spec synchronization are approved. The expected final changed-path count is 20. `approved-envelope.json` binds each resulting mode, size, SHA256 and Git blob plus the complete expected final manifest digest.

The publication helper inspection shows exact-tree transfer checks, normal hooks at `.githooks`, unchanged-upstream fetch, clean-lane fast-forwards and no main integration/cache cleanup. Pre-commit runs the normal Rust coverage driver; pre-push runs Rust tests and only mutates engine files selected by its engine-scoped diff. This candidate changes none, so the user's mutation exclusion requires no bypass. The proposed negative empty-integration and positive exact-tree preflights must occur before hooks. The reviewer does not treat their future success or hook test counts as already observed.

Before any hook, return the actual entire final manifest/tree and named final validation receipts to this same reviewer. The reviewer will compare every path to this candidate plus the five approved transformations and produce the requested final-precommit addendum. Any unexpected bytes, modes, paths, failed validation, upstream movement or publication rejection invalidates this PASS. Main must remain clean at the pinned commit. No merge to main, destructive cleanup, task closure, requirement synchronization, or archive operation is authorized by this verdict.

All seven required dimensions were assessed: task/spec scope, acceptance evidence, effectiveness, architecture/operational safety, materially better alternatives, reusable lessons, and reproducible publication. **Actionable findings: none.**
