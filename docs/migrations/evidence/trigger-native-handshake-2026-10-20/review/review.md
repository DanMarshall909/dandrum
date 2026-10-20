# Independent native handshake completion review

Verdict: **PASS** for the frozen Wry library initialization/delivery component and the exact finite publication envelope. No in-scope source or test correction is required. This is not completion of task 3.2, task 3.3, the actual Trigger CLAP product, or the full beta.

Reviewer: independent agent `/root/native_handshake_review`, 20 October 2026. I did not implement this candidate, delegate its approval, execute tests/native code, or modify repository/source/index/cache content. I read the applicable AGENTS/shared rules, agentic-delivery completion reference, coverage-and-mutation, security-by-design and reflect guidance, accepted proposal/design/tasks and both delta specifications. Findings below derive from direct source and retained-evidence inspection, not the packet's requested outcome.

## Candidate and integration identity

Worktree `/tmp/dandrum-lanes/integration`, branch `lane/trigger-integration-20261020`; base, HEAD, recorded publication upstream `origin/feat/juce-free-host-adapters`, and merge base all independently resolve to `b52a3b406c6f88f53308fdf52b112702c2b11bd8`. The complete 11-path manifest SHA-256 is `c6c03e8709846631599fd0e092ae7f2b797b3236f40e1e2ad6c81da9485b3330`.

I compared every manifest entry against porcelain status, index mode/object, staged bytes and worktree SHA-256. All paths are regular nonexecutable 100644 files, staged bytes equal worktree bytes, and the integration worktree has no extra tracked/unstaged/untracked changes. Original feature publication configuration points to origin and `refs/heads/feat/juce-free-host-adapters`. Main remains at `68dd6553a2f07e71e2fe09ee1d7094d518559937`; other worktrees are outside this candidate and must be preserved. Fetch timing is the author's retained prefreeze assertion; my independent check confirms the locally recorded upstream object. The unchanged-upstream fetch remains mandatory before publication.

## Task-to-implementation matrix

| Owning task or claim | Implementation/evidence | Decision |
| --- | --- | --- |
| 3.3: initialize and deliver through concrete native browser | `NativeEditor::create_initialized` installs host script before fixed entry load; `evaluate_script` checks GTK thread and delegates to Wry | Component complete only |
| 3.2: existing preview/library lifecycle | `create` delegates with empty script; existing custom scheme, navigation, parenting and destruction paths remain shared | Library regression evidence retained; actual CLAP order/thread/API acceptance remains open |
| 3.3: joined session, broker, notes/gestures and reconnect | No new session or protocol facade wiring in these two changed Rust files | Open, correctly not claimed |
| 4.x and full beta | No exported Trigger release identity, CLAP GUI projection, VST3/device/DAW/offline acceptance in this diff | Open |
| 5.1/5.2 component evidence and publication | Source-bound baseline/RED/GREEN, native and fast coverage, three registered editor CTests; current independent review and finite finalization | Evidence sufficient for this component; hooks/publication still pending |

The tasks file is unchanged with seven of fourteen tasks complete. No canonical acceptance scenario is newly marked proved, no spec-test fingerprint is silently refreshed, and the design addition explicitly describes a prerequisite rather than full scenario satisfaction.

## Criterion-to-test matrix

All new assertions run inside `native_x11::tests::real_child_loads_local_content_and_drops_without_owning_the_host_parent`, through helper `initialized_child_consumes_native_delivery_before_releasing_its_callback`.

