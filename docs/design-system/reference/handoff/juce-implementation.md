# Das Sampler — JUCE implementation guide

The mockups in `ui_kits/das-sampler/` are design targets. The right-click modulation workflow is **proposed**; it is not implemented in the current plugin.

## 1. What JUCE draws dynamically (never in images)

| Element | Drawn with | Repaint trigger |
|---|---|---|
| Knob track, value arc, mod rings, cap, pointer | `Path::addCentredArc`, `fillEllipse`, `drawLine` in `DandrumLookAndFeel::drawRotarySlider` | parameter change, mod timer |
| Slider track, fill, thumb, mod bars | `drawLinearSlider` | parameter change, mod timer |
| Labels | `Graphics::drawText` with embedded fonts | static |
| Value popup (value field, peak/trough, mod list) | `dd::ValuePopup : juce::Component` added to the editor's top-level overlay (`addChildComponent`, `toFront`), positioned under the knob; contains a `juce::TextEditor` for typing; shown by `mouseEnter`/`focusGained`/`sliderDragStarted`, hidden 250 ms after `mouseExit` of both | hover / focus / drag |
| Waveform | `juce::AudioThumbnail` (min/max), drawn once into a cached `Image` per region + size; overlays drawn live | patch load / resize |
| Spectrogram (Spectral display) | FFT (`juce::dsp::FFT`, order 10, hop 256) on a background thread at patch load → cached `Image` per region, log-frequency rows, colour from a 256-entry LUT built from tokens | patch load / resize |
| Region bounds, fades, loop, crossfade, slice markers | Lines, triangles, flags over the cached waveform | patch load |
| Start-offset marker | Line + 6px triangle | parameter change |
| Playback cursor | 1px line; only its strip is repainted (`repaint(x-2, …, 4, h)`) | 30 Hz timer while a voice plays |
| Pad activity, velocity bar, layer ticks, alternate dots, choke bracket | `PadComponent::paint` | note events (posted from audio thread via lock-free FIFO), 30 Hz decay |
| Meters, clip LED | `MeterComponent` | 30 Hz timer |
| Focus ring | `drawFocusOutline` helper: 2px Paper 1, 2px gap, radius = component radius + 2 | focus change |
| Host-automation tint | Value colour lerps host → paper over 400 ms after last host change | `AudioProcessorParameter::Listener` + timer |

Static assets are limited to **icons** (`assets/icons/*.svg`). No PNGs are required; every surface is a flat fill, stroke or single drop shadow.

## 2. Class mapping

**Shared LookAndFeel — `dd::DandrumLookAndFeel : juce::LookAndFeel_V4`**
- `drawRotarySlider` — Knob (reads `dd::ModRingState` from `slider.getProperties()["ddMods"]`).
- `drawLinearSlider`, `getSliderLayout` — Slider h/v.
- `drawButtonBackground`, `drawButtonText` — Button primary/secondary/ghost via `button.getProperties()["ddVariant"]`.
- `drawToggleButton` — Toggle (pill) and Switch (LED key) via `ddVariant`.
- `drawComboBox`, `drawPopupMenuBackground`, `drawPopupMenuItem`, `getPopupMenuFont` — MenuButton + all PopupMenus.
- `drawTabButton`, `drawTabAreaBehindFrontButton` — Tabs.
- `drawTooltip`, `getTooltipBounds` — Tooltip.
- `drawLabel`, `getLabelFont` — parameter labels/values.
- `drawScrollbar`, `getDefaultScrollbarWidth` — minimal hairline scrollbar (6px hit, 1px track, 4px thumb).
- `getSliderPopupPlacement`, `createSliderTextBox` — typed value entry.

**Reusable components (`dd::` namespace)**

