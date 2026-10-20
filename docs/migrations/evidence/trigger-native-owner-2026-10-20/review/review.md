# Independent completion review — first native Trigger owner

**Verdict: PASS**

The frozen candidate implements and evidences the bounded native ownership component. No in-scope correction remains. This verdict covers only the candidate and finite publication envelope identified below. OpenSpec task 3.2 and the full beta remain incomplete.

## Candidate identity and independence

- Worktree: `/tmp/dandrum-lanes/runtime`; branch: `lane/trigger-runtime-20261020`.
- Base, current HEAD, fetched upstream `origin/feat/juce-free-host-adapters`, and integration base: `6df759172f51556356a8901c53fedfa25ff9106f`.
- Independently reconstructed manifest: **70 paths**, SHA256 `845fda27496326e147a12ed1d44688e2bca4aa935c67f752d1179938e4606757`.
- Reconstruction compared the base tree with every index entry, rejected unmerged/untracked/unstaged candidate state, checked regular-file modes, compared worktree bytes with staged blobs, independently computed blob IDs and SHA256, and matched the supplied serialized manifest exactly. No unrelated path was excluded.
- Reviewed applicable project/shared instructions, agentic-delivery completion review, coverage-and-mutation, authoritative OpenSpec proposal/design/tasks and both delta specifications.
- This was an independent read-only review. No source/index/tracker edits, builds, Cargo commands, native launches, downloads, commits, cache mutations or delegated approval occurred. Only external reviewer artifacts were written.

## Task-to-implementation matrix

| Task or bounded portion | Implementation and evidence | Conclusion |
| --- | --- | --- |
| 3.2: initial native child acquisition and ownership | `NativeEditor::create`, checked `parent_connection`, `build_child`, owned GTK container and ordered WebView/container destruction | Real initially hidden 1200×800 child under an externally owned X11 parent. |
| 3.2: host-driven servicing | `service` and UI-thread check | At most 64 nonblocking GTK iterations per caller tick; no private loop. |
| 3.2: fixture local resources and IPC | Existing immutable `EditorAssets`, custom application scheme, explicit WebKit message handler, navigation after registration | Real external JavaScript and 404 response produce the independently expected IPC value. |
| 3.2: acquisition and destruction cleanup | RAII container, disconnectable IPC signal, checked post-WebKit reparent, GDK synchronization | Native and callback cleanup are observed on success and the exercised failure boundaries. |
| 5.1: applicable component verification | Optional native CTest lane, pinned Linux dependencies, source-bound final tests/coverage/lint/build, source-free scaffold guard | Evidence supports this component; remaining product and external-boundary gaps stay visible. |
| Remaining 3.2, 3.3, 4.x and full increment | No implementation or completion claim in this slice | Correctly remain open. No whole canonical scenario is newly mapped or synchronized. |

## Criterion-to-test matrix

All new native assertions execute in `native_x11::tests::real_child_loads_local_content_and_drops_without_owning_the_host_parent`, including its acquisition helper. One explicitly selected real-platform test keeps GTK initialization and object ownership on the same test thread. The ordinary asset suite remains separate.

