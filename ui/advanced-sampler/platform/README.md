# Desktop file drops

Slint 1.18.1's winit adapter omits `WindowEvent::DroppedFile`, although winit
completes the native X11 XDND exchange. `X11FileDrop` provides a Linux/X11-only
shared UI transport until that backend forwards the event. It does not change
Slint, the audio path, or other platform backends.

The host retains a `WindowFileDrop` owner beside its session binding. It creates
one adapter with the exact window title and polls it from a 20 ms Slint UI timer.
Receipt feeds the same application callback used by Slint's `DropArea`:

```cpp
auto desktopDrop = dandrum::sampler::bind_native_file_drop(window);
window->run(); // Keep desktopDrop alive until the event loop returns.
```

The owner stops its timer before destroying the adapter, and callbacks keep only
a weak window handle. CMake enables the adapter when XCB development files are
available; without them the owner is a no-op. This bridge supports X11 only.

The adapter uses a dedicated XCB connection and a standard `XdndProxy`. It only
attaches to one mapped window matching both the exact UTF-8 title and this
process's `_NET_WM_PID`. Existing proxy properties are left alone, including
malformed properties. Teardown removes the target property only when it still
points at our proxy. Each timer tick bounds event/reply processing and uses
nonblocking reply polling; atom setup and final teardown use server round trips.

URI selections are bounded to 64 KiB, 256 paths, and 4096 bytes per decoded path.
Only absolute `file:///...` and `file://localhost/...` URIs are accepted. Comments,
spaces, UTF-8 bytes, ordered multiple paths, and percent escapes are handled;
invalid or remote entries reject the entire selection. The host owns file
existence, file type, import policy, and any dialog choices. Leave, stale async
replies, repeated drops, unsupported MIME data, and timed-out selections do not
produce another receipt. Incremental (`INCR`) transfers are rejected by the
same bounded selection check.
Selection replies must match the current conversion timestamp, preventing a
late reply from a cancelled drag from being accepted for a subsequent drop.

`AdvancedSamplerNativeFileDropTest` uses a separate real X11 selection owner and
XDND producer. The pre-adapter run completes the protocol with zero application
receipts; the adapter run checks exact callback path vectors. Ownership and URI
decoding have separate native/pure tests. Native tests run through
`scripts/run-native-slint-test.py --headless` on a private authenticated Xvfb
server.

Primary references:

- [Slint 1.18.1 winit event dispatch](https://github.com/slint-ui/slint/blob/v1.18.1/internal/backends/winit/winitwindowadapter.rs)
- [XDND protocol and proxy rules](https://wiki.freedesktop.org/www/Specifications/XDND/)

The implementation now lives in [ui/slint/platform](../../slint/platform).
Sampler headers retain compatibility aliases; application receipt policy stays
in the sampler wrapper. `Dandrum::SlintFileUris` and optional
`Dandrum::SlintX11FileDrop` expose the shared CMake targets. Named bounds, protocol
identifiers and timeouts are in `FileDropProtocol.h`; callers keep UI transport
work off the audio thread. The independent shared transport test also completes
a real drop after the receiving component expires and verifies no delivery.
