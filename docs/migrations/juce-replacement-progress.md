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
