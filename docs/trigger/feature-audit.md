# Trigger shell feature audit

The standalone React shell is the phase-3 handoff. Real decoding, audible DSP,
host automation, disk presets and JUCE mounting belong to the subsequent native
adapter. This audit covers the supplied mock-shell scope, rather than asserting
native audio behavior. The immutable source and original plan are in `reference/`.
The user's decisions (11px minimum, components under 200 lines, confirmation on
patch replacement, and no test-first requirement for this shell) are recorded in
the OpenSpec design.

## Behavior and operation ownership

| Surface | Accepted operations and behavior | Owning executable evidence |
|---|---|---|
| Frame | Empty startup; Felt Kit; four exact outer geometries; navigation; tree/browser drawers; compact macros; dirty replacement confirmation | `presentation.test.mjs`, `store-boundary.test.mjs`, browser `frame`, `patch`, `shared-controls` |
| Sample | Region/loop/fade handles, unit entry, root, playback/reverse; signed mock zero-crossing snap; zoom/overview; source replacement/relink/folder matching; silent audition | `slices-history`, `control-regressions`, `import`, browser `editing`, `states` |
| History | Add, select, bypass, parameter edits, reorder, delete and collapse; Source locked; source remains immutable; changes and derived slices share command operands | `slices-history.test.mjs`, browser `slices`, `shared-controls` |
| Slices | Split/merge/remove; move boundary; count/even/grid division; transient apply; multi-selection; gain/pan/tune/choke/output; sequential MIDI mapping; Slice history count/bypass/delete | `adapter-operations`, `slices-history`, `model-bindings`, browser `slices`, `shared-controls` |
| Analysis | Progress and explicit apply for pitch/loops/loudness/transients; cancel/failure preserve regions; replacement cancels old jobs | `engine`, `preparation`, `slices-history`, browser `slices`, `states` |
| Mapping | Current rules; move/edge/root edits; neighbour movement and free Shift alternative; rectangular clipboard replacement/stack; duplicate/delete/gap choices; file/source drops | `mapping`, `import`, `model-bindings`, browser `editing`, `instrument` |
| File import | Sequential, root+velocity, stack and RR, with interpretation preview; unsupported imports preserved for recovery; one import command | `import`, `model-bindings`, browser `editing`, `states` |
| Adapter/undo | Declared EngineAdapter works without MockEngine internals; typed Store/telemetry consumer; required reversible operands; separate injected diagnostics; queued parameter inverses and replacement Source history | `contract-consumer`, `store-boundary`, `import`; TypeScript consumer check |
| Tree/Sources | Add group, rename/duplicate/delete; assignment to groups/sounds/selectors; actual region/module identity in preview; browser search/filter and source drag | `adapter-operations`, `model-bindings`, browser `frame`, `states` |
| Layers | Stack/velocity/RR/alternate/random/weighted; weights/order/mute/solo; velocity overlap/curve; cycle-reset policy; selected-candidate preview; current telemetry feedback | `adapter-operations`, `telemetry`, `model-bindings`, browser `instrument` |
| Voice | Template add/remove/order; module parameters/bypass; Poly/Mono/Legato and policy values; ADSR/AHDSR/Multi; shared curve/handle geometry; reversible ms/s/%/dB values | `adapter-operations`, `model-bindings`, `control-regressions`, browser `instrument`, `shared-controls` |
| Modulation | Sources add/rename/delete; source/destination grouping; route add/delete/invert/bypass/amount/polarity/curve; fixed Amp route protection; shapes/knobs/toggles/previews; drag-to-control | `adapter-operations`, `model-bindings`, `control-regressions`, browser `modulation` |
| Macros | Value, default reset, rename, destinations/ranges, simulated MIDI learn, destination navigation and host tint | `adapter-operations`, `control-regressions`, browser `modulation`, `runtime` |
| Routing | Inheritance, outputs, sends, bus controls, current-model graph | `adapter-operations`, `model-bindings`, browser `instrument` |
| Effects | Add/move/remove/bypass; parameter values; output/mute; every module and add button accessible at Min and Default | `adapter-operations`, `model-bindings`, browser `instrument`, `shared-controls` |
| Input/menus | Shared parameter drag, fine/keyboard/wheel/typing/reset; pointer cancellation; queued gesture boundaries; contextual menus; computer-key release/blur and modal focus | `store-boundary`, `control-regressions`, browser `editing`, `modulation`, `runtime`, `shared-controls` |
| Runtime | Separate 30Hz reader for silent voice decay, playheads/meters/selector/modulation/host; timer/listener cleanup; calls/events/results, filtering and failure switches | `engine`, `telemetry`, `preparation`, browser `runtime`, `states` |
| Catalog | All 97 component exports independently render in nine states, prop controls, width/background controls, props table and callback log; same controls as shell | `control-regressions`, browser `catalog` |

