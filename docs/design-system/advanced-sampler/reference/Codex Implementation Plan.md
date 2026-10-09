# Trigger: implementation plan for Codex

"Trigger" is the instrument name. Use it for the product name, window title, plugin name and the header label (the mockup header reads dandrum · TRIGGER). The mockup and spec files keep their current file names (`Advanced Sampler.dc.html`, `Sampler Platform Spec.dc.html`) so references below still resolve.

Source of truth for every visual and behaviour decision: `Advanced Sampler.dc.html` (the mockup, 24 states) and the Dandrum design system at `_ds/dandrum-design-system-3c2eabed-50f8-48be-a226-2fe7d8a388d3/` (tokens in `tokens/*.css`, components in `_ds_bundle.js`, specs in `handoff/component-specs.md`). If this document and the mockup disagree, the mockup wins. Open the mockup, pick a state from the bar above the window, and compare side by side. Use the "Engine contract annotations" toggle to see which engine concept backs each control.

Do not approximate. Build what is drawn. Do not introduce colours, fonts, radii or spacing that are not tokens.

---

## 0. Scope and target

- **Deliverable:** a working plugin editor shell that renders the mockup exactly, runs against a mock engine, and has a single interface (`EngineAdapter`) where Dandrum plugs in.
- **Stack decision to confirm before starting:** the mockup is HTML/JS (Design Components). If the target repo is the JUCE C++ plugin, use `handoff/juce-implementation.md` and `handoff/DandrumTokens.h` for the rendering rules and treat this document's layout, behaviour and engine contract sections as language-neutral. Everything below is written so it applies to either. Class names use the `dd::` prefix; map them to `dd::` JUCE components or web components as appropriate.
- **Non-goals:** real audio, real file decoding, real DSP, presets from disk. Mock them behind the adapter.

---

## 1. Design tokens (use, never copy values)

Fonts: `--font-ui` Barlow Semi Condensed, `--font-display` Barlow Bold (wordmark, lowercase), `--font-value` JetBrains Mono (every number).
Minimum text size 11px. Caps labels 12px, +0.06em tracking (section captions are 12px bold +0.1em, page-panel titles 13px bold +0.1em).

Surfaces: `--dd-ink-0` window chrome and wells, `--dd-ink-1` window, `--dd-ink-2` panels, `--dd-ink-3` panel headers and hover, `--dd-ink-4` control faces, `--dd-ink-5` control hover, `--dd-ink-6` deepest highlight.
Text: `--dd-paper-1` primary, `-2` secondary, `-3` tertiary, `-4` disabled.
Lines: `--dd-line-1` hairline between panels, `-2` control outline, `-3` stronger outline.
Selection and primary action: `--dd-vermilion` (ember), `--dd-vermilion-wash` (row selection fill), `--dd-vermilion-lo`, `--dd-vermilion-hi` (link hover).
Modulation slots: `--dd-mod-a|b|c|d` always with `ModGlyph` (● ▲ ■ ◆). Fifth and later modulators use a hollow 7px square, never a colour.
Host: `--dd-host`. Status: `--dd-ok`, `--dd-warn` (+`-wash`), `--dd-error`.
Radii: 2 (fields, waveform, chips), 4 (buttons, rows, menus), 6 (panels). Shadows only on knob caps, pads and floating menus.
Annotation chips (overlay only): 11px mono, `--dd-warn` text, `--dd-warn-wash` fill, 1px dashed `--dd-warn`, radius 2, padding 0 5px, height 16.

---

## 2. Window frame: exact layout

The editor is a fixed-size window with three sizes plus a collapsed mode. It is not scaled; it re-lays out. Inside the mockup it is wrapped in a scale-to-fit container only for preview; ignore that wrapper.

| Size | Logical px | Notes |
|---|---|---|
| Minimum | 820 × 560 | tree becomes a 36px icon rail, compact tabs |
| Default | 1200 × 800 | |
| Expanded | 1600 × 1000 | adds read-only KeyMap overview on the Voice page, taller waveform |
| Compact (macros only) | 900 × 126 | header (44) + macro strip (82); everything else hidden |

Vertical stack inside the window (flex column, background `--dd-ink-1`, 1px `#000` outer border, overflow hidden):

1. **Header** `flex:none`, height 44 (38 at minimum), padding 0 12, gap 12, background `--dd-ink-0`, bottom hairline `--dd-line-1`. Left to right:
   - Wordmark "dandrum" (`--font-display` 20px bold, `-0.01em`) + 6px ember dot, then "TRIGGER" 13px bold caps +0.1em `--dd-paper-2` (hidden at minimum).
   - 1px × 20px divider.
   - Undo and redo `IconButton` (sm, ghost), 2px apart. Tooltip text names the last action ("Undo Added route …"). Disabled when stack empty.
   - `MenuButton` "Patch" showing the patch name (width 220, 150 at minimum, compact at minimum).
   - Inline `StatusMessage` (kind ok/info/busy/warn/error, title + text; compact at minimum).
   - Spacer.
   - Annotation chip "Telemetry" (overlay only), then "Voices 6 / 32" 12px mono `--dd-paper-3`.
   - `SegmentedControl` compact: Min / Default / Expanded (hidden in compact mode).
   - "Collapse" / "Open editor" `Button` (sm, secondary, chevron icon).
   - 30px `Meter` (thickness 4).
