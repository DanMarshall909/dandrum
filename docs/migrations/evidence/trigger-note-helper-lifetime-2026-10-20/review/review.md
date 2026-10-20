# Independent completion review: frontend note-helper lifetime

Verdict: **PASS**. No in-scope correction remains in the frozen 13-path candidate plus the finite metadata/publication envelope. This approves a production frontend prerequisite of OpenSpec task 3.3, not completion of task 3.3, the editor, CLAP, or the full beta. The prescribed same-reviewer final-envelope addendum remains a pre-commit gate.

## Identity and independent reconstruction

Reviewed worktree `/tmp/dandrum-lanes/frontend`, branch `lane/trigger-frontend-20261020`. HEAD, base, integration base, explicit upstream `origin/feat/juce-free-host-adapters`, and integration lane independently resolve to `bb2c631efd63f19b93183f7e0b402bee94ab9e34`. Initial manifest SHA256 is `ca1bfc626ea72dc53fce0a2198cf92db354c49465852a56a63b099625a4c9a96`. I independently compared the complete status, every candidate file's bytes, SHA256, Git blob, stage and canonical 100644 mode. Exactly 13 paths are staged; there are no unstaged, untracked, special-file, or unmerged candidate changes. SHA256SUMS independently reconstructs exactly.

The only production source is `web/trigger/engine/native-live.mjs` (SHA256 `48a9c5c3e410e7e85bdc5bd66efd6d5f4602929a10c490d730f0fbcbb64c3eb9`); the only test source is `web/trigger/tests/native-engine.test.mjs` (`71fd785bce9da4ee62e98e27d809d97232000171249688f4abb78062d16e0be6`). The remaining 11 paths contain evidence or progress documentation. The final 15-path manifest and exact five metadata outputs are in the external `approved-envelope.json` beside this report.

I read applicable AGENTS.md, shared AI Rules, the independent completion-review reference, coverage-and-mutation and test-lane guidance, behavior-first test-quality criteria, reflection guidance, accepted proposal/design/tasks and both delta specs. The authoritative deferred-work item `DD-20261014-JUCE-FREE-HOST-ADAPTERS` in `/home/dan/code/dandrum/.git/codex/deferred-work.md` records the existing parallel same-goal lane and this two-source-path prerequisite. No new worktree or independent project was created. The existing dirty protocol/job lane and its incomplete combined verification are excluded.

The original feature, integration, and runtime lanes were independently clean at bb2c631; main was clean at `68dd6553a2f07e71e2fe09ee1d7094d518559937`. The published d166250-to-bb2c631 change contains Rust, design and evidence changes, with no web input changes. All 265 inputs in the author's final web manifest still match. Baseline source hashes and the retained baseline NativeLive snapshot match published d166250; final source snapshots match the staged candidate. No fetch, source/index/Git/cache edit, build, test, browser/native execution, delegation or deletion was performed by this reviewer. Only the three authorized external review files were written.

## Task-to-implementation matrix

| Accepted task/boundary | Implementation | Assessment |
| --- | --- | --- |
| Task 3.3 prerequisite: same instance/attachment retains held-note ownership across mapping, undo and generation refresh | NativeLive.createNotes snapshots parsed scalar instanceId/attachmentId; changed replaces the helper only when either changes | Implemented and bounded to frontend state |
| Original press generation and delayed On/release ordering survive refresh; next fresh press uses current generation | Preserving the existing createNoteAudition instance preserves its pitch slots and pump; unchanged NativeEngine.noteOn supplies current generation for each fresh press | Implemented without a second state machine |
| Heartbeat remains independent of ordinary document refresh | Existing timer addresses live.notes; retaining that helper preserves its desired-note map | Implemented; native heartbeat expiry remains future joined proof |
| Current note failures remain visible while retired/closed failures stay suppressed | Error closure compares actual current helper identity; unchanged NativeEngine.report/emit closes publication after engine close | Implemented with existing close guard |
| Existing changed-instance/attachment fresh-helper behavior | Scalar mismatch creates a fresh helper; pending old helper error fails identity check | Preserved; no new old-attachment retirement policy |
| Task 3.2/3.3, CLAP lifecycle and beta completion | No checkbox/spec/canonical acceptance-map change; progress and receipt explicitly retain open obligations | Correctly not claimed; accepted task list remains 7/14 |
| Evidence and publication | Ten portable evidence files, one qualified progress addition, exact manifest and finite envelope | Reproducible within the stated limits |

## Criterion-to-test matrix

All new tests are in `web/trigger/tests/native-engine.test.mjs`. Test names below identify executable rows; their assertions observe NativeEngine and backend command/error events, not NativeLive private fields.

