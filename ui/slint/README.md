# DanDrum Slint design library

A reusable native implementation of the **maintained DanDrum design guide**:
29 public guide components, a themed scrollbar, shared interaction and drawing
subcomponents, and a six-page interactive catalog. It uses Slint **1.18.1**.

The authority is [the maintained design guide](../../docs/design-system/README.md),
followed by [production tokens](../design-system/tokens.json),
[prepared display contracts](../../docs/design-system/native-display-specs.md),
and the preserved component handoff. This implements the guide vocabulary for
Trigger and other DanDrum editors. The catalog is silent; connecting a particular
plugin's engine and host parameter transport is a separate integration boundary.

## Run the catalog

Install the official Slint 1.18.1 C++ SDK for your platform, CMake 3.22 or later,
and a C++20 compiler. Set `CMAKE_PREFIX_PATH` to the SDK installation directory.
On Windows, add its `lib` directory to `PATH` so the application can load the
runtime DLL, as described in [Slint's setup guide](https://docs.slint.dev/latest/docs/cpp/cmake/).

```powershell
$env:CMAKE_PREFIX_PATH = "C:\Tools\Slint\1.18.1"
$env:PATH = "$env:CMAKE_PREFIX_PATH\lib;$env:PATH"
python scripts/demo_launcher.py slint-library
```

On Linux or macOS, with the equivalent SDK path:

```sh
export CMAKE_PREFIX_PATH=/path/to/Slint-cpp-1.18.1
./demo slint-library
```

The demo uses its own `build/slint-library` configuration. It builds only the
catalog and does not configure JUCE, Rust, Node, or the browser editor. The
equivalent direct commands, also suitable for PowerShell, are:

```sh
cmake -S . -B build/slint-library -DDANDRUM_SLINT_LIBRARY_ONLY=ON -DCMAKE_BUILD_TYPE=Release
cmake --build build/slint-library --config Release --parallel
ctest --test-dir build/slint-library -C Release --output-on-failure
```

The executable is `build/slint-library/slint-catalog/Release/dandrum-slint-catalog`
(`.exe` on Windows). `cmake -S ui/slint -B build/slint-library-standalone` also
configures this library directly. A display server is required to open a window.

For rapid UI authoring, use the Slint 1.18.1 viewer:

```sh
slint-viewer ui/slint/catalog/Catalog.slint
slint-viewer --check ui/slint/catalog/Catalog.slint
```

The viewer uses a pure Slint numeric fallback. Plain numbers work there; the
compiled C++ catalog additionally accepts units, a Unicode minus, and pan
notation through the supplied codec. Keep the SDK, compiler, and viewer at the
same version.

## Composition

Import public components from [dandrum.slint](dandrum.slint). Lower-level pieces
are individually importable from their own files; drawing, input gestures,
state, and data models are separated so a new control can reuse them. Authored
Slint files are kept to at most 200 lines. Generated tokens are identified in
their file header.

| Guide component | Slint implementation | Main reusable pieces | Catalog |
| --- | --- | --- | --- |
| Button | `controls/Button.slint` | ActionArea, ButtonFace, FocusRing | Controls |
| IconButton | `controls/IconButton.slint` | ActionArea, ButtonFace, Icon, Tooltip | Controls |
| Knob | `controls/Knob.slint` | RangeControl, RangeGesture, RotaryFace, RotaryArc, RotaryCap, RotaryModRing, ValuePopup | Controls |
| NumericField | `controls/NumericField.slint` | RangeControl, RangeGesture, ValueEditor | Controls |
| Slider | `controls/Slider.slint` | RangeControl, RangeGesture, SliderTrack, SliderThumb, TrackSegment | Controls |
| Toggle | `controls/Toggle.slint` | ActionArea, ToggleTrack | Controls |
| Switch | `controls/Switch.slint` | ActionArea, ButtonFace | Controls |
| ParamLabel | `controls/ParamLabel.slint` | ControlLabel, ValueReadout | Controls |
| ModGlyph | `modulation/ModGlyph.slint` | ModPalette, geometric slot glyphs | Controls |
| ModIndicator | `modulation/ModIndicator.slint` | ModGlyph, ModPalette | Controls |
| PadCell | `display/PadCell.slint` | PadFace, PadActivity, VelocityBar, LayerTicks, AlternateDots | Pads and key map |
| KeyMap | `display/KeyMap.slint` | PianoKeyboard, PianoKey, VelocityAxis, VelocityGrid, ZoneRectangle, ZoneHitOverlay, ScrollViewport | Pads and key map |
| LayerStack | `display/LayerStack.slint` | LayerChainRow, SourceBlock, ModuleChip, LayerSend, LayerDetails, LayerParameterGrid, LayerParameterControl, LayerSourceWaveform, ChainConnector, ScrollViewport | Layers and output |
| Meter | `display/Meter.slint` | ChannelMeter, MeterTrack, ClipAcknowledgement | Layers and output |
| OutputBusses | `display/OutputBusses.slint` | OutputBusRow, ChannelMeter, ScrollViewport | Layers and output |
| WaveformPanel | `display/WaveformPanel.slint` | WaveformTrace, RegionOverlay, FadeOverlay, LoopOverlay, SliceMarkers, SliceMarker, MarkerFlag, PlaybackCursor, StartModulationRail, Spectrogram | Waveforms |
| Panel | `layout/Panel.slint` | PanelHeader, SectionHeading, header actions | Layout and feedback |
| Rollout | `layout/Rollout.slint` | RolloutHeader, indented detail body | Layout and feedback |
| SectionHeading | `layout/SectionHeading.slint` | Heading, Icon | Layout and feedback |
| ListRow | `layout/ListRow.slint` | RowSurface, FocusRing | Layout and feedback |
| PropertyRow | `layout/PropertyRow.slint` | RowSurface, Icon, value typography | Layout and feedback |
| Tabs | `navigation/Tabs.slint` | ChoiceState, NavigationItem | Layout and feedback |
| SegmentedControl | `navigation/SegmentedControl.slint` | Tabs, ChoiceState, NavigationItem | Layout and feedback |
| ContextMenu | `navigation/ContextMenu.slint` | Native ContextMenuArea, typed MenuEntry model | Layout and feedback |
| MenuButton | `navigation/MenuButton.slint` | MenuSurface, MenuRow, MenuDepthRow, ActionArea | Layout and feedback |
| StatusMessage | `feedback/StatusMessage.slint` | FeedbackSurface, Icon, Button | Layout and feedback |
| Tooltip | `feedback/Tooltip.slint` | Bubble content; TooltipArea owns hover timing | Layout and feedback |
| EmptyState | `feedback/EmptyState.slint` | DashedBorder, Icon, Button | Layout and feedback |
| Icon | `icons.slint` | 33 original SVG assets | Foundations |
| Scrollbar | `layout/Scrollbar.slint` | ScrollThumb, FocusRing; ScrollViewport composes scrolling | All catalog pages |

[components.json](components.json) maps every public guide declaration to its
source, composition, and catalog page. The inventory test reads the preserved
TypeScript declarations independently, so a missing guide export fails the gate.

Additional shared displays are `Scope`, `Spectrum`, and `Spectrogram`. They use
the maintained waveform/spectral contracts and the shared `SpectralPalette`.
`IconNames.all` is the guide's icon-name list. `ModPalette.tint(slot)` and
`ModPalette.letter(slot)` provide the guide's four slots: indices **0–3 = A–D**.

`ControlLabel` is the small label inside editable controls. The public
`ParamLabel` composes it with `ValueReadout` for a noninteractive, stacked
`label` / `value` / `unit`. It supports `size: "sm" | "md" | "lg"`,
`align: "left" | "center" | "right"`, disabled colors, and host automation color.

## Bind a parameter

This example is intended for a `.slint` file beside `dandrum.slint`:

```slint
import { Knob, Tokens, ViewportMetrics, ValueCodec, ParsedValue } from "dandrum.slint";
export { ValueCodec, ParsedValue }

export component Editor inherits Window {
    in-out property <float> level: -3;
    callback level-changed(float);
    callback level-gesture-began();
    callback level-gesture-ended();
    preferred-width: 320px;
    preferred-height: 180px;
    background: Tokens.surface-window;
    init => { ViewportMetrics.width = root.width; ViewportMetrics.height = root.height; }
    changed width => { ViewportMetrics.width = root.width; }
    changed height => { ViewportMetrics.height = root.height; }
    Knob {
        x: 32px; y: 24px;
        label: "LEVEL"; unit: "dB";
        minimum: -60; maximum: 12; step: 0.1; default-value: 0;
        value <=> root.level;
        changed(value) => { root.level-changed(value); }
        gesture-began => { root.level-gesture-began(); }
        gesture-ended => { root.level-gesture-ended(); }
    }
}
```

`Knob`, `Slider`, and `NumericField` inherit the same `RangeControl` behavior:

- `value`, `minimum`, `maximum`, `step`, `default-value`, and `unit` use actual
  parameter units. `normalized-value` is a derived drawing coordinate.
- `changed(value)` reports user edits. `gesture-began` / `gesture-ended` bracket
  drag, keyboard, wheel, reset, and accepted typed edits. Property updates from
  the host do not echo those callbacks. `host-write(value)` also produces a
  short blue host-update indication.
- `enabled: false` disables interaction. `read-only: true` presents a prepared
  setting. `host-automated` is an indication supplied by the host, not an API for
  inspecting the DAW's automation sources.
- Drag changes the value; Shift gives finer motion. Arrow keys, Home/End, wheel,
  Enter to type, Escape to cancel, double click to reset, and Menu/Shift+F10
  parameter actions are supported. Invalid input remains visibly invalid until
  corrected or cancelled. An accepted input is clamped and snapped to its range.

Bind the codec after constructing the generated C++ window:

```cpp
#include "Editor.h"
#include "host/ValueCodec.h"

auto window = dandrum_ui::Editor::create();
dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(
    window->global<dandrum_ui::ValueCodec>());
window->on_level_changed([](float value) {
    // Forward to the host's declared public parameter on the UI thread.
});
window->run();
```

Generate the example with `slint_target_sources(... NAMESPACE dandrum_ui)` and
link `Dandrum::SlintLibrary`. The `auto` variable lets C++ infer Slint's component
handle type; the templated codec binder adapts to the generated `ParsedValue`
type without coupling the standalone parsing code to Slint headers.

## Data and host ownership

The typed models in [display/types.slint](display/types.slint) own value data,
strings and child lists. Supply opaque IDs and the current prepared generation;
callbacks return those IDs, never pointers into engine-owned storage.

| Component family | Caller supplies | Callback boundary |
| --- | --- | --- |
| Pads / keyboard | MIDI notes, selected mapping, measured activity, accepted generation | Paired note-on/note-off requests; release on cancellation, focus loss, disable or generation change |
| Key map | Inclusive key/velocity bounds, root note and selection policy | Selection and audition; zone edits only when `can-edit` is true |
| Layers | Actual sources, module chains, parameters, sends, outputs and capabilities | IDs and proposed values; unsupported add/remove/bypass/routing actions remain inert |
| Output buses | Arbitrary named buses, actual channel counts and known/unknown feed lists | Selection and clip acknowledgement including generation and ticket |
| Meters | dBFS peak/RMS/hold, clip latch, ticket, explicit measurement state | Host acknowledges the specific clip; clicking does not clear a newer latch locally |
| Waveforms | Signed min/max buckets, source-relative regions, fades, loop, slices, cursor | Display-mode and slice selection, retry request |
| Spectral views | Positive-frequency coordinates, dBFS values, declared frequency limits | Render-only analysis data; default floor −120 dBFS; DC omitted by the producer |

MIDI 60 is C4. Velocity ranges are inclusive 1–127; the maintained snare example
uses 1–63 / 64–127. A waveform's markers use normalized **source coordinates**;
fade amounts and `start-offset` are fractions of the selected region. The
optional `start-modulation-depth` is a signed fraction of that same region;
`start-modulation-enabled` gates the supplied rail, which is clipped to the
region and hidden in sliced or unavailable views. Reversing a source does not
silently reverse the coordinate system.

`SpectrumPoint.position` is a normalized **log-frequency coordinate**, not a
frequency in Hz. `SpectralCell.x/width` are normalized time coordinates and
`y/height` are normalized log-frequency coordinates. The producer supplies
the first positive bin and Nyquist labels for its actual source sample rate.

`MeasurementState.valid` can represent real silence. `waiting`, `unavailable`,
and `gap` remain distinct states; absent data must not be converted to zero.
Layer and map `rebuilding` inputs suppress structural edits during the caller's
automatic mute/rebuild/resume transaction. The library does no loading, DSP,
analysis, host enumeration, or work in an audio callback.

The catalog's illustrative models live only in `catalog/`. Its waveform/spectral
fixtures were calculated offline from the repository's `advanced-break.wav`.
The reusable display components contain no generated noise or invented signal.

### Layer details and parameter adapters

`LayerStack` composes chain rows with an expandable `LayerDetails` inspector.
That inspector delegates its supplied snapshot to `LayerSourceWaveform` and
its controls to `LayerParameterGrid` / `LayerParameterControl`. Opening the
same source again toggles its inspector; Escape closes the inspector and
returns focus to the source. These pieces can also be embedded separately.

`LayerRow.waveform` is a `LayerWaveform` snapshot. Set `supplied` only when the
caller has a source snapshot; its `state`, extrema, region, fades, loop, slices,
cursor, and optional spectral cells use the same contracts as `WaveformPanel`.
An absent snapshot stays absent. Display-mode changes and retries return to
the caller instead of starting loading or analysis inside the component.

`LayerParameter` describes either a numeric parameter or a supplied choice:
numeric controls use `value`, bounds, step, default, unit, and bipolar state;
an `options` list with `option-index` renders the available choices instead.
Each option retains its opaque ID and enabled state. The enclosing prepared
state and each parameter's `available` capability gate interaction. Callbacks
carry layer/module/parameter IDs and proposed values or option IDs, with
balanced gesture callbacks for continuous numeric edits. The host accepts a
proposal by publishing its updated model; later model updates refresh the
display silently. No option list, parameter range, or audio snapshot is inferred
from a source name.

Inline chain level and send controls expose `level-gesture-began/ended(layer-id)`
and `send-gesture-began/ended(layer-id, send-id)` alongside their value callbacks.
Persistent `LayerChainRow` and `LayerSend` instances also refresh silently from
host snapshots after user edits. Their horizontal scrolling, the expanded layer
list, key-map zoom viewport, and output-bus list all reuse `ScrollViewport`, so
switching between thumb, keyboard, and content-wheel scrolling shares one state.

## Layout, menus, and overlays

`Panel` and `Rollout` accept children and expose `collapsed`, `toggled(bool)` and
`action(id)`. Header actions are separate from the collapse target. Lists,
tabs and menus use caller-supplied ID models. Tabs have a roving keyboard focus
and skip disabled choices.

`MenuButton` composes the token-styled 240 px `MenuSurface`, including optional
modulation assignment/depth rows. `ContextMenu` delegates native context menu
behavior to Slint; its popup uses the platform/Slint menu style. Native menu
items support labels, checks and enabled state. Use `MenuSurface` when exact
guide colors, icon/shortcut columns, separators or destructive-action styling
are required. Context-menu input passes ordinary clicks to wrapped controls.

`Tooltip` is reusable content; wrap an interactive child in `TooltipArea` for
the 500 ms hover delay and bind `target-focused` to the child's focus state.
The `target-hovered` input supports an explicit hover source when needed.
Knob value popups and tooltips are nonmodal overlays. Bind `ViewportMetrics` at
window initialization and resize, as above, to keep them inside the window.
When controls live in a scrolling viewport, also supply its visible bounds in
absolute window coordinates through `ViewportMetrics.overlay-clip`, an
`OverlayBounds { x, y, width, height }` value. Zero-size clip bounds fall back to
the full window. The shared `clamp-x` and `place-y` helpers keep value popups and
tooltips within those bounds, choosing above or below the control as space
allows. The catalog demonstrates updating this rectangle when its viewport
geometry changes. `ValuePopup.anchor-x` is the control center relative to the
placed popup; it keeps the arrow attached after horizontal clamping and defaults
to the popup's center when the drawing component is used directly.
Ancestors still apply Slint's normal clipping and painting
order; raise the active overlay's containing row and gallery above siblings,
as the catalog does, or place it in the consumer's unclipped overlay layer.

`ScrollViewport` composes a `Flickable` and themed scrollbars. Supply content
dimensions and bind child width to its `viewport-width` for responsive content.
Its offsets are positive distances from the content origin. The scrollbar
thumb, keyboard, page click, and wheel share the same clamped range.
If wrapped child text determines `content-height`, reserve the 8 px vertical
gutter when binding child width (as the catalog does) to avoid a cyclic
dependency between text height, scrollbar visibility and available width.

## Tokens, fonts, and packaging

All 146 values in `tokens.slint` are generated by
`scripts/generate-ui-tokens.mjs` from `ui/design-system/tokens.json`, alongside
CSS and C++. Slint lengths retain logical pixel units; `em` letter spacing is
a ratio multiplied by font size. Native family tokens select the registered
font rather than a CSS fallback stack.

`theme.slint` imports the original eight Barlow, Barlow Semi Condensed, and
JetBrains Mono font files. The CMake catalog embeds these resources and the
33 original SVG icons. Preserve `ui/design-system/fonts` and each family's
`OFL.txt` when distributing the source library or embedded fonts. The original
design-guide archive remains unchanged.

## Verification

The [verification record](VERIFICATION.md) includes the checked environment,
contract/input results, compiled C++ evidence, acceptance-criterion mapping and
screenshots. Compact machine-readable receipts are committed alongside the
catalog so the scope of the recorded checks remains reviewable.

```sh
node scripts/generate-ui-tokens.mjs --check
node --test tests/js/DesignTokensTest.mjs tests/js/SlintDesignTokensTest.mjs tests/js/SlintLibraryInventoryTest.mjs
python scripts/check-slint-library.py
python -m unittest discover -s tests/python -p 'test_demo_*.py'
```

[tests/slint/README.md](../../tests/slint/README.md) describes real input checks,
headless display setup, fixture selection, and recorded evidence. CTest runs
the independent C++ value codec and, when the SDK supports `TESTING`, a generated
Slint component binding test. `--check-only` proves compilation and is reported
separately from interaction verification.

To include token/inventory and repository launcher checks in this CTest build,
configure with `-DDANDRUM_SLINT_TEST_REPOSITORY=ON`. Those optional development
gates need Node, Python and the repository's launcher-test prerequisites; the
legacy JUCE artifact calibration also uses its Linux `pkg-config` dependencies.
The catalog's normal build and native codec tests remain independent of them.

The catalog supports 1200×800 and 820×560 windows with scrollable content.
Its page IDs are `controls`, `mapping`, `waveforms`, `routing`, `layout`, and
`foundations`. The state selector exercises supported disabled, prepared,
host, empty, loading and error presentations. Side-by-side state specimens
remain fixed for comparison.
