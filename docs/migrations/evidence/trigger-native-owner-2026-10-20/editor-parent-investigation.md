# X11 parent and Wry creation failures — 20 October 2026

Read-only inspection of cached Wry 0.57.0, gtk/gdk/gdkx11 0.18.2, installed GTK3 metadata/ELF and the existing Wry probe. No source/cache changes, downloads, builds, Cargo commands or native failure launches. Core `src/trigger-host` must remain free of GUI dependencies/types.

Registry prefix below: `/home/dan/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`. File hashes are in `editor-parent-investigation-sources.json`.

**Correction, 20 October 2026:** the original shared-context duplicate recommendation was incorrect. The observed error is `DuplicateCustomProtocol` recorded by the public builder before WebKit allocation, not the backend's `ContextDuplicateCustomProtocol`. The exact corrected trace, observed diagnostic, independent edge rollback case and superseded claim are retained below. This correction used source/log inspection only; no build or probe was rerun. The parent-validation discussion records the original investigation's constraint context; it does not establish a new blanket prohibition on GTK internals changing Xlib handlers.

## Parent validation recommendation

**Do not validate arbitrary foreign XIDs using unchecked Xlib calls or GDK error traps under the no-process-global-handler-change constraint.**

- `gdkx11-0.18.2/src/auto/x11_display.rs:23–38` exposes `error_trap_push/pop`; installed `/usr/share/gir-1.0/GdkX11-3.0.gir:647–714` documents synchronous error-result pop. However read-only `objdump -d --disassemble=gdk_x11_display_error_trap_push /usr/lib/x86_64-linux-gnu/libgdk-3.so.0` shows a direct `XSetErrorHandler@plt` call at `0x6758c`. The apparently display-scoped trap still replaces the process-global Xlib handler.
- GDK’s `foreign_new_for_display` documents nullable failure for destroyed windows (GIR:1584–1604), but `gdkx11-0.18.2/src/auto/x11_window.rs:21–29` returns a non-Option Rust object. Its use as a blind validity check can panic on NULL. Installed GDK disassembly also shows its unknown-window path calling the error-trap push/pop around XGetWindowAttributes/XQueryTree. It therefore does not solve the strict handler constraint.
- Wry itself is unsafe to call with an arbitrary invalid XID: `wry-0.57.0/src/webkitgtk/mod.rs:1268–1273` ignores the result of `XGetWindowAttributes(parent)` and dereferences `attrs.screen`. Merely trapping BadWindow would not make that unchecked null-screen path safe.

Use a **separate protocol connection** on the same X server for validation. Cached `x11rb 0.13.2` provides the pure-Rust `RustConnection::connect` (`src/rust_connection/mod.rs:112`), `get_window_attributes` (`src/protocol/xproto.rs:158`), and `get_geometry` (`:542`). `Cookie::reply` returns `ReplyError` via `wait_for_reply_or_error` (`src/cookie.rs:248–262`), rather than Xlib’s error handler. Do not select its optional Xlib/XCB-FFI connection backend or reuse GDK’s connection. Match GDK’s actual display name; reject zero, non-u32 IDs, non-window/pixmap IDs, InputOnly parents, wrong-screen/depth/root as appropriate for the selected child, and connection/protocol errors before Wry sees a parent. Main-thread request/reply waits are outside audio and do not start a GTK loop. System libxcb 1.14/headers are also present, but x11rb avoids introducing handwritten FFI for checked requests.

A successful preflight does not prevent the host destroying its parent between validation and Wry’s later unchecked query. For the strongest create-failure boundary, use an **edge-owned GTK window/container + `build_gtk`**, realize it while hidden, and use the separate checked protocol connection to reparent the known owned XID to the host’s parent (`reparent_window`, xproto.rs:287; `VoidCookie::check`, cookie.rs:101). This avoids giving Wry’s `new_x11` an arbitrary foreign XID and provides an explicit owned cleanup target on every Err. Synchronize the known owned GDK window’s creation before the second connection uses its XID. This is a justified alternative requiring its own parenting/size/lifecycle proof, not an already proven implementation. GTK/GDK internals themselves can still use global Xlib handling; the claim here is that **our validation/checked reparent introduces no XErrorHandler change**, not that all GTK internals are process-global-state-free.

