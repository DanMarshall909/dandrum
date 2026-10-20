# Independent completion review: native visibility and fixed size

Verdict: **PASS**.

This verdict covers the bounded native-owner presentation slice of OpenSpec task 3.2, not completion of task 3.2 or the full Trigger beta. No actionable in-scope correction remains. Normal-hook publication and the exact finite metadata finalization below remain execution gates. The same reviewer must verify the final 22-path manifest before publication.

## Identity and independence

Reviewed read-only `/tmp/dandrum-lanes/runtime`, branch `lane/trigger-runtime-20261020`. Independently reconstructed the complete staged/worktree/untracked candidate from Git porcelain, stage entries, actual file bytes, canonical modes, staged blobs, and bytewise path order. All 20 paths are represented; no unstaged or untracked paths are excluded. All are ordinary 100644 files and each staged blob equals its worktree contents. Re-serialization exactly matches the supplied manifest, SHA256 `452cfd1404be3f69f5024517416d73b46fd5c17c41610cea1239ade6031463b4`.

Base, current HEAD, explicit integration base, current remote-tracking `origin/feat/juce-free-host-adapters`, and local existing feature tip all equal `9f9252cfee895f30e659e2f87a42bc833eef0e5a`. The lane itself has no configured tracking upstream; the packet's explicit existing-feature integration/publication target resolves that intentionally, and no lane-upstream claim is made. Main checkout is clean at `68dd6553a2f07e71e2fe09ee1d7094d518559937`. Root's immediately-pre-freeze fetch is retained context; I independently verified the local integration objects, without performing a fetch or other ref mutation.

Read project AGENTS.md, shared AI rules, completion-review, behavior-first-tdd, coverage-and-mutation, OpenSpec and reflection instructions. Read current OpenSpec status/apply output and all returned proposal/design/spec/task contexts. Review was independent of implementation, without delegated decisions, repository/index writes, native execution, builds, test execution, cache mutation or process ownership. Only this report and verdict JSON are written. Raw tests were inspected, not independently rerun by this reviewer.

## Task-to-implementation matrix

| Task or claim | Candidate implementation/evidence | Assessment |
|---|---|---|
| 3.2 native show/hide | `NativeEditor::set_visible`, existing GTK child and display synchronization; physical X11 oracle in existing two-cycle native test | Bounded presentation behavior implemented and evidenced |
| 3.2 exact fixed size and rejection | `NativeEditor::set_size(u32,u32)`, widening constants and `UnsupportedSize`; 4 accepts, 40 rejects across shown/hidden states | Implemented without narrowed input, native mutation or mirrored state |
| 3.2 parenting/local content/disposal regression | Existing owner constructor and Drop unchanged executably; existing native suite extended with presentation exercise | Adjacent boundary regression evidence retained |
| 3.2 complete host lifecycle | No CLAP facade, order/API/timer integration, full resource policy, session reopening or high-DPI proof added | Correctly remains unchecked; 7/14 tasks complete |
| 3.3 and 4.x integration | No session/backend/automation/audio changes or completed claims | Remain open; actual CLAP integration belongs to 4.2 |
| 5.1 focused verification | Portable raw runs, profiles, LLVM exports, source snapshots and qualified guard reuse | Supports this slice, not the whole final beta verification task |
| 5.2 review/publication | This review plus proposed finite publication envelope | Review passed for frozen candidate; publication still pending |
| Safety clarification | Seven comment lines in `OwnedWindow::drop`, separate pinned-source alias analysis | Accurate narrowing of the explanation; no executable disposal change or universal soundness claim |

No baseline spec or mapping was changed or synchronized. The delta scenario “Actual X11 child follows the complete host lifecycle” remains only partially discharged by this slice; its live-session reopening and actual CLAP obligations cannot be inferred from the native owner tests. Other delta scenarios concerning signed audio, held notes/gestures, native backend and independent instances are not claimed here. Retained strict scoped OpenSpec succeeds; retained canonical spec coverage reports 618 scenarios, 379 mapped and 239 explicit ratchet TODOs, not complete global acceptance coverage.

