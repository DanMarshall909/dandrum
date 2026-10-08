# Advanced Sampler shell: build plan

Decisions so far: HTML shell (JS) that runs in the browser; engine behind a mockable interface; in-memory state; starts Empty, Felt Kit loads as a preset; silent; Dandrum components only, composed into small DC files; Storybook-style catalog; event-log drawer docked at the bottom; flat layout (root, `components/`, `engine/`). Phase 1 then straight into phase 2. Stop after phase 3 for review.

## 1. Files

```
Advanced Sampler.dc.html        shell: header, macros, tabs, three-column body, log drawer
Sampler Catalog.dc.html         Storybook-style catalog
Sampler Platform Spec.dc.html   existing spec, updated in phase 3
Sampler Feature Audit.dc.html   checklist (phase 3)

engine/
  engine-contract.md            TypeScript-style interface doc (what Dandrum implements)
  mock-engine.js                MockEngine: implements the contract, seeded with presets
  store.js                      Store: state, commands, undo/redo, subscriptions
  commands.js                   command factories (one per edit)
  presets.js                    Empty, Felt Kit (and later Missing files)
  telemetry-sim.js              fake voices, playhead, meters, analysis jobs, host automation

components/                     small DCs, each with props and no engine knowledge
  (see section 4)
```

`Advanced Sampler.dc.html` is the only file that imports the store. Components receive plain props and call callbacks, so the catalog can render each one alone.

## 2. Architecture

```
UI components  ->  Store (commands, undo, selectors)  ->  EngineAdapter  ->  MockEngine | Dandrum
                                  ^                                              |
                                  +------------- events / telemetry -------------+
```

- **Store** owns the editor model (patch tree, selection, page, clipboard) and a mirror of engine state. UI reads selectors, writes by dispatching commands.
- **Command** = `{ id, label, do(engine), undo(engine) }`. Every edit is one command; drags coalesce into one. Global undo/redo is a stack of commands, not state snapshots, so it maps onto Dandrum's own undo later.
- **EngineAdapter** is the only thing the store calls. `MockEngine` implements it now; the Dandrum bridge implements it later. Prepared edits return a promise and emit `preparing` then `ready`. Live parameters are synchronous and rate-limited.
- **Telemetry** is pushed from the engine at 30 Hz as one object and held outside the store so the UI can sample it without re-rendering everything.

## 3. Engine interface (summary; full doc in `engine/engine-contract.md`)

| Area | Interface | Notes |
|---|---|---|
| Patch | `getPatch()`, `loadPreset(id)`, `newPatch()`, `reload()` | Preset browser in phase 3 |
| Tree | `Group`, `Sound`: `addGroup`, `removeNode`, `renameNode`, `duplicateNode`, `moveNode` | Hierarchy with inherited settings |
| Assets | `Asset {id, uri, state: loaded/loading/missing/unsupported/cached}`; `importAssets(files)`, `relink(id, uri)`, `getPeaks(id)` | Async load with progress |
| Regions | `Region {assetId, start, end, loop, fades, policy}`; `setRegion`, `splitSlice`, `mergeSlices`, `deleteSlice` | Slices are regions |
| Analysis | `analyse(assetId, kind)` returns a job; events `progress`, `done`, `failed`; kinds: transients, pitch, loops, loudness | Never blocks edits |
| Trigger rules | `Rule {note lo/hi, vel lo/hi, root, target}`; `addRule`, `updateRule`, `removeRule`, `pasteRules` | KeyMap edits |
| Selectors | `Selector {policy: stack/velocity/rr/alt/random/weighted, candidates[], reset}`; `setSelector`, `reorderCandidates` | Layers page |
| Voice | `Template {modules[]}`, `VoicePolicy`; `setModuleParam`, `setPolicy` | Voice page |
| Parameters | `Param {id, value, default, min, max, unit, curve, flags}`; `setParam(id, v, gesture)` | Gesture begin/end for undo |
| Modulation | `Modulator {id, kind, shape, config}`; `Route {src, dest, amount, polarity, curve, on}`; `addRoute`, `updateRoute`, `removeRoute`, `setModulator` | Multi-destination |
| Macros | `Macro {value, bindings[], host}`; `setMacro`, `bindMacro`, `learnMidi` | Host tint on automation |
| Routing | `Bus`, `Connection {from, to, gain}`; `setOutput(nodeId, busId)`, `setSend` | Inherit = no own connection |
| Processors | `Chain {insertNodes}`; `addProcessor`, `reorder`, `bypass` | LayerStack |
| Telemetry | `onTelemetry({voices, notes, playheads, selectorPos, modValues, meters})` | 30 Hz |
| Transport | `noteOn(note, vel)`, `noteOff(note)`, `audition(assetId, region)` | Keyboard and click |