| Criterion | Oracle and observation | Assessment |
| --- | --- | --- |
| Supplied host owns the parent; editor owns one child | Independent x11rb connection queries exactly one direct child for each of two create/destroy cycles | Checks real parenting, not an internal counter. |
| Child starts hidden at origin with fixed initial dimensions | X11 `MapState::UNMAPPED`; geometry equals literal `(0, 0, 1200, 800)` | Repaired independent oracle detects dimension drift. |
| Local external script executes through resource handler and IPC | Fixture HTML loads `/app.js`; script fetches `/absent`, checks 404, and emits literal `local-owner-ready`; Rust expects that literal | Both successful resource serving and missing-resource response are meaningfully checked. |
| Successful destruction releases owned native/browser/callback state | Parent remains queryable with no child; GTK toplevel count returns to baseline; WebKit and callback weak references expire; messages do not increase after servicing | Exercises actual lifetime behavior across repeated construction. |
| One external tick is finite even with a ready source | Ready GLib source self-stops at 65 if the bound regresses; service returns exactly 64 and observed source calls are positive and bounded | Fails promptly on a missing budget instead of depending on a timeout. |
| UI-thread entry discipline | Public `create` and `service` reject spawned-thread calls with `WrongThread` | Rejection precedes GTK object access; owned GTK/WebView types prevent safe cross-thread owner transfer. |
| Invalid parents do not acquire owned windows | Zero, nonexistent and InputOnly parents reject; parent tree and GTK baseline remain intact | Supported failure cases use actual X server responses. |
| A failed reparent after WebKit construction rolls back ownership | Validate then destroy a parent; actual checked reparent fails after `build_gtk`; no new root orphan, baseline GTK count, registered resource callback weak reference expires | Distinct post-WebKit edge rollback is proved. |
| Earlier Wry builder failure releases already owned container | Exact `DuplicateCustomProtocol("calibration")` variant; original retained owner unchanged; no extra GTK toplevel; rejected unregistered callback expires | Accurately proves pre-WebKit builder failure only. |
| Native dependencies remain optional and outside host core | Feature-gated Linux module/declarations; recorded default dependency tree; unchanged source-free guard checks host dependency graph, targets, copying and selected runtime CTests | Default editor remains std-only; format-neutral host receives no GUI dependency. |

## Evidence and provenance

The final seven source-path hashes match every inspected review1 snapshot: native and fast LLVM runs, three registered CTests, all-target native Clippy with warnings denied, formatting and whitespace checks. Root's successful non-test native library build also binds all seven final source hashes. Native production source remains `a407d045b0a413c4c9a53a854cf9191976cac692f5c9df82ca1339e0144a0dd5` through the review repair.

Final registered editor CTest is 3/3. The complete unchanged source-free scaffold guard passes at its default profile: exit 0, 89.453992 seconds. Its selected runtime CTests are also 3/3: trigger-host, default trigger-editor and dynamic CLAP host. Independently matched all 578 guard input hashes to the frozen candidate. The full guard checks build isolation, resource copying/rebuild, dependency direction and runtime fixtures; it does not claim production browser rendering.

The three earlier storage-floor interruptions remain explicitly incomplete. Their logs are not promoted into passing results, and pre-existing stage logs have a provenance disclosure. User-approved removal was limited to the named 30 GiB debug cache; retained cache relocation has exact preservation receipts. Final successful default-profile evidence supersedes neither the historical failures nor their disclosure.

Verified all 61 entries of portable `SHA256SUMS`; all 698 author archive members match recorded paths, byte lengths, modes and SHA256 (8,467,457 decoded bytes). Verified decoded hashes and lengths of all 39 guard and five root gzip artifacts. Root strict scoped OpenSpec/spec-coverage/whitespace log hashes also match their receipts.

Original creation-stub RED and strengthened external-script RED have matching contemporaneous source snapshots and fail at the unavailable owner. Later acquisition and budget tests are correctly described as GREEN characterizations. The retained intermediate IPC lifetime failure lacks its contemporaneous failing implementation snapshot; it is historical evidence with that limit, not a fully reconstructable RED. The canonical cross-connection teardown failure has an exact pre-run snapshot and three comparable passing synchronization repeats.

Independent pinned Wry source inspection confirms that the shared-context duplicate is accumulated by the public builder and returned before backend WebKit allocation. The corrected investigation retains the earlier wrong claim as superseded and asserts the exact observed variant. The separately exercised checked reparent is after successful WebKit/protocol construction. No Wry-internal post-WebKit constructor failure is claimed as reproduced.

## Coverage and test effectiveness

Raw final LLVM exports independently confirm:

| Measured owner/lane | Lines | Regions | Functions |
| --- | --- | --- | --- |
| Native owner, native lane | 129/134 | 184/203 | 11/13 |
| Native owner, fast lane | 0/134 | 0/203 | 0/13 |
| Existing assets, fast lane | 52/52 | 86/87 | 5/5 |

No exclusions or branch counters are present. Covered code is checked through external geometry/tree counts, literal IPC, explicit errors, weak lifetimes and a finite independent event-source oracle; the coverage score alone was not accepted as proof.

Remaining native gaps are the fresh-private-manager IPC-registration refusal path, non-X11 display rejection, connection establishment failure and reparent enqueue transport failure, plus disclosed region directions. The private manager prevents user-triggered duplicate registration under pinned Wry; the non-X11 branch is publicly reachable on another backend, not globally unreachable. Broken-server/auth/transport conditions were not induced on the shared display. Actual checked server rejection after WebKit construction is exercised. These limitations are explicit and proportionate to this unwired platform component; they do not justify a 100% or complete native acceptance claim. The module is new rather than extracted. Mutation remains excluded by recorded user direction, with no mutation-pass claim.

The native test depends on real GTK/WebKit/X11 and is explicitly isolated from the fast lane. Private helpers are used only to stage a real parent-disappearance race and calibrate Wry's builder failure. Those tests assert external ownership effects, avoiding a fake backend whose cleanup contract would itself need proof.

## Architecture, dependencies and alternatives

The concrete owner stays at the optional editor edge. It validates the actual GDK X11 display and a separate checked X11 connection before allocating its native child. Wry receives an owned GTK container rather than an unchecked foreign parent. The view drops before the GTK container; the IPC handler is explicitly disconnected first. GDK synchronization makes the deletion visible to an independent connection when teardown returns.

The small owned-window wrapper and explicit signal identity are justified ownership boundaries. Pinned Wry's built-in IPC handler strongly captures its WebView, so the explicit disconnectable handler addresses a real lifetime concern without introducing a general backend abstraction. A broader failure-injection interface or fake-native layer would add machinery without improving this bounded proof.

Direct Linux dependencies match exact Wry/GTK/gdkx11/x11rb/WebKit pins. Independently compared the retained investigation lock: no prior registry package/version/source tuple was removed or upgraded; six x11rb-related packages were added. The retained Linux metadata resolves 106 packages. All 191 retained license-text hashes match. Rust declarations/texts include option-ext's MPL-2.0 declaration; system shared-library and eventual redistribution obligations remain separate, with no installed-packaging clearance claim.

The resolved geometry finding reinforces the existing independent-oracle rule; no new standing instruction is needed. The corrected public-builder/backend error trace is retained as an evidence lesson in the investigation. No global guidance or memory was changed.

## Findings and publication envelope

R1 is resolved: the native geometry expectation now uses independently specified literal dimensions, rather than production constants. The subsequent shared test-fixture import and redundant-closure repairs are test-only, compile cleanly with warnings denied and preserve production bytes. No actionable finding remains.

PASS approves only the packet's five deterministic metadata transformations: exact reviewer report/verdict copies, the specified receipt review object and JSON serialization, the exact single progress-paragraph replacement, and a byte-sorted SHA256SUMS regeneration. Root must supply the complete transformed manifest and old/new hash/mode list for the required metadata-only addendum before committing. Source, tests, build logic, requirements, task checkboxes and other evidence remain unchanged.

The finite remaining envelope is the named validation commands, exact final-tree verification, normal hooks commit, fast-forward-only existing integration/feature worktrees, fetch requiring the recorded upstream object, normal feature push and live/main verification. No merge to main, force push, hook bypass or additional cleanup is approved. Any unexpected content, failed validation or moved upstream invalidates this verdict.

This PASS does not complete task 3.2 or prove production React/session initialization, browser fonts/network policy, show/hide/resize/API ordering, actual CLAP GUI/main-thread/timer integration, notes/gestures/session continuity, uninterrupted audio, installed products or named DAWs. Those remain the explicit subsequent gates.