## Criterion-to-test matrix

All new native assertions run inside `native_x11::tests::real_child_loads_local_content_and_drops_without_owning_the_host_parent`, registered as CTest `trigger-editor-native-x11` with `--ignored --exact --nocapture --test-threads=1` and the staged production-bundle argument.

| Observable criterion | Executable oracle | Strength and limit |
|---|---|---|
| Initially hidden child can show, hide and reshow idempotently | `exercise_visibility_and_fixed_size`: independent X11 `GetWindowAttributes` must return UNMAPPED/VIEWABLE through repeated transitions | Actual server state, not a production boolean; scale-1 selected platform only |
| Presentation keeps the host and child relationship | Independent query-tree checks same captured XID and parent, exactly one direct child; literal `(0,0,1200,800)` child geometry; unchanged host rectangle and VIEWABLE host | Independent physical oracle; catches reparent, recreation, resizing and host hiding |
| Browser/document survive presentation changes | Same WebKit object and fixed URI; ordinary page click counter emits `content|1`, twenty `state|1` receipts, then `content|2`, exactly one startup per cycle | Ordinary DOM/application state; native evaluation only triggers positive actions, not a script-policy negative proof |
| Exact declared dimensions succeed | Four `set_size(1200,800).unwrap()` calls across both states/cycles, followed by physical assertions | Expected dimensions are literals independent of production constants |
| Every unsupported input returns error without accepted-state effects | Ten distinct pairs per state/cycle: adjacent width/height, zero, u32 maximum and 65536-offset dimensions; exact `UnsupportedSize`, preserved physical state, object/URI and counter | Forty outcomes exercise width/height and narrowing faults; finite examples plus direct equality implementation justify full-u32 contract |
| Disposal works while shown and while hidden | One drop per state: exact prior XID gives BadWindow code 3 immediately, no intervening host tick/test sync, host survives, callbacks/browser weak references release, no late IPC | Retains meaningful native cleanup regression; weak disposal notification is not universal finalization proof |
| UI-thread discipline | Both new methods first invoke unchanged `check_thread`; existing service/create wrong-thread cases and native test on initialized GTK thread | New methods' error-propagation directions are unexecuted; safely moving this GTK/Wry owner to another thread is unavailable |
| Adjacent package/resource functionality remains usable | Same run invokes local fetch/CSP/navigation controls and unmodified production React observer, eight font faces and eleven exact resource responses; separate packaged-assets CTest | Positive boundary calibration retained; real JUCE-free backend/session initialization remains future work |

## Behavior-first evidence and test effectiveness

The archive retains the first absent-API RED, minimal GREEN, initial coverage, covered test refactor, exact final-test RED, pre-comment final evidence and release verification. I checked all fourteen listed phases against their own 16-path snapshots and digests. The final-test RED has exactly the release test bytes and all other final inputs except the owner file. Both RED logs contain eight E0599 errors for the absent inherent presentation methods/error variant and no other compiler-error class. Exit is 101. These establish test-first missing-API evidence, not a runtime mapping/rejection failure, setup failure, or native fault calibration. The trait suggestions originate from the GTK prelude; they do not supply the missing owner APIs. The report preserves that distinction, and I do not infer runtime regression sensitivity from a compiler exit alone.

The GREEN native assertions directly pin independent geometry, map state, object/content persistence and explicit rejection. They would detect observable no-op/inverted visibility, wrong sizes, narrowing, replacement and host side effects by inspection of the assertion structure. No mutation or injected-fault result is claimed; mutation is excluded by recorded user direction. A fresh runtime fault campaign is not silently substituted for that exclusion.

