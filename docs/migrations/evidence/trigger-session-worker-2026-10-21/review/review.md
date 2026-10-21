# Independent completion review: persistent preparation worker

Verdict: **PASS**. No actionable in-scope correction remains in the reviewed
worker component. This approves the exact candidate and finite publication
envelope below; it does not complete OpenSpec task 3.3 or the beta.

## Candidate identity and review independence

The reviewed worktree is `/tmp/dandrum-lanes/protocol`, branch
`lane/trigger-protocol-20261020`. Base, HEAD, intended upstream
`origin/feat/juce-free-host-adapters` and merge base independently resolve to
`ca36647a619034c96bc03f1ab2ffc4e166d356d6`. There is no configured tracking
requirement for this lane; the packet explicitly identifies its intended ref.

I independently reconstructed all 18 candidate entries from porcelain status,
index modes and stage-zero blobs, regular-file modes and worktree bytes. I
computed Git blob identities directly, compared index content with disk, and
serialized entries in bytewise path order. The exact manifest hash is
`cd2194fb540c7a043b0093255deb18720dd018595537cb17cd1738abc856b760`.
There are no unmerged entries, unstaged tracked changes or untracked files.
Main independently remains clean at
`68dd6553a2f07e71e2fe09ee1d7094d518559937`.

This is a whole-candidate reassessment following the preliminary source audit,
not approval inherited from that preliminary result. The two production/test
payloads retain these independently checked hashes:

| File | SHA-256 |
| --- | --- |
| `src/trigger-host/src/parameters.rs` | `c5d6c817940dd1255e74aac3a53094f7162e444c2a003cdd4ab3d0bef5081efb` |
| `src/trigger-host/tests/session_jobs.rs` | `95d3be564f52fc38156eb5c015d3299d5ce77fd0cef1424cff1faa0272e8eafa` |

Authority reviewed: shared AI Rules, repository AGENTS, completion-review
reference, coverage-and-mutation skill, accepted packet and current OpenSpec
proposal/design/tasks and both delta specifications. Review involved source,
artifact and Git read-only inspection; no builds, tests, browser/native runs,
delegation, repository/index/cache edits or deletion. The host-namespace fixture
check was an explicitly authorized read-only elevated operation.

## 1. Task and specification completion

| Task or boundary | Implementation and assessment |
| --- | --- |
| Persistent instrument preparation prerequisite of 3.3 | `start_session_worker` creates one named worker with capacity-one request/completion channels; installation is optional, explicit and independent of editor ownership. |
| Schedule before acknowledgement | `admit_session_command` admits through the native session, transfers its unique owned work, then encodes acknowledgement. A rejected transfer restores Queued ownership and reports unavailable. |
| Control-only publication | `service_session_jobs` drains at most one result and uses existing bounded reclamation/activation. Worker captures no controller, session or runtime reference. |
| Constructing-thread ownership | `PhantomData<Rc<()>>` prevents safe cross-thread movement/sharing of the controller. Actual host-main construction remains a native facade obligation. |
| Preserve manual and bounded behavior | Worker mode disables manual take; startup refuses an active manual job. Native single-active admission remains authoritative for duplicates and Busy. |
| Capacity continuation | Ready stays owned on Busy without self-wake. The real audio swap report supplies the future facade's rescheduling signal. |
| Permanent joined shutdown | Request disconnection precedes join; result drain follows join; worker field destruction precedes session/runtime owners. Both explicit shutdown and Drop use the same operation. |
| Documentation/evidence | Design describes installed mode and its limits. Progress and portable receipts accurately separate ordinary, LLVM, interrupted release and successful release evidence. |
| Scope discipline | Tasks 3.2/3.3 and later integration/acceptance remain open, 7/14 checked. No task checkbox, canonical scenario or spec map changed. |

The full delta scenario involving a real lost WebView, reconnect and native undo
requires later joined acceptance. The component's tests do not prematurely
claim that scenario. No native scheduling, notes/heartbeat controller, Wry/CLAP,
VST3, standalone, named-DAW or installed/offline product completion is approved.

