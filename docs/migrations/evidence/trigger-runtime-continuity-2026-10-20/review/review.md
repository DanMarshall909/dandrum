# Independent completion review: ordinary runtime reservation continuity

**PASS.** No in-scope correction remains for the exact 14-path candidate identified by manifest SHA-256 `5b19254623b287a2628362b5058c5c0808713ff9a70ef911cbe78f5893d947db`, subject to the finite finalization envelope below. This approves the bounded runtime component. Tasks 3.2 and 3.3, the actual Trigger CLAP/editor integration, and the full beta remain incomplete.

## Review scope and independence

I independently read project AGENTS.md, shared AI rules, completion-review, behavior-first-tdd, coverage-and-mutation and reflect guidance; the accepted OpenSpec design/tasks, canonical session publication requirements, and beta progress obligations. I inspected the complete runtime owner, changed tests and session contract, relevant existing state/parameter/session tests, rtrb 0.3.2 implementation, normal hooks, portable receipts, raw logs and LCOV, retained RED test, and final staged content. The historical options investigation was a decision input, not a desired verdict.

I performed read-only source, Git and evidence inspection. I did not execute tests/builds/native sessions, change repository/index/Git/cache state, or delegate approval. Output writes are confined to the external reviewer directory. Test results below are independently inspected retained execution evidence, not reviewer reruns.

## Candidate and integration reconstruction

- Branch: `lane/trigger-runtime-20261020`; explicitly intended upstream: `origin/feat/juce-free-host-adapters` (the lane itself has no configured tracking upstream).
- Base, HEAD, intended upstream object and merge/integration base independently resolve to `1d7efe62cf1dd2e4a623fb9215fce84bf26f9e83`.
- Independently reconstructed every path's change, index/worktree status, canonical mode, content SHA-256 and staged blob identity, including new evidence. All 14 paths equal the supplied serialized manifest byte-for-byte. All are regular mode100644 files; worktree equals index; no other dirty/untracked path appears.
- That integrated commit directly extends tested base `0f02f575ff2419082370f4d35986d086c03b5d0b`. Its 37 changed paths are frontend and migration documentation/evidence. I inspected the transport/native-engine/native-live contract changes for their effect on this boundary: they gate optional frontend services and retain required native work. They introduce no Rust runtime, session, dependency or build-input change.
- All three reviewed source hashes and all 586 copied source-free inputs still match tested bytes. Consequently the integration does not materially change this bounded runtime review context or require replacing the retained reviewer. Composed frontend/native/CLAP continuity remains unproved and is not inferred from either component.
- All 65 archive members match artifact names, byte lengths and SHA-256 values; no extra archive member exists. Final check receipts match their archived originals. The portable SHA256SUMS is complete and correct. Coverage qualification equals the corresponding receipt object.

## Task to implementation matrix

| Accepted scope | Implementation | Status supported here |
| --- | --- | --- |
| Task3.3 ordinary immutable mapping reservation keeps old audio live | RuntimeLease owns an optional admission guard; activation closes admission only for RestoredScalars | Bounded component complete |
| Task2.2 capacity-one ownership and realtime safety | Existing pending/retired rtrb slots, exclusive control lease, in-flight until reclamation, off-audio preparation/destruction | Retained |
| Task2.4 atomic external state and scalar/epoch publication | RestoringControl still holds ClosedAdmission while publishing candidate and shared scalar/epoch state | Retained, with paused-restore silence explicitly disclosed |
| Session adapter-neutral publication contract | ActivationLease comment distinguishes immutable replacement from shared-active-state reader exclusion | Clarified without changing other adapters |
| Full tasks3.2/3.3, actual CLAP and beta | No task checkbox or canonical spec completion claimed | Remain open |

## Criterion to executable evidence matrix

