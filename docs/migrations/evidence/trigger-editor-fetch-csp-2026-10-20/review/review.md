# Independent external-fetch CSP completion review

**Verdict: PASS**

This verdict covers only the task 3.2 external-fetch CSP foundation and the finite finalization envelope below. No actionable in-scope correction remains. Task 3.2 and the full beta remain incomplete.

## Candidate identity and independent reconstruction

- Worktree: `/tmp/dandrum-lanes/runtime`; branch: `lane/trigger-runtime-20261020`.
- Base, current HEAD, publication upstream `origin/feat/juce-free-host-adapters`, and independently computed integration base: `4d85b25deabcd533521fb51e5d160b74f9041ae1`.
- The lane has no tracking upstream. The explicit publication ref above is the applicable upstream, rather than an inferred lane remote.
- Independently reconstructed every changed path from base-tree diff, complete porcelain status, index entries, filesystem modes, worktree bytes and staged blob bytes. No unmerged index, special file, unstaged change, untracked path or unrelated exclusion was present.
- Complete 17-path manifest serialization matches the supplied manifest byte-for-byte: SHA256 `b4b1aa8367b8029b8516ca50a0ecba79224d4edaebdcfbfeb3686d5caabe7de1`.
- Main remains clean at `68dd6553a2f07e71e2fe09ee1d7094d518559937`.

Reviewed project/shared instructions, authoritative proposal/design/tasks/both delta specifications, agentic-delivery completion review, security-by-design, behavior-first-TDD test criteria, coverage-and-mutation and reflect. Repository inspection was read-only. The reviewer launched no Cargo/build/native process, mutated no cache or index, and delegated no approval decision.

## Task-to-implementation matrix

| Task or claim | Implementation and evidence | Assessment |
|---|---|---|
| Task 3.2, narrow external-fetch enforcement foundation | `NativeEditor::create` adds the fixed enforcing response header `Content-Security-Policy: connect-src 'self'` at the existing custom-protocol response boundary | Implemented and proved within the stated cut line. |
| Preserve normal packaged-page behavior | Existing native runner invokes `external_fetch_is_blocked_by_enforcing_csp`; ordinary packaged module and stylesheet plus inline style/local fetch supply positive controls | The new test does not require native evaluated script or a fake browser/backend. |
| Retain established native-owner behavior | Existing parent geometry, finite service tick, repeated child destruction, weak callback/view lifetimes and acquisition-error rollback assertions are unchanged and still execute | Current real native test and CTest evidence retain their prior protection. |
| Applicable verification portion of task 5.1 | Exact native/fast coverage, three registered editor CTests, all-target native Clippy, formatting, dependency and specification checks | Sufficient for this single-line policy addition; inherited owner gaps remain disclosed. |
| Full task 3.2, copied production bundle/backend scenario, task 3.3 and actual CLAP product boundary | Remain open in authoritative tasks and progress record | Correct. No broader scenario completion or new spec-test mapping is claimed. |

## Criterion-to-test matrix

The owning maintained test is `native_x11::tests::real_child_loads_local_content_and_drops_without_owning_the_host_parent`, selected explicitly by CTest `trigger-editor-native-x11`. Its new helper owns the following assertions.

| Criterion | Independent observable oracle | Evidence |
|---|---|---|
| Endpoint is live and serves the expected response | Rust TCP client requires HTTP 200, literal sentinel body and independently recorded calibration GET | `Endpoint::calibrate`, executed on baseline, RED and GREEN paths |
| Browser could perform the external request before enforcement | Real WebKit no-cors fetch resolves and listener records `/external-attempt`; local controls pass | `missing-policy-baseline.log`, exact source snapshots and hashes, exit 0 |
| New test fails for the missing policy | Named enforcing-connect-src event assertion fails while browser fetch resolves and actual external GET arrives | `red-missing-enforcing-connect-policy.log`, exact contemporaneous source, exit 101 |
| Enforcement specifically causes blocking | Exact browser event identifies `connect-src`, `enforce`, the ephemeral endpoint origin and `dandrum://localhost/index.html`; fetch rejects; listener records calibration GET only | GREEN and final native logs, exact final snapshots, exit 0 |
| Local behavior remains usable | Required packaged module runs; local fetch returns literal 404; stylesheet gives height `43px`, inline style gives width `37px`; completion message arrives | Same RED/GREEN/final real-browser runs |
| Existing native owner contract is preserved | Literal geometry, X11 tree state, callback and WebView weak lifetime, finite tick budget and cleanup/error assertions all precede the new helper | Final native run plus registered native CTest |
| Default dependency/build boundary remains isolated | Native production module retains its existing feature gate, helper dependencies are standard library only; Cargo manifest/lock/CMake bytes equal base; default graph contains only editor crate | Unchanged-boundary hash map, final default-graph run, existing published base guard |