2. **Macro strip** `flex:none`, position relative, display flex, centre-justified, padding `8px 8px 2px`, background `--dd-ink-2`, bottom hairline.
   - "MACROS" caption (11px bold caps +0.1em) absolutely pinned left 12, vertically centred; "Binding" annotation chip under it.
   - Eight `Knob` cells centred as a group, each cell 64px wide, 6px gap, `justify-content:center`, radius 4. Knob size `md` (`sm` at minimum). Label is the knob's own top label (not beside). Each cell has `title` = destination summary. Selected macro: `--dd-vermilion-wash` fill + inset 1px ember. Host-automated: knob host tint (`--dd-host`, 400ms fade). Unassigned: disabled knob, label `--dd-paper-4`.
   - Right-click opens the macro menu (see section 6). Middle-click resets.
3. **Tab bar** (hidden in compact mode) height 34, background `--dd-ink-1`, bottom hairline, padding 0 4px 0 0. `Tabs` (8 items): Sample, Slices (badge 8), Mapping, Layers, Voice, Modulation (badge 14), Routing, Effects, each with an icon at default+; text-only at minimum ("Mod"). Right side: "Browser" `Button` (sm, ghost, folder icon, selected state when open).
4. **Banners** (only when relevant) padding 4 4 0, gap 4, `StatusMessage` compact error/warn with title and body (see states 15 and 16).
5. **Body grid** `flex:1; min-height:0; display:grid; gap:4; padding:4`. Columns: `[tree] minmax(0,1fr) [inspector]`.
   - Tree column: 208 (default), 248 (expanded), 36 (minimum rail).
   - Inspector column: 272 (default), 320 (expanded), 232 (minimum).
   - Every column is a panel: background `--dd-ink-2`, 1px `--dd-line-1`, radius 6, overflow hidden.
6. **Status bar** height 26, padding 0 12, gap 12, background `--dd-ink-0`, top hairline, 12px `--dd-paper-3`. Left: page hint text (truncates first). Right: window size in 11px mono. While dragging a source or modulator it shows "Assigning …" / "Drop … on …" text; while a notice is pending it shows the notice.
7. **Overlay layer** (position fixed within the window): context menus (z 20–31), invisible click-catcher below them. Menus never leave the window; flip left or up when they would.

### 2.1 Left column (tree column), default size

Top to bottom, all inside one panel:
1. Header 30px: "INSTRUMENT" 12px bold caps, "Group tree" annotation chip, spacer, plus `IconButton` (add group). Background `--dd-ink-3`, bottom hairline.
2. Instrument tree: `flex:1 1 0; min-height:80; overflow:auto; padding 4px 0`. Each row is a `ListRow` (26px), indent `4 + depth×14`. Row fields: icon, label, detail. Selected: ember wash + 2px left edge. Active (playing) gets a small paper dot. Valid drop targets during a source drag show 1px `--dd-line-3` inset; hovered target shows 1px ember inset + wash. Items assigned by drag appear as children (icon `slice` or `level`, detail "layer", "alt" or a note).
3. "SOURCES" header (30px) with plus button (opens browser).
4. Sources list: `max-height:38%`, scrolls. Group rows 26px (chevron, 12px caps name, mono summary). Source rows 26px, indent 24, icon 14px, name 13px ellipsis, detail 11px mono. Groups: Felt Piano (10 files), Drum samples (7), amen_172.wav (8 slices), Dandrum patches (Lush). Click previews; the row shows a 2px progress bar along its bottom while playing. Missing files show `file-missing` icon in `--dd-error`.
5. "MODULATORS" header (30px) with plus button.
6. Modulator list: `max-height:55%`, scrolls. Category rows 26px (Envelopes, LFOs, Performance, Macros, Random; collapsed by default) with slot glyphs and "5 · 6" summary (tooltip "5 sources · 6 routes"). Child rows 26px, indent 24: glyph (or hollow square), name, route count, 24×3px live bar. Selected row ember wash. Draggable; tooltip states where it is applied.
7. Footer note, 12px `--dd-paper-3`: "Click to preview a source. Drag sources onto pads or zones, modulators onto controls. Right-click a control for Modulation."

When the Browser is open (button in tab bar) it replaces section 2 to 7 with: header "SAMPLES" + close; search field (26px, `--dd-ink-0`, 1px `--dd-line-2`), `SegmentedControl` All / Samples / IRs / Favourites (full width); grouped list (Recent, Felt Piano, Impulse responses) of `ListRow`s with duration; footer hint. At minimum size the column is a 36px rail of three `IconButton`s (tree, browser, key map) and Browser does not open.