Regression protection is strong at the external boundary. Refactoring resistance is good for physical and page assertions; WebKit-object identity and temporary toplevel counts intentionally protect the accepted retention/lifetime contract. Feedback cost is justified by real GTK/WebKit/X11 observations, with the new exercise reusing existing cycles. Native body timings are comparable (3.60, 3.59, 3.54 seconds) and retained in the named native boundary lane. Current fast assets tests finish in 0.00 seconds after setup; the complete selected CTest set takes 5.011941 seconds. Tests remain readable through a focused assertion helper and expected message sequence. No production module extraction or test-only production API was added. A second fixture, browser abstraction or state mirror would increase maintenance without improving this slice's oracle.

## Evidence and coverage audit

Independently verified the archive's complete 471-member index, byte lengths and SHA256 values, plus every portable file covered by SHA256SUMS. All fourteen phase manifests match every one of their 16 snapshot paths; run exits, elapsed times and log digests agree. All eight `release-*` phases match the complete final 16-path source set and its digest `32083444049ce3834b99710d600605aa59bcf1c857961735521bc6ffc6c54b06`. Earlier `final-*` artifacts are correctly retained as pre-comment evidence, not relabeled exact final bytes.

Inspected raw native output, CTest LastTest.log and commands, denied-warning Clippy, native library build, formatting/default dependency graph records and raw LLVM export/annotation. The three registered CTests pass; no missing or skipped native test is counted as success. Coverage owner summary independently matches raw LLVM: 148/153 lines, 14/16 functions and 208/229 regions. All seventeen new executable lines have counts: set_visible 11 calls (6 show, 5 hide), set_size 44 (4 accepted, 40 rejected). The two new `?` error propagation directions remain gaps, despite covered lines. The five inherited uncovered lines and nineteen inherited region directions remain explicit, including IPC registration refusal, unsupported display and transport/race paths. Zero LLVM branch/MC/DC counters establish no such coverage. Fast assets coverage is separately 52/52 lines and 86/87 regions; its native owner has zero coverage and is not combined to disguise native gaps.

These are execution measurements plus separately reviewed behavioral assertions. They are not a 100% owner claim or a universal platform/error-path proof. No unsafe cross-thread fixture or artificial seam should be added only to increase the score.

## Qualified default-guard reuse

Independently reconstructed the entire guard input set by the retained guard's directory/exclusion rules: exactly 583 current files, same pathset as the published guard manifest. All current file digests match `guard-reuse-current-inputs.json`. Exactly the two native owner/test files differ; the other 581 match. Verified the old manifest SHA256 `87c00e80f435b1c217438bf9ccaf9b2c4a446b2c0680f1dc38aa40579ba3af62` in the published disposal archive and retained successful run/commands.

The unchanged Cargo manifest has empty default features. `lib.rs` cfg excludes the whole native module and its nested tests from default compilation. The unchanged maintained Python guard builds/runs that default boundary, and its native-ON configure only inspects CTest inventory/arguments. The native owner bytes therefore do not enter the reused compiled behavior; all CMake/Cargo/lock/Python/resource/host inputs relevant to that execution are identical. Reuse is applicable to this precise default source-free build and inventory claim. It is neither a fresh guard run nor proof that the old 583-input manifest equals current bytes or that old execution proves new native behavior. Exact-source native compilation and boundary execution separately cover the latter.

## Architecture, safety and alternatives

Only two source files change. No engine/host/CLAP/frontend/CMake/dependency changes or audio-thread operations are introduced. Visibility is a direct toolkit transition with display synchronization, with no nested event loop, duplicate lifecycle state, browser reconstruction or added unsafe operation. Size validation widens u16 constants to u32; it cannot truncate host dimensions. Accepted size calls deliberately perform no mutation, which is the simplest implementation for a fixed-size child. Both methods respect existing GTK thread affinity, while actual CLAP callback-thread/order/timer discipline remains unproven until 4.2.