## 2. Acceptance evidence

The following matrix is based on source assertions and real dependencies,
not test names alone. Test names refer to `tests/session_jobs.rs` except the
compile-fail law in `src/parameters.rs`.

| Criterion | Executable evidence and asserted outcome |
| --- | --- |
| Editor-independent completion and control-only publication | `instrument_worker_finishes_after_last_editor_detaches_and_only_main_publishes` observes a distinct worker thread, old signed audio and native revision/history after wake, then new exact audio/key silence and one history/generation advance only after service. Reconnect sees the retained outcome; another instance stays unchanged. |
| Unique dispatch and real capacity deferral | `worker_dispatch_is_unique_and_real_audio_swap_reschedules_a_busy_completion` repeats the same native job, rejects another active request without cursor advance, holds Ready as Pending, observes an allocation-free real swap and forwards that report to service. It asserts no self-wake and the second signed mapping transition. |
| Manual startup compatibility | `worker_start_cannot_steal_an_already_admitted_manual_preparation` rejects startup without consuming manual work; manual completion retains the expected cursor and signed audio behavior. Nine inherited manual tests cover undo, failures, identity rejection, outcomes and gesture preservation. |
| Controller affinity | `ParameterController` compile-fail doctest genuinely compiled in RED before the marker; GREEN rejects the move with E0277 naming `Rc<()>`. Inherited completion move-once law remains. |
| Idle shutdown | `idle_worker_shutdown_is_terminal_idempotent_and_preserves_native_cursor_and_audio` calls shutdown twice, observes wake disconnection, unavailable service/admission and unchanged native cursor, history, revision and signed audio. |
| Occupied teardown | `instrument_shutdown_joins_occupied_completion_wake_before_returning_or_dropping` covers explicit and Drop paths, holds the worker after actual result insertion, releases it after observing withheld control completion, and observes resource destruction before return. Explicit stopped status is unavailable; another instance renders unchanged audio. |

The wake barrier deliberately blocks only in the test. It is not a production
scheduler and does not establish cancellation of arbitrary file I/O or paused
preparation. Its bounded 40 ms negative wait is scheduling evidence; source
inspection additionally establishes teardown order. Native scheduling calibration
is expressly deferred.

Retained TDD records show nine baseline tests; missing worker API E0405 RED;
ten-test GREEN; duplicate/Busy/manual characterization accurately labeled as
such; real move-law RED/GREEN; missing shutdown E0599 RED; fourteen-test GREEN.
An implementation compilation mistake is not mislabeled RED. Each behavior
cycle retains formatting, strict Clippy, coverage and refactor decisions.
The sender-disconnect-before-panic-wake repair was reverified, with panic itself
still unexecuted. No module extraction occurred.

## 3. Test effectiveness and coverage

Tests exercise the actual kit, native session, preparation and runtime. They
assert literal signed stereo `-0.5`, complete silence at unmapped notes, exact
revision/history/cursor changes, independent instances and callback allocation/
deallocation sentinels. These oracles protect output correctness rather than
only relationships between two potentially wrong renders. Private completion
identity and unique ownership are covered by inherited behavioral and compile
tests. Helpers preserve readable contracts without exposing fabricated internals.

Final ordinary evidence passes 107 runtime tests and two compile-fail doctests;
full-owner LLVM passes the same 107 runtime tests. All are unfiltered and none
ignored. Formatting and all-target Clippy with warnings denied pass. I verified
their source hashes against the current candidate and checked raw results and
the original LLVM export, not only curated summaries.

Parameters summary coverage is 71/71 functions, 660/680 lines and 1043/1082
regions. The 168 emitted mangled records, of which 99 execute, are generic
instantiations rather than 168 distinct source functions. There is no branch or
MCDC instrumentation. Coverage establishes execution, not assertion strength.