### 2.2 Right column (inspector), default size

1. Header block, padding 8 12, background `--dd-ink-3`, bottom hairline: breadcrumb (12px, last segment `--dd-paper-1`, separators `›`), then icon (16px ember) + title (15px semibold, ellipsis) + type tag (11px caps, 1px `--dd-line-3`, radius 2). Annotation chip below.
2. **History** block (Sample and Slices pages only): 28px header ("HISTORY", "3 of 5" mono, add and collapse `IconButton`s), rows 24px: grip `⋮⋮`, 14px checkbox, icon, name 13px semibold, right-aligned detail 11px mono. Source row is last, checkbox disabled at 40% opacity. Off operations use `--dd-paper-4`. Selected row ember wash + left edge. Caption 11px. Order: Loop (loop state only), Fade, Normalize (off by default), Trim, Source.
3. Property area: `flex:1; overflow:auto; padding 8 12 12; gap 2`. Rows are `PropertyRow` (compact, 24px). Section headings 12px bold caps. Mod rows use `ModIndicator`. Knob rows (processor view) are wrapped flex of `sm` knobs. Text notes 12px `--dd-paper-3`. Buttons `sm`.

Inspector content per selection is specified in section 5.

---

## 3. Workspace pages: exact layouts

All pages live in `<main>` (`min-width:0; min-height:0; overflow:auto; flex column; gap 4`). Each is a stack of panels (background `--dd-ink-2`, 1px line, radius 6) with a header bar (min 30–34px, `--dd-ink-3`, bottom hairline, padding `4px 8px 4px 12px`, gap 6–8, wrap). Titles 13px bold caps +0.1em `--dd-paper-2` for page panels, 12px for sub-panels.

### 3.1 Sample page
- Panel "SAMPLE": header contents: title, `MenuButton` (asset name, compact, 180), annotation chip, audition `IconButton`, spacer, `SegmentedControl` compact (Once, Gated, Loop, Ping-pong, Sustain loop), reverse `IconButton`.
- Body padding `8 12 12`, gap 6:
  - Overview `WaveformPanel` compact, height 28, with a 1px `--dd-paper-2` viewport rectangle (62% wide).
  - Main `WaveformPanel`: height 250 default, 190 in loop state, 150 minimum, 340 expanded. Region 0.012–0.94, fade in 0.004, fade out 0.08, optional loop and crossfade, playhead, missing hatch. Whole panel is click-to-preview. Loading state uses the partial peaks (62%) with a 280px progress block bottom left (busy `StatusMessage` + 4px bar). Unsupported state: dashed `--dd-warn` outline.
  - Metadata line: 12px, labels `--dd-paper-3`, values 12px mono `--dd-paper-2`: Format, Length, Peak, Loudness, Pitch, Source (3 items at minimum).
- Panel "LOOP" (loop state only): header with "Snap to zero crossings" toggle. Two-column body (1fr / 1.3fr), padding 12, gap 16. Left: three `NumericField`s (Loop start 96, Loop end 96, Crossfade 84), "SUGGESTED LOOPS · FROM ANALYSIS" caption, three `Button`s (sm, one selected). Right: "LOOP SEAM · END → START, ±20 MS" caption, two 56px `WaveformPanel`s side by side with a 1px ember centre line.
- Panel of knob groups: padding `6 8 8`, wrap, gap 12. Groups "REGION" (Start, End, Fade in, Fade out), "PITCH" (Tune, Fine; bipolar), "OUTPUT" (Gain, Pan), "ROOT" (`NumericField` 84px showing "60 C4" + "Detected C4 +3¢"). Each group caption is 12px bold caps with annotation chip. Knobs `sm`, label on top, gap 0 between knobs.

### 3.2 Slices page
- Panel "SLICES": header: title, "amen_172.wav · 8", divider, "Detect transients" `Button`, annotation, `SegmentedControl` Even / Grid, count `NumericField` (48), spacer, six `IconButton`s (add marker M, delete, clear, previous ←, next →).
- Body: `WaveformPanel kind="break"` height 190 (120 minimum, 260 expanded) with 8 slice markers; whole panel click selects and previews that slice. "TRANSIENTS" lane: 28px well (`--dd-ink-0`, 1px line); ticks 2px wide, height 6–24px, bright when above threshold. States: done (15 ticks, result bar with count, sensitivity `Slider` compact 120px, "Apply as 12 slices" primary, "Discard" ghost), running (progress 48% + busy `StatusMessage`; editing stays enabled), failed (error `StatusMessage`, slices unchanged).
- Panel "SLICE LIST": header caption, "2 selected", "Map from" + note `NumericField` + "Map sequentially" `Button`. Table columns (CSS grid, gap 8, row 26px): `28px #`, `minmax(80,1.4fr) Name`, `64 Note`, `minmax(90,1fr) Range`, `56 Tune`, `64 Gain`, `44 Pan`, `70 Play`, `54 Choke`, `minmax(70,1fr) Output`. Header row caps 11px. Rows draggable. Selected rows ember wash + left edge.