The destructor comment correction is justified. I independently inspected pinned gtk-rs destroy/list_toplevels, GLib disposal documentation and Wry Drop sources and matching hashes. GTK enumeration can produce retained owned aliases; private NativeEditor fields do not prove global uniqueness. Local cleanup releases IPC/browser activity and uses only a captured display after its own container destruction. The comment accurately distinguishes that controlled behavior from a guarantee about external safe GTK callers. The generic alias/early external close soundness question remains explicitly unresolved; no particular UB sequence was demonstrated. This review neither turns dependency precedent into a soundness proof nor treats alias retention alone as a new concrete memory-safety defect. Historical evidence remains immutable. A concrete future unsafe interaction would need its own source-specific diagnosis and safe regression oracle.

Simpler/safer alternatives considered: a boolean mirror would diverge from native state; rebuilding the view would violate document retention; narrowing dimensions would admit modulo matches; blindly resizing on an accepted request adds effects unnecessary for the fixed contract; raw X11 map/destruction would duplicate or bypass toolkit lifetime management. Returning to deferred close would lose the already accepted immediate-XID disposal contract and would not remove aliases. The direct existing-owner methods are preferable within scope. A broad new abstraction, backend replacement or forced process isolation has no demonstrated need here.

## Publication envelope

The initial candidate may undergo exactly five metadata-path transformations, yielding 22 total changed paths:

1. Add exact copies of this `review.md` and `verdict.json` at `docs/migrations/evidence/trigger-editor-visibility-2026-10-20/review/`, both mode 100644.
2. Replace only `receipt.json`'s review object with PASS and the initial candidate manifest digest plus exact report/verdict digests; no other receipt meaning changes.
3. In `docs/migrations/juce-replacement-progress.md`, replace exactly one sentence, `Independent completion review and normal-hook publication are the next gates.`, with `Independent completion review passed for the frozen source/evidence candidate; normal-hook publication is the remaining gate.`
4. Regenerate that evidence directory's `SHA256SUMS` for every evidence file except itself.

Root must supply exact old/new hashes and the independently reconstructible final manifest for this reviewer's addendum. No source, test, archived evidence, requirement, task checkbox, spec map or status-completion changes are permitted by this envelope. A semantic repair returns the whole candidate for review.

After the addendum confirms precisely those transformations, root may commit `Present fixed-size native Trigger editor` with normal hooks, verify the committed tree against the final manifest, fast-forward only the explicitly existing integration/original feature, fetch without integration and require the recorded upstream object, then normally push that feature and verify live refs/status/main. Hook configuration currently resolves to `.githooks`; inspected pre-push runs engine tests and selects mutation only for changed rust-engine source, which this candidate does not contain. No hook bypass is authorized or claimed. Any failed validation, unexpected bytes, ref movement or rejected push invalidates publication under this verdict.

The external publication receipt and existing deferred-work ledger may receive only the already authorized status append. No additional outbound message, merge to main, archive, beta completion claim, cache deletion/relocation or unknown-process cleanup is permitted. All reusable/ambiguous worktrees, builds, caches and processes remain preserved.

## Lessons and release

A material lesson candidate is already recorded in `global-alias-safety.md`: Rust field privacy does not exclude toolkit-global alias discovery. The appropriate owner is the native toolkit safety explanation/review procedure; the useful future action is to inspect global/native handle paths before asserting exclusivity. The downside of an overbroad rule would be speculative backend rewrites without a concrete unsafe path, so retain precise local claims and investigate specific operations. No standing guidance or memory was changed.

A second candidate is already applied in the evidence: reused verification must name the effective compiled/configured boundary and changed excluded inputs, rather than present a stale whole-source digest as current. No additional guidance change is necessary for this candidate.

All review reads are complete. No mutable native/build/cache resource was acquired. Ownership remains exclusively with root. Task 3.2, the remaining policy/session/CLAP work and full beta remain open.