The policy event and endpoint ledger are separate oracles. Generic network failure or a CORS rejection cannot satisfy the exact enforcing policy event; the unchanged-owner browser baseline independently proves the same endpoint can actually receive the request. Positive local controls reject a broken page or a blanket deny-all connection policy. The origin, document URI and sole external attempt correlate the event to this fixture.

## Evidence integrity, coverage and TDD

Verified every portable SHA256SUMS entry (11 files; 12 portable files including the sum list). All 200 author archive members match recorded paths, modes, lengths and SHA256, totaling 2,340,123 decoded bytes. Every member also matches its retained original author artifact. The portable source manifests, unchanged-base hashes and root check log hashes match independently observed content.

Inspected baseline/RED/GREEN and all final run source snapshots against their retained hash maps. Final native/fast coverage, Clippy, CTest, formatting, whitespace and default-graph snapshots match the current candidate. The only test changes between contemporaneous RED/GREEN and final source are the helper's narrower name and its call site. This preserves RED provenance while correcting an overly broad helper name.

Final evidence records:

- Three registered CTests pass: fast unit, packaged assets and native X11. The complete editor CTest selection finishes in 2.16 seconds; native case execution is approximately 0.78 seconds in the direct runs.
- Native all-target Clippy with warnings denied, formatting and whitespace checks pass.
- Scoped strict OpenSpec validation passes. Spec coverage passes with its preexisting 239-scenario ratchet backlog explicitly reported; no backlog completion is claimed.
- New production header line 62 executes 10 times in final native coverage.
- Native owner: 130/135 lines, 11/13 functions, 185/204 regions. Fast native owner: 0/135 lines, deliberately separate. Fast asset owner: 52/52 lines.
- No exclusions or branch-coverage claim. The five uncovered native lines and 19 regions are inherited from the reviewed published owner. IPC-refusal, alternative-display, transport-loss and related external error-direction gaps remain visible.
- Mutation is excluded by recorded user direction, not presented as passing evidence.

A new full source-free build guard was not run. Its prior published evidence remains applicable to this unchanged default/build boundary: only existing native-feature implementation/tests changed, and the relevant CMake/manifest/lock/default graph match the base. Current native and packaged-resource checks supply the affected boundary evidence. This is explicit reuse, not a claim of a new guard run.

## Safety, test quality and simpler alternatives

The fixed header places the security decision at one existing concrete response edge. It adds no policy configuration, public authority, dependency or native backend. No production extraction was needed. A configurable policy builder or JavaScript fetch interception would add complexity or fail to enforce the same browser boundary; a unit test of the response string would not prove WebKit enforcement. No materially simpler in-scope implementation is apparent.

The test uses an ephemeral loopback listener, nonblocking accept, finite socket read/write timeouts, an 8192-byte request-header bound, a stop flag and a joined worker. Rust local destruction releases inspection guards before endpoint cleanup, including assertion unwinding. The native owner is explicitly dropped through the established destructor that disconnects IPC before releasing the view/container. The existing single GTK runner avoids introducing initialization on another Rust test thread. Its CTest timeout also bounds catastrophic runner failure. This controlled fixture is not a production HTTP server or an adversarial slow-client contract.

The helper provides meaningful regression protection at the real consuming boundary, observes browser/network behavior rather than private representation, runs in under a second in the recorded ordinary runs, and has readable positive/negative oracles. Its listener complexity earns its cost by independently distinguishing enforced blocking from failure elsewhere. Retaining the explicit native boundary lane is appropriate despite this short execution time because real X11/WebKit is required. No new lane membership or speculative abstraction is warranted.

No new standing guidance or memory change is proposed. Existing rules already own endpoint calibration, independent oracles and source-bound evidence.

## Approved finite finalization envelope

PASS approves exactly the five metadata transformations in `review-packet.md`:

1. Copy this review and `reviewer/verdict.json` byte-for-byte to the two named portable `review/` paths, each mode 0644.
2. Replace only the portable receipt's review object with the prescribed ordered PASS/initial-manifest/review-hash/verdict-hash fields, preserving all other data and the specified JSON serialization.
3. Replace the single exact pending-review sentence in the progress document with the packet's exact three-line text.
4. Regenerate the portable SHA256SUMS in the prescribed bytewise order.

These are five paths in total. Root must provide the complete resulting manifest and old/new mode/hash list for the required metadata-only addendum before commit. This verdict does not approve any source, test, build, specification, task-checkbox or other evidence change.

After that addendum, only the named strict scoped/spec-coverage/whitespace/final-tree validations, normal-hook commit, fast-forward-only existing integration/feature updates, unchanged-upstream fetch, normal feature push and live/main verification are authorized. No merge to main, force push, hook bypass or cleanup is included. Unexpected content, failed validation or upstream movement invalidates the review.

This PASS proves external-fetch connect-src enforcement only. Complete resource policy, navigation/new-window/download denial, real production React/fonts/backend initialization, show/hide/resize/API ordering, actual CLAP threading/timers, notes/gestures/session/audio continuity, installed packaging and named DAW acceptance remain subsequent gates.