## Exact Wry error/ownership trace

`WebViewBuilder::build_as_child` first returns any builder error at `wry/src/lib.rs:1572`, then calls the backend. `new_x11` returns errors before allocation for raw-handle acquisition/type, opening Xlib, or missing GDK display (`webkitgtk/mod.rs:150–159`); downcasting a non-X11 default GDK display uses `unwrap`, so precheck API/display/thread explicitly. GTK initialization/thread checks can panic (`gtk-0.18.2/src/rt.rs:117–122`); wrong-thread/API rejection should happen before invoking it.

`new_x11` then allocates a child at `:165`, creates the GTK window/container at `:168`, and only installs `X11Data` at `:184` inside the successful `new_gtk(...).map` (`:172–191`). `X11Data::drop` (`:80–85`) owns native child destruction and GTK close; it does not exist on the Err path. This is a source-visible ownership gap, **not a reproduced leak**: GTK/GObject fallback cleanup has not been measured here.

If reached, `new_gtk` calls `create_webview` (`:290`); that method configures a UserContentManager (`:399–400`) and calls the WebKit builder's `build()` (`:417`). It attaches handlers/IPC and parents the view (`:309–325`), then constructs its RAII owner `w` (`:332–342`). Its source-visible post-allocation Result exits are:

- `w.init` at `:345` and `:349`: `InitScriptError` if the UserContentManager is absent (`:813–830`). The normal create path explicitly supplies that manager, so no reachable failure under the minimal configuration was demonstrated.
- Custom-scheme registration at `:369`: `register_uri_scheme` calls `register_custom_protocol` (`webkitgtk/web_context.rs:134`), which could return `ContextDuplicateCustomProtocol` (`web_context.rs:74–77`), or the security-manager lookup could return `MissingManager` (`webkitgtk/web_context.rs:140`). These are Wry-internal source paths, not reproduced failures here. The public shared-context duplicate construction described in the original report is intercepted earlier and does **not** reach this backend registration. No reachable fresh-context failure at these exits was established.

On these Err exits, `w` drops and calls `webview.destroy()` (`webkitgtk/mod.rs:100–103`); this establishes an attempted WebKit cleanup, not measured release of every native container/callback. IPC connects a manager callback capturing a WebView clone (`:723–747`), so callback lifetime requires an actual drop oracle. Do not infer it merely from `Err` or widget destruction.

## Corrected public builder → backend trace

The pinned cached Wry 0.57.0 implementation establishes this sequence:

1. `WebViewBuilder::new_with_web_context` (`wry/src/lib.rs:921–932`) stores `Some(&mut WebContext)` and initializes `error: Ok(())`.
2. A successful first build registers its custom name in that context via `webkitgtk/mod.rs:369` → `webkitgtk/web_context.rs:134` → `web_context.rs:74–79`. The context's set then contains the name.
3. On the second builder, `with_custom_protocol` checks that context's set on Linux (`lib.rs:1087–1093`), sets `self.error` to `Error::DuplicateCustomProtocol(name)` and returns immediately. It does not insert the rejected handler into `attrs.custom_protocols`. The separate same-builder duplicate check is `:1095–1099`. The async variant has equivalent early guards (`:1151–1163`).
4. `WebViewBuilderExtUnix::build_gtk` (`lib.rs:2059–2067`) executes `self.error?` **before** calling `InnerWebView::new_gtk`. Consequently no WebKit view is allocated by that rejected build. `build_as_child` (`:1571–1575`) likewise checks `self.error?` before its backend. A caller may already own a GTK container; this early Wry error still tests cleanup of that caller-owned allocation and release of the rejected closure.
5. For an accepted builder only, `new_gtk` selects its context (`webkitgtk/mod.rs:257–270`), allocates the WebKit view (`:290`, `create_webview :394–417`) and later runs the fallible initialization/registration paths described above. The two error variants have distinct messages in `error.rs:67–70`. Reaching a backend Result branch cannot be inferred from its existence or a generic `Err` assertion.

