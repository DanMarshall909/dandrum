# Global GTK aliases — post-publication safety clarification

20 October 2026. Separate read-only clarification, not a modification or metadata addendum to the published completion verdict. Examined only committed source at `9f9252cfee895f30e659e2f87a42bc833eef0e5a`, pinned installed Rust dependency sources, GTK 3.24.33 primary source, and the Rust safety model. No live author source was used to decide the published behavior. No build/test/native probe, repository/index/cache mutation, or delegated review occurred. Only this file was written.

**Conclusion:** GTK's global alias path is real and defeats any literal claim that private fields make the container undiscoverable or prevent all retained aliases. It does not, by itself, establish a new memory-safety defect in the synchronous destructor. Retaining a referenced but disposed widget is an existing GTK lifecycle state, also reachable through safe GTK close/event handling and Wry's safe native-handle API followed by Drop. The missing precision is in the safety argument: local non-escape is demonstrated; universal exclusion of external post-disposal use is not. That distinction must not be disguised as a compiler-enforced invariant or resolved merely by calling safe external code “misuse.”

## What the pinned APIs actually allow

| Fact | Primary evidence | Consequence |
|---|---|---|
| `Window::list_toplevels()` is safe, checks the GTK thread, and returns `Vec<Widget>`. | GTK 0.18.2 `src/auto/window.rs:58–61`. GLib 0.18.5 `translate.rs:2344–2365` converts each list entry via `from_glib_none`; `object.rs:468–478` takes a reference. | Same-thread safe Rust can obtain an owned wrapper for the Popup without NativeEditor exposing a field. It need not even clone again to retain it. |
| Object wrapper cloning refs; dropping unrefs. | GLib `object.rs:378–395`. | An external reference can outlive NativeEditor. Private Rust fields prevent direct access through this API, not discovery through GTK's registry. |
| Widget destruction deliberately supports surviving references. | GTK 3.24.33 `gtk_widget_destroy` documentation and body, lines 4528–4555. | Native disposal does not necessarily finalize the object. References may be released afterward, but widget-state queries are outside the stated contract. [GTK widget source](https://raw.githubusercontent.com/GNOME/gtk/3.24.33/gtk/gtkwidget.c) |
| The Rust binding marks destroy unsafe and repeats the no-subsequent-state-query condition. | GTK 0.18.2 `src/widget.rs:227–236`. | The direct caller needs a lifecycle argument, not merely proof that its own wrapper is private. The docs do not require reference count one. |
| Generic GLib disposal is also unsafe in this binding. | GLib `object.rs:2163–2174` warns that some C implementations have memory-safety defects after disposal despite the intended inert-object model. | A surviving strong reference prevents ordinary refcount finalization; it does not prove every subsequent method safe. Generic run_dispose is not a safer alternative. |

For example, an external same-thread GTK component can enumerate before owner Drop, keep the returned Widget, and later invoke one of its safe getters. This sequence needs no access to NativeEditor's private fields and no unsafe Rust in that external component. It refutes absolute non-escape. It is not an executed reproduction, and choosing an arbitrary getter does not establish undefined behavior. The actual C implementation matters: GTK's visible getter, for example, simply reads its retained private flag. The broad lifecycle prohibition is not evidence that every query is a dangling dereference. [GTK widget source, lines 8637–8642](https://raw.githubusercontent.com/GNOME/gtk/3.24.33/gtk/gtkwidget.c)

## The same alias issue already exists in the pinned lifecycle

GTK 0.18.2 exposes `WindowExt::close(&self)` safely (`auto/window.rs:639–642`). In GTK 3.24.33 a realized window queues `send_delete_event`; that function dispatches GDK_DELETE. Event handling calls `gtk_widget_destroy` when its checks permit, without first proving nobody retained another reference. The request can be inhibited, and the destruction is later than the call, but when destruction occurs the same widget-disposal contract applies. GTK also exposes destroy-with-transient-parent through safe setters. Its toplevel registry documents enumeration of existing windows; window destruction removes the entry, which prevents later enumeration but cannot revoke an already returned wrapper. [GTK window source, lines 1220–1262, 2915–2935, 3641–3674 and 5573–5601](https://raw.githubusercontent.com/GNOME/gtk/3.24.33/gtk/gtkwindow.c)

The ordinary delete-event and external native-destroy event paths also invoke widget destruction. The latter conditionally handles GTK toplevels, rather than universally treating every destroyed X child as a GTK parent lifecycle. Therefore an external X11 parent's destruction must not be conflated with GtkWindow's transient-parent feature. [GTK event dispatch, lines 1716–1733](https://raw.githubusercontent.com/GNOME/gtk/3.24.33/gtk/gtkmain.c)

Wry 0.57.0 is an even more direct comparison. Public safe `WebViewExtUnix::webview(&self)` returns a cloned `webkit2gtk::WebView` (`src/lib.rs:2426–2445`). `InnerWebView::drop` nevertheless calls its WebView's unsafe destroy (`src/webkitgtk/mod.rs:100–103`). A caller can retain that returned handle across Wry Drop. Wry therefore does not use “no strong aliases exist” as its destruction mechanism. NativeEditor already relied on this Wry behavior before the outer-container change.

These facts establish that retained handles and lifecycle disposal coexist in this stack. They do **not** prove Wry/gtk-rs sound against arbitrary post-disposal safe method calls, nor do they waive a direct unsafe caller's documented obligations. “The dependency does it too” is comparative evidence about the existing abstraction, not a soundness proof.

## What the committed owner proves, and what it assumes

The committed implementation proves by inspection that its public API returns no native wrapper; its own normal cleanup disconnects IPC, drops the Wry browser first, captures the GDK display while its container is live, destroys the container, then uses only that display. Acquisition-local wrappers only unref on unwind. Its tests' observation wrappers are deliberately short-lived. These remain useful and correctly scoped obligations.

It does not enforce that other same-process GTK code never enumerates this container, installs callbacks, retains descendants, reparents it, or closes it early. A retained global alias does not itself take ownership of the editor's intended disposal schedule or make native destruction invalid. The additional hazardous action is using disposed widget state, or externally changing lifecycle before the owner's expected disposal. Early external close could also invalidate the assumption that the initial display lookup is performed on a still-live widget; that possibility is not covered by the existing tests. This is an inference from the permitted API composition, not an observed fault or demonstrated unsafe memory access.

The practical integration discipline is that GTK components respect the destruction lifecycle of windows they observe and release or stop using aliases at destruction. That discipline is already required by this native toolkit stack. However, Rust permits those external operations through safe APIs. For a **universal Rust soundness claim**, a written convention alone is insufficient: safe client behavior must not cause undefined behavior. An actual UB path would require an implementation/dependency correction even if the client behaved unexpectedly. [Rustonomicon: safe/unsafe interaction](https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html)

Thus there are three distinct statements:

1. **Established:** this owner's controlled code does not query its widget after its own disposal; synchronous native release is independently tested.
2. **Disproved as an absolute:** no handle can escape or exist for later external live use because the owner is private.
3. **Not established either way here:** a particular all-safe global-alias sequence causes UB in this pinned concrete GtkWindow/Wry combination. No crash, UAF, corruption, or harmlessness for every method was proved, and no unsafe probe was run.

## Concrete correction and proportional follow-up

The supported correction is to narrow `OwnedWindow::drop`'s safety explanation and future review claims. Describe the actual local guarantees and explicitly acknowledge GTK-global discoverability. “Exclusive disposal authority” should mean the owner is responsible for disposal under the accepted editor lifecycle, not that GTK technically prevents another component obtaining or acting on a reference. Do not claim a comment or privacy establishes global uniqueness. This clarification does not call for rewriting the already published evidence bytes.

A precise future comment could say, in substance: the editor's controlled cleanup stays on the GTK thread; it releases IPC/browser activity first; it returns no native handles through its public API; its retained wrappers are only unreferenced after disposal; the captured display alone is used afterward. GTK can enumerate toplevels, so that is an integration lifecycle discipline, not global alias isolation. This wording corrects the proof's scope; it does not claim to discharge a universal safe-client soundness question.

The smallest concrete comment replacement for the current safety paragraph is:

```rust
// SAFETY: disposal runs on the GTK UI thread after IPC and the browser
// are released. This owner exports no native handles, but GTK can enumerate
// the container globally; retained references do not extend its live-widget
// lifetime and must not query it after destruction. This follows GTK's
// destruction lifecycle, not a guarantee that no aliases exist. Our local
// wrappers only unref afterward; only the display captured above is used
// after this call.
```

The retained references above are owned GObject wrapper references; they are not Rust borrows of NativeEditor. This paragraph intentionally names the toolkit lifecycle dependency instead of claiming enforcement over external safe GTK code.

No functional replacement is justified by the alias observation alone. Going back to close does not remove aliases or the inert-state condition and loses the immediate-XID contract. Waiting for a unique refcount would fail disposal while legitimate observers hold references. Raw X destruction would bypass GTK cleanup. Adding an unsafe public constructor, changing backend, or adding an arbitrary process isolation requirement is not supported by an identified concrete fault here.

If further work is selected, its useful question is narrow: identify a particular supported safe GTK operation on an enumerated/retained disposed GtkWindow (or early external disposal followed by owner cleanup) that reaches invalid memory or violates an owner-controlled postcondition. First examine the pinned implementation of that operation; do not intentionally invoke a documented unsafe precondition violation just to seek a crash. A harmless ref-retention/unref-only characterization can distinguish disposal from finalization but cannot certify arbitrary post-disposal methods. Any demonstrated source-specific invalid query or memory-safety path should then become an explicit correction with a non-UB regression oracle.

There is no newly established serious source bug warranting emergency source edits or a publication rollback from this clarification. There is a concrete overstatement in the alias justification and an explicitly bounded unresolved generic soundness question. The already proved synchronous XID/callback release outcome is not invalidated by the existence of a permissible retained inert reference.

## Frozen identities and resource release

Published owner SHA256: `d5a716081621410e4adbf6578cf5c5dcd4a1487691b6497a196c3803da9a39c1`.
Pinned Cargo.lock versions at the committed revision: GTK 0.18.2, GLib 0.18.5, Wry 0.57.0.

| Inspected installed source | SHA256 |
|---|---|
| gtk-0.18.2/src/widget.rs | ba3882c03325c01031a6cb46c52c191d799ad9b646a15d804792ef68e8f78c03 |
| gtk-0.18.2/src/auto/window.rs | 7feab490f4d59f06ab9d26d21d665ec45cb91c4bf7a654001d40d96965a2049d |
| glib-0.18.5/src/object.rs | 61304201fc2dd2ec7d0e8db7343bc62db6e9c81be07b2b16ad3f530a8972f017 |
| glib-0.18.5/src/translate.rs | f7b8c4b8a778e02d9ab18a1e018c27504e79e77b78353805bce156c694a0e85f |
| wry-0.57.0/src/lib.rs | f5c9e28fbb1b6a4ddfed7ddface796a9e91689e8663457ed69b651ad00149ba7 |
| wry-0.57.0/src/webkitgtk/mod.rs | 20a4064b352747b46d7e15ae6962922dfa85017660e109e87e255219c6ee2017 |

Tagged GTK C sources were read through the browser from the exact 3.24.33 URLs linked above; no local downloaded source/cache was created, and no whole-file hash is claimed for those web reads. The Rustonomicon supplies the language safety principle, not a version-specific GTK claim. No resource ownership was acquired; bounded reads have completed, and the current implementation author retains all source/build ownership.
