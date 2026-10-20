# Synchronous owned native editor disposal

This material slice advances open task 3.2. OwnedWindow now captures its GDK display,
destroys only its exclusively owned GTK container, then synchronizes that captured
display before returning. IPC disconnect and browser-first Rust field drop order
remain. No event loop, wait, raw XDestroyWindow, host-parent/display/server disposal
or new public API is introduced. The safe GTK close request had queued idle work.

GTK exposes widget destruction as unsafe because callers must not query disposed
widget state. The single documented block has a directly scoped expect(unsafe_code)
under crate deny. Non-native/default builds, assets and native tests forbid unsafe.
The UI-thread owner exposes no GTK handles for later live use. Acquisition-local
wrappers may still exist on error unwind and only unref afterward. Display capture
precedes disposal; only it is used afterward. This is a concrete native-edge safety
boundary; no GUI/unsafe change enters the format-neutral host or engine.

Read receipt.json, handoff.json and phase-bindings.json. Deterministic author-raw
plus its index preserve every attempt, exact command/env/cwd/exit/timing, source
snapshot, profile/export and binary identity. Each phase explicitly names its own
snapshot and SHA256 manifest. Initial RED is not final. The final test source is
also run against exact old close+sync and fails: its captured XID still exists
immediately after Drop, before ticks/waits/test sync. Final RED differs from GREEN
only in the two intended production/lint files. Earlier final-prefixed attempts are
explicitly superseded before immediate callback/browser assertions, never relabelled.
The WM-origin and callback-probe expectation failures are fixture calibrations,
not behavior RED. All original records remain unchanged.

GREEN independently observes literal BadWindow/code3 for captured normal, actual
post-WebKit checked-reparent rollback and actual pre-WebKit builder-preflight error
containers before ticks. Normal drops additionally preserve mapped host parent,
observed origin and literal1400x900 size; destroy/unrealize/weak-disposal counters
are1. Browser/IPC weak references release immediately. A distinct owned host parent
is destroyed only after its child is already absent, with no intervening GTK tick;
32bounded external ticks then produce no late IPC or new root child. The native
coverage run sets G_DEBUG=fatal-warnings; separate libEGL stderr is retained. Weak
disposal notification is not claimed as independent GObject finalization.

New destructor executable lines run10times. Owner LLVM131/136lines,189/208regions
retains19inherited gaps; exact function/instantiation counts and unhit source are
in raw exports. No extraction, score-driven tests, exclusions or branch/MC/DC claim.
Existing actual React/eightfonts/11resource-byte positives, fetch CSP and navigation
remain passing. Three normal CTests pass with inherited bundleenv removed; native
Clippy warnings denied/build/fmt/defaultgraph pass. Root source-free/default guard
passes88.445691seconds (body87.483), all583current inputs and16final editor paths
match. The tiny guard fixture remains build/inventory-only, not native React input.

root-guard-raw retains that separate run. cache-relocation-raw retains preservation
of6388inactive guard-cache entries/2,585,172,929bytes from per-user RAM to global RAM,
with old logical path intact. First post-switch check stopped on root-symlink
metadata; both copies remained and resolved target/original were reverified before
removing only duplicate. No cache deleted. These are relocation-time hashes; later
normal builds can add artifacts. No target directory or ELF executable is archived.
SHA256SUMS covers portable files except itself; raw absolute paths are audit
observations, not product fallback discovery.

The historical acquisition failure in the previous production evidence still has
unknown remaining-child identity/cause. This exact-XID RED independently proves
and fixes the unsupported immediate-close promise; it does not retrospectively
identify that historical child. See the prior production editor-close-readiness
report for pinned GTK source/installed-binary evidence.

Task3.2 remains unchecked. Show/hide/size, remaining resource policy and actual
CLAP API/order/mainthread/timer proof remain; native session/notes/gestures/audio,
DAW/VST3/standalone/installed offline beta acceptance follow. No session continuity,
full lifecycle, finalization or complete beta claim is made by this native slice.
Mutation remains excluded. Main stays clean68dd655; remaining caches are preserved.