### Observed diagnostic and corrected evidence boundary

The native author's retained `/tmp/dandrum-lanes/evidence-native-editor/specific-wry-diagnostic.log` records a failed assertion expecting the backend error and the actual value:

`Some(WebView("Duplicate custom protocol 'calibration' registered on the WebViewBuilder"))`

This is direct recorded evidence of the builder variant; the source trace above explains its pre-WebKit timing. It is not evidence of a post-WebKit Wry-internal failure or a vendor leak. The corrected current source `src/trigger-editor/tests/internal/native_x11.rs:270–318` explicitly matches `wry::Error::DuplicateCustomProtocol` and checks that the owner's already allocated GTK container and the **unregistered** rejected resource closure are released, while accounting for the intentionally retained first view/context. The diagnostic log predates that corrected assertion; it must not be presented as a passing corrected calibration run.

### Separate post-WebKit edge rollback

The current native source `src/trigger-editor/src/native_x11.rs:137–173` creates an `OwnedWindow` and fixed container, successfully calls `build_gtk`, constructs `NativeEditor`, and then issues a checked X11 reparent. A reparent error exits through `?` with that owner in scope, so ordinary Rust destruction drops its WebView then owned container (`:23–37,177–184`). Wry's view destructor calls `webview.destroy()` (`webkitgtk/mod.rs:100–103`). This is an edge-owned rollback path, not a forced Wry-internal constructor error, and needs no vendor modification.

The real test `real_child_loads_local_content_and_drops_without_owning_the_host_parent`, current `tests/internal/native_x11.rs:235–268`, validates a real parent, destroys it with a checked request, and then calls `build_child` using a fresh valid custom protocol. It requires `Err(NativeError::X11(_))`, no added X11 root child, restored GTK toplevel count and expiration of a weak probe captured by the registered resource callback after bounded host-driven servicing. `/tmp/dandrum-lanes/evidence-native-editor/post-webkit-resource-native.log` records this named test passing (1 passed, 0 failed). The source establishes the post-WebKit ordering; the retained log records the run result. This correction did not rerun it or independently reconstruct the exact build's source hash. It does not expand that specific resource-callback/window oracle into proof of all IPC/navigation/native ownership paths.

### Chronology and superseded claim

The original source-only report proposed the following, now **superseded and incorrect**:

> Use a real, deterministic post-allocation Wry error by creating one successful scheme registration in an explicitly shared test WebContext, then creating a second view with the same scheme. The second reaches `ContextDuplicateCustomProtocol` after view allocation.

Chronology: (1) that proposal inferred reachability from backend registration without tracing the public builder; (2) the native diagnostic observed the earlier `DuplicateCustomProtocol`; (3) independent pinned-source reconstruction confirms the guard; (4) this report corrects the claim and separates the observed builder failure from the checked-reparent rollback after successful WebKit creation. Wry-internal postallocation Result paths remain source-visible but unreproduced by this duplicate test. The original source-hash manifest is retained as `editor-parent-investigation-sources.original.json`; the current manifest adds the diagnostic, inspected edge/test snapshots and log without pretending those current source snapshots identify a historical build.

Narrow lesson candidate for this investigation/review record only: before proposing a deterministic failure oracle for a library backend, trace the exact public entry point and its accumulated preflight errors, then assert the concrete error variant and bind the claimed allocation boundary to source plus the observed diagnostic. Backend branch existence is insufficient evidence of public reachability. No standing rule or memory was changed.

The earlier `/tmp/dandrum-lanes/wry-probe/src/main.rs` proves only successful child creation/two-cycle destruction. Its XQueryTree/XGetWindowAttributes helpers use valid owned windows and are unsuitable as invalid-parent error oracles without replacement; it is distinct from the later edge-owned tests above.