| Criterion | Executable evidence and oracle | RED / final |
| --- | --- | --- |
| Held note survives mapping 36→60, undo, then parameter generation refresh 7→8→9→10 and releases with press generation | `same-attachment mapping undo and generation refresh preserve held note until release`: literal On `[36,1,7]`, no premature Off, exact Off `[36,7]`, accepted rule/generation assertions | Actual missing-Off assertion failure / PASS |
| Close releases an already admitted held note after those refreshes | Same test row ending `held note until close`: exactly one matching Off and no later timer calls/listeners after repeated close | Actual missing-Off assertion failure / PASS |
| Release waits for delayed On admission after refreshes | Row `in-flight note until release`: deferred native promise, no Off before resolving admission, exact old-generation Off afterward | Actual missing-Off assertion failure / PASS |
| Close waits for delayed admission through the retained helper pump | Row `in-flight note until close`: same deferred admission/ordering oracle, repeated close/timer quiescence | Actual missing-Off assertion failure / PASS |
| Heartbeat persists through each same-attachment refresh | All four rows tick the controlled interval once after each of three refreshes, then assert exactly three noteHeartbeat calls | Part of four lifecycle rows / PASS |
| Fresh press after release uses current generation | Both release rows assert second On `[36,1,10]` and second Off `[36,10]` against literal arrays | Part of lifecycle rows / PASS |
| Current helper's asynchronous admission error remains visible after a generation change | `current note errors remain visible after same-attachment generation refresh`: exact failed patch-event message after deferred error | Actual hidden-error assertion failure / PASS |
| Close and either connection identity replacement suppress late failures, preserving fresh-helper semantics | Three `late note errors after ... do not publish editor failures` rows: close / instance replacement / attachment replacement; exact absence of failed events; replacement rows also prove noteOff on fresh helper cannot release old held pitch | Three baseline characterization passes / PASS |
| Existing production entrypoint, transport selection, absent-service presentation and generation-rendered header | Six existing registered `tests/browser/native-transport.spec.mjs` tests, actual production entrypoint with controlled backend | Six PASS; supplementary regression, not held-note/native-audio proof |
| Maintained bundles include actual changed helper | Both maintained Vite configurations with external RAM output/cache overrides; archived compiled artifacts contain captured IDs, helper error identity and changed reset comparison | Both PASS; build proof, not current-bundle Wry execution |

## Evidence and test effectiveness

I verified every member's path, regular-file type, byte size and SHA256 in both portable archives: 156 original members and 116 appended build members. Archive hashes match their inventories and receipt. Elevated read-only inspection of the real `/run/user` namespace confirmed all 272 retained artifacts still match those portable members. The short `/run/user/1000/fnl` alias is absent; inspection found no matching task-owned evidence worker commandlines. No cache removal is authorized or inferred.

Raw baseline TAP proves 40 focused passes. RED uses the unchanged published production hash and the exact final test hash: eight selected tests, five ERR_ASSERTION failures (four missing matching Offs, one missing error event), three retirement characterization passes, exit 1. These are behavior failures, not setup or compilation failures. Final focused TAP proves all 48 pass. Ordinary `npm test` proves 139 pass (Node driver 1.303532 seconds; wrapper 1.727554 seconds). `npm run check` passes TypeScript and the 70-authored-React-file check. Six registered browser tests pass (runner 7.1 seconds, wrapper 8.039217 seconds). Both initial browser cwd/setup and UNIX-socket-length launch failures remain separately retained and are not counted as product RED.

The regression oracles are meaningful: the five real old-code failures calibrate the changed behaviors; literal command arrays pin pitch, normalized velocity and old/current generations; delayed admission pins ordering; counts pin heartbeat persistence and duplicate cleanup. Tests tolerate a different private implementation because they do not inspect helper internals. The four lifecycle rows share setup but retain different admitted/in-flight and release/close guarantees; the retirement rows distinguish each identity input. The few explicit microtask flushes accompany controlled promises/timers, not wall-clock sleeps. Feedback is inexpensive at the owning frontend layer. There is no justified test removal or further extraction for this slice.

The existing fast driver remains `npm test`; the retained slow boundary is the selected registered browser file. No suite membership is changed from one timing sample, and no all-browser-suite or combined native acceptance claim is made. Both builds use the existing Vite 6.4.4 configuration with only owned output/cache locations changed, preserve configuration/dependencies/staged native bundles, and contain the production helper. Actual preview/native wrapper durations are 2.512681 / 2.513440 seconds. Native app.js is 808,594 bytes and emits the standard configured single-bundle size warning; this is disclosed, not a failed build or a new performance claim.

## Coverage and boundary calibration

I inspected the actual baseline and final V8 records against their source snapshots and offsets. Combining the two focused-process records yields 12 baseline unexecuted ranges and 11 final unexecuted ranges; the final ranges all exist in the baseline. The changed helper error callback becomes exercised, and both connection comparisons and preserve/reset decisions are exercised by meaningful cases. NativeLive's reported aggregate is 100% lines, 87.18% branches, 85.71% functions. Compact same-line branches explain why 100% lines coexists with unexecuted code; it is not complete assertion or whole-owner coverage.