### 3.3 Mapping page
- Panel "KEY MAP": header with "23 zones · 3 groups", annotation, three ghost `Button`s (Map by root, Velocity layers, Sequential), divider, duplicate and delete `IconButton`s.
- Body padding 8: design-system `KeyMap` (24–96, grid height 300; 180 minimum, 420 expanded) with title "Felt Kit".
- Below the KeyMap: 14px "XF" lane (`--dd-ink-0`, left margin 30) with crossfade segments (hatched ember) and gap segments (dashed `--dd-warn`); under it a status line listing each crossfade/gap, "Shift · free edge" while dragging, last action note, clipboard chip.
- Rules strip: auto-fit grid (min 180) with 1px gaps; four cells (File on a zone, File on empty keys, Several files, Zone edges · Shift, Right-click).
- Behaviour: see section 6.4.

### 3.4 Layers page
- Panel "LAYERS": header with context path (e.g. "Snare › Hard · note 38 D2 · vel 96–127").
- Body padding 12, gap 12: play mode bar (`Button`s in a 2px-padded `--dd-ink-0` tray: Stack, Velocity, Round robin, Alternate, Random, Weighted), one-line description, then the mode view:
  - Velocity: two lanes (28px each, gap 2) in a `--dd-ink-0` well; SVG polygons for Hard (top, right-aligned label) and Soft (bottom, left label); crossfade band with dashed ember edges and wash; three drag handles (left edge, right edge, centre split); "last hit" 1px marker; axis row 1 / "last hit vel 74 · both layers" / 127. Controls row: "CROSSFADE" caption, range text, Width `NumericField` (72), Equal power / Linear `SegmentedControl`, "No crossfade" `Button`, hint. Fill opacity follows gain (100% gain = solid); crossfade region fades out to transparent (SVG gradients).
  - Round robin and others: sequence chips (28px, Last/Next tags, Next outlined ember) + "Restart cycle" `SegmentedControl` (Never, On play, Each bar).
- Candidate table (grid, row 30, header 26): `20 handle`, `28 #`, `minmax(120,1.6fr) Sample` (6px dot, name, mono file name that ellipsises first), `74 Velocity`, `70 Weight/Share` (4px bar + value), `24 M`, `24 S`, `60 Gain`, `48 Pan`, `52 Tune`, `minmax(70,1fr) Output`. Footer: "Add sample…" ghost + hint.

### 3.5 Voice page
- Strip panel (padding 6 8 6 12): "VOICE · KEYS", annotation, spacer, chain chips (24px, radius 4) separated by `+` and `→`: Sample player + Lush → Filter (ember outline) → Amplifier → Group out; "Edit template…" ghost.
- Sections grid (gap 4): columns 3 default, 5 expanded, 2 minimum. Each section: header 24px (caps title, mono sub), optional annotation, body padding `4 5 5`, gap 4: optional `SegmentedControl`, wrapped knobs (gap 0, `sm`, top labels). Sections: Source (Sample player), Source (Lush), Pitch, Filter (ember border), Amplitude.
- Envelopes grid: columns `1fr 1fr 200px` (one column at minimum): Amp envelope, Filter envelope (shape `SegmentedControl` ADSR/AHDSR/Multi in header, header wraps), and "VOICES" panel (Poly/Mono/Legato, then `PropertyRow`s: Voice limit, Steal, Same note, Exclusive group, Glide). Envelope body: well height 56 (44 minimum, 72 expanded) with SVG polyline, sustain dashed line, 9px handles, live position line `--dd-mod-a`; then 4 columns of caption+mono value.
- Expanded size adds the read-only KeyMap overview (grid 56, keyboard 44).