All ids are strings. All values are plain JSON so the Dandrum bridge can serialise them.

## 4. Components (small, composable)

Reused from the design system unchanged: Knob, Slider, Button, IconButton, Toggle, NumericField, Tabs, SegmentedControl, ContextMenu, MenuButton, PadCell, KeyMap, LayerStack, OutputBusses, WaveformPanel, Meter, ListRow, PropertyRow, ModIndicator, ModGlyph, StatusMessage, Tooltip, EmptyState, Panel, Rollout, SectionHeading, Icon.

New, each its own DC in `components/`:

| Group | Components |
|---|---|
| Chrome | `InstrumentHeader`, `MacroStrip`, `MacroControl`, `StatusBar`, `LogDrawer` |
| Tree and sources | `InstrumentTree`, `SourceList`, `ModulatorList`, `AssetBrowser` |
| Inspector | `Inspector`, `Breadcrumb`, `TypeTag`, `HistoryStack`, `HistoryRow` |
| Sample | `SamplePage`, `PlaybackBar`, `WaveformOverview`, `RegionKnobs` |
| Slices | `SlicesPage`, `AnalysisLane`, `SliceTable`, `DetectBar` |
| Mapping | `MappingPage`, `ZoneMenu`, `DropRules` |
| Layers | `LayersPage`, `SelectorModeBar`, `VelocityCrossfade`, `CycleSequence`, `CandidateTable` |
| Voice | `VoicePage`, `VoiceChain`, `VoiceSection`, `EnvelopeEditor`, `VoicePolicyPanel` |
| Modulation | `ModulationPage`, `ModulationTable`, `RouteGroup`, `ModulatorEditor`, `ShapePreview` |
| Routing / FX | `RoutingPage`, `RoutingTable`, `EffectsPage` |
| Menus | `ControlMenu` (modulation submenus), `DestinationMenu`, `GenericContextMenu` |
| Infra | `DragLayer`, `KeyboardPlayer` (computer keys), `Hotkeys` |

Rule: a component takes props and callbacks only. Pages compose components. Only `Advanced Sampler.dc.html` talks to the store.

## 5. Storybook-style catalog (`Sampler Catalog.dc.html`)

- Left list of every component (design-system ones and new ones), grouped as in section 4.
- One story per state: default, hover, selected, disabled, empty, loading, error, long text, minimum width.
- Controls panel per story (props as text, toggle, enum, range), the same editors the Tweaks panel uses.
- Each story shows its props table and the engine calls its callbacks would trigger.
- Background and width toggles (820 / 1200 / 1600).

## 6. Phases

**Phase 1**
1. `engine-contract.md`, `mock-engine.js`, `store.js`, `commands.js`, `presets.js` (Empty, Felt Kit).
2. Extract the existing pages into components with props only; wire every control in the shell to the store (knobs, macros, zones, routes, history, tree, sources, layers).
3. Global undo/redo as commands; remove snapshot undo.
4. Catalog page.

**Phase 2**
1. Event-log drawer: every engine call and event, failure and host-automation switches, mock controls.
2. Simulated voices, playhead and meters; computer-keyboard play (A W S E D F T G Y H U J K, Z/X octave); click pads and keys.
3. Simulated async asset load and transient analysis, with progress and failure.
4. Close menu-only items: rename, duplicate, invert, learn MIDI, split/merge/delete slice, delete modulator.
5. New editors: voice template editor, routing graph, LFO editor; History reorder; envelope knobs drive the curve.

**Phase 3**
1. Preset browser and Reload Patch flow.
2. Feature audit document: every feature in the mockup, where it lives, engine call, status.
3. Update the platform spec with the final contract.

## 7. Audit approach

The audit walks every region of the mockup (header, macros, each page, inspector, menus, drag and drop, shortcuts) and lists: control, component, store command, engine method, mock behaviour. Anything that only closes a menu is flagged until fixed.

## 8. Risks and limits

- Real Storybook needs Node; the catalog is an HTML equivalent.
- The existing 1,700-line mockup mixes layout and demo data, so phase 1 is a re-extraction, not a rename.
- Mock timing (analysis, loading) is invented; real Dandrum timings will differ.
- JUCE port is not part of this plan; the contract is written so it can be mirrored in C++ later.
