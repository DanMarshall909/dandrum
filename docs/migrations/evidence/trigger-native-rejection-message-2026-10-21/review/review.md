# Independent completion review: native command rejection presentation

Verdict: **PASS**. No actionable in-scope correction remains in the frozen nine-path candidate. This approves the bounded presentation fix and the exact five-output metadata envelope, subject to the named final validation, same-reviewer actual-tree addendum and normal-hook publication gates. It does not complete OpenSpec task 3.3 or the beta.

## Independence and reconstructed identity

I reviewed the actual staged candidate read-only, without running tests/builds, using the authoritative project/shared rules, completion-review reference, coverage-and-mutation guidance and existing OpenSpec proposal/design/tasks/delta requirements. I did not edit a repository, index, cache or another author's lane, run network/browser tools, or delegate this verdict. Outputs are confined to the external reviewer directory. Verification results below are inspected retained author/root evidence, not new reviewer executions.

Independent reconstruction from Git porcelain status, index modes/blobs, worktree bytes, branch/ref identities and merge-base reproduces the supplied manifest byte for byte:

- Initial manifest SHA-256: `daaf39c64635bdffb6f8df8caca7e298dd33564ae9ff372a5f7939682fc3be50`.
- Base/current/integration base/intended upstream: `0645f8ac844f3acdfdca7878c8b89cf1d3a7ddbf`.
- Candidate branch: `lane/trigger-frontend-20261020`; intended upstream: `origin/feat/juce-free-host-adapters`.
- Nine changed paths, all regular `100644` files; six added evidence files and three modified files. No unmerged index, unstaged tracked change or untracked file. SHA-256 and independently calculated Git blob SHA-1 match every staged object.
- Clean integration and publication checkouts are at the same base; the publication branch tracks `refs/remotes/origin/feat/juce-free-host-adapters`. Main is clean at `68dd6553a2f07e71e2fe09ee1d7094d518559937`.
- The integration from author base `ca36647a619034c96bc03f1ab2ffc4e166d356d6` changes no `web/` path. Both archived baseline files equal both Git bases; both final archived files equal the frozen worktree/index. The external disjoint-integration receipt is consistent with these independent checks.

The existing tracker records authorization and sole-writer assignments for continued work in these existing lanes. The active protocol author's `parameters.rs`, new controller test and root-owned design changes are outside this candidate; no synchronization or mutation of that lane is approved here. The remote identity was inspected locally; root must retain its immediately preceding fetch receipt and perform the unchanged-upstream fetch check before push.

## Task-to-implementation matrix

| Accepted task or claim | Actual implementation/evidence | Assessment |
| --- | --- | --- |
| Show useful native command rejection text | `web/trigger/engine/native-engine.mjs:96`, `NativeEngine.report`: early closed return, then Error message, string message, string code, String fallback | Complete for the bounded reporting boundary |
| Preserve original caller rejection | Existing `invoke`/parameter controller propagation remains unchanged; real transport test awaits `setParam` rejection separately from emitted failure | Preserved; JSON reply code and thrown Error/string identity are asserted |
| Preserve current/retired attachment and close behavior | Existing NativeLive note-owner comparison, report closed guard, parameter-controller guards and transport identity checks unchanged | Existing current/retired tests retained and included in passing evidence |
| Provide reproducible, honest verification | Six new portable evidence files plus progress addition; 65-member archive, baseline/final preimages, raw success/failure logs and source-bound reuse map | Complete with qualifications below |
| Continue task 3.3 without premature completion | `tasks.md` still leaves 3.2/3.3/4.x/5.x open; no canonical spec, map or checkbox changes | Correctly bounded; actual WebView/controller/CLAP and later beta gates remain open |

## Criterion-to-test matrix