New uncovered groups are join failure/retained error at 201, panic notification
263–264, result-channel invariant failures 280–286, OS spawn failure 291,
request-channel rejection 680–689, completion rejection 743 and disconnection
746. Older gaps at 385, 760 and 768–774 are separate pre-existing defenses.
Spawn and internal panic are real possible failures, not declared unreachable;
their handling is inspected, not runtime-proved. Safe channel/seal invariants
are justified below. This is a qualified core-coverage result, not a blanket
100% execution claim, and the residual failure-path measurement limits are
recorded honestly. Adding arbitrary state-corruption or executor/panic seams
solely to cover those lines would weaken this small ownership boundary.

Session suite timings remain subsecond in ordinary and release evidence;
release test bodies total 0.29 seconds, doctests 0.07 seconds. LTO/setup dominates
the 382.727-second release command. Individual medians were not collected;
no test-lane reclassification is claimed. Mutation was explicitly excluded by
the task. No mutation score or hidden exclusion is substituted for evidence.

## 4. Architecture and operational safety

Native admission permits only one active structural job. Duplicate Pending
does not create another preparation; another ID returns Busy. Preparation and
completion fields are private, and the per-job Arc identity cannot be fabricated
through supported safe calls. Manual take cannot obtain installed work. These
facts explain why request/result Full and wrong-seal worker completion are
invariant defenses rather than ordinary unchecked public input paths.

On request-send failure ownership returns to Queued and no acknowledgement is
invented. An undeliverable result is retained in the worker's join result rather
than overwritten. The sole result receiver remains owned until after join, so
normal teardown does not race receiver destruction against a worker send.
The worker does not publish or destroy session/runtime owners.

On internal panic the exit guard drops the sender before waking service.
Therefore service can observe Disconnected even before `is_finished` changes.
A nonpanicking, nonblocking, future-only wake is an explicit contract; a wake
that violates it is not a supported native scheduler. OS spawn failure happens
before installation/native admission. An installed stopped worker remains
distinct from manual mode, rejects later admission before cursor mutation and
cannot be restarted. Active unavailable status avoids a fabricated terminal
native outcome. The accepted document/audio remain intact.

The two standard channels are off audio. Main uses nonblocking queue operations;
audio behavior remains in the existing bounded runtime. There is no new native
GUI dependency, unsafe code, external authority, persistence format or DSP
algorithm. The future facade must retain callback authority until worker join
and construct control on actual main; this component does not claim those facts.

## 5. Better alternatives

Two bounded transfer channels and one private lifecycle owner keep ownership
explicit. A mutex/condition-variable slot would require additional manual state
and wake synchronization; per-job threads would complicate persistent ownership;
a runtime thread-ID assertion adds mutable state where a zero-sized compile-time
restriction suffices. No materially simpler, safer or cheaper alternative was
identified for this scope. Extracting the small worker into a new module would
add a coverage burden without improving a currently independent contract.

## 6. Reusable lessons

The exit notification race is already documented in the code and retained
refactor record: disconnect the observable completion endpoint before notifying
about failure, because a join handle may still appear running. The narrow owner
is this worker lifecycle boundary. Disposition: already applied locally; no
additional standing guidance or memory change proposed. A global rule would
overgeneralize an implementation-specific ordering concern.

No additional systemic finding emerged. Keeping explicit interrupted-run and
current-run provenance is correctly implemented, not an outstanding correction.

## 7. Publication soundness

All three archive hashes and every member size/hash independently match their
inventories: author 425, interrupted source-free 16, successful source-free 19,
460 total. Portable SHA256SUMS matches the complete current evidence directory.
The author archive includes its 424-artifact manifest; early source capture
omissions are disclosed and final captures cover all host Rust source/tests.

The successful source-free receipt and raw log independently establish 107
runtime tests across nine binaries plus two doctests, exit 0, no failures,
ignored or filtered tests. Actual compiler lines show release opt3/LTO/codegen1
and seven relinked test units; engine/session remain Fresh. The normal dependency
audit exits 0 and its parsed 112 distinct crate names include no Clack, Wry,
GTK, WebKit or JUCE. This is a host Cargo/source boundary proof, not a fresh
whole-CMake or native acceptance run.

