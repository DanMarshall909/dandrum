Author handoff: quiescent runtime task4.1 prerequisite

Two author-owned paths only: src/trigger-host/src/runtime.rs and new tests/runtime_quiescent.rs. HEAD28caf6ca02861bebdf1a2e09e49c762682ed1cce, no Git/index mutation. Root owns the separate dirty accepted design.md and all tracker/spec/docs/publication. Exact statuses, canonical Git modes, filesystem modes, content hashes and index entries are in author-freeze.json and its external SHA receipt. This is a candidate input for review and publication, not author self-approval or task4.1/beta completion.

RuntimeControl::service_quiescent exclusively borrows the matched RuntimeAudio, checks private admission Arc identity before mutation, reclaims/drops at most one retired owner, directly adopts/drops at most one pending owner, and restores capacity immediately. QuiescentReport distinguishes adoption and destroyed predecessor count. Repeated idle calls do nothing. Shared AudioState::adopt_editor_intent preserves the active callback's existing snapshot timing and applies the same gate/seen law to quiescent adoption. Scalar/settlement/epoch, ordinary queues, configuration/layout, host arbitration and reset suppression are untouched.

Criterion-to-test map:

- Paired pending adoption, control destruction, real Session acceptance unchanged, first signed graph and second real publication without render, idle no-op: quiescent_adoption_destroys_the_predecessor_and_restores_capacity_without_a_render.
- Failure atomic private identity and independent audio with both instances nontrivial/pending, plus both retirement slots occupied: foreign_quiescent_pair_is_rejected_before_either_nontrivial_instance_changes.
- Actual separate audio thread allocates/deallocates nothing during active swap, returns owner to original control thread, service destroys the actual retirement off audio, capacity returns, idle preserves a signed active tail: returned_audio_reclaims_a_real_retirement_on_control_and_accepts_another_publication.
- Held heard/unheard editor intent resumes only when a real block runs: quiescent_adoption_resumes_a_still_held_editor_note_only_on_an_actual_block.
- Heard/unheard released/detached/cancelled/reset onsets cannot replay, a new press remains usable: quiescent_adoption_cannot_replay_a_released_cancelled_or_reset_editor_onset.
- Host-held ownership survives quiescent graph adoption; no new host voice resumption, later host release permits editor intent under existing law: quiescent_adoption_retains_host_pitch_arbitration_until_a_real_host_release.
- Copied real product resources, applied values/nonzero-offset feedback/full ordinary queue survive, later remapped note and queued scalars produce known signed stereo: quiescent_adoption_preserves_applied_values_feedback_and_queued_scalar_and_note_events.

TDD/refactor evidence: 01-paired-api-red has explicit absent API/type/variant RED; 02-retired-red has genuine actual-retirement count assertion RED; 03-editor-red has two independent held-silence and released-tap replay signed-PCM assertion REDs. Minimal GREEN, strict all-targetClippy, focused LLVM and production/test refactoring complete before the next criterion. 01/02/03-assessment.md record each loop. Preservation tests pass without production change and are labeled characterization in04-assessment.md. Initial python launch failure, RAM target resource stop and first editor implementation's omitted fixed-array length are disclosed setup/implementation failures, never RED/PASS evidence. No extracted production module, engine seam, fake private state, unsafe code, new dependency, controller/session/native edit or mutation run.

Final unchanged-source verification:

- 05-format-check: pass.504s.
- 05-full-host:114ordinary tests+2compile-fail docs pass14.586s.
- 05-session-regression:30tests pass6.034s.
- 04-clippy: all targets -Dwarnings pass.508s.
- 04-coverage:57focused owner tests pass8.546s.
- 05-full-coverage:114ordinary host tests pass15.581s, verbose actual compiler/test/profile/LLVM commands retained, LCOV retained.
- 05-json-report: same raw profiles JSON export pass.505s.
- 06-timings:3comparable exact reruns per test; all7 pass. Peer median.0966s/max.2505s. The six-real-preparation no-replay matrix is the only >1.5x candidate; its cost remains below1second. Preserve existing trigger-host-fast membership; this owner's slow selection is explicitly empty.
- 07-profile-proof:10archived raw profiles remerge to byte-identical fab785637b7ab32df868f4b22a0e8cb68a2d94c673f9c62681bfb423ea3edf61 profdata; direct export reproduces exact paths/segments/functions/regions/integer counters. Original JSON and direct JSON are retained; only derived percentage presentation/cargo_llvm_cov metadata are excluded from semantic comparison. All10actual executed binaries SHA checked; host fingerprint files retained. No source/cache restore or deletion.

coverage-qualification.json and qualify.py give reproducible accounting:48/48 new service/closure/shared-helper normal code regions execute.97/97 runtime sourcefunctions and1150/1150 distinct LCOV DA line coordinates execute, including the existing cfg(test) module. Reported LLVM summary remains1180/1186lines,1888/1900regions,162/170instantiations. Production coordinate merge has1007/1008regions,73/73functions; unchanged apply_event '?' error-propagation region at runtime.rs932:78-79 is not executed under public budgets. No branch/MC/DC or blanket100%owner claim. Externally observable assertions, not coverage percentages, support the behavioral claims.

Explicit cut line: nonzero UI settlement and restore epoch are structurally preserved but not publicly observable with a RuntimeControl/RuntimeAudio pair, because ParameterController owns its endpoint privately. Root accepted this observer gap: later controller-owned quiescent route must prove nonzero applied/admitted UI settlement and nonzero restore epoch. No test seam/counter/API was added to manufacture evidence. No concrete controller/wake, rate/configuration, stopped-flush, CLAP/native/reactivation, copied binary or installed product acceptance is claimed. Task4.1 and overall editor/beta remain open.

Resources: use ordinary /tmp/dandrum-lanes/cache-runtime and LLVM /tmp/dandrum-lanes/cache-host-snapshot-cov only. All Cargo target/profile/TMPDIR outputs resolve to disk. Prior profiles are archived before each managed coverage invalidation. Disk1GiB/Mem4GiB floors retained; unused shared RAM sampled explicitly. Earlier RAM-target stopped attempt is preserved and never resumed or cleaned. All cache/dependency/evidence state retained. Root's final combined integration/review/publication is pending, and root owns any next cache lease.
