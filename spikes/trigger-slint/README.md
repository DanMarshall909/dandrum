# Trigger Slint performance preview

A silent native preview of **just Trigger's eight macros**. Tone, Body, Space,
Drive, Snap and Width start at the React Felt Kit defaults; Macro 7 and Macro 8
are unassigned and disabled. This evaluates Slint component authoring and AI
tooling before building the full sampler shell.

```sh
./demo trigger-slint
```

The launcher finds this implementation in a registered Dandrum worktree and
uses the optional Release/native-only `build/filter-ui-spikes` configuration.
The UI uses Slint 1.18.1 embedded in a JUCE GUI app, with software rendering.
It produces no audio and is not a plugin or engine integration.

## Controls

- Drag vertically; the full range spans 200px. Hold Shift for one-tenth precision,
  including while dragging.
- Wheel: 2%; Shift-wheel: 0.2%.
- Arrow keys: 5%; Shift-arrow: 0.5%. Page Up/Down: ten steps. Home/End: endpoints.
- Middle-click, Alt-click, Delete or Backspace: reset to the initial value.
- Double-click or press Enter to type a percentage. Enter commits; Escape cancels;
  leaving the field commits valid text. Invalid text preserves the value;
  numeric values are clamped to 0–100%.
- Tab moves between assigned macros. Hover/focus reveals the value.

## Authoring

The [official Slint skill](https://github.com/slint-ui/ai-plugins/tree/master/skills/slint)
is installed at `/home/dan/.codex/skills/slint`. Read its references alongside
[Slint's best practices](https://docs.slint.dev/latest/docs/slint/guide/development/best-practices/).
UI files, assets and native code are separate; theme tokens live in one global,
and controls declare slider accessibility roles, labels, ranges and actions.

The installed official `slint-viewer` 1.18.1 supports a quick preview loop:

```sh
~/.local/bin/slint-viewer --auto-reload spikes/trigger-slint/ui/App.slint
~/.local/bin/slint-viewer --check spikes/trigger-slint/ui/App.slint
~/.local/bin/slint-viewer --screenshot /tmp/trigger-slint.png spikes/trigger-slint/ui/App.slint
```

For AI inspection and interaction, launch the viewer with a free local MCP port:

```sh
SLINT_MCP_PORT=9325 SLINT_BACKEND=winit-software \
  ~/.local/bin/slint-viewer --auto-reload spikes/trigger-slint/ui/App.slint
```

The MCP endpoint is `http://127.0.0.1:9325/mcp`. The shipped viewer supports MCP
on a display, but its `headless` backend is unavailable; `--screenshot` works
without a display. The compiled JUCE embedding does not enable MCP. Both paths
load the same UI; native input forwarding is verified separately.

For a narrow screenshot, set the bound viewport too:

```sh
printf '{"viewport-width":720,"viewport-height":90}\n' > /tmp/trigger-slint-size.json
~/.local/bin/slint-viewer --screenshot /tmp/trigger-slint-720.png \
  --load-data /tmp/trigger-slint-size.json spikes/trigger-slint/ui/App.slint
```

## Native verification

```sh
PATH=/usr/bin:/bin:$PATH ~/.local/bin/cmake -S . -B build/filter-ui-spikes \
  -DDANDRUM_NATIVE_ONLY=ON -DDANDRUM_BUILD_TRIGGER_SLINT=ON \
  -DDANDRUM_BUILD_FILTER_UI_SPIKES=OFF -DCMAKE_BUILD_TYPE=Release
PATH=/usr/bin:/bin:$PATH ~/.local/bin/cmake --build build/filter-ui-spikes \
  --target dandrum-trigger-slint dandrum-trigger-slint-ui-check -j 2
ctest --test-dir build/filter-ui-spikes --output-on-failure \
  -R '^(trigger-slint-ui|demo-launcher|demo-launcher-inventory|demo-launcher-boundaries)$'
```

The native check needs a working display (`DISPLAY=:10` in this environment).
It uses the real JUCE component and Slint renderer, dispatches native input,
and saves snapshots under `build/filter-ui-spikes/trigger-slint-evidence`.

## Reference and limits

The reference is Trigger React at `43c01c7`, `MacroStrip.jsx`, `ParameterKnob.jsx`
and the supplied rotary drawing. Reused fonts retain their original OFL notices
in `assets/fonts`; [font provenance](assets/font-provenance.json) records their
source names and hashes. Each authored UI component and native host file is
under 200 lines.

The strip matches the charcoal palette, fonts, cell spacing and rotary geometry.
Native antialiasing differs from Chromium. The percentage editor fits underneath
the control; React uses a floating value dialog. The full-shell header, editor,
macro destinations, host/MIDI bindings and undo history are outside this preview.


## Experiment result — 2026-10-08

**Adopt for the next UI prototype.** The strip reproduces the reference geometry
and palette with small reusable declarative components. The pinned viewer offers
compile checks, screenshots, auto-reload and MCP interaction without rebuilding
the C++ host. Native compilation and input forwarding remain separate checks.
This result does not establish a full sampler shell or production plugin readiness.

Verified native [900px](evidence/native-900.png), [720px](evidence/native-720.png)
and [percentage editor](evidence/native-editing.png) renders. The native check
asserts initial values, arc changes at endpoints, drag, live Shift changes, wheel,
keyboard, typed and invalid values, clamping, Escape, blur commit, reset,
double-click, Tab/Shift-Tab skipping disabled slots and focus-loss cleanup.
The four owning native/launcher CTests pass with the filter experiments disabled;
the existing Slint and JIVE filter checks also pass with both experiments enabled.

Launcher verification includes real Git worktree discovery and a built CMake GUI
artifact, calibrated against vendored JUCE's `TARGET_FILE`. All 26 launcher tests
pass. Python trace reports 100% of the launcher's 115 executable lines; it does
not measure branch coverage. A focused mutation removing GUI target discovery
is rejected by the GUI launch test. Slint/native prototype coverage is interaction
and render evidence, with no code-coverage percentage claimed.

Native test clicks must respect Slint's 500ms double-click interval. Rapid
independent synthetic clicks unintentionally opened the percentage editor; the
check now separates those clicks and verifies intentional double-clicks explicitly.
React's brief keyboard/wheel arc emphasis is not timed in this preview; the arc
thickens during dragging. Native text rasterization and the inline percentage
editor are the other visible differences from the React reference.
