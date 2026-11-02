# Trigger CLAP beta final acceptance audit — 3 November 2026

Scope: task **8.2** of `add-juce-free-trigger-beta-products`, the private/local
Linux x86_64 X11 Trigger CLAP beta. This candidate awaits independent completion
review and the exact canonical-sync/publication envelope. The task checkbox is
the criterion-completion authority; `.git/codex/deferred-work.md` owns workflow
status. The broader JUCE migration, VST3, AU, standalone and other OS/host support
remain outside this completion claim.

## Product and evidence identities

The qualified current product remains the task 8.1 installed CLAP
`da9fc60fa566ce77e882bf584cd5c9a00a905fa0929438374badc5ef04478186`.
All 20 installed files were rehashed against the
[qualification manifest](../trigger-beta-qualification-2026-11-02/installed-manifest.json)
without mismatch. Production inputs match the tested `7ea453e2510fdd3c40f40248699356b2440bcf16`
checkpoint. Main at task start was `5a9de6be86d27d1e69517dbe823c595784887b6f`;
later sampler changes do not enter this CLAP product. This task adds historical
DAW replay assertions and their CTest registration; it changes no product code,
dependency, decoder, state format or accepted requirement wording.

The REAPER 7.84 package remains
`fd24495233b94466a72de31ec302fd0089f9a4293077766ee533cda77aa552d8`.
All 20 of its retained installed files also matched their original manifest.
Its [DAW packet](../trigger-clap-daw-acceptance-2026-11-02/README.md) retains actual
host execution, selected WAV inputs, saved/restored state, automation observations
and four signed rendered WAVs. The auditor independently checked all 448 archive
members, all 96,000 channel samples in each render and both 114,028-byte restored
states. All 342 members of the current qualification archive were also checked.

The maintained replay
[`test_trigger_clap_daw_evidence.py`](../../../../tests/python/test_trigger_clap_daw_evidence.py)
checks the four renders against literal PCM16 patterns, note onsets and physical
gains; it checks complete saved/fresh state bytes, owned payload hashes and
decoded PCM, 23 named values, instance independence, host/process identities and
nonempty timed automation/held-gesture/release observations. Controlled sign,
fresh-state and held-gesture faults must be rejected. It reads archive members
without extracting files, launching REAPER or trusting PASS summaries. It is a
historical artifact oracle, not a fresh live-DAW run or a current-source CI DAW
launcher. Original-source deletion and fresh-process configuration are retained
execution observations; a replay cannot recreate those historical conditions.

Draft independent review found that equal saved/fresh states could both rename
the selected resource as packaged and skip its content oracle. A genuine RED
test demonstrated that coherent fault; the repaired check requires exactly one
literal selected path/hash per instance and all five tests pass. Cancelled/invalid
selection mappings also explicitly include real acquisition/read-error tests.
These repairs strengthen the audit without changing the product or its contract.

## Why the historical DAW evidence still applies

Acceptance is composite, with each artifact identity retained. No claim relabels
the older REAPER process as a run of the current binary. From the DAW checkpoint
through task 8.1, the CLAP/editor/host/session production Rust sources are unchanged.
The affected boundaries are covered as follows:

| Changed boundary | Current evidence supporting continuity |
| --- | --- |
| Engine preparation/render refactors, raw FFI output initialization and safety contracts | Task 8.1 strict engine coverage/checks and signed numerical characterizations; actual current-product dynamic signed rendering, fresh selected/packaged restoration, independent runtimes, native continuous audio and complete callback allocator audits. Repeated task 8.2 CTests retain the same product. No performance or complete engine coverage claim follows. |
| rtrb 0.3.2 → 0.3.5 and ZIP 9.0.5 | Locked current dependencies, actual capacity-one runtime handoff/reclamation and native server overlap/state/parameter checks. ZIP header-consistency, seed and library regression checks are in task 8.1. The queue API/ownership and serialized CLAP contract remain unchanged. |
| Frontend rejected-value rollback now always ends its gesture | Genuine task 8.1 regression requires once-only end, preserved accepted value, retryable Undo and an error. Successful pointer/host gestures are covered by rendered-browser/native event checks and current installed overlap. This repairs the error path; the historical successful DAW gesture/timing observations remain applicable. |
| Removed unused alternate drag handling, frontend Vite update and current production bundle | Current authored types/components, 314 Node tests and 43 browser tests, actual React/Wry lifecycle and XTest-selected import/drag/Undo/Redo on the current installed product. Historical DAW PCM/state expectations are independent of bundle hashes. |
| Later sampler Vite update | No Trigger production input changes after task 8.1; the sampler output is a separate consumer. |

