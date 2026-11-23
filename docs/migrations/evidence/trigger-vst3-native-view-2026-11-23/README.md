# Native VST3 view integration checkpoint

This is unreviewed Linux implementation evidence for tasks 4.2/4.3 of
`prove-trigger-vst3-linux`, not completed product or installed-host qualification.
The Trigger UI redesign is outside this work.

The real X11/Wry child now starts only after its scoped standard-message
namespace arrives. The processor remains the sole session owner. Native tests
observe actual parent pixels and child destruction; a standard-interface-only
proxy records authenticated browser parameter reads across recreation.
The browser oracle independently requires normalized pan 0.625, corresponding
to physical pan 0.25.

The follow-up teardown slice retains one bounded, GTK-free retirement responder
after view release, allowing a lost final acknowledgement to be retried. Refused
acknowledgements and host timer unregistration retain cleanup tokens. An
incomplete view cannot be replaced. Parent disconnect/termination revokes the
child, and controller termination releases its factory host context even when
its COM interface remains retained.

Task-private complete logs/receipts are under
`.git/codex/trigger-vst3-resume-20261123/`. Genuine failing behavior runs include
`native-paint-red`, `browser-corrected-red2`, `browser-delayed-red`,
`browser-teardown-red`, `native-controller-context-red`,
`native-parent-loss-red2`, and `native-cleanup-replacement-red`. Earlier browser
trials used an incorrect physical/normalized oracle; an earlier recorder aborted
and one parent-loss trial failed compilation. Those are not valid behavior REDs.

`native-parent-cleanup-green` passed all five selected view/native/browser tests.
`parent-cleanup-coverage` passed those tests plus six scoped channel tests. Its
fresh profiles cover 43/43 functions and 397/430 lines in `components/view.rs`;
branch coverage is unavailable. Uncovered defensive/error paths remain visible
in the private fresh coverage receipt. `parent-cleanup-clippy` passed native
feature all-target checks with warnings denied.

Production refactoring consolidates parent revocation through one controller
helper and normal/parent view cleanup through one lifecycle function. Tests
extend existing native scenarios to cover actual lifecycle outcomes rather than
introducing a second browser harness. Further extraction is not justified here.

The shared editor deferred-start change also passed the existing actual React
NativeEditor test and a fresh CLAP native lifecycle/audio regression. These
protect the shared owner, but do not substitute for VST3 production React or
REAPER acceptance. Current native VST3 browser tests use a minimal paint fixture.
Native mapping/history, complete callback
sentinels, package/offline qualification, installed REAPER, full validation and
independent candidate review remain outstanding; no task checkbox is completed
by this checkpoint. Windows/macOS are mandatory later native milestones.

## Native browser automation follow-up

The real WebView now submits Begin/Value/End commands through the scoped
standard proxy. One shared host-delivery policy serves controller and native
roles with separate receipts and retained accepting handlers. The genuine
`native-gesture-red2` reached processor automation but stalled at the missing
native receiver; `native-gesture-red` was a compile error, not behavior evidence.
`native-gesture-green` passed after routing native deliveries through that policy.

`native-gesture-faults` and `native-gesture-final-coverage` prove a lost
automation acknowledgement cannot duplicate Begin, a refused Value retries,
reentrant view/controller/processor teardown is fenced during host callbacks,
and a held gesture prevents handler replacement. Removing the native child
with a held gesture retries a refused End through editor-absent service.
After all four views disappear, actual MIDI/audio processing asserts silence
through frame4 and literal signed stereo (-0.9375,-1.25) at frame5: source
-0.5 times physical level2.5 with pan0.25. The selected coverage run also
passes31 existing controller transport/automation regressions and all five
view/native tests. Native all-target Clippy passed before the final borrow
scoping/format-only refinement; final combined gates remain required.

The production refactor groups receipt/gesture state as HostDelivery and
parameterizes the existing policy by its scoped transport; it introduces no
new module or musical authority. Test variants share one native scenario and
record only externally accepted host callbacks. No further extraction is
justified before the remaining real editor integration is covered.

## Native feedback follow-up

`native-feedback-red` genuinely proved that stopped host automation reached the
controller but never reached the browser namespace. `native-feedback-green`
passes after publishing the one shared feedback record to each attached role,
with separate event encoding and bounded queue space reserved before consumption.
`native-feedback-final-coverage` also fills the native queue beyond eight
packets while refusing delivery, then observes the final browser value without
a new request or host gesture;31 controller and6 browser-channel regressions
pass. The existing native-ring overflow/snapshot regressions remain green.
`native-feedback-clippy` passes with warnings denied.

Production keeps one feedback consumption loop and encodes each attachment
inside the same covered owner boundary, avoiding parallel feedback authorities.
Tests extend the existing actual-browser scenario with a distinct pixel oracle
for host updates and reuse its bounded recorder. Error-only send/owner-borrow
paths remain uncovered; no whole-owner100% or branch-coverage claim is made.

## Production React and native acquisition follow-up

`production-editor-package` builds the existing unchanged React UI into private
task output. `production-selection-red2` genuinely reaches that packaged React
backend and receives capabilityUnavailable; the first trial was a test type
compilation error. `production-selection-green` passes after native chooser
admission is connected to the existing shared owner. No UI redesign is included.

The processor retains the NativeSelectionPermit, while a scoped native-only
message carries bounded exact Linux path bytes. Browser commands still cannot
supply paths. Results retain an application acknowledgement and one outcome
receipt, so accepted-but-lost delivery cannot reopen files or admit work twice.
Shared selection status owns terminal acquisition/decode errors.

`production-selection-faults` drives actual GTK selection/cancellation through
the production WebView. It checks the literal60byte WAV's independent hash,
32000Hz,2channels,4frames; file removal and view recreation preserve the result.
Lost acknowledgements retry without reopening the deleted file, then stop after
acceptance. View removal destroys an open chooser and cancels choosing work.
Already admitted decoding completes through persistent host service after source
and editor loss while stopped. A separate standard-message boundary test rejects
metadata-role acquisition, malformed batches, wrong correlation and conflicting
duplicates; preserves the permit after rejected acquisition; retains failed and
cancelled outcomes; and acquires a real WAV through a non-UTF8 directory.

`selection-owner-coverage` passes31 controller,7 scoped browser-channel and the
production native selection test after the final RequestLane refactor. Native
and headless all-target strict Clippy pass. Earlier headless dead-code/closure
lint failures are retained as setup/refactor diagnostics. RequestLane groups
borrowed transport/progress data and excludes native permit machinery from the
headless build. No new production module was extracted. Public GTK/WebKit test
helpers inspect the exact child and do not prove code residency; that requires
the separate loader-only native lane. Complete owner coverage and final gates
remain outstanding, as do actual native mapping/history and installed REAPER.
