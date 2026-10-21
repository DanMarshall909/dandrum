# NativeEditor application-handler ownership

This focused repair extends published `73af4709835d88dfe7b1f0bae64ae7e835485772`.
It supports the open tasks 3.2/3.3/4.2 of
`add-juce-free-trigger-clap-editor`; it closes none of those tasks or the beta goal.

WebKit can retain a custom-protocol wrapper after its WebView dies. Previously
that wrapper retained the application resource closure and assets. `NativeEditor`
now owns a guard that clears the application handler on destruction and on any
returned construction error. The framework wrapper remains valid and returns
404 after its application owner is gone. IPC disconnection, public API,
navigation policy, manifests and lockfile are unchanged. The mutex is confined
to native UI resource serving; the audio callback and format-neutral core are
unchanged.

The real X11/WebKit test retains an actual context, observes live resource and
navigation callbacks, drops the owner, and asserts immediate WebView/XID and
application-payload release. A later request through the retained context gets
404 without another application call. A separate phase destroys a previously
validated X11 parent so reparenting fails after protocol registration, then
asserts closure/assets release and no added native child or GTK window.

CMake now prepares the native release test binary in a separate serial fixture
with a 1,200-second build timeout. Both exact ignored native tests require that
fixture, run serially with 60-second runtime limits, and reject libtest's
otherwise-successful `running 0 tests` result. A configure-only Python test
checks enabled/disabled registration. Root calibration executes both actual
native cases, then changes only disposable generated CTest metadata to a missing
filter and checks the explicit zero-test failure diagnostic before restoring it.

## Evidence and qualifications

`author.tar.gz` retains author receipts, raw logs, before/final source snapshots,
compiler and source bindings, RED evidence, refactoring evidence, LLVM raw exports
and the three fresh final raw profiles. `author-artifacts.json` records every
member's size and SHA-256. ELF bytes remain in the reusable caches; their hashes
are recorded rather than shipping build binaries in Git. `root.tar.gz` retains
CMake RED/GREEN, actual native positive/negative calibration, source-free build
logs and specification checks. Local absolute paths in raw receipts describe the
original execution; archived source snapshots, hashes and commands bind them
portably.

- Retention calibration 57 is runtime assertion RED: assets/scheme/navigation
  release was `(false, false, true)`. The missing private-constructor tests
  58/60/62 are compiler RED, not runtime assertion failures. Calibration used
  the combined consumer graph; later ordinary/coverage checks use the unchanged
  standalone editor lock, with 17 transitive differences explicitly retained.
- Final ordinary checks 81–84/91 pass fast tests, the new ownership case, the
  existing complete public native regression, strict all-target Clippy and fmt.
  Native production assets are the current React silent-preview bundle with
  eight fonts; this is editor-library calibration, not the actual CLAP session.
- Exact owner LLVM coverage is 194/200 lines, 266/288 regions, 19/21 functions
  and 54/65 instantiations. All new guard/helper/release source-coordinate
  ranges execute. Platform, IPC and public error-propagation gaps remain;
  LLVM exports no branch entries. No whole-owner or branch 100% claim applies.
- Covered refactoring boxes the existing private resource-handler trait object,
  reducing emitted helper/wrapper copies from 24 to 2 without changing assertions.
- Mutation testing is explicitly excluded. No callback allocator, plugin
  residency, browser/global quiescence, named DAW or full-beta proof is claimed.

The unpublished adapter's executing-module identity and native code-residency
checks remain failing and are excluded from this candidate. Their repair and
complete GUI/session/automation integration follow this independently useful
library correction. Main remains at `68dd6553a2f07e71e2fe09ee1d7094d518559937`.