| Class | Base | Notes |
|---|---|---|
| `dd::Knob` | `juce::Slider` (RotaryHorizontalVerticalDrag) | Adds label + value `Label`s, `ModRingState`, `onContextMenu`; attach with `SliderParameterAttachment` / `APVTS::SliderAttachment` unchanged. |
| `dd::LinearSlider` | `juce::Slider` | Same attachment; mod bars drawn by LookAndFeel. |
| `dd::NumericField` | `juce::Slider` (IncDecButtons hidden, `TextBoxOnly`-style drag) | Prepared variant = `juce::Label` with dashed outline. |
| `dd::SegmentedControl` | `juce::Component` + `juce::ToggleButton`s in a radio group | `ButtonParameterAttachment` / `ComboBoxParameterAttachment` adapter. |
| `dd::PadComponent` | `juce::Component` | Not parameter-bound; sends MIDI via `MidiKeyboardState`. |
| `dd::PadGrid` | `juce::Component` | Lays out 4×4, draws choke bracket. |
| `dd::WaveformPanel` | `juce::Component` + `ChangeListener` | Cached thumbnail image + overlays. |
| `dd::Meter` | `juce::Component` + `Timer` | Reads atomics from processor. |
| `dd::Panel` | `juce::Component` | Header + body; always collapsible (header is a button); `prepared` draws lock + tag. Parent column lays out with `juce::FlexBox`, collapsed panels get fixed header height. |
| `dd::Rollout` | `juce::Component` | Collapsible region inside a panel; hosts PropertyRows; a vertical stack in a `juce::Viewport` with the minimal scrollbar. |
| `dd::PropertyRow`, `dd::ListRow` | `juce::Component` | Prepared rows ignore mouse. |
| `dd::StatusBanner` | `juce::Component` | Icon + text + optional `TextButton`. |
| `dd::ModulationMenu` | builds a `juce::PopupMenu` with `PopupMenu::CustomComponent` rows for depth | Opened by right-click, Shift+F10, Menu key, M. |
| `dd::ModAssignController` | — | Holds assigning state; tells eligible Knobs/Sliders to draw the assigning ring. |

Parameter-bound controls use ordinary `AudioProcessorValueTreeState` attachments. Modulation never writes the host parameter: it is applied in the processor as an offset on top of the parameter value (`effective = base + Σ depth × source`), and the UI reads the effective value from an atomic for the live dot.

## 3. Modulation drawing rules
- Reserve a 6px band (md/lg) or 5px (sm) outside the track. Ring 1 radius = r − 1, ring 2 = r − 4 (md).
- Arc thickness is per element: the value arc uses an `isBeingChanged` flag; each mod ring uses its own `sourceActive || depthEditing` flag. The background track is always thin. `isBeingChanged` is true during drag (`Slider::Listener::sliderDragStarted/Ended`), for 600 ms after key/wheel input, and while host writes are arriving. Animate nothing — just switch stroke width.
- Peak / trough and clip state are computed in the processor per block (min/max of the effective value; flags when clamped) and published via atomics; the editor holds peaks until reset.
- Draw the combined modulation inside the cap (r = cap − 3): faint total range, bright base → effective, dot at effective. The effective value comes from the processor's atomic.
- Ring arc spans base value → base + depth, clipped to the sweep. Idle 60% alpha; while the source is active 100% + a dot (radius stroke + 0.8) at the live position.
- More than two assignments: draw two rings, add a “+N” chip after the label.
- Assigning: dashed ring (3 on / 3 off, 1.5px, mod A colour) + `modWash` behind the control cell.
- Host-automated: lerp value arc + value text to `dd::colour::host` for 400 ms after the last host-originated change (track `parameterValueChanged` calls that arrive without a UI gesture in progress).
- Disabled: no rings, paper4 pointer, ink4 cap.

## 4. Performance
- One 30 Hz `juce::Timer` on the editor drives meters, pad decay, cursor and live mod dots; components repaint only their dirty rect.
- The audio thread never touches components; it writes note events and levels to a lock-free FIFO / atomics.
- Cache text layouts for static labels; values re-layout only on change.
- `setBufferedToImage(true)` on panels with static content (Pad details, panel headers).
- Waveform thumbnails are generated off the message thread at patch load.

## 5. Scaling
- Design at 1200 × 800 logical. Offer zoom 75 / 100 / 125 / 150 / 200% via `AudioProcessorEditor::setScaleFactor`.
- At 75% or windows narrower than 1000 logical px, switch to the compact layout (820 × 560) instead of shrinking text: 52px pads, 48px knobs, 150px waveform, single-column pad details.
- Never render text below 11px logical. Strokes snap to whole physical px at 150%/200%.

## 6. Fonts
Embed in BinaryData and load with `Typeface::createSystemTypefaceFor`: Barlow Semi Condensed (Medium 500, SemiBold 600, Bold 700), Barlow Bold 700, JetBrains Mono (Medium 500, SemiBold 600). All SIL Open Font License — download from Google Fonts / JetBrains.
