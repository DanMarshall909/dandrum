# Linux native Trigger VST3 editor evidence — 23 November 2026

Local implementation evidence for tasks4.2/4.3 of `prove-trigger-vst3-linux`.
The combined product remains unreviewed and unpublished; installed/offline
REAPER qualification, final gates and independent completion review remain
required by [issue45](https://github.com/DanMarshall909/dandrum/issues/45).
Native Windows/macOS remain mandatory subsequent milestones. The existing
production React UI is used unchanged; its redesign is outside this change.

## Demonstrated boundaries

| Boundary | Proving assertion |
| --- | --- |
| Native parenting/paint | Actual1200x800 X11/Wry child; literal paint fixture and production React header colour#E08A4E; host parent survives actual destruction/recreation. |
| Failed creation/service | Missing assets, invalid parent, refused registration/unregistration and reentrant lifecycle calls fail explicitly; accepted signed playback and nonempty native history survive, then a valid attach succeeds. |
| Scoped browser transport | Standard-interface-only proxies, delayed authenticated bootstrap and normalized pan0.625/physical0.25; no second instrument owner. |
| Host gestures | Actual WebView Begin/Value/End, lost application Ack, refused Value/End, retained accepting handler, reentrant teardown fencing and editor-absent cleanup; no duplicate accepted host callback. |
| Live feedback | Separate attachment namespaces; refused native route beyond its eight-packet queue converges to the final browser value without a new request or extra host gesture. |
| Native acquisition | Real React/Wry/GTK chooser; processor-owned permit; exact bounded Linux path bytes; malformed, foreign, stale/conflicting and browser-path requests rejected without consuming valid authority. |
| Retained outcomes | Independent60byte WAV hash,32000Hz,2channels,4frames; once-only acquisition survives lost Ack, source removal and recreation. Choosing cancels on view loss; admitted decoding finishes while stopped/editor-absent. |
| Native mapping/history | Actual React numeric entry and Add source; signed MIDI48/49 and packaged MIDI36 renders; source deletion, native recreation and actual React Undo/Redo recover the expected sound/history and preserve a second processor's state/audio. |
| Active callbacks | Actual React public editing, GTK selection, mapping and graph replacement overlap a real audio thread; independently specified signed old/new values, newly mapped source and zero tail are asserted. |
| Held notes | A representative composed gated sample-player child renders [-0.5,0.25,-0.25,0.5], sustains -0.125, becomes exact silence after native view loss and cannot replay its old press after recreation. |
| Code residency | Separate loader-only host has no preloaded WebKit. Native objects/timers/context references disappear, module releases, GLib dispatches, UI-thread/TLS destructors finish, then exact product/WebKit mappings survive a balanced NOLOAD rescan. |

Native acquisition does not serialize the NativeSelectionPermit or accept paths
from browser commands. One bounded result/receipt pair provides application-level
retry and deduplication. Shared selection status retains failures and cancellation.
Rust OsStr bytes preserve acquisition paths without forcing UTF8; a real WAV
under a nonUTF8 directory is covered at the standard-message boundary.

Closing has an explicit application receipt before Closed can supersede it.
Refused Closing, lost/refused ClosingAck and refused/lost RetiredAck all settle
through editor-absent service. The original requirement that Closing precedes
Closed, and the automation foreign-call fence, remain enforced.

## Native checkpoint verification (9f3b31f6)

Complete private commands, source hashes, timings, terminal status and raw output
are retained under `.git/codex/trigger-vst3-resume-20261123/`.

- `native-owner-final-coverage`:83 ordinary owner tests pass; six ignored entries
  are three separately run native lanes and three parent-only subprocess entries.
- `native-boundary-final-coverage`: all three actual native lanes pass, including
  the calibrated allocator sentinel during actual editing/graph replacement.
  This run measures718 complete start/process/zero/stop callbacks and358 signed
  renders, each with zero allocations and zero frees. Counts depend on scheduling.
- `native-sentinel-calibration`: independent real-DSO allocation and preexisting
  destruction controls pass; complete valid/error/zero/note-off/reset,1024-input
  curves and prepared replacement/reclamation callbacks remain clean.
- `native-final-clippy`: native-feature all-target Clippy passes with warnings
  denied. Headless all-target Clippy passed at the selection-owner refactor.
- `native-combined-receipt.json`: only fresh profiles from the current-source
  ordinary and boundary runs are merged after checking identical source/object
  identities and every raw profile hash. View52/52functions,516/566lines(91.17%);
  components150/153functions,1992/2087lines(95.45%). Defensive/error gaps remain
  explicit; branch/MC/DC coverage is unavailable. This is no100% owner claim.

The shared NativeEditor deferred-start change passed its actual production React
owner test and a fresh CLAP native lifecycle/audio/residency regression earlier
in this session. Its source has not changed since those runs. These are shared
owner regression evidence, not substitutes for VST3 native assertions above.
The final combined product gates remain task6.1.

## TDD and refactoring record

Genuine missing-behavior REDs: `native-paint-red`,
`browser-corrected-red2`, `browser-delayed-red`, `browser-teardown-red`,
`native-controller-context-red`, `native-parent-loss-red2`,
`native-cleanup-replacement-red`, `native-gesture-red2`, `native-feedback-red`,
`production-selection-red2` and `native-lost-closing-recovery-red`. Later React
mapping/gated-note/concurrency checks characterize the already shared behavior
through its newly connected real adapter boundary; they required no DSP changes.

Invalid trials remain separate: early browser failures used a physical value
in a normalized wire oracle; a recorder aborted; several first trials failed
compilation/linting. The first lost-Closing repair weakened the existing ordering
guarantee, was caught by the wider wire tests and was reverted. The accepted
ClosingAck repair preserves that guarantee. None of these diagnostic failures
counts as a valid behavior RED or passing acceptance evidence.

Production refactoring groups host receipt/gesture state as HostDelivery and
parameterizes one callback policy by scoped transport. RequestLane groups borrowed
transport/progress inputs and excludes native permit machinery from headless
builds. Parent revocation and normal/parent cleanup share single lifecycle paths.
No additional production module was extracted. Tests reuse calibrated public
GTK/WebKit inspection and exercise distinct state transitions in one UI-thread
lane; a separate loader-only executable preserves the residency oracle.

No mutation jobs ran. Private product, host and test binaries are not published.

## Validator host-capability follow-up

The first unfiltered extensive official candidate validation discovered both
classes, then component initialization failed and the validator crashed. The
retained traced rerun and pinned SDK source identify its DummyRunLoop: it
advertises IRunLoop but every registration method returns kNotImplemented.
Neither failed trial is qualification evidence.

`dummy-loop-red` specifies signed headless operation in that host capability
case. `dummy-loop-green` passes all six service tests; `dummy-loop-coverage`
passes the affected discovery, processor-owner and service selections. Worker
startup now follows successful service registration; explicit unsupported
registration leaves the optional service absent. Ordinary registration refusal
remains a failure, and disabled cleanup tokens still block duplicate startup.
`dummy-loop-clippy` passes all native targets. `dummy-loop-native-regression`
passes the actual production editing lane with380 signed renders/762 complete
allocator-clean callbacks. The source change is isolated to initialization;
full current-candidate validator and final combined qualification remain pending.

Refactoring delays worker creation until a service actually exists, removing
unserviced background authority. Existing rollback/cleanup tests retain their
observable contract. No new module was extracted.

The second extensive validator trial completes without a crash and reports274
passed/3failed tests, plus cascading setup/teardown failures. The retained output
identifies processor terminate/reinitialize and mandatory process-context support.
`component-restart-red` proves the same-component restart failure;
`component-restart-coverage` passes all seven service and31 controller tests after
restoring fresh initialization with a weak factory-service origin. No context
reference is retained solely to enable restart. Nondefault old sound is discarded,
new setup is required, literal default signed sound returns and both timer
lifetimes release. The small lifecycle refactor adds no module; wider controller
retirement/foreign-call regressions remain green. Validator requalification is
still pending the remaining identified contract.

The mandatory SDK3.7+ IProcessContextRequirements interface now declares a zero
mask: Trigger consumes no optional ProcessContext fields.
`process-context-contract-red` loads the retained v2 candidate and fails at the
missing public interface; the earlier compile-error trial is not behavior RED.
The existing discovery expectation rejecting restart is updated to the new SDK
contract; its stronger signed sound/service test remains the lifecycle oracle.
An initial edit matched the wrong repeated assertion; the wider test caught it
and the missing-resource rejection was restored before acceptance.

`process-context-coverage-corrected` passes all eight discovery and seven service
tests. Fresh profiles are retained separately; they exercise the new bitmask
implementation, not the native-view owner. The declaration adds no module or
state and needs no further extraction. Strict native Clippy is retained as
`process-context-clippy`. Full extensive candidate validation follows packaging.

## Physical keyboard follow-up

Actual installed REAPER pointer selection works, but numeric keystrokes exposed a
focus defect in the original foreign GtkWindow wrapper. Those host trials are
diagnostic, not task5.2 acceptance. `native-keyboard-red` first specifies real
XTest key entry through IPlugView focus; no JavaScript value setter supplies the
new value. An intermediate raw-X focus assertion further isolates the missing
focus transfer. Early event-mask/GDK-toplevel repairs fail startup or delivery
and are retained as failed trials.

The accepted container uses GtkPlug inside the existing checked/owned outer X11
child. Explicit fixed-size allocation/mapping preserves the existing host loop
and1200x800 contract; IPlugView focus grants the inner native child keyboard
focus. GTK owns its widget/window; the existing outer owner still releases the
child after GTK disposal and leaves the host parent intact. No musical/editor
behavior or module is forked/extracted. Public GTK test discovery now accepts
a descendant of the exact owned X11 child, still requiring one WebKitWebView.

`native-keyboard-plug-map-green` passes actual key entry (physical2 -> normalized
0.5), exact host callbacks and the full production journey.
`native-keyboard-shared-owner` passes the owning GTK lifecycle/resource/production
asset/font/error cases under fresh coverage. `keyboard-owner-final-coverage`
passes all ordinary VST3 owner tests; `keyboard-native-final-coverage` passes
all three explicit native lanes, including loader-only residency and calibrated
736 complete callbacks/367 signed renders with zero allocations/frees during
actual editing. Counts are scheduling-dependent. Further current-candidate
CLAP/Clippy/installed/REAPER gates remain separately recorded.

`keyboard-clap-native-regression` passes fresh CLAP native lifecycle, literal audio,
React startup, recreation and residency checks with the shared container change.
`keyboard-shared-focus-coverage` adds the owner-layer focus-subtree assertion so
the shared focus method has its own instrumented owning executable evidence;
plugin-only coverage does not cover dependency source. `keyboard-combined-receipt`
checks identical current VST3 sources/object and36 fresh profile hashes: view
53/53functions,527/578lines(91.18%). Defensive/error gaps remain explicit; no whole
owner100%, branch or MC/DC claim. No further production extraction is warranted.

## Pointer-initiated keyboard focus

REAPER does not supply IPlugView::onFocus for the observed pointer-entry path.
`native-pointer-focus-red` and the controlled
`native-pointer-isolated-baseline-red` replay against retained candidatev4 fail
at the missing pointer-focus assertion. The final repair uses the same checked
X11 helper for host grants and WebKit button presses. The callback captures a
weak Rc connection: it cannot keep the native connection alive or resurrect it
after the window owner drops. No musical authority is retained.

OS key qualification uses a dedicated task-owned Xvfb display. The shared desktop
did not deliver keys during diagnosis; its exact cause is unestablished. Isolated
GTK/DOM event observation showed key2 arriving and appending to1 during focus
activation. The maintained test now physically uses Ctrl+A, waits for selection,
types2, waits for the draft, then commits with Return. It asserts the no-change
pointer gesture and the subsequent balanced value gesture separately. Protocol/
event-mask diagnostic changes were removed after the minimal repair passed
`native-pointer-pruned-green`; they are not part of the accepted implementation.

`pointer-shared-owner-coverage` proves both explicit focus and native-click focus
in the owning executable along with its full lifecycle/resource/native cases.
An isolated coverage run exposed an immediate-cancellation test assumption; the
contract permits pendingchoosing until the asynchronous cancellation receipt.
The test now bounds that wait and still requires the exactcancelled outcome.
Complete current-source owner/native/CLAP and strict Clippy receipts follow below.

Current pointer repair: `pointer-owner-corrected-coverage` passes all86 ordinary
VST3 tests. `pointer-native-corrected-coverage` passes all three native lanes on
the dedicated display, including722 complete allocator-clean callbacks/360 signed
renders during actual editing. `pointer-clap-native-regression` passes with a
fresh CLAP binary and the current complete production editor assets. Strict native
Clippy is retained as `pointer-final-clippy`. The shared owner test independently
covers the new click callback/helper; defensive transport/foreign-lifetime errors
remain explicit gaps rather than a100%/branch-coverage claim. No new module was
extracted, mutation remains excluded and actual installed-REAPER is still open.