### 3.6 Modulation page
- Panel "MODULATION": header: title, "14 routes · Keys", annotation, spacer, "Group by" + `SegmentedControl` (Source, Destination), "Add route" `Button`.
- Route table, grid `minmax(88,1.2fr) minmax(56,96) 44 36 48 minmax(80,1fr) 24 30`, gap 6, padding 0 12, header 26 / rows 28: Source (glyph, name, lock for locked default), Amount (6px bar, zero tick, left/right fill in slot colour), value, "Pol.", Curve, "→ Destination", Live (3px bar), On (`Toggle` compact). Selected row ember wash + left edge; disabled rows 50% opacity. A source with more than one destination becomes a group: a 28px `--dd-ink-3` row (chevron, glyph, name, summary "Filter cutoff +15% · Amp gain +30%", "+ Destination" ghost, live bar, count), children indented 22 with `└`. Groups are open by default.
- Panel "MODULATOR": header: glyph, name, "→ dest · amt · polarity", annotation, "Add destination", spacer, shape `SegmentedControl` (LFO: Sine/Tri/Saw/Square/S&H; envelope: ADSR/AHDSR/Multi; random: Steps/Smooth; others: Linear/Exp/Log/S-curve). Body: flex wrap, gap `8 12`, padding `8 12`: preview well (flex 1 1 320, min 240, height 84 / 64 min / 110 exp) with dashed full-range polyline, solid reaching-destination polyline in slot colour, live marker, 11px mono y-labels and x-label; then knobs (LFO: Rate, Phase, Fade in, Smooth, Amount; envelope: Attack, Decay, Sustain, Release, Amount; random: Smooth, Amount; other: In low, In high, Smooth, Amount). Toggle row below plus caption "Dashed: full-range shape · solid: what reaches {dest} after amount and polarity".
- Contextual panel: a `md` Cutoff knob with popup open and modulation rings, plus explanation text.
- No sources column here: modulators live in the left tree column.

### 3.7 Routing page
- Two columns `minmax(0,1fr) minmax(600px,1.2fr)` at expanded; single column otherwise (busses panel below).
- Panel "ROUTING": header with "Show graph" ghost. Table grid `minmax(110,1fr) 130 40 40`, gap 8, header 26 (Source, Output bus, Reverb, Delay; last two centred), rows min 32 (padding 3px 12). Source cell: name 13px (ellipsis) over kind 11px; only the source cell is indented (`depth×14`). Output cell: compact `MenuButton` 130. Send cells: centred `xs` knobs. Selected row ember wash. Footer note on Inherit.
- `OutputBusses` panel (title "Output busses", busses Main 1/2, Drums 3/4, Snare 5/6 with feeds, meters, levels).

### 3.8 Effects page
- Two `LayerStack`s: "Group inserts" (Drums with EQ, Compressor, Saturate; Keys with EQ; Break with a bypassed Bitcrush) and "FX busses" (Reverb: Convolution, EQ; Delay: Delay, Filter). Both use outputs Main / Drums / Snare.

### 3.9 Empty and drop states
- Empty: dashed `--dd-line-3` panel on `--dd-ink-0`, centred `EmptyState` ("Drop samples here"), "Browse samples…" primary and "Load multiple…" secondary, format line.
- Multi-file drop: ember outline, `--dd-vermilion-wash` fill, heading "12 files · release to map", four choice cards (One per key, By root + velocity [suggested], Stack, Round robin) each with a 44px 6×2 mini-grid preview, name, description; footer hint; Esc cancels.

---

## 4. Components to build (small, composable)

Every component takes props and callbacks only (no engine imports). Reuse the Dandrum bundle for anything it provides (Knob, Slider, Button, IconButton, Toggle, NumericField, Tabs, SegmentedControl, ContextMenu, MenuButton, PadCell, KeyMap, LayerStack, OutputBusses, WaveformPanel, Meter, ListRow, PropertyRow, ModIndicator, ModGlyph, StatusMessage, Tooltip, EmptyState, Panel, Rollout, SectionHeading, Icon).

| Component | Props (main) | Emits |
|---|---|---|
| `InstrumentHeader` | patchName, status, voices, size, canUndo/Redo, undoLabel, perf | onUndo, onRedo, onSize, onPerf, onPatchMenu |
| `MacroStrip` / `MacroControl` | macros[{name,value,bindingCount,host,unassigned,selected}], size | onChange(name,v), onReset, onContext |
| `PageTabs` | pages, current, compact | onChange |
| `InstrumentTree` | nodes[{id,label,detail,depth,icon,selected,active,dropState}] | onSelect, onContext, onDropSource |
| `SourceList` | groups[{name,items[{name,detail,icon,playing,progress}]}] | onPreview, onDragStart, onContext |
| `ModulatorList` | categories[{name,items[{name,slot,routes,live,scopeTip}]}], open | onToggle, onPick, onDragStart, onContext |
| `AssetBrowser` | query, filter, groups | onPreview, onDragStart |
| `Inspector` | breadcrumb, icon, title, type, rows[] | onEdit(row) |
| `HistoryStack` | ops[{id,name,icon,detail,on,selected}] | onToggle, onSelect, onReorder, onDelete |
| `Breadcrumb`, `TypeTag` | segments / label | |
| `WaveformEditor` | peaks, region, fades, loop, crossfade, cursor, slices, selection, state | onRegion, onLoop, onSlice, onClick |
| `AnalysisLane` | ticks, state, progress | |
| `SliceTable` | rows, selection | onSelect, onDragStart, onContext |
| `ZoneLane` | zones, relations | |
| `SelectorModeBar` | modes, current | onChange |
| `VelocityCrossfade` | softHi, hardLo, curve, gains, lastHit | onChange, onCurve |
| `CandidateTable` | rows, mode | onReorder, onMute, onSolo, onContext |
| `VoiceChain` | modules[] | onEdit |
| `VoiceSection` | title, sub, segmented, knobs[] | onKnob |
| `EnvelopeEditor` | points, shape, live, values | onPoint, onShape |
| `ModulationTable` | groups[{src,slot,routes[]}], selected | onSelect, onToggle, onAmount, onAddDestination, onContext |
| `ModulatorEditor` | modulator, shape, knobs, toggles, preview | onShape, onKnob, onToggle, onAddDestination |
| `ShapePreview` | fn, amount, polarity, live | |
| `RoutingTable` | rows | onBus, onSend |
| `ControlMenu`, `DestinationMenu`, `GenericMenu` | items (with submenu, header, separator, danger, disabled, shortcut) | onSelect |
| `DragLayer` | dragging, hint | |
| `KeyboardPlayer` | octave | onNoteOn, onNoteOff |
| `LogDrawer` | events, switches | onSwitch |
| `StatusBar` | hint, size | |

