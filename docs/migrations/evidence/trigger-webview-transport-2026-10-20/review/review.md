# Independent completion review: bounded browser native broker

Verdict: **PASS**.

This verdict covers the isolated, unwired `createWebViewTransport` component and its evidence, not OpenSpec task 3.3, a working editor, native method dispatch, or a completed beta. No actionable in-scope correction remains. Publication requires the exact five-file metadata envelope and a same-reviewer final-manifest addendum before commit.

## Frozen identity and independent inspection

Worktree: `/tmp/dandrum-lanes/frontend`; branch: `lane/trigger-frontend-20261020`. Base, HEAD, integration base and explicit upstream `origin/feat/juce-free-host-adapters` all independently resolve to `29b8e90e9074aa667b2d7443376ba7aab5376fcb`. The lane has no tracking requirement substituted for that explicit upstream. Initial 11-path manifest SHA256: `d691970ae8732d57b0845d27dc3d39c699d72f06dba80cb1cede740c150f280f`.

I independently matched every path's status, canonical index mode, content SHA256 and Git blob identity, the complete porcelain path set, branch, HEAD, upstream and merge base. All entries are regular 100644 files; no outside-candidate dirty/untracked path appeared. I verified the supporting source/lock/protocol hashes, portable SHA256SUMS, every one of the 221 raw archive members including size/mode/hash, and all retained phase source hashes against their receipts. The production owner is 104 lines; the test owner contains 22 tests. Only those two new source files, portable evidence and one progress addition are staged. No dependency, application entrypoint, Rust, spec, task checkbox or canonical map changes are included.

Read: applicable AGENTS.md and shared AI Rules; completion-review reference; behavior-first-tdd test-quality guidance; coverage-and-mutation and reflect; assignment packet; actual proposal/design/tasks and both delta specs; production selector, broker, tests, Rust protocol encoding and its contract test; archived phase source/output/receipt evidence; raw V8 data; retained Rust calibration source/artifact provenance; progress diff; active hooks. This was an independent read-only review: no tests, builds, native execution, fetch, repository/index/cache writes or delegated decisions were performed. Writes are limited to this external review directory. Network/preflight and publication execution remain the root's existing session responsibility.

## Task-to-implementation matrix

| Owning task or boundary | Candidate contribution | Completion assessment |
|---|---|---|
| Assigned task 3.3 browser broker subcomponent | `native-webview-transport.mjs`: descriptor, request correlation, bounded admission, event sequence and lifetime ownership | Complete for the assigned component only |
| Task 3.1 published selector contract | Descriptor consumed by unchanged `selectNativeTransport`; four optional services explicitly false | Existing contract preserved; no additional selector completion claimed |
| Task 3.3 session snapshot interoperability | Retained real Rust accepted initial/applied replies consumed by frozen broker and unchanged session parsers | Accepted snapshot envelope compatibility evidenced; other native dispatch and reconnect behavior remain open |
| Tasks 3.2, 3.3 and 4.x actual editor/CLAP lifecycle | No bootstrap, UI entrypoint, Wry/CLAP or note/gesture/native-worker wiring | Correctly open; broker disposal does not prove native cleanup |
| Task 5.1 verification facets | Focused and package tests, check, original-source V8 coverage, source-bound evidence | Component evidence sufficient; mutation excluded by user, broader native/browser gates retained |
| Task 5.2 review/publication facets | Frozen manifest, this independent review, finite metadata transformation | Review passes; final addendum, normal hooks and publication remain |
| Canonical specs/map | No scenario text, fingerprint, map or task-state mutation | No premature synchronization; retained gate reports 618 scenarios / 379 mapped / 239 backlog |

The delta's copied production bundle, invalid native input preserving audio, held-note destruction, real automation, lost-editor mapping/undo and two-plugin-instance scenarios are not completed by these browser tests. The progress record and receipt preserve this distinction and retain 7/14 tasks complete.

## Criterion-to-test matrix

Test references below are the ordinal tests in `web/trigger/tests/native-webview-transport.test.mjs`, with descriptive names sufficient to locate them.