| Criterion | Executable assertion and layer | Inspected evidence |
| --- | --- | --- |
| Code-only native rejection yields useful text | New `native command failures preserve their useful message through the webview transport` / `code-only native rejection`: production broker receives JSON rejected reply, production engine `setParam` rejects with code and emits exactly one failed message `capabilityUnavailable` | `red-host.log` fails with `[object Object]`; final focused GREEN and full Node run pass |
| Supplied message precedes native code | Same public-path test / `native rejection message` provides both fields and requires `Parameter is unavailable`, separately requiring caller rejection code | Genuine old-reporter assertion RED; final GREEN |
| Error and string posting failures retain messages and rejection reasons | Two corresponding subtests throw during actual broker posting, require the identical rejection reason and exact `IPC unavailable` failure message | These already pass against old code: characterization, not new RED |
| Current same-attachment errors remain visible | Existing `current note errors remain visible after same-attachment generation refresh` drives a pending note across refresh and asserts its exact failure text | Baseline-focused, owner and full-suite evidence |
| Close/instance/attachment retirement filters late errors | Three existing `late note errors after ... do not publish editor failures` tests resolve a delayed failure after retirement and require no failed events; replacement cases assert accepted identity and no stale note-off | Owner/full-suite evidence, with unchanged NativeLive/close implementation |
| Broker continues to admit only current replies | Existing malformed/foreign/unknown/retired reply and disposal tests assert promise preservation/closed outcomes | Full 151-test package run; broker source unchanged |

The Rust protocol's tagged error representation and `tests/protocol.rs` code-only JSON assertions corroborate the new fixture shape. This is cross-checking the wire contract, not proof of a running native WebView. The reporter's generic fallback remains exercised by the string posting case; its newly introduced message/code decisions have explicit output assertions.

## Seven required review dimensions

### 1. Task and specification completion

The fix addresses an actual mismatch: the broker rejects with the native error object, while the old reporter stringified that object. The existing report boundary is the appropriate owner for presentation. The docs accurately limit completion to this prerequisite and retain actual native delivery, controller leases, semantic detach, CLAP, VST3, standalone, named-DAW and installed/offline acceptance. The changed content makes no new canonical scenario or task-completion claim. Retained strict scoped validation passes; all 51 canonical specs pass; the spec map reports 618 criteria, 379 mapped and 239 ratcheted backlog. Backlog is not represented as completed coverage.

### 2. Acceptance evidence

I verified all 65 archive members against their byte lengths and SHA-256 entries and checked all portable SHA256SUMS entries. Baseline and final source copies match Git and the candidate. The old-reporter RED failures name the expected text mismatch in the intended assertions, rather than a setup failure. Restored final focused tests are 5/5 including the parent, full Node tests 151/151, TypeScript/component check covers 70 authored React files, and both retained Vite builds succeed (preview 3.45s, native 3.60s). The final package test takes approximately 1.277s; focused final feedback approximately 152ms.

The pre-implementation coverage step was missed. The retrospective exact-base 35-test run (100% lines, 81.32% branches, 87.10% functions for the owner), verified source restore and final reruns recover evidence about baseline behavior but do **not** satisfy the original chronology. This historical workflow deviation is disclosed in handoff, receipt and progress. It is not repairable by relabeling a later run. For this small covered reporting branch, the exact preimages, real behavioral RED, final source identity and output assertions provide adequate present regression evidence; no source or test correction is required solely to rewrite that history.

Final owner coverage is 40 tests, 100% lines, 82.20% branches and 87.10% functions. These are whole-owner execution metrics, not claims of full branch/function coverage or assertion strength. Inherited gaps are acknowledged. No module extraction occurred. Mutation is explicitly excluded by user direction and is neither passed nor pending here.

The original sandbox component failure is retained; the actual-host full suite and direct checks pass without guard changes. The EPERM diagnosis is described in the handoff; the archive's component-failure log itself shows the failed guard assertion, not a printed EPERM probe. I rely on the retained successful actual-host execution and unchanged guard for acceptance, not on independently reproducing that diagnosis. Likewise, the original 16GiB-free shared-memory statement is explicitly superseded by retained actual-host samples showing about 1.1GiB free. Neither failed/setup run is counted as passing evidence. Browser tests were not rerun; no current rendered-browser or native-host proof is inferred.

### 3. Test effectiveness