Wrap every parameter control as `ParamKnob` = `Knob` + `data-ctx` (context menu), `data-reset` (middle-click), modulation rings, drop target highlight. Reuse it everywhere (sample, voice, modulator editor, inspector, macros).

---

## 5. Inspector content by selection

(Title, type tag, rows. `PropertyRow` unless noted.)

- **Empty / drop:** Untitled · Instrument. Sounds None, Voice limit 32, Output Main 1/2, note.
- **Sample (`felt_C4_f.wav` · Sample):** State, Format 48 kHz · 24-bit, Channels, Used by; Region: Start, End, Loop, Root 60 C4; Analysis (read-only): Pitch, Loudness, Zero crossings; Replace… button. Missing variant: State Missing, Last seen, Used by, explanatory text, Locate file… (primary), Search folder…, Replace with….
- **Slices (2 slices):** Notes, Play, Tune, Gain, Pan (Mixed shows both values), Choke group, Output, Envelope, note on Mixed.
- **Mapping zone:** Keys (Low, High, Root), Velocity (Low, High, Vel crossfade), Neighbours (Key crossfade, Gap, Edge drag read-only), Sample (File, Overlaps, Replace sample…).
- **Layers:** Hard layer (Trigger, Contents, Crossfade) or candidate (Sample, Selection: Order, Weight, Plays).
- **Voice (Filter module):** Cutoff header + `ModIndicator` rows + "Assign modulation…" button; Module rows Type, Slope, Position, Bypass.
- **Modulation route:** Route (source indicator, Destination, Amount, Polarity, Curve, Range, Scope), Source (Shape, Trigger, Used by, Edit envelope).
- **Modulator (from list):** Applied to (groups), Scope, explanation, Routes list, "Apply to group…", "Edit …".
- **Routing (Snare):** Output (Bus, Plugin output, Inherited from), Sends (Reverb, Delay, Send point).
- **Effects (Compressor):** six `sm` knobs, Gain reduction, Detector, Sidechain, Bypass.
- **Macro:** Value, MIDI, host row, Learn MIDI, Destinations list with ranges, Add destination…, Mode, Name.
- **History operation:** operation name/type "Operation", its own rows (Trim: Start, End, Snap; Fade: In, In curve, Out, Out curve; Normalize: Target, Mode, Gain applied; Loop: Start, End, Crossfade, Mode; Source: File, Format, Length, Replace…).

---

## 6. Interactions

### 6.1 Knobs and values
Vertical drag (200px = full range), Shift fine ×0.1, double-click or Enter types a value, ↑/↓ steps, PgUp/PgDn ×10, Home/End min/max, **middle-click resets** to default. Hover popup after 300ms (focus immediate): value, peak/trough, modulation sources and depths. Rings: up to two plus a "+N" chip. Alt-click also resets. Drop target ring while a modulator is dragged.

### 6.2 Control context menu (every knob except macros)
Title = control name. Items: **Modulation ▸**, Learn MIDI, Map to macro…, separator, Reset to default (shortcut Middle-click).
- Modulation ▸ submenu: **Add ▸** (every modulator by category with headers; already-used ones disabled "added"), separator, header "On {control}", then one entry per route "{Source}  +25%" each with submenu **Edit**, Invert, Bypass, separator, **Remove**.
- Edit opens the route in the inspector. Remove deletes the route (undoable).
- Dragging a modulator from the list onto a control adds a route at +25% (Alt: −25%).

