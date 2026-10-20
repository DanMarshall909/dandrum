# Issue #24 implementation evidence

Scope: [JUCE replacement](https://github.com/DanMarshall909/dandrum/issues/24), retaining the existing Rust engine and canonical React UI. The [investigation](../investigations/juce-replacement-investigation.md) and its scope correction are integrated in main at `68dd6553a2f07e71e2fe09ee1d7094d518559937`. No legacy JUCE state/project/ID compatibility is required.

This document records implementation evidence as slices land. An independent engine build is a prerequisite; it does not meet the full issue definition of done.

| Issue stage | Current evidence | Remaining required result |
| --- | --- | --- |
| J0 inventory and baseline | Investigation has 25 responsibility rows, source/resource receipt and proposed platform/candidate matrix. Existing signed native ABI baseline passes; sample-offset onset characterized. | Confirm release/licence/driver scope and capture the remaining signed-render/resource/performance comparisons. No native DAW/device baseline is claimed. |
| J1 shared boundary | Independent configure/build and absent-JUCE/browser fixture pass; retained JUCE target inventory passes. All 18 engine-only CTest checks passed, including 1,091 Rust tests after compiler-checked runtime ownership transfer. | Agree #23's minimum owned session/SDK/EditorHost contracts, adapt comparison consumer and prove independent resources/protocol. |
| J2 concrete feasibility | Final aligned-release Linux CLAP proof passes 13 dynamic-host tests, 2 signed baselines and all 20 engine-only CTests. Official validator: 53 successes, 28 skips, no warnings/failures. Four canonical scenarios are synced and mapped. Verified slice published as `11c3bd2`; live remote head verified. | Actual CLAP host/validator signed audio/events/state/lifecycle/instances; parented production React, one real gesture/feedback, lifecycle failure cases; early VST3 and second native OS evidence. |
| J3 replacement state and identity | Backwards-compatibility exclusions are agreed. | Deliberate replacement IDs, bounded versioned state, fresh-instance state/automation/assets round trips and invalid-state recovery. |
| J4 services and session | Existing non-JUCE services and remaining JUCE FFT/window/watch owners are inventoried. | Migrate covered services; consume #23's single session/journal; destructive-editor/undo and lost-acknowledgement proofs, reader/reclamation and job recovery. |
| J5 formats and devices | Existing AU/VST3/Standalone declarations remain the scope until explicitly revised. | Required actual format validators/DAWs and standalone drivers/device recovery/settings/chrome/MIDI. |
| J6 hardening and cutover | Working JUCE comparison source and reusable builds are preserved. | Cross-cutting realtime/event/reconfigure/browser/input/security/resource matrix; installed offline package, licence/link/build audits with JUCE absent, maintained launchers/CI/specs and verified fallback/cutover. |

Gate A requires J0-J2 native evidence; Gate B requires J3-J4 state/session evidence; Gate C requires J5-J6 formats/devices/packaging. All three remain open. Linux/WSLg checks cannot prove native Windows, macOS or physical-device compatibility. Product-scope/licence/ASIO questions are pending; no release support is silently dropped.

## Active full-beta continuation — 20 October 2026

The active stopping condition is the **full first JUCE-free Trigger beta**, not completion of the current CLAP increment. Deliver the production React editor over the Rust engine and persistent host-independent session, then standalone and VST3 products. Selected Linux x86_64 X11/WebKitGTK 2.50.4 acceptance must prove sample loading, mapping, literal signed playback, editor destruction/recreation with uninterrupted audio and retained edits/undo, fresh-instance patch/sample/parameter round trips, named-DAW automation and independent instances. Installed offline artifacts must work outside the checkout without any JUCE build/runtime dependency and with correct lifecycle and callback ownership. Broader OS/AU migration obligations remain visible; the selected first-beta proof does not silently close them.

Continue in `/home/dan/code/dandrum-worktrees/juce-free-host-adapters`, branch `feat/juce-free-host-adapters`. The supplied `a116c2cc3ff0d14f3fc128010db3a4e9365964e2` checkpoint is historical: verified focused commits `86c198a`, `2480ad8` and `8ccbb54` have since landed locally. Preserve these advances. The main checkout `/home/dan/code/dandrum` must remain clean at `68dd6553a2f07e71e2fe09ee1d7094d518559937`; no main merge is authorized by this continuation.

The current OpenSpec milestone is `add-juce-free-trigger-clap-editor`. Run `openspec status --change add-juce-free-trigger-clap-editor --json` and `openspec instructions apply --change add-juce-free-trigger-clap-editor --json`, then read all returned context files before each implementation wave. Its task list owns implementation status. Tasks 1.1, 1.2, 2.1, 2.2, 2.3 and 3.1 are verified; the latter two were expressly authorized as parallel lanes. Do not redo completed work or reset to the historical checkpoint.

| Next milestone work | Dependency and completion boundary |
| --- | --- |
| 2.4 gestures, host feedback and atomic version-2 state | Use the reviewed runtime/protocol; balance automation, preserve nonzero offsets and structural history, restore a fresh instance atomically and reject invalid state without partial publication. |
| 3.2 production bundle/fonts and Wry X11 child | Pin/audit Wry 0.57.0 on implementation; prove real parent ownership, host-driven servicing, local-only resources, fixed size, show/hide and creation/order/API failure cleanup. The feasibility probe is evidence, not completion. |
| 3.3 disposable editor connection | Session snapshot, mapping/job recovery, native undo, notes/gestures and exact-once destruction cleanup; retained accepted work/history across reconnect. |
| 4.1–4.3 actual CLAP product | Real Trigger identity and copied resources; GUI/parameter extensions; rendered control automation, signed offsets, state, recovery, lifecycle failures and independent instances through the dynamically loaded binary. |
| 5.1–5.2 CLAP verification/publication | Full owner coverage/audits, native/browser/build/callback checks, independent review, genuine scenario mappings, strict validation and normal publication of focused commits. This ends only the CLAP milestone. |
| Next OpenSpec increment after CLAP publication | Define and deliver VST3 projection over the same host, standalone audio/MIDI ownership, named-DAW automation/two-instance acceptance, installed offline products, complete JUCE build/runtime/link audits and fresh-instance state/resource proof. Continue until the full beta stopping condition is met. |

For every behavior change explain the accepted criterion and its observable tests; retain genuine RED before the smallest implementation, then formatting, Clippy with warnings denied, focused tests/LLVM coverage and covered refactoring. Mark tasks only after required checks pass. Commit each verified material boundary before starting the next, inspecting status, worktree/staged diffs, recent history and whitespace; stage only intended source/evidence. Normal hooks remain mandatory and currently enforce 1,084 engine tests plus strict coverage. Mutation remains explicitly excluded. Preserve reusable builds/Cargo/dependencies and check disk before coverage or large runs; never delete those caches without explicit direction.

Preserve task 2.2's capacity-one SPSC pending/reclamation handoffs: off-audio construction/scratch and destruction, block-boundary swaps, refusal until consumption/reclamation, bounded event backpressure and a callback without locks, allocation, I/O, serialization, browser access or destruction. Cross-thread allocation/deallocation sentinels, reset, signed output and independent-instance proof remain required across composed adapters. The implemented, pinned and MIT/Apache-audited `rtrb =0.3.2` boundary already passed review; the continuation's suggested `0.4.0` is a candidate rather than an instruction to replace verified code without need. Keep the format-neutral host free of Clack/Wry/GTK/WebKit/JUCE types.

The 20 October continuation explicitly authorizes autonomous completion and normal publication on the existing feature branch. Finish concrete verification/review before publication; never bypass hooks or approval review. Record missing physical-device, DAW, licence or installed-artifact acceptance honestly and continue independent engineering while resolving external prerequisites. Do not claim the beta complete from a CLI, synthetic host, library presence or the CLAP milestone alone. Keep concise progress updates at least once per minute and explain relevant Rust ownership/borrowing briefly as it arises.

## Build prerequisite receipt

- Baseline: current C++ signed-gain/reset ABI test compiled and passed against the retained Rust archive; this is distinguished from a fresh engine build.
- Red: the new absent-JUCE/absent-browser build guard and six conflicting-option cases failed against the unchanged root configuration (three methods, seven failures). Default/explicit-disabled engine-only selections still reached JUCE as expected.
- Green: fresh absent-JUCE/web/ui fixture built every independent native target and passed signed stereo/reset and literal note-offset waveform tests (three guard methods, 71.496 seconds). Retained native-only JUCE comparison configure/target/test inventory passed (one method, 2.164 seconds). README root engine-only configure/build and all 18 CTest checks passed in 106.56 seconds, including 1,088 Rust tests, both signed native tests, retirement/safety guards and independent service tests. The build-boundary check also passed through CTest (28.62 seconds). Final review and whitespace pass; code committed and normally pushed as [`fcd8eaf`](https://github.com/DanMarshall909/dandrum/commit/fcd8eaf93c74a5e17d83dc7d3424408b4761fbb4), with exact live remote/upstream match. No native DAW/device/other-OS or complete J1 claim is made.
- Acceptance: three reviewed scenarios synced and genuinely mapped; strict canonical OpenSpec passes 48/48 plus the scoped change; coverage map passes 598 scenarios / 359 mapped / 239 unchanged pre-existing todo entries.
- Raw task-local logs: `/tmp/issue24-j1-logs/` and `build/engine-only-regression/`. These local audit paths are not portable release artifacts. The selected build regression uses a fresh source/configuration snapshot and retains the task-owned Cargo cache; no primary build or vendored source was removed.
- Mutation testing remains excluded by the user's instruction. No DSP implementation change or numeric performance improvement is claimed.

## Owned runtime transfer prerequisite receipt

- Baseline: public gain/filter/reset integration test passed on the control thread before production changes. Its stereo impulse samples are literal, independently derived values (48 kHz, 8 kHz RBJ low-pass, Q=1, preceding gain=0.5), rather than a comparison with another potentially incorrect render.
- Red: the external ownership test failed to compile with E0277 at both actual thread transfer and the explicit `Send` assertion. Requiring `Send` on the erased prepared executor then exposed the retained filter algorithm as the remaining owner. Both compiler logs are retained.
- Green: requiring `Send` on the two actual owner traits lets Rust verify every concrete implementation. No unsafe marker, lock, shared mutable runtime or DSP processing change was added. All three public integration tests pass, including return/reset/retransfer and independent filter tails; runtime and prepared planes return to the control thread for destruction.
- Regression: normal engine-only configure/build passed using the existing task cache (Rust release rebuild 4m03s); all 18 CTest checks passed in 67.42 seconds. Rust passed 1,084 library tests, three existing binary tests, three new public integration tests and one doctest (1,091 total). Signed C ABI/MIDI and realtime callback guards remain passing.
- Acceptance: two additive `headless-engine` scenarios are synced and mapped to the real public ownership tests. Strict canonical OpenSpec passes 48/48 and this scoped change; spec coverage passes 600 scenarios / 361 mapped / 239 unchanged todo entries.
- Limits: this proves exclusive transfer (`Send`), not simultaneous shared access (`Sync`), native DAW/device/editor behavior or a selected replacement stack. Downstream implementations of the public filter trait must also satisfy `Send`. Mutation testing remains excluded by the user. Raw logs: `/tmp/issue24-thread-transfer-logs/`. Reviewed code committed and normally pushed as [`677dfd0`](https://github.com/DanMarshall909/dandrum/commit/677dfd0c1aed07cc6525ed76c027eac7a0fe3cd7); exact live remote/upstream matches and main remains `68dd655`. Physical C: free space after the checks was 2,852,978,688 bytes (about 2.66 GiB); the existing task cache is retained. All J0-J6 and Gate A/B/C obligations above remain pending except the explicitly proved prerequisites.

## Home-PC handover

The [2026-10-14 handover](issue-24-home-pc-handover.md) preserves the unfinished CLAP implementation and portable evidence. That continuation checkpoint left OpenSpec tasks 3.1–3.4 unchecked. The current Linux completion evidence follows; the historical receipt does not claim a completed issue #24.

## Final headless CLAP proof — 15 October 2026

The native Linux home-PC build uses the original pinned Clack revision and lockfiles, with matching engine/adapter release LTO and one codegen unit. The exported artifact is `build/engine-only/clap/DandrumHeadlessProof.clap` (SHA-256 `e1b527f190db4be8b79f8271927f0f7e821702fdb81c5112d37cf4647ddd823c`). [Machine-readable receipt](evidence/clap-proof-2026-10-15/receipt.json) records source hashes, toolchain, artifact identity, dependencies and actual physical Linux capacity. C: is not present on this host; the handover's Windows capacity remains historical.

- All 20 engine-only CTests pass in 122.39 s, including 1,091 Rust engine tests, signed native ABI/MIDI checks, 13 dynamic plugin tests, two public signed baselines and both absent-source build boundaries. The CLAP build boundary also verifies that changing the embedded schema or module seed schedules a fresh packaged plugin. The retained JUCE comparison target/test inventory passes in 2.69 s with the native toolchain. An initial Linuxbrew linker/system-library mismatch is retained as failed setup evidence; setting `PATH=/home/dan/.local/bin:/home/dan/.cargo/bin:/usr/bin:/bin` resolves it without source changes.
- The pinned official validator runs out of process against the actual release artifact, without includes/excludes: **53 successes, 28 skipped checks, zero warnings and zero failures**. The [complete result](evidence/clap-proof-2026-10-15/validator-final.json) records each skip: absent state/preset/port-configuration features, unsupported float64 and descriptor-specific absent audio/note dialects. The initial denormal warning did not recur on this machine; this is not a general performance or realtime-safety claim.
- Review repairs have genuine RED/GREEN evidence: missing CMake schema/library dependencies and a malformed multi-output callback that left extra buffers untouched. Rejection now clears every accessible output representation, preserves parameter state and returns Continue. The two-synth lifecycle test now restarts one synth at a changed rate/block size while the other retains its held-note/filter history. Its oracle includes the silent interval before the new note, because the existing oscillator advances while silent.
- Coverage instruments the dynamically loaded adapter, retaining the real engine and SDK. Dynamic tests plus the separate instrumented validator execute 391/423 adapter lines (92.43%) and 41/42 functions. Every changed output-silencing path is covered, including float32, float64, both, and absent accessible planes. Remaining gaps are recorded in the receipt; full allocation/I/O, stopped-flush and malformed-host hardening are still later #24 work. Mutation testing remains excluded by the user's instruction.
- Production refactoring consolidates rejection silencing at one boundary, removing repeated clearing branches. Tests reuse the existing lifecycle case and add one explicit foreign-buffer regression; further extraction would hide the small host setup without improving the contract.
- Four canonical scenarios are synced and mapped to their proving assertions: **604 scenarios / 365 mapped / 239 unchanged backlog**. Strict validation passes for the scoped change and all 49 canonical specs. The independently reviewed implementation was committed and normally pushed as [`11c3bd2`](https://github.com/DanMarshall909/dandrum/commit/11c3bd2d99b8607560ad6a68d1a7b630355ef952) on 16 October 2026 (Australia/Brisbane). The pre-push hook passed all 1,091 Rust tests; no hooks were bypassed. Live remote/upstream matched the exact implementation commit, recorded in the [publication receipt](evidence/clap-proof-2026-10-15/publication.json). All tasks for this bounded proof are complete; no main merge or completed issue #24 is claimed.

Raw commands/results are retained under [the completion evidence directory](evidence/clap-proof-2026-10-15/); larger local profiles, build outputs and review manifests remain in `/tmp/dandrum-clap-completion` and this feature worktree's `build/`. No reusable build cache or installed dependency was removed. The primary checkout remains on main. The next #24 work remains state/stopped-flush/callback hardening and parented React/required-format/native-platform evidence; all Gates A–C stay open.


## Fixture state and stopped controls — 16 October 2026

The accepted goal is a first JUCE-free Trigger beta with one React editor, a
persistent host-independent session, standalone and VST3 artifacts, and actual
native-host acceptance. This increment establishes replacement-owned scalar
state for the existing two experimental CLAP fixtures; the full beta and all
Gates A–C remain open.

The real binary now saves version-1 instrument/named-parameter JSON, bounds
input at 16 KiB, validates every entry before publication, and restores values
before activation or between processing calls. Invalid/incompatible/oversized
state and read failures preserve the working controls and signed audio. Short
streams, failed writes, reordered names, optional host notifications, 128 exact
plain-value round trips and inactive/active-stopped flush are executable tests.
No structural graph, asset, authoring document or undo history is introduced.

Verification: **19 dynamic-host tests plus two public signed baselines**, all
**20 independent CTests**, and the retained JUCE comparison configure pass. The
latest pinned official validator reports **62 successes / 18 skips / one
denormal timing warning / zero failures**, including successful state
reproducibility checks. The gain denormal case measured a 3.04× slowdown;
callback hardening must resolve it before beta acceptance. The earlier
63-success/no-warning run remains historical evidence. All **68 changed executable
source lines** run through the real exported instrument; the receipt separately
records inherited owner-wide coverage gaps and tooling limits. Production
package membership stays unchanged; serde dependencies were already transitive.
Mutation remains excluded by the recorded user direction. Independent review passed; the five new scenarios are synced and mapped.
Implementation publication is verified below; no main integration is claimed.

[Portable evidence](evidence/clap-state-2026-10-16/receipt.json) retains source and
artifact hashes, commands, failures, scoped counts and explicit limitations.
Larger raw profiles and coverage exports remain in `/tmp/dandrum-beta-state`;
reusable builds and all unrelated branch/worktree state are preserved. Next:
consume #23's minimum session/SDK/EditorHost contracts for a parented production
React proof, then prove required VST3/standalone and native-platform behavior.

Read-only review caught two assertion gaps despite complete changed-line
execution: the reversed control table rejected too early, and stream tests used
only defaults. Repairs now change an early canonical control before a later
rejection, and restore/preserve distinct nondefault values through short and
prefix-failing streams. The repaired actual-binary suite and final independent
20-test checkpoint pass. Final independent publication review passed.


The reviewed state increment is normally committed and pushed as
[`324f5da`](https://github.com/DanMarshall909/dandrum/commit/324f5da78aa80a657e8a3371d973c84d1474bc99),
with exact live remote verification in the
[publication receipt](evidence/clap-state-2026-10-16/publication.json).
Strict validation passes all 50 canonical specs and the scoped change; the
acceptance map is 609 scenarios / 370 mapped / 239 unchanged backlog entries.
Normal pre-commit coverage and pre-push tests passed without hook bypass.
All five tasks for this bounded state increment are complete. The full beta,
#23/#24 and Gates A–C remain open; main remains unchanged.


## Persistent session foundation — 16 October 2026

The #23 session owner now has an independent Rust crate with accepted YAML,
actual prepared sample resources, copied snapshots and up to 16 disposable
connections. Reattachment retains the document and signed sample output even
after the test-owned source WAV is removed; foreign and explicitly retired
connections fail, including retained copies of retired tokens. Dropped weak
connections are reclaimed. The callback still consumes the engine directly.

Five component tests pass with 68/68 implementation lines, 89/89 regions and
12/12 functions executed. The independent 21-test checkpoint passes; after
extending the absent-host-source guard, that affected guard passes separately
and runs the session behavior tests with JUCE/browser/UI source trees absent.
Retained JUCE comparison configuration passes. No branch-coverage or mutation
claim is made. [Evidence](evidence/session-foundation-2026-10-16/receipt.json)
records scope, commands and the split final checks.

This completes only tasks 1.1 and 1.2 of
`add-persistent-instrument-session`. Typed mapping edits, native history,
request/job recovery, actual consumers and editor destruction remain open;
no new canonical scenario is synced by this initial ownership foundation.
Independent publication review passed for this initial ownership foundation;
mapping/history/native integration and the full beta remain open.


The initial ownership foundation is published at
[`6f8167b`](https://github.com/DanMarshall909/dandrum/commit/6f8167ba73314119b18c78efa0cdfffe9f022a03).
Normal hooks passed and the exact live feature head was verified; main is unchanged.

## Prepared mapping candidate — 16 October 2026

Task 1.3 adds one typed native map/zone key-range candidate. Native code derives
its inverse from the accepted YAML, changes only that field's authored value,
and reloads/prepares the existing real sampler with its retained resource root.
Serialization normalizes YAML presentation/comments; all unrelated values and
asset references remain equal. Candidate preparation leaves the accepted
runtime, document and empty history untouched. It does not yet accept an edit.

Eleven component tests include a non-first map/zone, same-named zones in distinct
maps, nondefault inverse data, inclusive MIDI 0/127, ambiguous/unknown identities,
invalid ranges/connections, absent sample maps and failed resource repreparation.
The candidate produces literal stereo -0.5 at MIDI 60 and silence at 36; the
accepted instrument retains its original signed note-36 sound. Tests retain
independent state and source-file loss assertions. Measured owner coverage is
139/139 lines, 187/187 regions and 20/20 functions; no branch instrumentation or
mutation claim is made. [Evidence](evidence/session-mapping-2026-10-16/receipt.json)
records retained commands and limits.

Independent publication review passed for task 1.3 only; full session and beta acceptance remain pending. Atomic activation,
native undo/redo, bounded jobs/recovery and concrete consumers remain pending;
no complete session scenario is synchronized and the full beta stays active.

## Session transaction and native history — 19 October 2026

Task 2.1 adds one off-audio transaction boundary for accepted runtime, YAML,
revision and native journal state. A prepared mapping candidate is staged with
its next snapshot and history before a host activation lease closes admission.
After installation, the session swaps the already allocated accepted state and
publishes borrowed metadata through an infallible lease operation; old resources
retire off audio after release. Failed validation, preparation or activation
preserves the prior runtime, document, revision and undo/redo cursor.

Native undo and redo derive their target from the versioned journal, prepare the
real Advanced Drum Kit again and use the same activation boundary. The executable
protocol proof maps the kick from MIDI 36 to 60, renders literal stereo `-0.5` at
60 with silence at 36, undoes to the original signed output, redoes the edit and
keeps a second instrument unchanged. Admitted prepared work can complete after
its originating attachment detaches. Foreign/stale candidates, invalid history
connections, empty history and revision exhaustion reject before activation.

All 16 session tests pass in the named fast lane; the session slow lane remains
empty. The complete 21-test engine-only checkpoint passes in 111.75 seconds,
including the absent-JUCE/browser/UI source build, C++ FFI/callback checks and
CLAP boundaries. Owner coverage is 268/268 lines, 28/28 functions and 351/352
LLVM regions; the single region is a compiler-generated error-conversion span in
a second generic monomorphization, while the revision-exhaustion behavior itself
passes. Strict OpenSpec validation and the unchanged 609/370/239 spec-coverage
ratchet pass. Mutation remains excluded by the recorded user direction.

[Portable evidence](evidence/session-transactions-2026-10-19/receipt.json)
records the commands, source hashes, RED base reconstruction and explicit limits.
This protocol fixture does not prove the concrete native host's reader handoff or
editor lifecycle. Tasks 2.2–2.3 and 3.1–3.3 retain request recovery, bounds,
actual host calibration, React/WebView destruction and independent-instance host
proof. No canonical scenario is synced, no main merge is included and the full
JUCE-free beta remains active. Independent publication review passed for task 2.1.

## Bounded request recovery — 19 October 2026

Task 2.2 adds an owned structural request boundary around the existing mapping
transaction. Admission validates the current attachment, instrument identity,
request identity and expected revision, then moves accepted YAML, resource
context and render settings into one `Send` worker job. A session admits only one
structural job; overlapping admission, direct preparation, commit, undo and redo
return explicit rejections without replacing or queueing work.

The worker no longer borrows the editor or session. It can prepare after the
originating attachment is retired, and completion publishes through the same
activation lease. The latest 128 terminal outcomes are retained by exact request
content. A fresh attachment can query or retry a lost result and receives the
same reference without new preparation, activation or history. Conflicting ID
reuse, stale revisions, foreign instances, retired connections and misrouted
completion cannot mutate either tested instrument. Request IDs increase within
the live session; a one-value high-water mark rejects admitted identities after
their result is evicted. Misrouted completion returns the owned result so its
source can still accept it. Preparation, staging and activation failures are
terminal retained outcomes and release the pending slot.

Twenty-two session tests cover worker movement to another thread, detach before
completion, lost acknowledgement recovery, one accepted history entry, explicit
busy/conflict/stale/foreign failures, failed sample preparation, activation
rollback, revision exhaustion and 129 failed requests evicting the oldest result.
The accepted edit retains the literal stereo `-0.5` sample at MIDI 60 and silence
at 36; the independent instrument retains `-0.5` at 36. Owner coverage reports
437/441 lines, 43/43 functions and 544/548 LLVM regions. The four zero-count
items are compiler generic-instantiation spans; merged source counters execute
every source line and all request outcome branches. Mutation remains excluded by
the recorded user direction.

The named fast lane passes all 22 session tests and the slow selection remains
empty. The complete engine-only checkpoint passes 21/21 targets in 113.98
seconds, including C++ FFI/callback checks, the absent-JUCE/browser/UI source
build, dynamic CLAP hosting and the isolated CLAP build boundary. Clippy with
warnings denied, strict OpenSpec validation and the unchanged 609/370/239
spec-coverage ratchet pass.

[Portable evidence](evidence/session-requests-2026-10-19/receipt.json) records
the final commands, hashes and limits. This remains an in-process protocol proof.
Task 2.3 retains bounded journal/replacement reconciliation, and tasks 3.1–3.3
retain the concrete native host, typed React transport, real WebView destruction
and independent-instance host proof. No canonical scenario is synced, no main
merge is included and the full JUCE-free beta remains active.

Independent publication review passed for the exact repaired task 2.2 candidate.

## Bounded history and document replacement — 19 October 2026

Task 2.3 caps the native authoring journal at 100 accepted edits. When the cap
is exceeded, the oldest inverse is discarded and undo reaches the state after
that retired edit. Failed validation, preparation or activation preserves the
redo branch; the next accepted edit from an undone state truncates redo. The
existing terminal request recovery remains capped at 128 outcomes.

An external canonical YAML document can now be prepared without mutation and
published through the existing activation lease. Acceptance advances the same
session revision, installs the prepared runtime/document, resets incompatible
history and swaps in a fresh preallocated outcome window while admission is
closed. The monotonic request high-water mark remains, so retired identities
cannot execute again and requests based on the replaced revision are stale.
Invalid, foreign, stale, busy and activation-rejected replacements preserve the
working session.

The protocol host keeps parameter automation outside the authoring owner and
reapplies it to each installed runtime. At half kick level, both the edited and
externally replaced mappings render literal signed `-0.25`; automation does not
change revision or history. The retained-bound test accepts 101 real edits,
rejects a branch, accepts a new branch and executes 100 real undos, ending at
the earliest still-reconstructible signed mapping.

All 25 session tests pass. The named session lane passes in 4.44 seconds, and
the complete engine-only checkpoint passes 21/21 targets in 113.46 seconds.
Owner coverage executes 48/48 functions, 495/499 LLVM-counted lines and 624/627
regions; the merged source report lists no uncovered source line, with the four
zero counts confined to generic monomorphization spans. Clippy with warnings
denied, formatting, strict OpenSpec validation and the unchanged 609/370/239
spec-coverage ratchet pass. Mutation remains excluded by the recorded user
direction.

This remains an in-process real-engine protocol proof. Tasks 3.1–3.3 retain the
concrete native reader handoff, typed React contract, actual WebView destruction
and independent-instance host proof. No canonical scenario is synced, no main
merge is included and the full JUCE-free beta remains active. Independent
publication review passed for the exact task-2.3 candidate.

## Concrete native session handoff — 19 October 2026

Task 3.1 connects the persistent Rust owner to the real JUCE comparison
processor for sample-map instruments. The Rust crate now exports one combined
static library containing the session facade and engine. Its native mapping
worker owns preparation independently of any editor. A C++ RAII activation
lease closes the existing audio-reader gate, stages metadata and parameter
bindings, publishes only the prepared engine and accepted revision/history in
the committed swap, then retires the previous engine before reopening
admission. Parameter-surface notifications run after the lease returns, outside
that swap. Exact publication-window instrumentation proves the publisher takes
no native mutex while the reader gate is closed. In C++ terms the lease is
deterministic scope cleanup, similar to a C# `IDisposable` guard. The Rust job
enum boxes its larger prepared variant so the enum itself stays bounded; Rust
enums otherwise take the size of their largest variant.

The actual consumer proof holds a real `processBlock` reader while mapping the
Advanced Drum Kit `kit/kick` zone from MIDI 36 to 60. It proves the old engine is
not destroyed while held and is destroyed exactly once after release, the
editor-independent worker publishes revision 1 with one undo entry, native UI
map/zone metadata and YAML agree with the session, MIDI 36 becomes literal zero,
and MIDI 60 renders literal `-0.5`. An injected activation rejection preserves
the exact runtime, YAML and history. Render reconfiguration preserves the same
accepted state. A separate held-preparation case proves a host block-size change
waits for admitted work, then rebuilds the newly accepted revision at the new
size rather than publishing stale render settings.

A concurrent plugin-state restore now waits for an admitted mapping worker,
lets that accepted job reach a terminal outcome, then retires the persistent
session before replacing the embedded document. The proof verifies later host
preparation uses the restored embedded edit rather than either the retired
mapping or the original disk file: restored MIDI 62 renders `-0.5`, while
superseded MIDI 63 and original-disk MIDI 36 render literal zero. Ordinary
file-loaded instruments retain their existing file-based host-preparation
behavior. Native callback tables are validated before use: an incomplete
completion table becomes a retained terminal activation failure without
changing accepted state, duplicate completion is rejected, and the next
mapping request remains admissible.

The link-graph gate inspects the proof executable and the real sampler
Standalone and VST3 link commands and binaries. Each links the combined session
archive exactly once, excludes the standalone engine archive, and contains one
`dandrum_instrument_session_create` plus one `dandrum_kernel_render` definition.
The retained engine-only utilities continue to link the smaller engine archive.
Release builds produced both native sampler products; the JUCE-free nested build
continues to pass, so this adoption does not introduce JUCE into the independent
engine/session boundary.

The session package has 28 passing tests. Combined crate coverage executes
85/90 functions, 1180/1275 LLVM-counted lines and 1460/1616 regions. The core
owner executes all 56 functions and 592/596 LLVM-counted lines; the four
remaining line counts are generic-instantiation spans, and the annotated report
lists no uncovered owner source line. The native facade additionally has direct
success/failure/reconfiguration coverage and the real C++ boundary proof.
Mutation remains excluded by the recorded user direction. The portable receipt under
`evidence/session-native-host-2026-10-19/` records final commands, hashes,
timings, link evidence and explicit limits.

This increment adopts native mapping and host render reconfiguration only.
Legacy whole-document reload deliberately retires the session until that path is
migrated. Tasks 3.2–3.3 retain the typed React contract, native undo command,
actual WebView destruction/recreation, reply recovery and two-instance proof.
No canonical scenario is synced, no main merge is included, selected native
desktop/DAW proof remains open, and the full JUCE-free beta stays active.

## Typed session connection and React mapping — 19 October 2026

Task 3.2 exposes the Rust-owned session contract through the concrete sampler
processor and the canonical Trigger React adapter. Contract version 1 carries
instrument, attachment, revision and next-request identities as canonical
unsigned decimal strings, preserving values beyond JavaScript's safe integer
range. It also publishes the exact limits of 16 attachments, 100 journal
entries, 128 retained outcomes and one active structural request. Rust retains
reference-counted lifetime identity internally; the monotonic `u64` values are
transport identities rather than pointers or operating-system handles.

Each `InstrumentHostWebBridge` now opens one disposable native connection and
retires it by scope cleanup. The bridge accepts exact map/zone identities for a
bounded key-range command, returns string request tickets, exposes terminal
status, and runs native undo through the same processor-owned off-audio worker
and activation path. The real bridge proof maps the Advanced Drum Kit kick from
MIDI 36 to 60, renders literal signed `-0.5` at 60, then invokes native undo and
restores `-0.5` at 36. The processor session and native link-graph regressions
remain passing.

The React projection retains the original native map and zone IDs separately
from editor rule IDs. Only key-range `updateRule` operations with those native
identities bypass the page closure journal; unmigrated operations and the mock
preview retain the established path. The store orders native and local undo
entries with numeric sequence tokens, without placing an executable closure in
native history. Runtime validation rejects changed versions, noncanonical or
overflowing identities and altered limits. Failed admitted work refreshes the
consumed request high-water mark before retry, so the next command cannot reuse
a retired identity.

All 29 session crate tests, 89 Trigger tests, TypeScript/component checks, the
native production bundle, three Trigger CTest boundaries and the concrete C++
session/link checks pass. Each implementation commit's strict hook also passed
all 1,084 engine tests and coverage. Mutation remains excluded by the recorded
user direction. Portable commands and source hashes are retained under
`evidence/session-react-contract-2026-10-19/`.

This increment does not claim the task 3.3 destruction/recreation proof. The
actual packaged WebView must still be destroyed around lost acknowledgement,
reattach, recover undo and prove signed output while a second instance remains
unchanged. No canonical scenario is synchronized yet; selected native desktop
and named DAW acceptance, the JUCE-free installed artifacts and the full beta
remain open.

## Packaged WebView session recovery — 19 October 2026

Task 3.3 now exercises complete editor and WebView destruction through the
original packaged Trigger runtime. A production bridge admits the kick mapping
from MIDI 36 to 60 without consuming its reply. The test destroys the complete
editor/browser owner, waits for the instrument-owned job, creates a fresh real
editor and reads the accepted revision, prepared native map/zone and undo count
through the rendered React page. Clicking that page's actual Undo button sends a
new native request and restores literal signed `-0.5` output at MIDI 36 with
silence at 60. A second live sampler retains its original revision, empty
history and signed output throughout.

The headless bridge regression separately proves the same lost-reply boundary
without a display. It destroys the attachment-owning bridge before observing
the result, reattaches, recovers exactly one accepted history entry and invokes
native undo. In C++ terms, the `std::unique_ptr` owner gives the editor and
WebView deterministic scope cleanup, similar to a C# `IDisposable` owner, while
the processor-owned session outlives that disposable scope.

The final Linux display run used `DISPLAY=:10.0`; both
`cxx-trigger-native-bridge` and `trigger-web-runtime` passed in 6.81 seconds,
and the WebView test executed rather than returning its unavailable-platform
skip code. The concrete session/link pair and both adjacent React adapter lanes
also passed. The task record retains the initial headless `SIGPIPE`, the rejected
presentation-level YAML oracle and a missing-product build-directory selection.
The wrong-directory failure and an incomplete build attempt are preserved as raw
setup logs but are not used as acceptance evidence. The corrected maintained
build directories pass without source workarounds. Independent review of exact
head `bed967ce0235e97cd0a0afd28b5eb8624d70b12c` found no actionable P1/P2
issue, and its fresh two-test run passed on the actual packaged WebView.

This is the retained JUCE comparison WebView on Linux, so it proves the session
lifecycle contract before framework cutover. It does not satisfy the selected
JUCE-free desktop/format matrix, named-DAW automation, installed offline
Standalone/VST3 artifacts or absence-of-JUCE audit required by the beta goal.
Tasks 4.1–4.2 retain the complete verification and acceptance publication for
this OpenSpec increment. Mutation remains excluded by the recorded user
direction.

All nine session scenarios are now synchronized into the canonical
`instrument-session` specification and mapped only to their proving Rust,
engine-only build, native bridge or packaged WebView tests. Strict validation
passes the scoped change and all 51 canonical specs. The acceptance map passes
with **618 scenarios / 379 mapped / 239 unchanged backlog entries**. The full
task-4.1 checkpoint and task-4.2 completion/publication review remain open.

## Persistent session completion checkpoint — 19 October 2026

Task 4.1 rebuilds and verifies the whole persistent-session increment from
source head `f362346`. Rust formatting and Clippy with warnings denied pass, as
do all 29 session tests. The independent configuration passes 21/21 tests in
121.40 seconds, including the absent-JUCE/browser/UI source build, callback
safety and dynamic CLAP boundaries. The maintained native comparison passes
52/52 in 67.33 seconds, including the concrete session handoff, editor smoke,
Standalone/VST3 link graph and its own isolated build checks. The Trigger slice
passes 5/5 in 92.92 seconds: 95 React tests, TypeScript/component checks, the
production bundle, 27 Playwright browser scenarios, bridge coverage and the
actual packaged WebView lifecycle on `DISPLAY=:10.0`.

The first independent task-4.2 review rejected the coverage interpretation: the
reported `reserve_control_request` lines belonged only to an uncalled synchronous
undo export, while every maintained native and React consumer uses the admitted,
owned off-audio job path. Repair head `89efa74` first proves the obsolete export
present with a failing native link-graph assertion, then removes the export,
binding and private reservation helper. The concrete native session, link graph,
Trigger bridge and actual packaged WebView tests pass after the removal, and the
commit hook passes all 1,084 engine tests plus its strict coverage gate.

Repaired Rust-only session coverage records 104/109 functions, 1456/1594
LLVM-counted lines and 1750/1980 regions across the core and C facade. The core
owner records 63/63 functions, 666/670 lines and 812/821 regions, with no visible
missed core source line. Its four unaccounted LLVM line counts and nine regions
come from generic/instantiation accounting. The C facade records 41/46
functions, 790/924 lines and 938/1159 regions; the annotated report retains all
defensive ABI misses rather than presenting merged cross-language evidence as
100% Rust coverage. Branches are not instrumented, and mutation remains excluded
by the recorded user direction.

The same review found the failed-preparation/activation acceptance map
incomplete. It now also cites the invalid-or-ambiguous mapping preservation test
and the concrete C++ session consumer, covering rejection before preparation as
well as activation rollback. The initial React checkpoint used system Node 18
and failed only when Playwright required Node 20; the final lane uses the
installed Node 22.19.0 runtime. All reusable build and Cargo caches are
preserved. Task 4.2 retains the repeated independent completion review and normal
publication; the full JUCE-free beta goal remains open beyond this increment.

The repeated independent task-4.2 review passes exact head `53c796a`. It
confirms both earlier findings are repaired and finds no remaining actionable
P1/P2 defect or unsupported completion claim. Fresh reviewer checks pass all 29
session tests, formatting and Clippy, both native session/link tests, both
Trigger bridge/packaged-WebView tests without a skip, strict validation of the
change and all 51 canonical specs, the 618/379/239 acceptance-coverage gate, all
33 receipt hashes and both retained comparison-artifact hashes. This completes
the bounded retained-JUCE session increment; archive and normal publication do
not complete the larger JUCE-free beta goal.

The completed `add-persistent-instrument-session` change is archived at
`openspec/changes/archive/2026-10-19-add-persistent-instrument-session`. Its five
requirements and nine scenarios were already synchronized into the canonical
specification before archival. All artifacts and all 11 tasks were complete;
no incomplete-work warning or sync exception applies.

Archive head `de655b5` is published on
`origin/feat/juce-free-host-adapters` with exact local/remote identity. The
active pre-push hook passed the full Rust suite and reported no changed engine
Rust source files, so its mutation phase skipped normally. The main checkout
remains clean and unchanged.

## JUCE-free Trigger CLAP editor baseline — 19 October 2026

The next bounded increment starts from committed plan `1c09208` and targets a
Linux X11 CLAP child editor over the production Trigger bundle and persistent
instrument session. It keeps VST3, standalone device IO, named-DAW acceptance
and installed release packaging open for the following increment, so the full
beta and Gates A–C remain active.

Before behavior changed, both existing CLAP boundary CTests passed, all 95
Trigger JavaScript tests passed, and all 29 session tests passed with formatting
and Clippy warnings denied. The current headless CLAP source and artifact hashes
match the preceding state receipt exactly: its latest source-bound evidence is
68/68 changed executable lines and 424/485 owner lines. This baseline does not
reinterpret the older 391/423 pre-state measurement as current coverage.

Fresh JavaScript coverage records 100% line, 82.95% branch and 89.36% function
coverage for `native-engine.mjs`; `session-connection.mjs` records 100% line and
function coverage with 95.45% branch coverage. Fresh Rust session coverage
records 104/109 functions, 1456/1594 lines and 1750/1980 regions overall. Its
format-neutral core records 63/63 functions, 666/670 lines and 812/821 regions;
the defensive C facade records 41/46 functions, 790/924 lines and 938/1159
regions. Branch coverage is unavailable in that LLVM Rust run.

The selected dependency baseline pins Clack at
`27ca28391b57a61bad4bb5099288a7bf9247350d` and evaluates Wry 0.57.0 with its
default Linux X11/WebKitGTK path. Both Rust projects declare MIT or Apache-2.0.
The host provides WebKitGTK 2.50.4, GTK 3.24.33 and X11 1.7.5; their system
package copyright manifests are multipart, so redistribution attribution stays
open until installed packaging work. Wry is not yet a repository dependency,
and the current headless CLAP binary links only libc, libm and libgcc_s, with no
JUCE, GTK or WebKit runtime edge.

[Portable baseline evidence](evidence/trigger-clap-editor-baseline-2026-10-19/receipt.json)
contains the exact source, artifact, licence and log hashes. At measurement the
filesystem had 58 GiB free; the 64 GiB build tree and both 5.6 GiB Cargo targets
were retained. Mutation remains excluded by the recorded user direction.

## Trigger host and copied-resource scaffold — 19 October 2026

The first implementation boundary adds an opt-in
`DANDRUM_TRIGGER_CLAP_EDITOR` build and a `dandrum-trigger-host` Rust crate over
the completed instrument session. The host crate has no Clack, Wry, WebKitGTK,
GTK or JUCE package edge. The build stages a supplied prebuilt editor bundle,
the Advanced Drum Kit document and its sample into a regular-file sibling
resource tree. The existing CLAP binary and its two fixture descriptors are
unchanged.

The initial build-boundary test failed because the editor and host targets did
not exist. A second RED proved that tracking only `index.html`, `app.js` and
`app.css` left a changed nested font stale. The final rule tracks the complete
bundle recursively. Its isolated fixture has no `third_party/JUCE`, `web` or
`ui` tree; it rebuilds a changed nested asset, copies the binary and resources
outside the source snapshot, removes all resource inputs, and verifies every
copied payload byte-for-byte without a source path. The real dynamic host test
then confirms both prior descriptors still load.

All four CLAP build-boundary tests pass in 72.33 seconds. The new crate passes
formatting, Clippy with warnings denied and its empty scaffold test/doc-test
lanes. [Portable evidence](evidence/trigger-clap-editor-scaffold-2026-10-19/receipt.json)
retains source and log hashes. Its local Cargo target is retained and explicitly
ignored beside the other crate caches. This is build scaffolding only: it does not yet
resolve packaged resources at runtime, own a prepared Trigger session, add Wry
or expose a Trigger CLAP descriptor.

## Packaged Trigger session owner — 19 October 2026

The format-neutral host now opens one explicit `DandrumTrigger.resources`
root, resolves the fixed Advanced Drum Kit document and sample beneath its
`instrument/` directory, and owns the resulting persistent
`InstrumentSession`. Resource roots are canonicalized; missing, unreadable,
nonregular, permission-denied and symlinked payloads return explicit errors.
There is no current-directory, executable-relative or checkout fallback.

Seven behavior tests use a copied package outside the checkout. They prove the
accepted document identity, literal signed `-0.5` stereo output for MIDI 36,
silence at MIDI 60, stable diagnostics and rejection of a symlink back to the
still-present repository sample. Malformed YAML and a removed installed sample
both fail a reload candidate while the original snapshot and signed audio stay
usable. The isolated opt-in build/resource/descriptor boundary also passes with
the real host owner in 121.85 seconds.

Focused LLVM coverage records 19/19 host functions, 106/107 counted lines and
149/150 regions, with no annotated zero-hit source line. The one line and region
are generic-instantiation accounting, matching the distinction already used for
the session core; branch coverage is unavailable. Formatting, Clippy with
warnings denied and all seven tests pass. [Portable evidence](evidence/trigger-host-resources-2026-10-19/receipt.json)
retains the RED compile failure, final logs, coverage and source hashes. Runtime
activation/reclamation and callback ownership remain task 2.2.


## Parallel Trigger host foundations — 20 October 2026

Tasks 2.2, 2.3 and 3.1 of `add-juce-free-trigger-clap-editor` now have verified implementations and independent PASS reviews. The user authorized three isolated lanes from `a116c2cc`, targeting the existing `feat/juce-free-host-adapters` branch. They add the format-neutral runtime, bounded protocol and frontend selector; they do not complete the Trigger CLAP editor or the broader JUCE-free beta. The OpenSpec task list owns delivery status. No main merge or canonical scenario synchronization is included.

The runtime transfers one prepared processor at a block boundary and returns retirement to the control thread before admitting another replacement. Bounded event/feedback queues, atomic scalar values, validated offsets and reset keep the callback free of allocation, locks and reclamation. A cancellable activation lease closes callback admission, waits off audio for an admitted callback and publishes before reopening. Rust moves the owning `Box` through the slot; this transfers its pointer without copying or destroying DSP state. Tests include signed sample offsets, backpressure, cancellation, reset, feedback overflow and control-first teardown, with a calibrated allocation/destruction audit.

The signed parameter test caught a real integration gap in the earlier packaged host: its prepared root controls were unbound. The host now prepares explicit named mono control buses and stereo output. The actual packaged `TriggerHost` feeds the runtime's bounded level/note queue and renders literal signed `-0.125` stereo at level `0.25`, while the accepted session snapshot and history remain unchanged. Unsupported layouts, unbound inputs and unsupported realtime schedules are rejected off audio.

Protocol version 1 bounds incoming JSON to 16 KiB, encoded replies/events to 64 KiB, native attachments to 16 and retained typed transport requests to 128 per attachment. Canonical decimal identities, typed positional arguments, capability errors, replay/conflict retirement and reply/event correlation remain separate from native structural job identities. Invalid inputs preserve the accepted snapshot, five level defaults and literal signed `-0.5` stereo output. Covered refactoring replaced an uncalled custom writer method with a fixed-size standard-library `Cursor`. Command dispatch, session-result projection and real WebView wiring remain task 3.3; task 2.4 retains gestures and atomic version-2 state.

The frontend selects `window.__DANDRUM__` version 1, retains the existing JUCE comparison backend and explicitly silent preview, and exposes incompatible native versions and capability errors as UI failures. All 102 fast JavaScript tests, 31 browser scenarios, static checks and both production builds pass. The real retained native bridge and packaged comparison WebView pass on `DISPLAY=:10.0` in 6.81 seconds. The selector has 100% measured line, branch and function coverage.

The combined host has 33 passing tests. Source-bound LLVM coverage measures protocol 329/329 lines and 21/21 functions, runtime 393/393 counted file lines and 45/45 functions, and the resource/host owner 119/120 counted lines with all 21 functions executed and no annotated zero-hit source line. Runtime production physical lines separately measure 346/346. Generic/instantiation accounting and non-100% region coverage remain visible; branch coverage is unavailable. The opt-in CTest pipeline now enumerates and executes the host tests. All four absent-source/copied-resource CLAP build checks pass in 146.64 seconds, and the final full checkpoint passes 22/22 CTests in 126.99 seconds. The first full invocation's restricted module-library writes and omitted opt-in artifact are retained as setup failures; the corrected run explicitly builds `dandrum-trigger-clap-editor` and grants the existing engine fixtures their required filesystem access. No production workaround or test weakening was introduced.

The separate [Wry investigation](evidence/trigger-wry-investigation-2026-10-20/findings.md) passes two real X11 child/local-IPC/hide/show/destruction cycles with Wry 0.57.0. It records pinned dependencies, Linux licence declarations, host-driven GTK servicing and remaining failure-path fixtures. It adopts Wry for the next implementation fixture while deferring production integration; task 3.2 stays open.

[Portable evidence and source hashes](evidence/trigger-parallel-foundations-2026-10-20/receipt.json) retain the lane and combined checks, RED evidence, independent review matrices and explicit limits. Mutation remains excluded by recorded user direction. Reusable build trees and dependencies are preserved. Automatic approval review initially rejected the normal push for lack of explicit payload/destination authorization. The subsequent full-beta continuation explicitly authorizes normal publication on the existing feature branch; the reviewed commits are awaiting that normal push. No publication is claimed until it succeeds. Tasks 2.4, 3.2, 3.3 and 4.1–5.2 remain open, and the full beta remains active.


## Trigger public parameter boundary — 20 October 2026

The format-neutral parameter portion of task 2.4 now has an owned controller over the real packaged Trigger session, runtime and protocol. The 23 alias-derived public controls convert normalized editor values to physical runtime/host values. Bounded gestures reserve their closing event, dual-queue rejection preserves accepted values, and peek/ack retains host automation under output backpressure. Detach closes each owned gesture once. Feedback preserves its original sample offset and settled UI sequence; reset explicitly reports canceled queued writes without changing applied scalar values or structural history. Ordinary reads never close audio admission. A valid nonbinary interval exposed an endpoint-rounding rejection; a minimal clamp fixes it with retained RED/GREEN evidence.

All 48 host tests pass, including literal signed audio, independent controllers, queue limits, stale/foreign owners, reset cancellation and a calibrated cross-thread allocation/destruction sentinel. The registered trigger-host-tests CTest enumerates and passes all 48 in 8.10 seconds. Formatting and Clippy with warnings denied pass. Exact-source focused LLVM coverage measures parameters 233/233 lines and 24/24 source function/closure starts; runtime 492/492 lines and 54/54 source starts, including embedded unit tests. Raw duplicate instantiations and unavailable branch/region evidence are disclosed in the [portable receipt](evidence/trigger-parameters-2026-10-20/receipt.json). Exact raw logs are preserved as deterministic gzip artifacts.

This independently reviewed parameter boundary does not complete task 2.4 or prove actual CLAP automation acceptance. Version-2 atomic document/resource/value restore, queue epochs and native/DAW continuity remain open. The inherited structural publication gate returns zero buffers while held; its unit coverage does not prove uninterrupted native lifecycle audio. The foundations and full-beta plan were normally published at 811ff37; main remains clean at 68dd655. The full beta stopping condition remains unchanged.

## Atomic native Trigger state — 20 October 2026

Task 2.4 now has native version-2 state over the published parameter boundary at
749d9e0. A bounded 64 KiB JSON envelope stores the accepted YAML, fixed packaged
resource identity and all 23 physical named values. Restore validates and
prepares the complete candidate before publication. Unknown or incompatible
resources, documents, fields, names, values and stream failures preserve accepted
state, feedback, queued commands and nonempty native history. Valid external
restore clears incompatible structural history. Native save/load needs no editor
attachment and succeeds with all 16 attachment slots occupied.

Successful restore advances the queue epoch, discarding old notes, parameter
commands and feedback while preserving later activity. Repeated restoration
reclaims retired processors on the control thread. Coherent saves read a complete
scalar vector using an atomic writer stamp; only the main reader retries. Audio
performs bounded atomic work, and ordinary saves never close callback admission.
The inherited structural publication gate still returns zero buffers while held;
uninterrupted native lifecycle audio remains a later acceptance obligation.

All 62 host and 30 session tests pass, including independent literal PCM-derived
stereo frames, semantic name permutations, invalid-state rollback, balanced
automation, independent instances and calibrated callback allocation checks.
Formatting and Clippy with warnings denied pass for both crates. Registered
CMake owner tests pass 2/2 in 11.09 seconds; the affected absent-JUCE/browser-source
build/resource guard passes in 143.20 seconds. Its initial wrong Python-class
selection was a retained setup error, not a behavior RED.

Exact-source LLVM coverage measures the new state module at 101/101 lines and
10/10 source function starts, runtime at 678/678 and 66/66, and parameters at
283/284 and 28/28. The one unexecuted defensive session-error fallback is explained
by exclusive immediate preparation/commit. Changed session ownership measures
662/662 lines and 64/64 source starts; unchanged FFI coverage gaps and raw generic
instantiation counts remain explicit. Embedded unit tests are included in owner
totals; branch and region evidence is unavailable in these runs.

[Portable source-bound evidence](evidence/trigger-state-2026-10-20/receipt.json)
retains raw logs, hashes, exact commands, genuine REDs and honest GREEN
characterization. Independent completion review passed for the exact core candidate with no
required corrections; its exact review and verdict are retained beside the receipt.
Final metadata verification and normal-hook publication remain the next gates. No canonical scenario is prematurely mapped to core
tests in place of the required dynamic CLAP/native acceptance. Task 3.2 is next;
the full beta, later formats, devices, DAW and installed offline products remain
active. Main is unchanged and all reusable caches are preserved.


## Packaged Trigger editor assets — 20 October 2026

The asset component of task 3.2 loads the production React bundle, eight local
fonts and three font licence notices from the explicit copied resource root.
The new concrete editor crate preloads the complete package before exposing an
owner; browser requests borrow its immutable bytes through exact local URLs.
Missing, empty, unreadable, nonregular or symlinked required assets reject the
package. Undeclared paths, alternate origins, queries and encoded names are not
served. This component has no browser dependency and adds no GUI types to the
format-neutral host.

Four fast tests and a separately selected production-bundle calibration pass.
The registered CTest lanes execute both scopes against freshly built, staged
production assets; their missing-staged-file calibration fails as expected.
Focused LLVM coverage hits all 49 distinct emitted asset-module source lines and
all five unique source function/closure starts. LLVM's raw line summary is 52/52;
duplicate generic instantiations remain explicit in the evidence. The source-free build
fixture uses a deliberately tiny bundle, so its build isolation and test
registration evidence is kept distinct from production font calibration.

Task 3.2 remains open for the Wry/X11 owner, real local-only browser loading,
parenting, main-thread servicing, show/hide and failure cleanup. This asset
component does not prove native lifecycle or uninterrupted audio. Task 2.4 was
normally published at dbabae61; main remains unchanged at 68dd655. Mutation
remains excluded and reusable caches are preserved.

The affected source-free guard passes from the hash-matched RAM candidate in
138.70 seconds, including all three selected runtime CTests. Its initial
disk-threshold interruption is retained as an incomplete verification run.
[Portable evidence](evidence/trigger-editor-assets-2026-10-20/receipt.json)
retains exact source, logs, commands and coverage accounting. Independent
completion review passed for the exact asset component; its review and verdict
are retained with the receipt. Final metadata verification and normal-hook
publication remain the next gates.


## First native Trigger editor owner — 20 October 2026

The asset component was normally published at 6df7591. The next isolated part
of task 3.2 adds an optional Linux-X11 Wry owner: it validates the real host
parent, creates its own initially hidden 1200 by 800 child, loads fixture local
JavaScript and IPC, and services at most 64 nonblocking GTK iterations per host
tick. Actual X11 queries and browser/callback weak references prove cleanup,
including checked reparent failure after WebKit construction. Explicitly owned
IPC disconnection prevents a retained browser cycle; synchronizing GDK after
close makes destruction visible to the host's independent X11 connection.
Rust drops fields in declaration order, so the browser precedes its GTK container.

The registered fast, production-package and real native lanes pass 3/3. Final
formatting, Clippy with warnings denied, native build and source-bound LLVM
checks pass. Native coverage is 129/134 lines, 11/13 functions and 184/203 regions,
with no exclusions or branch claim. External/invariant error gaps are retained.
Review strengthened geometry to independent literal dimensions; initial lint
findings were fixed through shared test-fixture wiring without suppression.
The affected maintained source-free guard also passes from hash-matched RAM
inputs; original caches and every raw evidence byte are preserved.
Three attempts stopped at storage floors. After explicit user approval, only
the named 30 GiB debug cache was removed and the RAM guard cache was preserved
on disk with matching hashes and its logical path retained. The final guard
passes at its default profile; interruption and resource receipts remain explicit.

The duplicate-protocol calibration rejects in Wry's builder before WebKit
allocation. A stricter diagnostic disproved the earlier investigation claim;
the separate checked-reparent fixture supplies the actual post-WebKit rollback
proof. Historical source-capture limits remain explicit.

[Portable evidence](evidence/trigger-native-owner-2026-10-20/receipt.json)
retains commands, source snapshots/hashes, full raw archives and dependency/licence
audit. Independent completion review passed for the exact native-owner component;
its review and verdict are retained with the receipt. Final metadata verification
and normal-hook publication remain the next gates. Task 3.2 remains unchecked for production React/font loading, local-only
browser policy, show/hide, size/order/API lifecycle and actual CLAP main-thread
integration. No native audio/session continuity claim is made. The full beta
remains active through CLAP and the later VST3, standalone, named-DAW and installed
offline/native acceptance increments. Main remains clean at 68dd655, mutation
is excluded, and reusable caches are preserved.


## Trigger editor external-fetch CSP — 20 October 2026

The native owner was normally published at 4d85b25. Its next covered task 3.2
criterion adds one fixed enforcing `connect-src 'self'` response header.
A live loopback endpoint is calibrated independently; unchanged WebKit proves
the external request succeeds before the genuine missing-policy RED. GREEN
requires the exact enforcing directive/origin/document event, a rejected fetch
and no request at that endpoint, while packaged module/CSS/inline styling and
local 404 fetch remain usable. The listener's Rust Drop implementation stops
and joins its worker when the fixture leaves scope, including assertion failure.

Final three registered CTests, all-target native Clippy with warnings denied,
formatting and source-bound native/fast LLVM pass. The new production line is
hit 10times; owner 130/135 lines, 11/13 functions and 185/204 regions retains its inherited
five-line/19-region gaps with no exclusions or branch claim. Fast assets remain
52/52 lines. CMake/manifest/lock/default dependency graph are unchanged; the
published source-free/default-boundary evidence remains applicable without
repeating that unaffected large guard.

[Portable evidence](evidence/trigger-editor-fetch-csp-2026-10-20/receipt.json)
retains exact source, raw logs and profiles, endpoint facts and command receipts.
Independent completion review passed for the exact external-fetch CSP component;
its review and verdict are retained with the receipt. Final metadata verification
and normal-hook publication remain the next gates.
Task 3.2 stays unchecked. Complete resource restrictions, navigation, new windows,
downloads, production React/fonts and the remaining owner lifecycle are next;
actual CLAP main-thread/API/order acceptance belongs to the later 4.2 boundary
and remains unproved here. The full beta continues through all remaining CLAP
tasks and the subsequent VST3, standalone, DAW and installed offline/native
acceptance increments. Main stays at 68dd655, mutation is excluded and all caches
apart from the explicitly approved 30 GiB debug deletion remain preserved.


## Trigger editor fixed-entry navigation — 20 October 2026

The external-fetch CSP criterion was normally published at 8d002a1. Its next
covered task 3.2 criterion restricts the top-level document to the fixed packaged
entry. Independently calibrated owned HTTP HTML plus actual WebKit URI/title/GET
observations prove genuine unwanted navigation before implementation; the local
root alias also commits in baseline. Exact RED fails on the unwanted actual URI.
GREEN rejects both targets, keeps the exact local URI/title, and continues the
ordinary page's local fetch/IPC with no browser request at the healthy endpoint.
Existing local module/CSS/inline styling and enforcing-fetch controls still pass.

Covered refactoring shares a private immutable entry constant for initial load
and exact comparison; tests keep independent literal expected URLs. The endpoint
and page helpers are reused under both existing and new native assertions.
Final 3/3 registered CTests pass in 1.572698 s, including native 1.42 s; Clippy-Dwarnings,
native build, fmt, source-bound native/fast LLVM and default graph pass. The new
predicate executes7times across5entry accepts and 2 denials. Native 129/134 lines,
12/14functions,190/209regions retains5 inherited lines/19 regions without exclusions
or branch claims; fast assets 52/52 and native 0/134 remain distinct scopes.

[Portable evidence](evidence/trigger-editor-navigation-2026-10-20/receipt.json)
retains exact raw source snapshots/logs/profiles and all command identities.
CMake/manifests/lock/default graph are unchanged; published 4d85b25 source-free
boundary remains applicable through 8d002a1 without another large guard run.
Independent completion review passed for the frozen source/evidence candidate; normal-hook publication is the remaining gate.
Task 3.2 stays unchecked, 7/14 overall. Resource policy, actual production React/fonts,
show/hide/size and popup/download behavior remain; actual CLAP main-thread/API/order
proof depends on 4.2 integration. Full beta continues through 3.3/4.x/5.x and the
next VST3/standalone/DAW/offline/native increments. Main remains at 68dd655, mutation
excluded, all remaining reusable caches preserved; no further deletion authorized.


## Native production Trigger rendering — 20 October 2026

Fixed-entry navigation was normally published at e9f7e09. The next task 3.2
slice proves the actual unmodified production bundle in the existing Silent/mock
empty preview through the native owner. Real Trigger structure/controls, external
CSS and React inline frame/SVG are checked. All eight nonempty loaded font-face
sets match family/weight. Original WebKit HTML/JS/CSS/TTF responses compare byte
for byte with explicit copied input, with correct MIME/status and no failed
resource: 11 consumed resources. Notices are hash-bound, not claimed rendered.
The observer supplies no native backend and proves no negative CSP behavior.

Only one production CMake argument changes: the native CTest receives its exact
staged bundle path. The maintained native-ON inventory oracle first fails on that
missing argument (1.024822 s), then the full source-free scaffold guard passes
(88.456489 s), matching all 583 frozen inputs and selected runtime CTests 3/3.
The tiny build/inventory fixture is not the production rendering input. Final
normal editor CTests pass 3/3 in 1.971053 s with inherited bundle env removed;
Clippy warnings denied, native build, fmt, native/fast LLVM and std-only graph pass.
Rust production owner/dependencies/frontend remain unchanged. Native 129/134 lines,
12/14 functions/190/209 regions retain inherited gaps; fast assets 52/52. No new
production module/exclusion or branch claim. Fixture failures/corrections and
covered refactoring are retained in the [portable evidence](evidence/trigger-editor-production-2026-10-20/receipt.json).

One earlier characterization failed in pre-existing acquisition cleanup before
React/font loading. Its remaining child was not recorded; repeats do not resolve
it. Added diagnostic facts preserve identities/timing without weakening checks.
Pinned GTK source and installed binary independently show queued close work, so
current close()+pre-idle sync cannot guarantee immediate disposal. That ownership
finding remains open for the next lifecycle correction, documented with primary
sources in the evidence; this slice does not claim to repair it.

Independent completion review passed for the frozen source/evidence candidate; normal-hook publication is the remaining gate.
Task 3.2 remains unchecked, 7/14 overall; no native session/backend/audio or full
lifecycle claim. Continue its concrete facets, then 3.3/4.x/5.x and the subsequent
VST3/standalone/DAW/installed offline/native beta increments. Main remains clean
68dd655, mutation excluded and all remaining reusable caches preserved.


## Synchronous native editor disposal — 20 October 2026

Production React/font loading and native CTest bundle wiring were normally
published at a4cf062. The next bounded task3.2 slice replaces queued GTK close with
explicit destruction of the privately owned container, followed by a barrier on
the display captured before disposal. IPC disconnect and browser-first field drop
order remain. One directly scoped unsafe block documents GTK's no-post-disposal-
query contract; default/non-native builds, assets and native owner tests still forbid unsafe.
No host/engine/frontend/dependency/public-API change, GTK pumping or parent/display
ownership is introduced.

Genuine immediate-XID RED runs first; exact final test source also fails against
old production. Final RED differs from GREEN only by the two intended production/
lint files, with every phase snapshot separately bound. GREEN proves captured
normal and pre/post-WebKit rollback XIDs are BadWindow before ticks, preserves the
mapped parent geometry, and releases browser/IPC weak references immediately. A
separate host parent is then destroyed without intervening ticks; bounded service
finds no late callback or new root child. Weak-disposal notification is not a
GObject-finalization claim. The historical unidentified-child failure remains
unidentified; these new oracles qualify the current ownership contract independently.

All changed destructor lines execute10times. Owner131/136lines,189/208regions
retains19inherited gaps. Three registered CTests, denied-warningClippy/build/fmt,
native+fast LLVM/defaultgraph and scopedOpenSpec/specmap pass. Root full source-free
guard passes88.445691s (583exactinputs/16editorpaths), with caches preserved. The
[portable evidence](evidence/trigger-editor-disposal-2026-10-20/receipt.json) includes
all failed fixture assumptions, phase identities, raw profiles and the verified
cache relocation; no mutation or compiled executable is included.

Independent completion review passed for the frozen source/evidence candidate; normal-hook publication is the remaining gate.
Task3.2 stays unchecked,7/14overall. Continue show/hide/fixed-size and remaining
resource-origin/popup/download facets, then3.3/4.x/5.x and nextVST3/standalone/DAW/
installed-offline/native beta increments. Actual CLAPmainthread/API/order proof
still depends on4.2. Full beta remains active; main stays clean68dd655.


## Native visibility and fixed size — 20 October 2026

Synchronous disposal was normally published at9f9252c. The next bounded task3.2
slice adds UI-thread-checked show/hide and exact1200x800 acceptance. Two real X11
cycles retain the same browser/document/XID, one child, unchanged mapped parent
and literal physical geometry through hidden/shown/idempotent transitions. Forty
unsupported full-width requests fail without physical/content change; four exact
requests succeed. Ordinary page state persists from counter1 to counter2, with
one startup receipt. Existing immediate disposal/rollback, fetch/navigation and
production React/eightfonts/11resource positives continue passing.

Exact final tests first fail with specific E0599 absent-API errors; these are
compile RED, not runtime/setup failures. All17new executable owner lines are hit;
LLVM148/153lines,14/16functions,208/229regions retains inherited gaps and two
per-method thread-check propagation limits, with no unsafe score fixture or
extraction. Three CTests pass5.011941s plus denied-warningClippy/nativebuild/fmt,
native+fastLLVM/defaultgraph/scopedOpenSpec/specmap. The default source-free guard
is explicitly qualified reuse: same583pathset,581identicalinputs, only2native-cfg
files changed; no fresh-run or all-input-identical claim. All eight release checks
bind16exact source paths. The [portable evidence](evidence/trigger-editor-visibility-2026-10-20/receipt.json)
retains14phase bindings/raw profiles and the GTK global-alias safety clarification.
That comment correction changes no executable disposal behavior and makes no
universal Rustsoundness claim.

Independent completion review passed for the frozen source/evidence candidate; normal-hook publication is the remaining gate.
Task3.2 remains unchecked,7/14overall. Resource policy and popup/download behavior
remain; actual CLAPmainthread/API/order/timer proof depends on4.2. Full beta stays
active through3.3/4.x/5.x and nextVST3/standalone/DAW/installed-offline/native
increments. Main remains clean68dd655; mutation excluded; caches preserved.


## Native packaged resource origins — 20 October 2026

Visibility/fixed-size behavior was normally published atc3f6ca8. The next bounded
task3.2 slice changes only the native CSP response header to defaultnone plus
required script/style/font/connectself and the established literal-style allowance.
Four ordinary-page external resource baselines genuinely execute script, apply
CSS61px, load a valid font and load PNG1x1, each with browserGET. Aggregate denial
assertions then fail under the old header; corrected exact-final tests pass with
the header and zero browserGETs, exact enforcing effective/violated directive,
disposition,fullblockedURL and localdocumentURI receipts. Initial fullURL-or-origin
assumption failure is retained as oracle calibration, not a production defect.
All local and unmodified production Silent-preview/eight-font/11responsebyte
positives, physical presentation and immediate-disposal/rollback oracles remain.

[Portable evidence](evidence/trigger-editor-resource-csp-2026-10-20/receipt.json)
binds391rawmembers/15phase snapshots/19sources and all8checks. Owner148/153lines,
14/16functions,208/229regions retains inherited5line/21region limits; changedheader
covered, no extraction/exclusion/branch/mutation claim.3CTests4.034962s plus lint,
fmt/build/native-fastLLVM/defaultgraph/scopedOpenSpec/specmap pass. Qualified old
defaultguardreuse compares583published to586currentinputs:580identical,3changed/
3new native-cfg-onlypaths, no equalpathset/allinputidentity/fresh-run claim. Staged
extra unserved.gitattributes metadata is explicit:14declared vs15physicalfiles;
no unrelated packaging change. Existing exactsource nativecoverage is reused,
not rerun for a name; an alreadylaunched comparable RED/GREEN pair is retained.

Independent completion review passed for the frozen source/evidence candidate; normal-hook publication is the remaining gate.
Task3.2 remainsunchecked,7/14overall. Popup/download characterization remains;
actualCLAPmainthread/API/order/timer proof depends on4.2. No allunusedfacility-
denial/nativebackend/session/audio acceptance claim. Fullbeta continues through
3.3/4.x/5.x and nextVST3/standalone/DAW/installedoffline/native increments. Main
stays clean68dd655; allcaches preserved, mutationexcluded.
