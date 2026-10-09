# COMPONENTS — hierarchy and responsibilities

## 1. Hierarchy (v2)

```
SamplerEditor
├─ Header
│  ├─ Wordmark
│  ├─ UndoRedo (IconButton ×2)
│  ├─ PatchBox (MenuButton) ──► PatchBrowserPopover | PatchContextMenu (right-click)
│  ├─ LoadStatus (StatusMessage)
│  ├─ VoiceCount (text, telemetry)
│  ├─ SizeSwitch (SegmentedControl Min/Def/Exp)
│  ├─ DiagnosticsButton ("Why did this play?")
│  ├─ PerformToggle (Button)
│  └─ OutputMeter (Meter)
├─ Banners (StatusMessage: reload pending, missing, unsupported)
├─ Body
│  ├─ PageRail (grouped icon+label items, badge)
│  ├─ TreeColumn
│  │  ├─ InstrumentTree (ListRow, drop targets)
│  │  ├─ SourceList (groups, preview progress)
│  │  ├─ ModulatorList (categories, ModGlyph, live bar)
│  │  └─ AssetBrowser (replaces the above when open)
│  ├─ Workspace (one page)
│  │  ├─ PadsPage (PadCell grid, choke brackets)
│  │  ├─ MacrosPage
│  │  ├─ SamplePage (WaveformPanel ×2, RegionFields, knob groups, LoopPanel)
│  │  ├─ SlicesPage (WaveformPanel break, AnalysisLane, SliceTable)
│  │  ├─ MappingPage (KeyMap, ZoneLane, rules strip)
│  │  ├─ LayersPage (SelectorModeBar, VelocityCrossfade | CycleSequence, CandidateTable)
│  │  ├─ VoicePage (VoiceChain, VoiceSection ×n, EnvelopeEditor ×2, VoicePolicy, FilterResponse)
│  │  ├─ ModulationPage (ModulationTable, ModulatorEditor, ShapePreview)
│  │  ├─ RoutingPage (RoutingTable, OutputBusses)
│  │  ├─ EffectsPage (LayerStack ×2)
│  │  ├─ OverviewPage (inheritance)
│  │  ├─ DiagnosticsPanel (trace stages)
│  │  └─ EmptyState / MultiDropChooser
│  └─ Inspector (Breadcrumb, TypeTag, HistoryStack, PropertyRow list)
├─ PlayDrawer
│  ├─ DrawerBar (toggle, summary, section toggles, PcKeysToggle, OctaveControl)
│  ├─ DrawerResizeEdge
│  ├─ MacroGrid (Knob ×8)
│  ├─ SplitHandle (macros | pads)
│  ├─ PlayPads ◄── shared
│  ├─ SplitHandle (pads | keys)
│  └─ PlayKeys ◄── shared
├─ StatusBar
└─ OverlayLayer (ContextMenu, value popups, drag hint)

PerformView (alternative body)
├─ Header (same)
└─ PlayPanel
   ├─ PlayBar (PcKeysToggle, OctaveControl)
   ├─ PatchList (search + list; same data as PatchBrowserPopover)
   ├─ PlayPads ◄── shared
   ├─ MacroGrid (2 × 4, md)
   └─ PlayKeys ◄── shared
```

## 2. Design-system components used (from `docs/design-system/`)

Use these as-is; do not restyle raw elements to imitate them.

| Component | Where | Count in v2 |
|---|---|---|
| Knob | macros, voice sections, modulator editor, inspector, effects | 10 sites |
| Slider | transient sensitivity | 1 |
| Button | actions, mode bars, state bar | 31 |
| IconButton | toolbars, drawer bar, zoom, browser footer | 63 |
| Toggle | route On, snap, PC keys (v1) | 9 |
| NumericField | region fields, loop fields, root, counts | 10 |
| SegmentedControl | play modes, shapes, size, filters | 24 |
| MenuButton | Patch, asset, output bus | 5 |
| ContextMenu | all right-click menus | 2 |
| PadCell | Pads page, PlayPads | 1 template, many instances |
| KeyMap | Mapping page, Expanded Voice overview | 2 |
| LayerStack | Effects | 2 |
| OutputBusses | Routing | 1 |
| WaveformPanel | Sample, Slices, loop seam | 7 |
| Meter | header | 1 |
| ListRow, PropertyRow, ModIndicator, ModGlyph | tree, inspector, modulation | — |
| StatusMessage, EmptyState, Rollout, Icon | status, empty, inspector groups | — |