### 6.3 Other context menus
- **Macro:** header "Go to destination" + each destination (switches page and outlines the control 1.8s), Add destination…, Learn MIDI, Rename…, separator, Reset to default.
- **Tree node:** Open, Audition, Rename…, Duplicate (Ctrl+D), separator, Delete (disabled for the instrument root).
- **Source:** Preview, Show in browser, Replace…, Detect transients, separator, Remove from patch.
- **Modulator:** Edit, Add destination…, Show routes, Rename…, separator, Delete modulator (disabled for Amp envelope).
- **Route row:** Edit, Bypass/Enable, Invert, Go to {destination}, separator, Remove route (disabled for Amp envelope).
- **Slice row:** Preview, Map to key…, Split at cursor, Merge with next, separator, Delete slice.
- **History operation:** Edit, Turn on/off, Collapse to here, separator, Delete operation (Source disabled).
- **KeyMap:** on a zone: Insert zone at {note}, separator, Copy key range {range} ({n}), Copy group {name}, Paste {clip} at {note}, Paste as layer at {note}, separator, Delete zone, Delete leave gap. On empty keys: Add zone in empty keys, Paste items. Title is the zone name and range, or "Key 60 C4 · vel 80".
- Menus: 260px wide, flip to stay in the window, Esc or outside click closes, nested submenus open to the right (left if no room).

### 6.4 KeyMap editing
- Dragging an edge pushes the neighbouring zone on the same velocity layer (neighbour keeps ≥1 key). Moving a whole zone moves the root with it.
- **Shift** held during the drag: free edge, zones may overlap (key crossfade) or leave a gap (silent). "Shift · free edge" label while held.
- Ctrl+C copies the selected key range (all velocity layers on those keys), Ctrl+V pastes at the key under the pointer replacing/trimming what is there, Ctrl+Shift+V pastes as a layer, Delete deletes and neighbours fill, Shift+Delete leaves a gap. Pasted zones keep their sample pitch: roots shift with the zone and names update.

### 6.5 Drag and drop
Sources and slices drag onto tree nodes (Snare: velocity layer, Hard: alternate, Closed Hat/Kick: layer, Drums: new pad, Felt keys: layer); eligible nodes outline during the drag and the status bar names the result. Modulators drag onto controls. Files drop on the window (single file creates a sound; several files show the choice cards). Esc cancels a drag.

### 6.6 Preview
Click a source, slice, slice row, sample waveform or slice in the waveform: plays it. Playhead moves on the waveform, progress bar on the source row, status bar says "Previewing …". Visual only.

### 6.7 Global undo
Ctrl+Z, Ctrl+Shift+Z / Ctrl+Y, header buttons. Each edit is one command; drags and rapid changes within 600ms merge. Covers zones, routes (add, remove, bypass), modulation, history operations, tree and source deletes, knob and macro values, crossfade, slice edits. Tooltip names the next undo.

### 6.8 Keyboard
Ctrl+1…8 pages, ←/→ previous/next slice, zone or candidate, Space auditions, Delete removes, Ctrl+D duplicates, M assign modulation on the focused control, Esc cancels/clears to parent. Focus ring: 2px `--dd-paper-1`, 2px gap. Computer keys A W S E D F T G Y H U J K play notes, Z/X octave.

---

## 7. State management

Single store, command pattern:

```
state = { patch, selection{node, zone, slice, route, op, modulator, macro}, page, layout, perf,
          ui{browserOpen, openCats, openSources, routeOpen, menu}, clipboard, preview, drag }
dispatch(command)   // command = { id, label, do(engine), undo(engine), mergeKey? }
undo(), redo(), canUndo, canRedo, undoLabel
subscribe(selector, fn)
```

Patch data matches the mock content: Felt Kit with groups Keys (10 zones: five roots × p/f), Drums (Kick 36, Snare 38 soft 1–95 + hard 96–127 with 3 alternates, Closed Hat 42 choke 1, Open Hat 46 choke 1), Break (8 slices on 24–31). Start Empty; "Felt Kit" loads from the patch menu.

---

## 8. Engine interface (what Dandrum implements)

Write `engine/engine-contract.md` as TypeScript-style declarations and implement `engine/mock-engine.js` against it.