The paths above are under `web/trigger/tests/` and `web/trigger/tests/browser/`.
The repository entry point `tests/js/TriggerReactShellTest.mjs` runs static/type,
unit, build and browser checks and is registered as the `trigger-react-shell`
CTest. Each delta-spec scenario is linked to its asserting tests in the table below.

## Acceptance-criterion proof

| Scenario in `trigger-react-shell` | Proof |
|---|---|
| Four editor geometries | `presentation` + browser `frame`, `shared-controls`; geometry evidence for all pages at Min/Default/Expanded and Compact |
| Eight workspaces and all selections | `model-bindings` + browser `frame`, `instrument`, `modulation`, `slices`, `shared-controls` |
| Empty, preset, reload and command undo | `store-boundary`, `import`, `contract-consumer`, `preparation` + browser `patch`, `shell`, `editing` |
| Sample, slices and analysis | `slices-history`, `model-bindings` + browser `slices`, `editing`, `shared-controls` |
| Zone and source editing | `mapping`, `import`, `model-bindings` + browser `editing`, `states` |
| Layers, voices and modulation | `adapter-operations`, `model-bindings`, `control-regressions` + browser `instrument`, `modulation`, `shared-controls` |
| Routing and effects | `adapter-operations`, `model-bindings` + browser `instrument`, `shared-controls` |
| Gestures, menus and keyboard | `store-boundary`, `control-regressions` + browser `editing`, `modulation`, `runtime`, `shared-controls` |
| Action-reachable states and log drawer | `telemetry`, `preparation` + browser `runtime`, `states` (24 captures) |
| Reviewable complete shell | `contract-consumer`, TypeScript consumer check, browser `catalog`, component guard, this audit, platform/contract documents and visual evidence |

## Component reuse and callback inventory

The plan's InstrumentHeader/PageTabs/ParamKnob names correspond to
AppHeader/WorkspaceTabs/ParameterKnob. No duplicated drawing implementation is
introduced for these aliases. Drawing primitives come from the supplied bundle;
authored wrappers adapt their interaction and accessibility behavior. Every
component takes props/callbacks, without importing the engine. All 70 authored
React files are below 200 lines (largest: SamplePage, 99). The catalog includes
these same components and the received primitives.

