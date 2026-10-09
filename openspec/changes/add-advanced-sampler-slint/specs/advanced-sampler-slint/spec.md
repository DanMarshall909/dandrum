## Purpose

Provide the complete advanced Trigger sampler editing and performance interface as a native Slint application, with faithful v3 appearance and a functional mockable command adapter.

## ADDED Requirements

### Requirement: Updated reference provenance
The application SHALL retain the updated 74-file guide unchanged, with archive and per-file SHA-256 provenance. The root v3 prototype SHALL govern visual details and all theme options; the prior reference SHALL remain available.

#### Scenario: Import the updated guide
- **WHEN** the new guide is imported
- **THEN** all imported bytes match the archive, both references remain accessible, and v3 is identified as authoritative

### Requirement: Complete responsive editor
The application SHALL expose Pads, Macros, Sample, Slices, Mapping, Layers, Voice, Modulation, Routing, Effects, Overview, Diagnostics and Empty/drop workspaces, with functional tree/inspector navigation and editor sizes 820x560, 1200x800 and 1600x1000.

#### Scenario: Navigate every workspace at every editor size
- **WHEN** each page and editor size is selected
- **THEN** the selected workspace, tree/inspector state and unclipped accessible actions reflect that selection

### Requirement: Every operational design state
The application SHALL reproduce all 30 guide states: Empty, Single sample, Sample editing, Loop editing, Slice editing, Transient analysis, Key map, Velocity layers, Round robin, Voice shaping, Modulation, Macro editing, Routing, Effects, Missing asset, Unsupported asset, Loading, Analysis running, Analysis failed, Minimum, Default, Expanded, Multi-file drop, Perform, Pads performance, Reload pending, Diagnostics, Overview inheritance, Freeze and Macros.

#### Scenario: Exercise all design states
- **WHEN** each named state is selected or reached through its workflow
- **THEN** the relevant controls, feedback, state transitions and associated geometry are observable

### Requirement: Complete v3 theming
The application SHALL offer aluminium, brushed, ember, graphite, midnight, forest, camo and paper surfaces; soft/flat finishes; all seven accent, nine secondary, four modulation palette and four host-color options, including editable role colors. Theme changes SHALL propagate to every component using the exact v3 role/contrast rules, retained fonts, gradients, bevels, fine highlights and shadows.

#### Scenario: Change every appearance setting
- **WHEN** each palette option, finish and custom role color is chosen
- **THEN** all semantic roles update consistently, light-theme contrast is preserved, and soft/brushed details render according to the v3 reference

### Requirement: Command-based editing and undo
Every edit SHALL dispatch through the editor command adapter. Undo/redo SHALL reverse/reapply the edit; a continuous gesture SHALL form one undo entry and cancellation SHALL restore its starting value. Selection and layout changes SHALL not corrupt editing history.

#### Scenario: Commit and cancel parameter gestures
- **WHEN** a user performs multiple drag updates, then commits or cancels and invokes undo/redo
- **THEN** one committed edit is reversed/reapplied or cancellation restores the original without adding an undo entry

### Requirement: Patch and tree workflows
The application SHALL provide the three acceptance patches Expressive Kit, Multisampled Keys and Sample + Synth, an Empty patch, searchable/category/favourite browser, actual load/import/export/copy operations and tree rename/add/remove/duplicate/move operations with observable results.

#### Scenario: Load and edit patch definitions
- **WHEN** a patch is found, favourited, loaded, edited, exported and imported
- **THEN** the visible tree and definition reflect the actions and valid definitions round-trip without losing edits

### Requirement: Sample assets and region workflows
The application SHALL import/relink assets, accept multiple file paths/drop input, expose loading/missing/unsupported/cached feedback, allow waveform/region/loop/fade/playback edits and maintain ordered source history with reorder/bypass/remove/freeze/reload actions.

#### Scenario: Edit and recover a sample asset
- **WHEN** assets are imported, a region/loop/fade is edited and an unavailable asset is relinked
- **THEN** progress and failures are visible, sample edits persist, history reflects the operation and recovery replaces stale failure feedback

