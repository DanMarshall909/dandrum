# Slint library verification

The fixtures compile and render the actual reusable `.slint` components. Each
fixture exposes a **Run tests** button and an accessible result label. The
Python runner clicks that button through Slint's MCP server, asserts its result,
and runs independent mouse and keyboard checks in a fresh window. The control
checks exercise focus, declared ranges, snapping, precision, dragging, resetting,
integer input, read-only controls, and latching behavior. Native context menus,
delayed tooltips, audition lifecycles, editable zones, source inspectors, and the
composed catalog are exercised through their actual input paths as well. The
readout checks measure the actual caption, value, and unit rectangles. Layer
parameter checks also verify that later host model updates replace locally
edited values without echoing input callbacks.

Use the official Slint **1.18.1** viewer release with MCP support. The viewer
version is recorded with each run. No additional Python packages are required.

## Windows / an existing desktop session

```powershell
$env:DANDRUM_SLINT_VIEWER = "C:\Tools\Slint\slint-viewer.exe"
python scripts/check-slint-library.py
```

Leave the test windows unobstructed while the run is active. The runner launches
and closes its own viewer processes. An occupied MCP port is rejected before
launching anything, so the runner does not attach to another application.

## Linux CI

```sh
xvfb-run -a python3 scripts/check-slint-library.py
```

Alternatively set `DANDRUM_XVFB` to an Xvfb executable and let the runner manage
the virtual display. It creates a temporary authorization cookie and closes the
server when the test ends. Environments without Unix-domain socket support can
set `DANDRUM_XVFB_TCP=1` to use an authenticated TCP display. These options are
only needed in containers without a display; all executable paths remain local
environment settings.

`DANDRUM_SLINT_VIEWER` or `--viewer` selects the viewer. `SLINT_MCP_PORT` can select
a fixed free port; otherwise the runner chooses one. `SLINT_BACKEND` can override
the default `winit-software` renderer.

## Focused checks and evidence

```powershell
python scripts/check-slint-library.py --fixture Controls
python scripts/check-slint-library.py --fixture Displays
python scripts/check-slint-library.py --fixture Layout
python scripts/check-slint-library.py --fixture Menus
python scripts/check-slint-library.py --fixture Waveform
python scripts/check-slint-library.py --fixture DisplayEdges
python scripts/check-slint-library.py --fixture DisplayBindings
python scripts/check-slint-library.py --fixture LayerDetails
python scripts/check-slint-library.py --fixture Readouts
python scripts/check-slint-library.py --catalog-only
python scripts/check-slint-library.py --check-only
```

The default command also compiles and exercises the catalog at **1200×800** and
**820×560**, checking each window's actual size. It navigates every
page, changes the state picker, edits a supplied control value, scrolls actual
content, checks that page changes reset the scroll offset, and collapses panels.
It also checks complete knob and slider popup bounds inside the content clip,
including opening the rightmost slider near the bottom of the compact viewport,
and verifies that each repositioned popup's arrow still points to its control.
A successful `--check-only` run proves compilation; it does not prove interaction behavior. Live runs fail when
a fixture is missing, a contract reports failure, an independent input assertion
fails, or a required MCP capability is unavailable.

`build/slint-library-evidence/result.json` records the viewer version, runtime
backend, platform, mode, compiled entrypoints, contract results and independent
interaction results, including the requested catalog dimensions.
The same directory receives screenshots from the live windows and viewer logs.
The compact catalog run has its own `catalog-compact` subdirectory.
Failed assertions also capture the current window when its MCP server remains available.
Use `--evidence PATH` to choose another output directory.

## Fixture contract

Export a window with a stable `run-tests-button` ID and a `report` Text ID (or
register another ID in `REPORT_IDS`). The initial report must be `Not run` or
`READY: ...`; clicking the button must execute assertions
and replace it with `PASS: ...` or failure details. Add the window's exported name
to `ROOT_TYPES` in the runner. Independent physical-input assertions are registered
in `live_checks.py`; they must observe actual element state or rendered reports.
Contract tests may allow UI ticks for deferred bindings before reporting success.
Physical checks use a fresh window so the contract's mutations cannot supply their result.

The display corner fixture includes a transparent one-pixel rectangle at a known
corner coordinate. The MCP drag API starts at an element's center; this rectangle
provides that start position and has no input handlers. The real zone control
receives the complete press, interpolated movement, and release gesture.

## Native C++ tests

Configure the library build with the Slint C++ SDK and run its CTest suite as
documented in `ui/slint/README.md`. `slint-value-codec` checks actual value parsing
and formatting. An SDK with `TESTING` support also builds `slint-codec-binding`,
which instantiates the generated Slint controls, binds the real C++ global, and
commits unit-bearing and Unicode-minus text through the native accessibility path.
The viewer input checks complement these generated-code tests.

These checks verify the component library and its native Slint input path. Audio,
host automation transports, plugin lifecycle, and JUCE forwarding have their own
integration tests.