| Criterion | Outcome asserted by retained tests |
| --- | --- |
| Producer pause cannot silence ordinary reserved mapping | `session_publication_uses_a_cancellable_lease_and_rolls_back_under_backpressure`: distinct audio thread completes32 blocks while control retains lease and joins; every512 stereo sample pair matches independently decoded continuing PCM; no admission denial or swap |
| Test exercises real continuing DSP | Actual AdvancedKit sampler starts before reservation; first16 frames match independent WAV decoding; first four sample words pinned to literals;12,000-frame one-shot exceeds528 observed frames; no repeated note hides interruption |
| Cancellation preserves accepted runtime and metadata | Same test checks original session snapshot/history, candidate frees off audio, and next16 waveform positions528..543 after cancellation |
| Publication is real and capacity remains bounded | Same test commits36→60, checks revision1/undo1, Busy before consumption and after swap until reclamation, literal stereo-0.5 at offset3, then accepted revision2 after reclaim |
| Callback performs no allocation or deallocation | Held-lease audio loop checks count_total/count_current/bytes_current=(0,0,0); retained independent sentinel test detects actual allocation and preexisting destruction; retained first-swap/events/reset/short-block/endpoint-drop cases cover existing transfer paths |
| Restore retains reader exclusion and cancellation | Expanded actual RestoringControl lease test checks denied reset/process, complete zero buffers, no swap, zero allocation/free, off-audio candidate destruction and unchanged session/scalars |
| Restore queues, scalar coherence and state remain correct | Existing same test retains old-generation note/command retirement and literal-0.125 output; unchanged state suite verifies23 distinguishable physical values, fresh-instance immediate load/save before any callback, stale feedback rejection, coherent concurrent saves, reset settlement and signed output |
| Rejection/reset retain useful prior behavior | Existing invalid blocks/event batches preserve pending replacement; reset clears active/queued notes while retaining scalar values and admitted replacement; boundary and session rollback suites remain green |

The main continuity test observes public API outcomes and actual samples, not queue layout. Its publication/cancellation/backpressure phases are distinct useful regression checks in one existing scenario. The restored admission characterization invokes a genuine restore adapter rather than merely executing the private guard to raise coverage. Existing private concurrency checks remain supporting evidence, not the sole public behavior oracle.

## TDD, coverage and retained verification

Qualified RED's production runtime hash equals the tested Git base. The preserved RED test differs from the final owning integration test only in formatting. Preparation and initial signed playback succeed before RED fails specifically with `a reserved immutable mapping must not silence audio`; this is behavioral RED. The earlier four-frame loop fixture failure occurs before reservation and is correctly labeled fixture calibration, not RED. The actual one-shot oracle corrects the fixture assumption without changing DSP.

Prechange published state coverage binds to the exact unchanged base runtime hash `97d06354b35e5d264fef5f11e933f9e0127b9f71fb132e24aaac166e4954cec4`, with no missed source lines. Final format, strict Clippy, host LLVM/test and session test manifests match current source. Raw final host results total62 tests and session results total30 tests, all passing. The main focused test completes in0.11s in GREEN; final host component suites report0.04–0.16s test execution, with existing session history cases in a2.98s suite. No new slow-lane classification is justified by this small change.

Final runtime coverage is707/707 distinct DA source lines and735/735 emitted LCOV entries, with69/69 source-function summary entries. Embedded unit-test code is included. Prechange corresponding totals are678 distinct source lines,703 emitted entries and66 source-function entries. I independently recomputed this distinction; the qualification now records it. Coverage proves execution, not branch coverage, atomic correctness, assertion adequacy or every generic instantiation. Branch coverage is not collected. Mutation is excluded by recorded user direction, not represented as a pass.

The full source-free attempt proves successful release build, copied resources/resource rebuild, target/test inventories and dependency boundary before a test-only nonexistent `PreparedDocumentReplacement.prepared()` API causes E0599 in the runtime-test stage. The final test uses the accepted prepared graph for a genuine cancelled scalar restore, and retains actual changed-document publication afterward. All three selected CTests subsequently pass against the actual copied fixture with CARGO_INCREMENTAL=0 and disk-backed compiler output. All586 final inputs match. The intermediate resource-floor stop is retained. This combined evidence is sufficient for this bounded change, but is expressly not a fresh complete default-profile guard, real production React/Wry proof, or Trigger CLAP product acceptance.

## Architecture, safety and simpler alternatives

