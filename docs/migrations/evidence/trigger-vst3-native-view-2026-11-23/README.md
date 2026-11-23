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
Real browser host automation, native selection, feedback, complete callback
sentinels, package/offline qualification, installed REAPER, full validation and
independent candidate review remain outstanding; no task checkbox is completed
by this checkpoint. Windows/macOS are mandatory later native milestones.
