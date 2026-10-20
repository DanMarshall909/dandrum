# Owned GTK window close/destruction readiness

20 October 2026. Bounded read-only investigation against the current editor owner and retained production-characterization source/logs; requested base `e9f7e09`. No Git query, code/cache change, build, test, native launch, display/loop operation or shared-resource mutation. Only this report and its source manifest were written. The failure stays open. Local hashes, binary inspection commands, and pinned upstream source URLs are bound in `editor-close-readiness-sources.json`.

## What is observed

`/tmp/dandrum-lanes/evidence-editor-production/production-characterization-compiled.log` records one failure at the final `query_tree(parent).children.is_empty()` after dropping the deliberately retained first owner/context and eight immediate service calls. Its frozen source (`production-characterization-compiled-source/src/trigger-editor/tests/internal/native_x11.rs:323–338`) had already checked the GTK toplevel count against baseline before the failing X11 assertion. It did **not** record remaining child IDs, owner IDs, per-tick work or elapsed time. The child cannot be identified retrospectively, and the passed aggregate GTK count does not identify which object disappeared.

Later runs passed. `production-consumed-resource-characterization.log` records `children=[]`, service counts `[3,0,0,0,0,0,0,0]`, elapsed `396.346µs`. Other retained successes report similar observations. They establish those successes, not the cause or absence of the earlier failure. No new run occurred in this investigation.

Current `OwnedWindow::drop` (`src/trigger-editor/src/native_x11.rs:34–42`) obtains the display, calls safe `Window::close()`, then `display.sync()`. Its comment promises another connection can query the parent immediately after Drop. Wry is dropped first; pinned Wry's destructor calls widget destruction on its **WebKit widget**, not this edge-owned outer GTK window (`wry-0.57.0/src/webkitgtk/mod.rs:100–103`; owner fields/drop `native_x11.rs:24–30,177–184`).

## Actual close dispatch