I independently matched all nine current executed binary hashes, sizes,
depinfo hashes and fingerprint JSON to records and actual Cargo Running lines.
The editor-notes depinfo lacks a manifest-env line; its recorded fingerprint and
actual Cargo invocation supply the disclosed binding instead. All 590 recorded
source hashes match the current integrated repository. In the actual host
namespace all 590 fixture hashes and retained mtimes also match; web/ui/
third_party are absent. The before/after root and fixture maps are identical.

Execution occurred at `d2ef616377ce34b446cb0dd1f5d0a3a4bd10c1c8`. I independently
examined its ancestry/diff to `ca36647`: only frontend bootstrap and related
documentation/evidence changed, no headless input. Evidence reuse is therefore
explicitly source-bound across that integration, not a claimed repeat run.
The earlier release attempt stopped before tests at the fixed 1 GiB floor;
its cache inventory is not passing evidence. Successful resource samples stay
above every fixed floor. Process handback records show no owned survivors.

The approved debug-cache deletion and source-bound ordinary records remain
distinct from retained release binaries. This review authorizes no cache cleanup.
Five candidate validation receipts/log digests independently match: scoped and
51 canonical strict OpenSpec checks, spec coverage 618/379/239, and staged/
unstaged whitespace checks. No affected canonical criterion was prematurely
mapped or completed. Existing work and publication context remain in the declared
deferred-work tracker; no new independent worktree or main change is authorized.

The packet's final operational update excludes an active frontend follow-up in
`web/trigger/engine/native-engine.mjs` and
`web/trigger/tests/native-engine.test.mjs`. I checked that frontend status names
exactly those two paths while this protocol candidate remains unchanged. These
bytes are not reviewed or authorized for publication here. Do not fast-forward
that lane while its author is active. Initial commit/sync is limited to
integration, clean feature/runtime and exact-staged protocol. Only after the
frontend author freezes may root inventory and hash every dirty byte, require
exactly those two disjoint paths, fast-forward the verified worker commit and
verify all dirty bytes unchanged. Any other state stops that lane sync for
inspection. This does not authorize a frontend commit or publication.

The actual configured hooks path is `.githooks`. Pre-commit invokes the Rust
coverage gate; pre-push runs engine tests and only selects changed engine Rust
files for mutation. No engine file changes here. Normal-hook results remain
future publication gates, not tests this reviewer claims to have executed.

## Exact finite post-review envelope

Only five metadata outputs are approved, with exact resulting mode, byte count,
SHA-256 and Git blob identity in adjacent `approved-envelope.json`:

1. Copy this exact review and adjacent verdict into the portable evidence
   `review/review.md` and `review/verdict.json`, both mode 100644.
2. Replace only `receipt.json`'s `review` object with PASS, initial manifest hash
   and these exact review/verdict hashes, using indent-2 JSON plus final newline.
3. Replace the one specified pending-review progress sentence with the exact
   passed-review/normal-hooks-pending sentence in the packet.
4. Regenerate portable SHA256SUMS over every other file in bytewise relative
   order, two spaces between digest/path and final newline.

These are exactly five changed metadata files, two newly added, resulting in
20 candidate paths. No source/test/design/raw-evidence/task/scenario edit is
approved. The envelope independently computes the resulting manifest hash.
Root must return the complete actual final candidate for this same reviewer's
precommit addendum and run the named scoped/canonical/spec-map/whitespace/clean
index checks. Exact reviewed-tree transfer uses the calibrated fail-before-hook
guard, followed by normal commit/coverage and normal feature push hooks. Fetch
must still identify the recorded upstream; no merge/rebase or force push is
approved. Already reviewed commit fast-forwards to existing lanes are allowed
only under the clean/exact-staged and frozen-frontend preservation rules above.
No main merge or cache cleanup is included. Content drift, upstream movement,
failed validation/hook, push rejection or unexpected generated content invalidates
this PASS and requires reassessment.