The new table-driven cases use the real bounded broker and production adapter with ordinary initialization and parameter APIs. They independently assert both user-facing event text and caller failure, detect the demonstrated regression, and avoid a production-only seam or a direct test of private formatting implementation. Four cases share one contract and remain individually named. The suite is fast, the fixture is local and deterministic, and no arbitrary wait or browser dependency was added. Existing lifetime tests protect observable absence/presence of late failures. Testing more speculative JavaScript objects would not improve the accepted wire/posting contract enough to justify added scope.

### 4. Architecture and operational safety

Only frontend reporting changes; transport admission, capabilities, session/history, polling, title policy, close ordering and headless engine dependencies remain intact. Native replies retain their raw reason for callers. JSON fields become plain event text through existing presentation rather than introducing HTML interpretation or native authority. No new dependency, realtime state, ownership seam, persistence format or generated runtime source is introduced.

The proposed operational helpers were inspected, not executed. The exact-staging/tree guards precede normal commit hooks; resource admission precedes starting the mutating child. Disk 1GiB, shared memory 256MiB and available memory 4GiB floors remain, with owned-process monitoring and no cache deletion. Root's hook targets remain distinct from the active protocol author's caches. Prior calibration is not a substitute for the specified final calibration/receipts.

### 5. Better alternatives

Normalizing errors in the broker would broaden its contract and risk changing raw caller rejection. Extracting a generic formatter for a single short existing branch would add an unnecessary owner and coverage burden. Keeping the ordered selection in `report` is the smallest clear change. The existing chained conditional remains readable at this size and handles optional object fields without changing the plain-string fallback.

### 6. Reusable lessons

No standing guidance is edited. Two supported candidates are recorded here, deduplicated against existing rules:

| Evidence | Proposed owner/action | Cost and disposition |
| --- | --- | --- |
| Baseline coverage was captured only after implementation | Existing project TDD/coverage workflow: record baseline owner coverage before source edits, with source hash and run receipt | Small up-front run cost; already required by AGENTS/shared rules, so enforce existing procedure rather than add duplicate policy |
| Sandbox resource sample differed materially from actual-host shared memory; sandbox child launch failed while host suite passed | Task runner/evidence procedure: bind resource and execution receipts to the same host namespace and distinguish setup failure from product assertion | Additional small provenance record; proposed procedural clarification only, current record already corrected |

The EPERM raw-probe limitation above is a provenance lesson, not a required product change. Future automated receipts should retain probe output alongside the interpretation. No new guardrail or global rule is authorized by this review.

### 7. Publication soundness

The initial manifest is independently reproduced. Qualified source-free reuse is substantiated: all 590 current headless input hashes equal the published executed map; archived raw release output contains per-target counts totaling 107 ordinary plus two doctests, matching the zero-exit receipt and nine executed-binary bindings. This is reused source-bound default-release evidence, not a fresh fixture/native/CLAP/browser run.

The finite envelope is defined by `approved-envelope.json`: exactly two added portable review files and three deterministic metadata updates (receipt review field, one progress sentence and SHA256SUMS), resulting in 11 changed paths. No semantic source/test/requirement or raw-evidence bytes may change. The expected final manifest is separately recorded. Root must return the actual complete final tree and manifest to this same reviewer before hooks.

Named remaining gates are scoped strict OpenSpec validation, all canonical strict validation, spec coverage, cached/worktree diff checks, exact final manifest/tree comparison, calibrated empty-tree rejection before hooks, exact transfer to clean integration, and normal commit/push hooks. The installed `core.hooksPath` is `.githooks`; the entrypoints run engine coverage on commit and engine tests plus the changed-Rust selector on push. No changed Rust engine file is in this candidate, so a normal selector skip is expected, not a bypass. Runtime result counts must come from the actual later hook logs.

Fetch must confirm the frozen upstream before normal non-force push; no merge/rebase/pull after PASS. Any content drift, changed upstream, failed gate or rejected push requires renewed review under the contract. Only clean authorized feature/runtime/frontend synchronization is contemplated after publication; the active protocol lane remains excluded until its owner freezes and hands it back. No main merge, new worktree, dependency change, cache cleanup or hook bypass is approved.