Remaining ranges are inherited meter packet guards, waveform cache/validation/admission/staleness/result paths, waveform error publication, cancellation and swallowed cleanup error callbacks. They are outside the new note-helper behavior, remain visible, and have not been excluded or declared dead. No module was extracted. Mutation is explicitly excluded by the user; no mutation score or native fault-injection result is claimed.

The test backend is a stand-in for frontend argument, admission-order and failed-event contracts. It does not establish real native controller admission, emitted rejection/event wire calibration, heartbeat expiry, physical keyboard identity, signed DSP output or Wry cleanup. Prior published accepted-session parser/native runtime evidence is a separate boundary. New native joins must retain their own literal signed audio and real lifecycle oracles. Same-attachment external restore and physical-press nonce semantics remain explicitly unclaimed.

I independently matched all 589 current headless input hashes against the real retained source-free fixture and the recorded published manifest; web/ui/third_party are absent there. Published optimized 92-test source-free Cargo/dependency logs were inspected. This justifies unchanged-headless-input reuse only, not a new complete default-profile guard, CMake restaging, fresh browser build execution inside a native host, or finished product acceptance.

## Architecture, alternatives and operational safety

This change belongs in NativeLive, which already owns note/telemetry state independently of React render and document history. Capturing two parsed scalar strings avoids aliasing mutable session connection metadata and does not add a dependency into the headless engine, protocol, transport or audio callback. The helper closure refers to its initialized object only when an asynchronous error occurs; createNoteAudition construction invokes no callbacks. Engine close sets the existing closed guard before pending asynchronous error continuations publish. Identity replacement and ordinary document change remain distinct lifetimes.

Keeping the helper forever would be smaller but would conflate different attachments; recreating it for every generation causes the demonstrated loss of held slots and hidden errors. Duplicating slot transfer or adding a separate note owner would add synchronization obligations without an in-scope benefit. Reusing createNotes with scalar identity and its existing pump is the simpler sound alternative. No new security, persistence, resource resolver or realtime authority surface is introduced. Existing limitations around retired-helper pending operations and same-generation re-press coalescing are not silently redesigned.

The four candidate named validations have exit 0 receipts tied to the initial manifest: strict scoped OpenSpec, spec coverage, staged whitespace and unstaged whitespace. Spec coverage is 618 scenarios / 379 mapped / 239 unchanged backlog. No scenario was added, edited or prematurely synced; the full native scenarios remain open. The configured `.githooks` path and actual pre-commit/pre-push scripts were read: commit invokes strict Rust coverage, push runs Cargo tests and selects changed engine Rust files for mutation. With this web/evidence-only candidate the selector normally skips mutation; neither hook may be bypassed. Counts from earlier hook receipts are expectations, not claims of a future hook run.

## Publication envelope and remaining boundary

The exact five metadata transformations, output SHA256/Git blob/modes, complete final 15-path manifest and full expected tree are in `approved-envelope.json`. They copy this immutable report/verdict into portable review paths, change only receipt.review, replace one unique pending progress sentence and regenerate sorted SHA256SUMS. No source, test, raw archive, accepted requirement, task, canonical spec or other metadata change is permitted.

The root must return the actual final manifest and all four named validation receipts to this same reviewer for an external addendum before committing. Only then may the exact reviewed staged tree be transferred to clean integration at the same bb2c631 base; verify whole index/worktree/tree equality and absence of unexpected state before normal-hook commit. Original clean feature/runtime lanes may fast-forward. The frontend lane may fast-forward only after HEAD is still bb2c631 and its complete staged index/worktree already equal the resulting commit tree with exactly the reviewed 15 staged paths; use ordinary fast-forward only, no reset/force. Preserve the dirty job lane for separate explicit integration and verification.

Publication requires a fetch without merge/rebase and an unchanged recorded upstream, then a normal non-force feature push with hooks. Content/check/upstream/push drift invalidates this PASS. Main is excluded. Cleanup authority is inspection only; preserve reusable caches, evidence and uncertain ownership. No pending cache deletion permission is inferred.

## Narrow lesson candidates

| Evidence | Proposed owner and future decision | Downside / disposition |
| --- | --- | --- |
| A document generation change discarded a still-owned press; five actual failures distinguish document and helper lifetimes | Local native adapter review guidance: identify attachment lifetime separately from document revision when reviewing view-owned state | Avoid creating a general abstraction from one owner; candidate only, no standing guidance changed |
| 100% line coverage masked 11 inherited unexecuted snippets in compressed JavaScript | Existing coverage skill already directs source-offset and assertion inspection | Duplicate of existing guidance; no new lesson persisted |
| Chromium's UNIX socket path limit prevented launch despite adequate RAM | Task evidence records exact temporary alias, target and removal proof for future constrained RAM runs | Do not generalize alias creation or deletion authority; local operational evidence only |

No findings require a repair. Native controller/actor/wake, heartbeat expiry, facade/CLAP, external restore semantics, physical press distinction, VST3/standalone, named DAW and installed offline/native beta acceptance remain outstanding under the accepted larger goal.