The RuntimeLease exclusive `&mut RuntimeControl` borrow prevents another producer operation while it owns the reserved chunk. rtrb0.3.2 write_chunk_uninit does not publish storage. fill_from_iter initializes the item then Release-publishes the tail; consumer synchronization observes fully initialized data. Unpublished candidate ownership stays on control, so cancellation cannot expose or destroy it on audio.

Audio still reserves retirement storage before popping pending, then moves the previous runtime into that slot. in_flight stays set until control consumes retirement, preserving capacity and avoiding callback destruction even if the control endpoint disappears. Render/event loops remain bounded by configured capacity; no new locks, I/O, serialization, browser access, allocation, unsafe marker or shared mutable DSP state is introduced.

Ordinary mapping publishes no shared scalar or epoch writes, so it does not need callback exclusion. Explicit restore still closes admission, waits only on control for an admitted callback, and retains its guard through queue/scalar/settlement/epoch publication. ScalarWrite's existing SeqCst interval and off-audio retry reader remain intact. Session fallible preparation precedes accepted-state installation and infallible publication; exclusive session/control ownership prevents observers from seeing unpublished metadata.

An optional guard reuses the established lease and publication implementation. Separate lease types would duplicate this small boundary; moving restore scalars into candidates would add epoch, immediate-save and future-event semantics not required by this criterion. Removing restore admission indiscriminately would be unsafe. Migrating active voices across actual structural swap is outside scope. Production and tests need no further extraction; no new module or test seam was introduced.

## Findings and lesson disposition

No actionable in-scope finding remains. The preliminary evidence wording observation is resolved by coverage-qualification.json, receipt.coverage and progress text. Two useful local evidence lessons are already embodied in this packet: distinguish behavioral RED from fixture failure, and distinguish LCOV emitted entries from distinct source lines. No new standing-guidance or memory change is requested or authorized. No source repair is outstanding.

## Exact finite post-PASS envelope

Only these five metadata paths may change before commit:

1. `docs/migrations/evidence/trigger-runtime-continuity-2026-10-20/review/review.md`: byte-for-byte copy of this report, mode100644.
2. `docs/migrations/evidence/trigger-runtime-continuity-2026-10-20/review/verdict.json`: byte-for-byte copy of this review's verdict.json, mode100644.
3. `docs/migrations/evidence/trigger-runtime-continuity-2026-10-20/receipt.json`: replace only `review` with status PASS, initial manifest SHA above, copied report SHA and copied verdict SHA; preserve all other values. Serialize JSON with two-space indentation and trailing newline.
4. `docs/migrations/juce-replacement-progress.md`: replace the unique exact sentence `Independent completion review and normal-hook publication are pending for the runtime continuity candidate.` with `Independent completion review passed for the frozen runtime continuity source/evidence candidate; normal-hook publication remains.`
5. `docs/migrations/evidence/trigger-runtime-continuity-2026-10-20/SHA256SUMS`: regenerate SHA-256 entries for every other regular portable file in bytewise relative-path order, preserving the existing two-space separator and trailing newlines.

The external approved-envelope.json records the exact resulting mode and content hash of each transformation. Root must stage these exact bytes, rebuild the complete manifest, and return the final candidate to this same reviewer for an external final-manifest addendum before commit. Do not embed that addendum in the candidate.

Named final validations are `openspec validate add-juce-free-trigger-clap-editor --strict`, `scripts/check-spec-coverage`, `git diff --cached --check`, and `git diff --check`, with actual cwd/arguments/exit/log evidence retained externally. Normal commit and push hooks must run. The active hooks are .githooks/pre-commit delegating to scripts/check-rust-coverage and .githooks/pre-push running engine Cargo tests then its changed-engine-file mutation selector. Engine bytes are unchanged; no hook bypass or invented mutation pass is permitted.

After final addendum, compare the complete final tree to reviewed content plus these transformations, commit normally, fetch without integration and require intended upstream object `1d7efe62cf1dd2e4a623fb9215fce84bf26f9e83`, then push normally to the intended feature ref. Any unexpected content/upstream drift, failed validation or rejected push invalidates this approval. Preserve main and unrelated worktrees. The only cleanup covered is inspection of task-owned process state; no caches, evidence or unknown state may be deleted. Any separately authorized clean worktree fast-forward is operational integration, not evidence that beta tasks are complete.