The qualified DAW configuration remains REAPER **7.84 Linux x86_64**,
`linux_ime=1`, X11/GTK scale one; live timing observations disable anticipative
FX. Default REAPER settings and Qtractor's tested AppImage remain unqualified.
The task notes' earlier hands-on selection result is historical context: its
portable raw human-input log was not located by this audit. Completion relies
on current actual XTest/native GTK/server-callback evidence, not a new physical
MIDI/DAC, human-approval or fresh DAW claim. The requirements call for actual
native input; XTest enters through the real native chooser and pointer boundary.

## Every retained scenario

[`scenario-evidence.json`](scenario-evidence.json) lists the exact test identities
and reviewed fingerprints for all **18** scenarios. Multiple tests supply
different clauses where needed; the table names the observable assertions and
the external evidence that a test name alone cannot supply.

| Scenario | Proving behavior and evidence |
| --- | --- |
| Installed CLAP renders the real Trigger instrument | Actual dynamic descriptor/public interfaces, offset notes/nondefault physical values and literal signed sample outputs; task 8.1 installed native and isolated headless artifact executions. |
| Fractional host clocks render at the requested rate | Exact 1234.5678/48000.5 clocks, independent sample cursors/reconfiguration/reset and signed timing through real owners; task 8.1 unfiltered official Trigger validator: 34 successes, 10 declared skips, no warnings/failures. |
| Failed configuration preserves accepted playback | Invalid clocks, failed resource preparation and equal/reactivated configurations preserve accepted history/values/signed output and another instance. |
| Native editing survives complete editor replacement | Current installed chooser/import/public drag with actual server callbacks, source removal, complete WebView destruction/absence/recreation and signed Undo/Redo; separate held-note/gesture destruction and lost-reply recovery checks cover once-only cleanup. [Native boundary](../trigger-clap-native-overlap-2026-11-01/README.md), rerun in task 8.1. |
| Failed native creation leaves the instrument usable | Invalid ordering/parent cases plus actual duplicate Wry IPC-registration failure; partial view/callback cleanup, saved nondefault state, signed audio and successful subsequent creation. |
| Native callbacks retain bounded ownership during replacement | Complete current CLAP process/reset allocation/free sentinels on real server threads, watched progress, signed selected transitions and unchanged second instance; native continuous-audio and runtime transfer/reclamation tests cover complementary ownership. |
| Two DAW instances retain automation and selected samples | Actual REAPER native gestures/automation/editor absence/reopening; A state unchanged by B import; fresh process with original sources gone, exact complete state/named values and four signed renders. Maintained artifact replay plus current gesture/state/queue/native regressions justify continuity. Exact begin/end counts are composite browser/native-boundary evidence; the uninstrumented DAW did not record raw CLAP event counts. |
| Source-hidden offline installation remains usable | Task 8.1 actual network/PID-isolated bubblewrap runs hide checkout/Cargo/cache/original tmp, use unrelated cwd and synthetic `/dev`: three headless cases include fresh selected restoration after package-source removal; a separate real React/Wry lifecycle probe proves GUI/audio. Same installed manifest and isolation policy, distinct complementary probes; no assertion that chooser import ran inside the isolated GUI probe. Source-absent builds, native linkage and notices are separately audited. |
| A distinct selected sample is mapped and played natively | Real chooser/React import metadata (including a distinct stereo/rate fixture), verified owned content identity, once-only normal mapping and literal signed output at mapped keys; current XTest installed overlap. |
| Cancelled or invalid selection preserves the instrument | Actual chooser cancellation; malformed/unreadable/unsupported acquisition and bounded file/resource decoding rejection, with accepted state/history/values/audio and later valid selection preserved. |
| Original-file changes do not alter admitted playback | Immutable encoded/decoded ownership, source replacement/removal, control-thread lifetime/reclamation, native Undo/Redo after removal and independent installed instance. [Foundation audit](../trigger-selected-foundation-completion-2026-10-29/README.md). |
| Import preserves nondefault controls and native history | Native document publication changes authored defaults while retaining live physical values/identities; retries publish once, mapping edits and UI Undo/Redo assert signed outputs/history. |
| Empty patch and ordinary import form a usable transition | Resource-free Empty is silent with all 23 supported controls; new editor imports selected PCM at a valid unoccupied mapping and plays signed output without layout/kit-overlap failure. |
| Failed patch candidates retain the accepted instrument | Unsupported public interface/resource profile, stale/conflicting/replayed identities, bounded busy admission and real failed preparation preserve accepted state/feedback/history/audio and permit valid retry. |
| Lost editor recovers one admitted sample edit | Native withheld-reply/backpressure scenario removes source and destroys editor; a new production consumer discovers one retained outcome, publishes once and uses native Undo/Redo, with no late old-view continuation. |
| Fresh instance restores selected content after source removal | Actual CLAP state restoration in another DSO after source instance/file loss and package WAV removal; logical identities/all 23 values and selected/kit/unmapped literal PCM, plus actual REAPER fresh-process evidence. [Fresh-state packet](../trigger-clap-fresh-state-2026-10-30/README.md). |
| Corrupt portable state preserves the live instrument | Unsupported versions/content/hash/layout, malformed counts/lengths, excessive payload and failing/short streams reject before partial publication; nonempty accepted history/feedback/commands/audio survive. Actual maximum-payload slow-lane test validates size/tail geometry/reclamation. |
| Packaged-kit state remains readable | Literal prior version-2 fixture loads through real CLAP state into the beta with named values and signed output; invalid legacy state and failing reads preserve the live instrument. |