```ts
interface EngineAdapter {
  // patch
  getPatch(): Patch; newPatch(): void; loadPreset(id: string): Promise<void>; reload(): Promise<void>;
  // tree
  addNode(parent: Id, kind: 'group'|'sound'): Id; removeNode(id: Id): void; renameNode(id: Id, name: string): void;
  duplicateNode(id: Id): Id; moveNode(id: Id, parent: Id, index: number): void;
  // assets and regions
  importAssets(files: FileRef[]): Promise<Asset[]>; relink(id: Id, uri: string): Promise<void>;
  getPeaks(id: Id, bins: number): Float32Array;
  setRegion(id: Id, r: Partial<Region>): void; splitSlice(id: Id, at: number): Id; mergeSlices(a: Id, b: Id): void; removeSlice(id: Id): void;
  // analysis (async)
  analyse(asset: Id, kind: 'transients'|'pitch'|'loops'|'loudness'): Job; cancel(job: Id): void;
  // trigger rules and selectors
  addRule(r: Rule): Id; updateRule(id: Id, r: Partial<Rule>): void; removeRule(id: Id): void;
  setSelector(target: Id, s: Partial<Selector>): void; reorderCandidates(target: Id, order: Id[]): void;
  // voice
  setModuleParam(module: Id, param: string, v: number): void; setVoicePolicy(p: Partial<VoicePolicy>): void;
  // parameters (live)
  setParam(id: Id, v: number, gesture?: 'begin'|'end'): void; resetParam(id: Id): void;
  // modulation
  addRoute(r: Route): Id; updateRoute(id: Id, r: Partial<Route>): void; removeRoute(id: Id): void;
  setModulator(id: Id, m: Partial<Modulator>): void; addModulator(kind: string): Id; removeModulator(id: Id): void;
  // macros
  setMacro(id: Id, v: number): void; bindMacro(id: Id, dest: Id, range: [number, number]): void; learnMidi(target: Id): void;
  // routing and processing
  setOutput(node: Id, bus: Id|'inherit'): void; setSend(node: Id, fx: Id, gain: number): void;
  addProcessor(chain: Id, type: string, index?: number): Id; moveProcessor(id: Id, chain: Id, index: number): void; bypass(id: Id, on: boolean): void;
  // transport
  noteOn(n: number, v: number): void; noteOff(n: number): void; audition(asset: Id, region?: Region): void;
  // events
  on(event: 'patch'|'asset'|'job'|'param'|'telemetry'|'host', fn: (e: any) => void): () => void;
}
```
Telemetry object (30 Hz): `{ voices:{active,max}, notes:[{note,vel,layer}], playheads:[{asset,pos}], selectorPos:{[target]: {last,next}}, mod:{[routeId]: value}, meters:{[bus]: [L,R]}, host:{[param]: boolean} }`.

Prepared edits resolve a promise and emit `patch` events ("preparing", then "ready"). Live parameters apply immediately. Assets report `loading` progress and can end `missing` or `unsupported` (AAC). Analysis jobs emit progress and `done` or `failed`; failure leaves regions unchanged.

Mock behaviour to implement: voices appear and decay when notes play (computer keys, clicks, audition), playheads advance, meters follow active voices, loading and analysis run on timers with a failure switch, a "host automation" switch moves a macro (host tint), every call and event is written to the log drawer.

Event-log drawer: docked at the window bottom, toggled from the header, 180px tall: scrolling list (time, call, args, result), filter, clear, switches (fail next analysis, mark file missing, host automation on/off, unsupported file).

---

## 9. States that must be reachable by real actions

1 Empty. 2 Single sample loaded. 3 Sample editing. 4 Loop editing. 5 Slice editing. 6 Transient analysis done. 7 Key map. 8 Velocity layers. 9 Round robin. 10 Voice shaping. 11 Modulation. 12 Macro editing. 13 Routing. 14 Effects. 15 Missing asset. 16 Unsupported asset. 17 Loading. 18 Analysis running. 19 Analysis failed. 20 Minimum size. 21 Default. 22 Expanded. 23 Multi-file drop. 24 Compact (macros only). Each must be reachable through the real UI plus the log-drawer switches, not through a state picker.

---

## 10. Catalog (Storybook-style)

`Sampler Catalog.dc.html`: left list of every component in section 4 plus the Dandrum components used, right canvas with width toggle (820 / 1200 / 1600) and background toggle, a controls panel (text, toggle, enum, range) bound to the story's props, a props table, and a list of the engine calls the story's callbacks would trigger. One story per state: default, hover, selected, disabled, empty, loading, error, long text, narrow.

---

## 11. Build order

1. Tokens and base: confirm the bundle loads; build `ParamKnob`, `ZoneLane`, menus (`GenericMenu` with submenus), `DragLayer`.
2. Engine: contract doc, mock, store, commands, undo. Seed Empty and Felt Kit.
3. Frame: header, macro strip, tabs, three-column grid, status bar at all four sizes.
4. Left column: tree, sources, modulators, browser, rail.
5. Inspector with History and all selection views.
6. Pages in this order: Sample, Slices, Mapping, Layers, Voice, Modulation, Routing, Effects, Empty and Drop.
7. Menus, drag and drop, keyboard, preview, middle-click reset.
8. Log drawer and telemetry simulation.
9. Gaps from the mockup: rename, duplicate, invert, learn MIDI, split, merge, delete slice, delete modulator; voice template editor, routing graph, LFO editor; History reorder; envelope knobs drive the curve; preset browser and Reload Patch.
10. Catalog, feature audit, spec update.

## 12. Acceptance checklist

- Side-by-side with the mockup at 1200×800, 820×560, 1600×1000 and 900×126: no layout difference larger than 1px.
- Every control in the mockup reads from and writes to the store through the engine interface; no demo constants remain in components.
- Every menu item either works or is listed as disabled with a reason.
- Undo reverses every edit listed in 6.7.
- No text below 11px, no colours outside tokens, no horizontal scrolling in any page at any size.
- Contract doc covers every call the mock makes.
- Catalog shows each component in isolation with its states.