GTK **3.24.33** `gtk_window_close` returns for an unrealized window; for a realized window it schedules `send_delete_event` as a default-priority idle. That callback later builds a local GDK_DELETE event and invokes GTK event dispatch. This is asynchronous GTK work, not an immediate destroy call. [Pinned gtkwindow.c, lines 1220–1262](https://raw.githubusercontent.com/GNOME/gtk/3.24.33/gtk/gtkwindow.c).

The installed Ubuntu `libgtk-3.so.0` independently agrees: `gtk_window_close` at `0x3aa140` calls `gdk_threads_add_idle_full@plt` at `0x3aa16b`; its callback at `0x3aa0e0` creates a GDK event and calls `gtk_main_do_event` at `0x3aa126`. Package metadata is `libgtk-3-0` / `libgtk-3-dev` **3.24.33-1ubuntu2.2**. Thus the queued-dispatch conclusion is supported by both the selected tag and the installed primary binary.

GTK's delete dispatch destroys only when its current-grab condition permits and the widget event handler does not inhibit it. Close is a user-request path and may legitimately leave a window alive. This is distinct from whether the current owner actually has an inhibiting handler/grab; none was established as the failure cause. [Pinned gtkmain.c, lines 1716–1721](https://raw.githubusercontent.com/GNOME/gtk/3.24.33/gtk/gtkmain.c).

Installed `Gtk-3.0.gir:206659–206666` describes close as a request. More directly, `:206135–206142` documents GTK's internal toplevel reference and instructs callers to delete a GtkWindow with `gtk_widget_destroy`. Releasing only the Rust wrapper is therefore not equivalent to deleting the owned toplevel.

## What sync orders, and what it does not

Installed `Gdk-3.0.gir:5929–5940` says display sync flushes window-system requests and waits for them to be handled. On the selected X11 backend its implementation is `XSync(display, False)`. It orders **already-issued requests on that GDK connection**, not an idle callback that has not run. [Pinned gdkdisplay-x11.c, lines 1836–1840](https://raw.githubusercontent.com/GNOME/gtk/3.24.33/gdk/x11/gdkdisplay-x11.c).

Consequently `close(); sync()` cannot support the current immediate-query promise: the barrier can finish before later GTK disposal issues native destruction. A checked QueryTree reply on the independent x11rb connection orders that connection's requests only; it is not a replacement for a barrier after destruction on GDK's connection.

The later-service hypothesis also needs precision. `gtk_events_pending` uses `g_main_context_pending`; `gtk_main_iteration_do(false)` dispatches a nonblocking context iteration. The X11 event source can call `XPending` while preparing/checking events. Therefore zero subsequent service counts do not prove that all GDK output is still buffered, nor do they acknowledge server completion. An already-flushed request can still lack cross-connection completion ordering. [Pinned gdkeventsource.c, lines 246–290](https://raw.githubusercontent.com/GNOME/gtk/3.24.33/gdk/x11/gdkeventsource.c).

GDK's owned-window destruction ultimately issues `XDestroyWindow` except in recursion/foreign-destroy cases; source shows no completion round trip at that call. [Pinned gdkwindow-x11.c, lines 1264–1278](https://raw.githubusercontent.com/GNOME/gtk/3.24.33/gdk/x11/gdkwindow-x11.c).

This supports three **different possibilities**, not a retrospective verdict:

| New observation | Supported interpretation |
|---|---|
| Owned GTK destroy/unrealize not observed; its XID remains | GTK delete work not yet dispatched, inhibited, or some real ownership retention; inspect exact signal/order facts. |
| Owned GTK disposal observed; XID remains before a post-disposal GDK sync, disappears after it | Supports GDK/server ordering as the distinction. It does not alone distinguish not-yet-flushed from flushed-but-not-yet-processed. |
| Same owned XID remains after confirmed disposal and a post-disposal GDK sync | More than a missing wait: investigate wrong ownership/native target or actual retained native window. Do not enlarge a wait to hide it. |
| Owned XID gone but weak GObject notification delayed | Native release and object finalization differ; identify retained references separately. |

The original log supplies none of these object-specific observations. Its cause remains unresolved.

## Correct ownership operation versus a test-side wait

Safe close plus later servicing can be suitable for a **close request** whose lifecycle remains owned until completion. It is insufficient for the existing RAII Drop promise that the host may immediately reuse/destroy its parent. More service calls or a test-side post-service sync could diagnose the failure, but would not make Drop itself uphold that promise. Pumping GTK inside Drop would introduce reentrant host-loop work and is not a justified substitute here.

For deterministic release of this exclusively owned toplevel, explicit widget destruction, then a sync on the **previously captured display**, is the smallest semantically appropriate native operation. It bypasses delete-event permission handling. GTK widget destruction runs disposal; disposal unrealizes the widget and emits destruction. Additional references can leave an inert GObject, so native destruction must not be conflated with immediate finalization. [Pinned gtkwidget.c, lines 4550–4555 and 11469–11504](https://raw.githubusercontent.com/GNOME/gtk/3.24.33/gtk/gtkwidget.c).

This is a source-informed ownership recommendation, **not a patch or proof that it fixes the observed failure**. Rust GTK 0.18.2 deliberately exposes `WidgetExtManual::destroy` as unsafe (`src/widget.rs:227–236`): callers must not query widget state afterward. A valid narrow edge boundary would require UI-thread use, sole disposal authority, no subsequent widget-state calls or retained live-use aliases, released IPC/browser activity first, and only the captured display used for the completion barrier. The wrapper reference may then be dropped without querying the inert widget. The current editor crate has `#![forbid(unsafe_code)]`; a direct destroy call cannot be silently inserted under that constraint. Any narrowly justified edge change requires the root's explicit implementation/design handling. No such change was made, and no unsafe code was executed.

Destroy the **edge-owned container only**, never the external host parent, shared GDK display or X server. This owner used `build_gtk`, not Wry's `new_x11` container ownership; its outer XID remains the edge's responsibility. Recursive native destruction affects that owned window's descendants, not its parent. Child/browser-first teardown must be retained. The editor stores no instrument/runtime owner; browser-container disposal must not retire accepted instrument state or stop audio. Actual signed audio and session continuity still need the later connected/dynamic proof; a GTK source inspection cannot establish them.

## Host immediately destroys its own parent

After plugin GUI destruction returns, a host may destroy its parent without supplying more GTK ticks. With queued close still pending, the server can destroy the child through the parent's native destruction while GTK still tracks it; later delivery/disposal then meets an externally destroyed window. That is a potential ordering hazard, not an observed crash or proven failure mechanism here. Letting host-parent destruction clean up the child would also mask an owner leak.

The immediate-release criterion must therefore inspect the captured owned XID **before** destroying the test-owned host parent. A separate scenario can then destroy that parent immediately after Drop, service bounded host ticks and assert no late callbacks, GTK/X11 failures or new root child. It must use a test-owned host parent, never another application's window. A test-side wait before parent destruction would miss the stated host behavior.

## Bounded observations that preserve the oracle

Before a future authorized run, retain identities for **every** owned container in acquisition tests, including the successful first view and failed builder's already-realized container. Record serial/phase, XID, original parent, realization time and weak-object/closure probes. Install observation-only destroy/unrealize signals and GObject weak-finalization notification without capturing a strong reference to the widget. Cache XIDs/display beforehand; never query a destroyed widget's state. GTK bindings provide `connect_destroy`, `connect_unrealize` and GLib `add_weak_ref_notify_local`; aggregate toplevel counts remain supplementary.

Use explicit phase records: immediately before Drop; immediately after Drop with **no service tick**; after each bounded externally driven tick; and after a separate diagnostic GDK sync following observed disposal. At each phase use checked independent X11 requests on the captured IDs and host parent, recording exact surviving children/parents, response/error and elapsed time. QueryTree/GetWindowAttributes must use x11rb's checked replies, not unchecked invalid-XID Xlib calls. Do not recreate windows during the diagnostic interval, to avoid identity-reuse ambiguity.

Require the same no-added-child/toplevel and expired view-callback assertions. If cleanup is not complete at the immediate promised boundary, retain that failure even if later phases succeed. A bounded observation deadline distinguishes delayed completion from persistent retention for diagnosis; it must not redefine successful immediate disposal. Keep original logs and each diagnostic source/hash; unchanged successful repeats do not erase the failure.

Unresolved: the identity of the original remaining child, whether it was already logically disposed, exact GDK/server ordering in that run, any relevant GTK event inhibition/reference retention, and the result of a correctly bounded owned-container destruction implementation. All remain execution questions; the goal ledger should retain the failure open.