| Criterion | Implementation and asserted evidence | Assessment |
|---|---|---|
| C1 version-1 positional requests, actual settlement and removable tokens | Tests 1–2: selected backend, literal two-request envelopes, out-of-order actual result, capability rejection, distinct tokens/removal | Proven at component boundary; no invented native success |
| C2 canonical nonzero u64 configuration/cursor and precision/exhaustion | Tests 3–4: invalid-field tables, literal >2^53 and final-u64 IDs, exact result, exhausted rejection/no post | BigInt preserves cursor precision; transport seed remains independent of structural cursor |
| C3 bounded outbound admission and JSON arguments | Tests 5–7: independent UTF-8 literal boundary at 16384 bytes, repeated oversize rejection with no retention, invalid method/non-JSON/cycle rejection, 128 pending then capacity recovery | Explicit failures; successful admitted IDs monotonic; no retry |
| C4 synchronous post/reply failure atomicity | Tests 8–9: 129 identical thrown reasons then successful request130; real synchronous reply before throwing post | Pending registered before callback; deletion owns cleanup; real settlement cannot be reversed |
| C5 bounded correlated inbound replies | Tests 10–11: malformed/foreign/unknown/retired cases leave promises pending, distinct out-of-order results, exact 65536-byte multibyte boundary | Validate before lookup/removal; rejection and acceptance shapes distinguished |
| C6 sequenced bounded event delivery | Tests 12–16: supported names, exact payloads, high sequence/gaps, invalid/foreign/oversize cases, shared128 listener limit, nested delivery ordering, registration mutation, visible exception | Attachment-local monotonic delivery; snapshots bound iteration; consumed sequences cannot replay after exceptions |
| C7 disposal and independent owner | Tests 17–19: all128 calls reject closed, tokens gone, cached calls reject, receive inert, idempotency, different fresh attachment, dispose within event/post | JS lifetime barrier proven; no native cleanup claim |
| C8 serialization reentrancy | Tests 20–21: nested toJSON call gets ID1 and outer ID2; disposal prevents outer post; nested128 and final-u64 prevent outer admission | Snapshot arguments before final lifetime/capacity/cursor checks; no retained slot leaks |
| C9 explicit malformed options failure | Test22 plus field table: missing/null/primitive/array/object cases | Stable invalidConfiguration replaces incidental destructuring errors |
| Real accepted snapshot boundary | Archived `calibrate-real-replies.mjs`, `final-real-replies/run.json/output.log`; production parsers | Literal request90, structural cursor1, instance9007199254740993, revision0, 23 parameters, six regions, five rules, levels0.25/0.5 independently asserted |

## TDD, coverage and assertion quality

Retained C1 missing-module and C7 missing-dispose API REDs are genuine absence evidence, not mislabeled behavioral assertion failures. C1's additional token contract is present in the retained pre-GREEN contract snapshot. C2 combines an explicit invalid-identity assertion failure with an honestly disclosed pending-promise cancellation. C3's first cancellation was followed, before production changed, by retained assertion REDs for publication counts 2 versus1, 1 versus0 and129 versus128. C4 detects leaked capacity via the actual rejection reason; C5 detects malformed input and oversize acceptance; C6 detects missing event delivery/admission; C8 detects duplicate IDs and129 publications; C9 detects wrong configuration error. I inspected source diffs across all nine RED/GREEN transitions; assertions were not weakened to achieve GREEN. C6's exception test characterizes newly established behavior during covered refactoring; it is not presented as a separate failing-before-implementation criterion.

All phases retain source copies, command/cwd, exit and timing receipts. Final ordinary `npm test` passes131 tests in1.254020 seconds; `npm run check` passes in1.119625 seconds; focused coverage passes22 tests in0.138496 seconds. The original early package failures are retained as Node child-process EPERM setup failures, with independent guard diagnostics; final ordinary host runs resolve the limitation without guard edits/skips. I relied on retained runs rather than executing them again.

Raw V8 independently contains13 owner functions and105 ranges, all positive, on exact owner hash `b0e0ecb976d79b012487216c16216d1cc33b4f811d5c0f05b1a0a506075f0ed1`. Reported100% lines/branches/functions means execution under V8's overlapping-region model, not exhaustive operand truth tables or proof of assertions. Tests provide separate public-result oracles, independent literal request shapes/byte boundaries, explicit failure codes, expected event order and bounded counts. They avoid private maps/counters and remain resistant to internal refactoring. Focused tests are fast; package timing supports the existing lane selection. No mutation or broker fault-injection pass is claimed, and none is required contrary to the user's exclusion.

