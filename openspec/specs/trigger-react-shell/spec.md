# Trigger React shell

## Purpose

Provide the supplied Trigger editor as a standalone, reviewable React shell with an isolated silent EngineAdapter, reversible editing and complete component/interaction evidence for a later native adapter.

## Requirements

### Requirement: Faithful Trigger presentation
The shell SHALL use the name Trigger, the supplied Dandrum primitives/tokens/fonts, and the canonical mockup layouts and states.

#### Scenario: Four editor geometries
- **WHEN** the user chooses Minimum, Default, Expanded or Compact
- **THEN** the editor uses respectively 820×560, 1200×800, 1600×1000 or 900×126 logical pixels with the specified rail, columns, macros and expanded overview and no internal horizontal scrolling.

#### Scenario: Eight workspaces and all selections
- **WHEN** the user navigates Sample, Slices, Mapping, Layers, Voice, Modulation, Routing or Effects and selects tree, source, zone, slice, route, module, macro or history items
- **THEN** the workspace, breadcrumb and inspector render the corresponding supplied design from current model data.

### Requirement: Adapter-owned state and reversible commands
The shell SHALL start Empty, load Felt Kit through its patch menu, isolate all engine access to one EngineAdapter and apply edits through reversible operation commands, with gestures and rapid same-control updates coalesced.

#### Scenario: Empty, preset, reload and command undo
- **WHEN** the user loads Felt Kit, edits a value or structure, undoes/redoes, reloads or creates a new patch
- **THEN** preparing/ready transitions are visible, current data is authoritative, undo reverses the edited operands and redo reapplies them, with the header naming the next action.

### Requirement: Complete sample and mapping editing
The shell SHALL implement region/loop/fade edits, history editing/reordering, slices, analysis, KeyMap edge/whole-zone dragging, neighbour pushing, free Shift edges, clipboard and source/file assignment.

#### Scenario: Sample, slices and analysis
- **WHEN** the user edits a region/history operation or splits/merges/deletes a slice and requests/cancels analysis
- **THEN** waveform, metadata and inspector reflect accepted edits, async progress is visible and analysis failure preserves existing regions.

#### Scenario: Zone and source editing
- **WHEN** the user moves/extends/copies/pastes/deletes a zone, uses Shift alternatives or drops sources/files
- **THEN** ranges, neighbouring boundaries, roots, overlaps/gaps and destinations obey the supplied editing rules and multi-file mapping choices commit their selected policy.

### Requirement: Complete instrument editing
The shell SHALL implement selector/candidate operations, voice template/modules/policy/envelopes, modulation sources/routes, macros/MIDI learn, routing/sends/graph and insert/FX operations.

#### Scenario: Layers, voices and modulation
- **WHEN** the user edits any supported selector, candidate, voice, envelope, modulator, macro or route control or context-menu action
- **THEN** the current model, previews and inspector reflect its actual adapter operation, with undo, route safety for the fixed Amp envelope and no inert enabled menu items.

#### Scenario: Routing and effects
- **WHEN** the user changes inherited outputs/sends or adds/moves/bypasses a processor
- **THEN** routing tables/graph and bus/chain views reflect the accepted operation and undo restores it.

### Requirement: Input and mock telemetry
The shell SHALL implement supplied parameter gestures, value entry/reset, nested menus, drag/drop, keyboard navigation/audition and silent mock telemetry without leaking listeners, voices or timers.

#### Scenario: Gestures, menus and keyboard
- **WHEN** the user drags/types/steps/resets parameters, opens nested menus, uses supplied shortcuts or plays/releases computer keys
- **THEN** changes use the specified precision/gesture boundaries, menus remain inside the editor, inputs are not hijacked and release/blur cleans up held notes.

#### Scenario: Action-reachable states and log drawer
- **WHEN** the user uses ordinary actions and the docked log's failure/host switches
- **THEN** all 24 supplied states can be reached without a design-state picker, with every adapter call/event logged, loading/analysis progress, missing/unsupported recovery, voices/playheads/meters and host tint.

### Requirement: Catalog, documentation and verification
The shell SHALL include the specified isolated-component catalog with state/width/background/prop controls, final engine contract, updated platform document and complete feature audit.

#### Scenario: Reviewable complete shell
- **WHEN** the shell is delivered for phase-3 review
- **THEN** every component and required interaction is audited with its command/engine operation, browser checks cover the full scope, matching visual comparisons are recorded and any verification gap remains explicitly incomplete.
