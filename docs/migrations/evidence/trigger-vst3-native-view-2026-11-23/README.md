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
Native selection, feedback, complete callback
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