| Component | Source | Callback operation |
|---|---|---|
| AmountBar | `components/AmountBar.jsx` | updateRoute |
| AnalysisLane | `components/AnalysisLane.jsx` | None; read-only analysis |
| AnalysisStatus | `components/AnalysisStatus.jsx` | analyse, cancel |
| AppHeader | `components/AppHeader.jsx` | newPatch, loadPreset, reload; command undo/redo |
| AssetBrowser | `components/AssetBrowser.jsx` | audition, replaceRegionAsset |
| AssetSidebar | `components/AssetSidebar.jsx` | addModulator, audition, assignSource |
| Breadcrumb | `components/Breadcrumb.jsx` | None directly; presentation or parent callback composition |
| CandidateTable | `components/CandidateTable.jsx` | setCandidate, reorderCandidates |
| ChoiceDialog | `components/ChoiceDialog.jsx` | addProcessor, addModulator |
| ChoiceMenu | `components/ChoiceMenu.jsx` | Selected value callback; see owning field |
| ConfirmDialog | `components/ConfirmDialog.jsx` | newPatch, loadPreset, reload |
| CycleSequence | `components/CycleSequence.jsx` | None directly; presentation or parent callback composition |
| DragLayer | `components/DragLayer.jsx` | None; pointer presentation |
| EnvelopeEditor | `components/EnvelopeEditor.jsx` | setModulator |
| ControlMenu | `components/GenericMenu.jsx` | addRoute, updateRoute, removeRoute, bindMacro, learnMidi, resetParam |
| DestinationMenu | `components/GenericMenu.jsx` | addRoute, bindMacro |
| GenericMenu | `components/GenericMenu.jsx` | Selected item callback; see owning control |
| HistoryStack | `components/HistoryStack.jsx` | setHistory, reorderHistory, collapseHistory |
| Inspector | `components/Inspector.jsx` | Parent-bound row callbacks; setRegion, setHistory, setModulator, setVoicePolicy, updateRule, setSelector, setOutput, setSend, setProcessorParam, bindMacro |
| InstrumentTree | `components/InstrumentTree.jsx` | renameNode, duplicateNode, removeNode, assignSource |
| KeyboardPlayer | `components/KeyboardPlayer.jsx` | noteOn, noteOff, audition |
| LogDrawer | `components/LogDrawer.jsx` | MockDiagnostics getLog, clearLog, setMockSwitch through Store |
| MacroControl | `components/MacroControl.jsx` | setMacro, bindMacro, learnMidi |
| MacroStrip | `components/MacroStrip.jsx` | setMacro, bindMacro, learnMidi |
| MidiLearnDialog | `components/MidiLearnDialog.jsx` | None directly; presentation or parent callback composition |
| ModulationTable | `components/ModulationTable.jsx` | addRoute, updateRoute, removeRoute |
| ModulatorEditor | `components/ModulatorEditor.jsx` | setModulator, updateRoute |
| ModulatorList | `components/ModulatorList.jsx` | addModulator, setModulator, addRoute |
| NumericField | `components/NumericField.jsx` | Parent-bound entity operation |
| OutputBusPanel | `components/OutputBusPanel.jsx` | setBus |
| ParameterKnob | `components/ParameterKnob.jsx` | Parent-bound setParam, setRegion, setModulator or setProcessorParam |
| PresetBrowser | `components/PresetBrowser.jsx` | loadPreset |
| ProcessorStack | `components/ProcessorStack.jsx` | addProcessor, moveProcessor, bypass, setProcessorParam |
| PropertyEditor | `components/PropertyEditor.jsx` | Parent-bound field callback; the selected entity determines the operation |
| RenameDialog | `components/RenameDialog.jsx` | renameNode, renameMacro, setModulator |
| RouteDialog | `components/RouteDialog.jsx` | addRoute |
| RoutingGraph | `components/RoutingGraph.jsx` | None; read-only routing |
| RoutingTable | `components/RoutingTable.jsx` | setOutput, setSend |
| SelectorModeBar | `components/SelectorModeBar.jsx` | setSelector |
| ShapePreview | `components/ShapePreview.jsx` | None; read-only projection |
| SliceTable | `components/SliceTable.jsx` | editSlices, mapSlices, splitSlice, mergeSlices, removeSlice |
| SliceWaveform | `components/SliceWaveform.jsx` | moveSliceBoundary, splitSlice, audition |
| SourceList | `components/SourceList.jsx` | audition, auditionSource, assignSource |
| StatusBar | `components/StatusBar.jsx` | None; telemetry subscription |
| Toggle | `components/Toggle.jsx` | Parent-bound entity operation |
| TriggerFrame | `components/TriggerFrame.jsx` | None directly; presentation or parent callback composition |
| TypeTag | `components/TypeTag.jsx` | None directly; presentation or parent callback composition |
| ValueDialog | `components/ValueDialog.jsx` | Parent-bound parameter operation |
| VelocityCrossfade | `components/VelocityCrossfade.jsx` | setSelector |
| VoiceChain | `components/VoiceChain.jsx` | setTemplate |
| VoicePolicyPanel | `components/VoicePolicyPanel.jsx` | setVoicePolicy |
| VoiceSection | `components/VoiceSection.jsx` | setParam, setModuleParam |
| VoiceTemplateDialog | `components/VoiceTemplateDialog.jsx` | setTemplate |
| WaveformEditor | `components/WaveformEditor.jsx` | setRegion, audition |
| WaveformPreview | `components/WaveformPreview.jsx` | audition |
| Workspace | `components/Workspace.jsx` | None directly; presentation or parent callback composition |
| WorkspaceTabs | `components/WorkspaceTabs.jsx` | None; editor navigation |
| ZoneLane | `components/ZoneLane.jsx` | setRules, updateRule, noteOn, noteOff |
| DropPage | `components/pages/DropPage.jsx` | importSamples |
| EffectsPage | `components/pages/EffectsPage.jsx` | setChain, addProcessor, moveProcessor, setProcessorParam, bypass |
| EmptyPage | `components/pages/EmptyPage.jsx` | importSamples |
| LayersPage | `components/pages/LayersPage.jsx` | auditionSource, setSelector, setCandidate, reorderCandidates, addCandidate, removeCandidate |
| MappingPage | `components/pages/MappingPage.jsx` | setRules, updateRule, mapAssets, assignSource |
| ModulationPage | `components/pages/ModulationPage.jsx` | setModulator, addRoute, updateRoute, removeRoute |
| RoutingPage | `components/pages/RoutingPage.jsx` | setOutput, setSend, setBus |
| SamplePage | `components/pages/SamplePage.jsx` | setRegion, replaceRegionAsset, analyse, audition, setParam |
| SlicesPage | `components/pages/SlicesPage.jsx` | setSlices, splitSlice, mergeSlices, removeSlices, editSlices, mapSlices, analyse, cancel |
| VoicePage | `components/pages/VoicePage.jsx` | setTemplate, setModuleParam, setParam, setVoicePolicy, setModulator |
| DD.Button | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.ContextMenu | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.EmptyState | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.Icon | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.IconButton | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.KeyMap | `components/design-system/index.jsx` | Parent-bound callbacks; setParam, setRegion, setRules, setProcessorParam, setBus or noteOn/noteOff according to binding |
| DD.Knob | `components/design-system/index.jsx` | Parent-bound callbacks; setParam, setRegion, setRules, setProcessorParam, setBus or noteOn/noteOff according to binding |
| DD.LayerStack | `components/design-system/index.jsx` | Parent-bound callbacks; setParam, setRegion, setRules, setProcessorParam, setBus or noteOn/noteOff according to binding |
| DD.ListRow | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.MenuButton | `components/design-system/index.jsx` | Parent-bound callbacks; setParam, setRegion, setRules, setProcessorParam, setBus or noteOn/noteOff according to binding |
| DD.Meter | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.ModGlyph | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.ModIndicator | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.NumericField | `components/design-system/index.jsx` | Parent-bound callbacks; setParam, setRegion, setRules, setProcessorParam, setBus or noteOn/noteOff according to binding |
| DD.OutputBusses | `components/design-system/index.jsx` | Parent-bound callbacks; setParam, setRegion, setRules, setProcessorParam, setBus or noteOn/noteOff according to binding |
| DD.PadCell | `components/design-system/index.jsx` | Parent-bound callbacks; setParam, setRegion, setRules, setProcessorParam, setBus or noteOn/noteOff according to binding |
| DD.Panel | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.ParamLabel | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.PropertyRow | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.Rollout | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.SectionHeading | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.SegmentedControl | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.Slider | `components/design-system/index.jsx` | Parent-bound callbacks; setParam, setRegion, setRules, setProcessorParam, setBus or noteOn/noteOff according to binding |
| DD.StatusMessage | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.Switch | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.Tabs | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.Toggle | `components/design-system/index.jsx` | Parent-bound callbacks; setParam, setRegion, setRules, setProcessorParam, setBus or noteOn/noteOff according to binding |
| DD.Tooltip | `components/design-system/index.jsx` | None directly; supplied presentation primitive |
| DD.WaveformPanel | `components/design-system/index.jsx` | None directly; supplied presentation primitive |

## Mock boundary and interpretation

- Mock assets use synthetic peaks, a deterministic signed signal for zero-crossing
  snapping and fixture analysis. They are never decoded or played audibly.
- Voice policy, envelope/module/FX configuration and selector reset policy are
  handed to the adapter; the simulation demonstrates selection, voice count and
  decay rather than emulating every DSP, stealing, glide or host transport rule.
- Detector/sidechain and measured reduction are read-only. Fixed Amp routes and
  the Source operation are disabled with an explicit reason. Learn MIDI is
  disabled for controls that are not live parameters; supported targets use the
  simulated receive dialog. Native preset saving is outside this package.
- Folder search matches the selected source filename using the browser directory
  chooser; the native adapter will supply real paths and discovery.
- The source says Fade in2ms while its normalized fixture0.004×4.82s gives19ms.
  The displayed value follows the actual stored fixture and remains reversible.
- Filename conventions have no universal standard. Explicit pitch/MIDI/dynamic/
  velocity/RR tokens are supported and the import dialog exposes the interpretation.

See [verification](verification.md) for measurements and their limits and
[visual comparisons](visual-comparisons.md) for accepted source differences.
