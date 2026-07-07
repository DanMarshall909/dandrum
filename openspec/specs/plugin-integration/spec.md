## Purpose

Specify how the DAW plugin loads immutable YAML instrument definitions, exposes a stable public parameter surface, and drives the Rust audio engine while preserving realtime safety and off-audio-thread instrument preparation.

## Requirements

### Requirement: Plugin runtime loads immutable instrument definitions

The DAW plugin SHALL treat a loaded YAML instrument definition as immutable for the lifetime of that loaded instrument instance.

#### Scenario: Loaded instrument establishes structure

- **GIVEN** a plugin instance loads a YAML instrument definition
- **WHEN** the instrument load succeeds
- **THEN** the loaded definition establishes the DSP graph, routing, modules, assets, public preset surface, parameter identities, labels, ranges, defaults, and mappings
- **AND** those structural definitions remain stable until an explicit instrument reload/replacement or plugin recreation occurs

#### Scenario: Audio callback does not mutate instrument structure

- **GIVEN** the plugin is processing audio
- **WHEN** the host calls the audio callback
- **THEN** the callback SHALL NOT parse YAML, compile graphs, load samples, change routing, create modules, destroy modules, or mutate the loaded instrument definition

### Requirement: Instrument authoring is external to the plugin

The plugin SHALL NOT provide graph or YAML authoring capabilities in its DAW editor.

#### Scenario: User wants to change the instrument graph

- **GIVEN** a user wants to edit modules, routing, YAML structure, assets, scripts, scheduling, feedback, or graph topology
- **WHEN** they are using the DAW plugin editor
- **THEN** the plugin SHALL NOT expose those edits as plugin UI actions
- **AND** instrument authoring SHALL be performed through the CLI or a dedicated external authoring interface

### Requirement: Plugin editor uses native JUCE generic controls

The plugin editor SHALL use native JUCE controls for the v1 DAW UI.

#### Scenario: Loaded instrument declares public parameters

- **GIVEN** a loaded instrument declares public parameters in `preset_surface.parameters`
- **WHEN** the plugin editor is opened
- **THEN** the editor SHALL display one generic JUCE control for each declared public parameter
- **AND** the controls SHOULD use declared display labels, ranges, defaults, and units when available

#### Scenario: Plugin editor displays runtime information

- **GIVEN** a plugin instance has loaded or attempted to load an instrument
- **WHEN** the plugin editor is visible
- **THEN** it SHALL display the instrument identity or name when available
- **AND** it SHALL display load/prepare status or error text when available

### Requirement: Parameter surface is stable for a loaded instrument

The plugin SHALL keep the public parameter surface stable while an instrument definition is loaded.

#### Scenario: Host has automation bound to loaded parameters

- **GIVEN** a host has created automation for one or more plugin parameters
- **WHEN** presets or parameter values change
- **THEN** the parameter IDs, parameter count, parameter types, and parameter order SHALL NOT change

#### Scenario: New instrument has a different public surface

- **GIVEN** a plugin instance has one instrument loaded
- **WHEN** a different instrument definition with a different public parameter surface is selected
- **THEN** the plugin SHALL treat this as a full instrument replacement/reload
- **AND** it SHALL NOT mutate the existing parameter layout from the audio callback

### Requirement: Presets modify values only

Presets SHALL be compatible value overlays for the currently loaded instrument surface.

#### Scenario: Compatible preset is loaded

- **GIVEN** an instrument is loaded
- **AND** a preset targets the same instrument identity and compatible preset schema version
- **WHEN** the preset is loaded
- **THEN** the plugin SHALL apply the preset's declared public parameter values and public asset choices
- **AND** the plugin SHALL NOT change graph topology, module declarations, routing, render settings, scheduling, script definitions, feedback declarations, or undeclared structure

#### Scenario: Incompatible preset is rejected

- **GIVEN** an instrument is loaded
- **AND** a preset targets a different instrument identity or incompatible preset schema version
- **WHEN** the preset is loaded
- **THEN** the plugin SHALL reject the preset
- **AND** the plugin SHALL report a clear error off the audio thread

### Requirement: Instrument loading is prepared off the audio thread

The plugin SHALL load, validate, compile, and prepare replacement instruments off the audio thread.

#### Scenario: User reloads an instrument

- **GIVEN** the user requests an instrument reload from the plugin UI
- **WHEN** the reload begins
- **THEN** the plugin SHALL create and prepare a replacement Rust engine away from the audio callback
- **AND** the plugin SHALL publish the replacement to the audio thread only after preparation succeeds
- **AND** the existing active engine SHALL remain usable by the audio callback until the replacement is ready

#### Scenario: Instrument load fails

- **GIVEN** a replacement instrument load fails validation, compilation, asset preparation, or parsing
- **WHEN** the failure is detected
- **THEN** the current active instrument SHALL remain active if one exists
- **AND** the plugin SHALL expose the failure through editor/status reporting off the audio thread

### Requirement: Audio processing remains realtime safe

The plugin audio callback SHALL preserve the realtime callback contract.

#### Scenario: Host calls processBlock

- **GIVEN** a loaded instrument is prepared
- **WHEN** the host calls `processBlock`
- **THEN** the callback MAY read prepared parameter values, forward bounded MIDI events, clear output channels, and call Rust rendering
- **AND** the callback SHALL NOT allocate, acquire locks, perform file I/O, parse YAML, load samples, compile graphs, log, or create/destroy engines

### Requirement: MIDI handoff is sample accurate

The plugin SHALL forward MIDI note events to Rust with block-local frame offsets.

#### Scenario: Host sends note event with non-zero sample offset

- **GIVEN** the host provides a JUCE `MidiBuffer` containing a note event at sample offset `N`
- **WHEN** the plugin processes the block
- **THEN** the plugin SHALL submit the note event to Rust with frame offset `N`
- **AND** Rust SHALL preserve that frame offset as a bounded pending block event for rendering

### Requirement: Plugin state restores loaded instruments and values

The plugin SHALL persist enough state to restore a session without relying only on absolute file paths.

#### Scenario: Host saves plugin state

- **GIVEN** an instrument is loaded and parameters have current values
- **WHEN** the host requests plugin state
- **THEN** the state SHALL include a schema version
- **AND** it SHALL include the loaded instrument identity and either embedded instrument content or a bundled instrument identifier
- **AND** it SHALL include current public parameter values
- **AND** it MAY include original file paths only as restore hints

#### Scenario: Host restores plugin state

- **GIVEN** saved plugin state exists
- **WHEN** the host restores the plugin instance
- **THEN** the plugin SHALL prepare the restored instrument off the audio thread
- **AND** it SHALL restore compatible public parameter values
- **AND** it SHALL report any restore failure through editor/status reporting