## 3. Prototype components that need a production equivalent

Responsibilities are presentation only: props in, callbacks out, no engine knowledge.

| Component | Responsibility | Inputs | Emits |
|---|---|---|---|
| PageRail | Page navigation, grouped | pages, current, compact, badges | onPage |
| PatchBrowser (popover + list) | Find and load a patch; favourites; definition import/export | patches[{id,name,category,origin,contents}], current, query, category, favourites | onLoad, onFavourite, onImport, onExport, onCopy |
| PlayDrawer | Container: open/closed, height, section visibility, splits | open, height, sections, splits, size | onToggle, onResize, onSection, onSplit |
| **PlayPads** (shared) | Scrollable PadCell grid with zoom/Fit | pads[], cols, size, compact, fitLabel | onTrigger(note, vel), onSelect(note), onZoom(±), onFit |
| **PlayKeys** (shared) | Full-range scrolling keyboard with mapped/unmapped/sounding keys, zoom/Fit, custom scrollbar | keyWidth, mappedRange, sounding[], pcLabels, scroll | onNoteOn(note, vel), onNoteOff(note), onZoom, onFit, onScroll |
| PcKeysToggle | Enables computer-keyboard playing | on | onChange |
| OctaveControl | Shifts keyboard / PC-key base note | baseNote | onShift(±12) |
| FilterResponse | Draws response; drag sets cutoff (x) and resonance (y) | cutoff, resonance, points | onChange(cutoff, res, gesture) |
| EnvelopeEditor | ADSR curve with draggable corners | points, sustain, live | onPoint(gesture) |
| ShapePreview | Modulator shape, full vs. reaching destination | fn, amount, polarity, live | — |
| RegionFields | Start / End / Fade in / Fade out numeric fields with wedges | region | onChange |
| ModulationTable, ModulatorEditor | Routes grouped by source; editor for one modulator | routes, modulator | onSelect, onAmount, onToggle, onAddDestination |
| DiagnosticsPanel | Seven-stage trace of the last hit (Event → Trigger rules → Selection → Voice → Output → Modulation → Result); idle state lists stages dashed | trace | onClear |
| ScopeBar | Shows what level an edit applies to (Instrument / Group / Sound / Selection) | scope | onScope |

PlayPads and PlayKeys are deliberately single shared components: the Play drawer and the Perform view must render the same component with different size budgets, and zoom state is shared between them.

## 4. Design-system additions made during this design pass

These exist only in `prototype/_ds/…/_ds_bundle.js`. They are **not** in the repo's `docs/design-system/`. Treat them as proposals to upstream, not as existing assets.

- Icons (24 grid, 1.5 stroke, same style): `import`, `export`, `search`, `star`, `pc-keys`, `rows-more`, `rows-less`, `tap` (pointing finger, used for Pads).
- Knob, Slider and PadCell read optional CSS variables `--dd-cap`, `--dd-cap-press`, `--dd-pad` (fallbacks are the original `--dd-ink-*` values, so behaviour is unchanged when unset).
- Earlier in the pass (see v1 notes): fade wedges, effects wrapping and KeyMap edit changes in shared components.

`rows-more` / `rows-less` are no longer used in v2 (the drawer is resized by its edge).

## 5. Reuse notes

- `ParamKnob` wrapper (Knob + context menu + middle-click reset + modulation rings + drop target) should be one component reused by macros, voice sections, modulator editor, inspector and effects.
- PadCell is used by PadsPage and PlayPads; keep one PadCell and pass `compact` below 72 px.
- KeyMap (Mapping) and PlayKeys (drawer) are different components: KeyMap edits zones; PlayKeys plays notes. Both must read the same mapped-range data.