## Current verification

[`verification.json`](verification.json) records the latest proving result for
each of all **72** configured CTest cases. This is a composite checkpoint:

| Run | Result |
| --- | --- |
| Full configured suite, 633.86 s | 44 pass; 28 native GTK cases fail because this first invocation omitted DISPLAY. The actual maximum-payload slow test, all source-free builds, Rust/static gates, install/notices and dynamic hosts pass. Failure output remains retained. |
| Owned Xvfb/GTK retry, 138.96 s | All 28 failed cases plus four required fixture preparations pass: **32/32**. Includes real XTest chooser/input, server audio/MIDI overlap, complete callback allocator audits, failed creation, held activities, independent instances and selected import/recovery. |
| Final affected registered replay, 0.42 s | **1/1 CTest**, all five artifact-oracle tests pass after the reviewed selected-resource repair. Initial run's earlier four-test result is superseded by this final test source. |
| Independent artifact audit | All 790 retained archive members and both 20-file installed manifests match. Current build and installed CLAP have the same qualified hash. |
| Proposed canonical specs/mappings | **55/55** strict specs; **657/418/239** scenario/mapped/unchanged-backlog counts. Existing fingerprints and accepted requirement bodies are unchanged. Actual canonical validation repeats after the approved sync. |

[`verification-raw.tar.gz`](verification-raw.tar.gz) retains exact commands,
XMLs, failures, RED/GREEN logs, the native environment recipe and both supervisor
cleanup receipts, indexed and read back by [`archive-manifest.json`](archive-manifest.json).
Both supervisors report no remaining workers and removed owned scratch
(13,185,024 allocated bytes across the two scratch directories). This is the
owned scratch measurement, not a claim that unrelated host free-space changes
were caused by this task.

## Canonical synchronization and completion gate

[`canonical-sync.patch`](canonical-sync.patch) is the exact proposed post-review
transformation: add the two canonical specs without changing any accepted
requirement/scenario body, and append 18 genuine test mappings. No existing
fingerprint or ratchet entry is rewritten. The proposed result already passes
strict validation of all **55** canonical specs and the mapping check:
**657 scenarios / 418 mapped / 239 unchanged backlog**. Canonical files remain
unchanged until independent PASS. The active change remains available for a
separate archive operation after completion.

The publication envelope records the candidate/base IDs, exact content hashes,
status/task updates, normal hook/validation commands, source-only PR/merge and
bounded cleanup. Product/test/replay binaries stay local. Review cannot be
self-approved. Final execution results and the review receipt are retained here.

## Limits and retention

The [task 8.1 packet](../trigger-beta-qualification-2026-11-02/README.md) remains
the detailed qualification/coverage/dependency/notice authority: partial engine,
host and frontend coverage; unsupported native slice remapping; current GLib and
system-library advisory/version limitations; five Rust archives without root
licence text; licence-text packaging limits; and the experimental signed-gain
descriptor's denormal timing warning. These are disclosed limits, not hidden
passes, a security-clear finding or redistribution clearance. No first-party
distribution licence was selected. The Trigger descriptor itself has no current
validator warnings/failures. Mutation remains excluded by Dan's direction.

Task 8.2 uses disk-backed reusable caches and one heavy job at a time. The
maintained owned-run supervisor retains commands/statuses and proves stopped
workers before removing its scratch. No prior shared cache, build, dependency,
private binary or uncertain historical evidence is deleted. The first replay
calibration exposed slow unittest whole-list diff formatting; that task-owned
process was interrupted (exit 130), and indexed sample assertions now fail
promptly with the exact frame/channel. This was test reporting, not a product
failure or discarded acceptance failure. There is no new production extraction
or refactor; the assertions remain beside their historical external evidence.