| Criterion | Executable oracle and layer | Assessment |
| --- | --- | --- |
| Initializer precedes first packaged application script | Real app.js reads bootstrap immediately; exact IPC `boot|9007199254740993|host π` | Independent literal oracle covers order, precise decimal string and Unicode |
| Native delivery is actually consumed | Native call invokes receiver; receiver sets DOM text, then reads it back into exact IPC `reply|18446744073709551615|-0.5|9007199254740993` | Tests real WebKit/DOM/IPC, not merely successful queueing |
| Initialized callback is released | Weak callback owner cannot upgrade synchronously after `drop(owner)`; eight later host ticks leave message count at two | Strong release assertion plus representative no-later-message check |
| Empty initializer keeps prior preview behavior | Existing `create` callers run existing two-cycle parenting/visibility/size/content/destruction tests and production shell/eight-font tests | Actual established consumer, not speculative compatibility |
| Existing error cleanup and UI ownership survive | Existing fixture rejects creation/service on wrong thread and invalid parents; acquisition and immediate parent-destruction tests retained | Shared constructor/lifetime behavior remains covered; no claim of every OS failure |
| Local resource and document restrictions survive | Existing CSP/resource and fixed-entry navigation helpers in the same native test | Policy remains unchanged and independently exercised |

The new DOM value `-0.5` is a literal text oracle, not an audio-output assertion. The new initialized helper does not itself schedule hostile late work; synchronous weak-owner release and the retained broader lifecycle tests provide the meaningful cleanup protection. No emitted protocol rejection/event, lost-acknowledgement recovery, or real session facade is implied.

## TDD, coverage and test effectiveness

All 227 archived regular members are unique and match artifacts.json sizes/digests; all portable SHA256SUMS entries match. Run receipts' log hashes match. All seventeen captured crate files match current source in each GREEN/Clippy/fast/configure/resource/CTest/fmt/default-graph phase. Baseline and RED production native source equal HEAD. RED test content differs from GREEN only in rustfmt layout of two assertions.

The unchanged real native baseline passes in 6.781459 seconds. RED exits 101 in 0.504712 seconds with E0599 specifically for absent `create_initialized`; this is honest missing-API compilation RED, not proof of a runtime failure calibration. New assertions existed before production changes. Native GREEN passes in 5.029944 seconds; strict all-target Clippy passes in 0.755964 seconds; formatting and default dependency checks pass; four fast tests pass. The separately configured CMake fixture points to the integration source, stages production resources with the repository resource target, and runs all three registered editor CTests successfully in 4.775284 seconds (CTest reports 4.67 seconds). The rust-target link refers to the retained shared editor target; this is cache reuse with checked source identity, not an isolated clean build.

Native LLVM summaries independently read as baseline 148/153 lines, 14/16 functions, 208/229 regions; GREEN 163/168 lines, 16/18 functions, 224/246 regions. The fifteen additional counted lines are hit. Fast/native exports bind the same binary SHA `323a78c3fc1368c5f0e24d2d3b420cd3bbd70c479708253ebf59b96155945e85`, and raw-profile hashes match. I independently hashed the retained 84,220,824-byte binary through read-only host access. Ordinary sandbox /dev/shm showed a different empty view; no binary disappearance or cache deletion occurred. The fast profile executes none of this native owner, so native execution evidence belongs to the separate real-boundary lane.

Coverage precision: the packet's phrase “five physical zero lines” should be read as five inherited LLVM counted line gaps. The rendered source shows three entirely zero-hit physical lines (109, 110, 172), while the inherited error closures at 175 and 229 have zero-count regions on otherwise executed physical lines. This conservative terminology does not conceal an additional gap or change the verdict; this report supplies the exact accounting. Generic instantiation counts are distinct from source-function coverage. No branch/MCDC or assertion-completeness result exists.

The newly unexecuted error-propagation region is the `check_thread()?` return at line 120. A live NativeEditor contains GTK/WebKit owners that safe Rust cannot send/share onto an arbitrary worker, and valid construction already requires GTK's initialized main thread; adding unsafe cross-thread transport or a fake native backend merely to hit this guard would weaken the design. The shared WrongThread behavior is independently exercised for construction/service. Pinned local Wry 0.57.0's WebKitGTK `eval` queues/runs JavaScript and returns Ok; it does not synchronously return script exceptions. Consequently the wrapper's mapped WebView error is not a testable JavaScript-rejection contract on this backend. These limits are acceptable for this trusted queueing API and are not generalized to future backends.

