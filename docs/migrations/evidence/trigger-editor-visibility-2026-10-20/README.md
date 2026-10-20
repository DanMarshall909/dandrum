# Native visibility and fixed size

This bounded task 3.2 slice adds UI-thread-checked presentation methods to the
existing owner. set_visible maps/unmaps its GTK child and synchronizes its display;
it retains the browser and document. set_size accepts exactly 1200x800 without
changing the child and returns UnsupportedSize for every other u32 pair. It adds
no resize state mirror, cast/narrowing workaround, browser rebuild, private loop,
dependency or format-specific type. No production module was extracted.

The existing two real X11 create/drop cycles now observe actual child attributes,
parent, geometry, same WebKit object/URI and ordinary page counter state through
hidden/shown/idempotent transitions. Four accepted requests and forty rejected
requests exercise both presentation states; zero, adjacent, u32::MAX and
65536-offset dimensions protect the full-width boundary. Child geometry remains
literal (0,0,1200,800), the host remains mapped with unchanged geometry, there is
one child/one added GTK toplevel, exactly one page-start receipt per cycle, and
counter1 survives each rejection before counter2 on reshow. Native evaluation is
only a positive click/inspection driver; it proves no negative policy behavior.
One cycle drops shown and the other hidden. Existing immediate exact-XID disposal,
rollback, host-parent destruction, no-late-IPC, fetch CSP, fixed navigation and
actual production React/eight-font/11-resource-byte positives remain asserted.

receipt.json, handoff.json and phase-bindings.json identify exact commands, exits,
timings and evidence scope. Deterministic author-raw.tar.gz plus its complete index
retain all 471 original files, including fourteen separately bound 16-path source
snapshots, logs, profiles/LLVM exports and tool/binary identities. The two REDs
are specific E0599 compile failures for absent inherent presentation APIs/variant,
not native/runtime/setup failures. The later RED has exact final test bytes; only
the owner file differs from release. Earlier final-prefixed receipts bind their
own pre-comment source, not release-final bytes. All eight release checks match
all sixteen final source paths. The diagnostic test refactor and safety-comment
clarification are followed by fresh exact-source verification.

All seventeen new executable owner lines are covered. Native owner LLVM is
148/153lines,14/16functions,208/229regions. Five unhit lines/nineteen regions are
inherited; two additional unhit propagation directions are the per-method UI-thread
check errors. The GTK/Wry owner cannot safely transfer across threads; no unsafe
fixture or score-only API was added. Shared WrongThread behavior is separately
exercised. Raw branch counters are zero: no branch/MC/DC claim or exclusion.
Fast assets coverage is separately52/52lines; it does not exercise the native owner.
Three registered CTests pass in5.011941seconds (native body3.54seconds), plus denied-
warning Clippy, native build, formatting, separate native/fast LLVM and default
dependency graph. Scoped strict OpenSpec and spec coverage also pass. Mutation is
excluded by user direction.

The default source-free build execution is qualified reuse, not a fresh run:
guard-reuse.json names the published disposal archive and its exact manifest.
The current complete 583-input inventory has the same pathset;581inputs match,
with exactly the two changed native-cfg owner/test files different. lib.rs gates
that whole module out of the unchanged default build; Cargo/CMake/Python and the
native-ON inventory-only command are unchanged. New native code is independently
compiled and exercised by the release checks. No claim that the old583manifest
matches current native bytes is made. Reusable caches remain intact.

The seven-line destructor safety comment now states GTK's global enumeration
limit precisely. global-alias-safety.md retains the separate pinned-source review:
private ownership/export policy does not mean no GTK aliases exist. Retained
references do not extend the live-widget lifetime, and must not query it after
destruction. This is a GTK lifecycle contract, not a universal Rust soundness
proof; no executable disposal change or new concrete memory-safety defect is
claimed. Historical published evidence is not rewritten.

Task3.2 remains unchecked,7/14 overall. This selected Linux X11/WebKitGTK2.50.4
scale1 fixture does not prove highDPI/scaling or actual CLAP API/order/mainthread/
timer behavior. Resource-origin/popup/download facets, real session/backend/audio
continuity,3.3/4.x/5.x and the subsequent VST3/standalone/DAW/installed offline beta
work remain. SHA256SUMS covers portable files except itself; archived absolute
paths are observations, not product resource-discovery fallbacks. Main stays clean
68dd655; no additional cache deletion is authorized.