The accepted reply artifact SHA256 `556a660e40061706c585a6481be9013c490ed62899f680105166f4dc14cda71e` matches both the calibration input and the previously published snapshot evidence archive. Its retained Rust projection invokes the real host/protocol and checks signed output; the new calibration uses unchanged production parsers with literal independent field/count/value oracles. Rejected replies and events in this candidate are literal inputs, not an executed cross-language calibration. Their envelope shape agrees with `EditorProtocol::encode_reply/encode_event` and the existing Rust `replies_and_events_are_versioned_correlated_bounded_and_retired_with_the_editor` test, including explicit capability-error and event objects. This is sufficient to review the bounded unwired broker against a frozen wire contract, but it does not earn a native rejection/event integration claim. Actual emitted rejection/event calibration remains required when wiring that boundary; do not reuse this PASS as evidence that it ran.

## Architecture, bounds, lifetime and alternatives

One closure owns each connection's cursor, pending promises, event cursor and listener registrations. Decimal validation bounds digit count before BigInt conversion. Requests contain exactly the native five-field input envelope. Bounds concern admitted encoded messages and retained entries, not a general memory sandbox for arbitrary caller serialization; caller getters/toJSON are deliberately treated as callbacks. JSON data is copied before final admission, preventing those callbacks from invalidating the selected ID/capacity/lifetime state. The second serialization sees parsed JSON data. No native/browser global or format type enters the component.

Receive validates identity and shape before touching correlation state. Native results are returned and native error objects rejected, without manufacturing acknowledgements. The callback exception cleanup only rejects requests still owned by pending. Event dispatch snapshots at most128 registrations, checks removal, suppresses remaining old delivery after nested newer events, and consumes a sequence before exposing listener exceptions. Disposal closes first, then removes all callbacks and rejects/clears pending entries. A fresh owner retains independent state. Native authority, session acceptance, replay ownership, note/gesture release and worker progress remain Rust/integration responsibilities.

A generic RPC framework, timer retries, extracted public helpers or a separate event dispatcher would add state/abstraction without an accepted need. A superficially simpler single JSON.stringify after choosing an ID would reintroduce the demonstrated reentrancy defect. The present private decimal/error helpers and one lifetime owner are the simpler justified design. No new extraction or dead compatibility path needs correction; the unwired module has an explicitly accepted near-term consumer.

## Publication and remaining scope

The five authorized metadata outcomes are enumerated with exact mode, SHA256 and Git blob identity in `approved-envelope.json`; all other initial candidate bytes must remain identical. Copy this report and verdict byte-for-byte, replace only receipt.review, replace the unique specified progress sentence, regenerate portable sums deterministically. Rebuild the complete13-path final manifest and return it plus named validation logs to this reviewer for an external addendum BEFORE commit. No source/test/raw archive/spec/task change is administrative.

Named final validations: `openspec validate add-juce-free-trigger-clap-editor --strict`, `scripts/check-spec-coverage`, `git diff --cached --check`, `git diff --check`. Their results must match the expected tree with no generated drift. Candidate retained scoped strict/spec-map records pass; the staged diff check remains an explicit final-envelope gate, not assumed from the unstaged check.

Active hooksPath is `.githooks`; pre-commit delegates to `scripts/check-rust-coverage`, and pre-push runs engine tests then selects changed engine Rust files. No such Rust file is changed here, so ordinary hook selection should omit mutation without bypass. Before hooks, root must obtain the recorded shared-cache execution handback and enforce the packet's resource floors. Commit normally in the candidate lane, fast-forward only clean authorized integration/original feature worktrees, fetch without integration and require the exact upstream29b8 object, then normal nonforce push from the configured original feature branch. This review does not claim those operations have occurred.

Main remains outside integration, and active Rust/runtime/job lanes plus reusable caches/evidence must be preserved. No cleanup deletion is authorized. Any unexpected candidate/content/upstream/validation/push drift invalidates this PASS and requires renewed review. The full beta remains active through actual bootstrap, Wry/CLAP, required native methods/lifecycle, VST3, standalone, named DAW and installed/offline/native acceptance.

## Findings and reflection

No failing findings. The serialization callback invariant is already recorded and tested at its narrow owner, so no new standing-guidance change is proposed. Missing native rejection/event calibration and native lifecycle proof remain disclosed integration work, not silently completed criteria.