Mutation is excluded by user direction, not reported passing. The one native fixture avoids competing GTK test threads; the helper reuses parent ownership and bounded waits with exact outcomes. A shared 15-second deadline covers both stages and bounds total waiting. Existing tests stay intact. Production refactoring sensibly delegates once rather than duplicating resource/IPC construction. No new extracted module needs separate coverage.

## Architecture, lifetime, bounds and alternatives

The change adds no dependency, manifest, unsafe operation, engine/DSP policy, CLAP type or wire parser. Initializer strings become owned by Wry; the child is constructed before manual IPC registration and the fixed document is loaded only after registration. Existing Drop disconnects IPC before browser/container fields are destroyed. Fixed-entry navigation and local CSP remain in place.

Trusted host JavaScript is intentionally arbitrary execution authority. This method is not an untrusted browser IPC dispatcher, and it must only receive scripts prepared by the caller's bounded/admitted/escaped protocol facade. Adding a second serializer or policy layer here would duplicate the accepted broker design. A typed reply-only native API could later narrow a completed facade, but it would couple this generic execution edge to still-unjoined protocol semantics now. Reusing Wry's document-start initializer is clearer and safer than post-load injection or HTML rewriting; the real first-script assertion demonstrates the required X11 behavior. No invented readiness guarantee, JavaScript exception reporting, audio health or success of unsupported capabilities is introduced.

## Source-free and release qualifications

I independently compared the retained reference manifest hash `e04fc360aefe42fa70c65e0a0cc98c608705a362d8f0d94d1c225afdf7567218`, all 587 current input hashes, and the exact changed set: 585 identical, only native_x11 production/test files changed. Current cfg/feature declarations exclude both from the default source-free graph, whose retained cargo-tree output contains only dandrum-trigger-editor. Published snapshot evidence records the earlier affected source-free release/dependency/three-CTest pass. Reusing that affected-boundary result is justified; it is not a fresh complete guard, unchanged whole-artifact claim, native release product build, or actual Trigger CLAP acceptance. Native all-target compilation and registered real-boundary CTests are sufficient for the explicitly limited library component.

## Publication and operational envelope

The external approved-envelope.json fixes the exact five metadata outputs and complete thirteen-path final manifest. Only byte-for-byte report/verdict copies, the specified receipt.review replacement, one exact progress sentence replacement, and deterministic SHA256SUMS regeneration are authorized as content changes. Recheck the initial manifest first; apply those bytes; run scoped strict OpenSpec validation, spec coverage, staged and unstaged diff checks; then return the actual final manifest and validation evidence to this same reviewer for an external addendum before commit.

Commit and push must use normal hooks. Active core.hooksPath is .githooks; I read the actual pre-commit coverage and pre-push test/changed-engine mutation entrypoints. No changed engine files means the existing mutation selector should skip normally, without bypass. Publication must fetch without integration and require upstream b52a3b406c6f88f53308fdf52b112702c2b11bd8, then push non-force from the configured original feature. Any content/upstream drift, validation failure or rejected push invalidates this approval. Byte-preserving fast-forward of clean original feature/frontend worktrees is administrative; integration of dirty runtime/protocol lanes is outside this candidate and occurs later under separate checks. No merge to main is approved.

Recorded run process groups have no live processes in my read-only scan. Preserve all evidence, reusable targets and caches. Resource headroom must satisfy the root's hook floor before execution. This verdict grants no deletion authority for the distinct 24-GiB cache; a separate user decision is pending and the earlier named 30-GiB path is already absent. A resource decision does not itself change candidate bytes, but cleanup must remain within its exact authority.

No new standing-guidance change is required. The useful review lessons—LLVM aggregate versus physical-line accounting, queueing versus acknowledged consumption, and sandbox versus host shared-memory visibility—are recorded here without changing global rules or memory.