### Requirement: Analysis and slicing workflows
The application SHALL support async transient/pitch/loop/loudness analysis, progress/cancel/failure/retry, candidate preview/apply/discard, slice selection/split/merge/delete and sensitivity/spacing edits.

#### Scenario: Apply analysis and edit slices
- **WHEN** analysis is started and completed or fails, and candidate slices are applied or discarded
- **THEN** the job state advances visibly, retry/cancel are effective, and resulting slice regions match the selected operation

### Requirement: Mapping and selector workflows
The application SHALL support key/velocity zone creation, selection, movement, corner resize, root/split/remove/copy/paste and selector stack/velocity/round-robin/alternate/random/weighted policies with candidate weight/reorder/reset and velocity crossfade controls.

#### Scenario: Edit mapping and selector policies
- **WHEN** zones and candidates are edited using each selector policy
- **THEN** the model and rendered regions reflect bounded note/velocity ranges, root relationships, candidate order and policy behavior

### Requirement: Voice and modulation workflows
The application SHALL provide editable voice chains, source/pitch/filter/amplifier controls, envelopes and filter graph handles, voice/choke/retrigger policy, modulator create/edit/remove and modulation destination/depth/polarity/curve/on/off routing.

#### Scenario: Shape voices and route modulation
- **WHEN** voice parameters/graph handles and modulation routes are edited
- **THEN** curves and values reflect the edits, route destinations/depths persist, and undo restores the prior settings

### Requirement: Macros routing effects and inheritance
The application SHALL implement macro values/bindings/learn/invert/clear, host automation tint, output assignment and sends, FX add/reorder/bypass/delete, inherited setting display and overrides.

#### Scenario: Edit processing and inherited settings
- **WHEN** macros, routing, sends, effects and inherited overrides are changed
- **THEN** dependent views show the resulting model and effective inherited values, with correct host/modulation role colors

### Requirement: Shared performance and note lifecycle
The application SHALL provide a resizable/toggleable Play drawer with optional macros/pads/keys, draggable splits, shared zoom/Fit, octave/PC input and a distinct 860x610 Perform view with searchable patch list. Pointer, keyboard and PC-key sources SHALL own independent note lifecycles and release notes on focus/mode/patch changes.

#### Scenario: Perform with overlapping input sources
- **WHEN** multiple input sources hold the same note and the drawer/Perform view, zoom or focus changes
- **THEN** each source releases its own note correctly, no stuck note remains, and shared zoom/PC state persists

### Requirement: Observable mockable adapter
The application SHALL identify its silent preview backend and expose event log, diagnostics, simulated voices/playhead/meters, preparation/analysis progress and failure controls. UI, serialization and telemetry work SHALL stay outside realtime DSP.

#### Scenario: Inspect preparation and diagnostics
- **WHEN** commands, simulated telemetry and failed/stale jobs occur
- **THEN** the visible event log and diagnostics identify their outcomes, stale results are rejected, and no unsupported real-engine connection is claimed

### Requirement: Maintained native demo and headless verification
The application SHALL launch through a maintained demo entry with packaged fonts/notices and matching native Slint runtime. Optional headless tests SHALL exercise the same interactions/screenshots as desktop mode, distinct from compile-only checks.

#### Scenario: Launch and test the native editor
- **WHEN** the maintained demo and desktop/headless verification commands run
- **THEN** the compiled native editor displays the complete UI, resources remain local, and both test modes validate the same functional scope

### Requirement: Visual fidelity evidence and cycle limit
The application SHALL be visually compared using actual native screenshots against the unmodified v3 reference at all editor sizes, Perform view, all design states and all themes. The feature audit SHALL map every guide control to an implemented action/test. No more than three refinement cycles SHALL be performed after functional implementation.

#### Scenario: Audit the completed editor
- **WHEN** functional controls are implemented and native screenshots are compared
- **THEN** differences and refinements are recorded, every feature has direct evidence, and comparison stops after at most three refinement cycles without narrowing functional scope
