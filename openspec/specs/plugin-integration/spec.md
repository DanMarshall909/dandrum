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

### Requirement: Host boundary preserves authored defaults and note velocity

The plugin SHALL keep authored public defaults synchronized between host parameter state and the running Rust instrument, and SHALL forward JUCE MIDI note velocity without rescaling its already encoded 0–127 value.

#### Scenario: Fresh instrument starts with authored defaults

- **GIVEN** a new plugin instance loads an instrument without saved state or parameter automation
- **WHEN** the first audio block is processed
- **THEN** its host parameter values and Rust render SHALL use the instrument's authored public defaults

#### Scenario: Host MIDI velocity reaches the instrument

- **GIVEN** the host sends a JUCE note-on with velocity `V` and a block-local offset
- **WHEN** the plugin processes that block
- **THEN** Rust SHALL receive velocity `V` at the same offset
- **AND** its render SHALL match a direct Rust note event with the same note, velocity, and offset

### Requirement: Plugin editor presents declared public controls

The plugin editor SHALL present playable controls from the loaded instrument's public parameter metadata through the configured editor UI. The UI technology and appearance SHALL not change public parameter identity or host automation slots.

#### Scenario: Loaded instrument declares public parameters

- **GIVEN** a loaded instrument declares public parameters in `preset_surface.parameters`
- **WHEN** the plugin editor opens or the public surface changes after explicit replacement
- **THEN** the editor SHALL display one control for each active public parameter
- **AND** each control SHALL use its declared identity and display name
- **AND** changing a control SHALL notify the host through the corresponding stable parameter slot

#### Scenario: Plugin editor displays runtime information

- **GIVEN** a plugin instance has loaded or attempted to load an instrument
- **WHEN** the plugin editor is visible
- **THEN** it SHALL display the instrument identity or name when available
- **AND** it SHALL display load/prepare status or error text when available

#### Scenario: Editor sends playable notes

- **WHEN** a user presses and releases a key in the configured editor
- **THEN** note-on and note-off SHALL enter the bounded editor MIDI queue
- **AND** a full queue SHALL be reported without blocking the audio callback

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

### Requirement: Host preparation refreshes the loaded instrument runtime

The engine SHALL prepare the loaded instrument for the host's sample rate and maximum block size regardless of whether loading precedes preparation. Preparation SHALL occur outside the audio callback and begin a fresh processing session while retaining the immutable instrument definition, current parameter values, and resolved parameter slot identities.

#### Scenario: Initial load and preparation order agree

- **WHEN** the same instrument and note events are rendered after load-then-prepare or prepare-then-load at 44100, 48000, or 96000 Hz
- **THEN** the audio SHALL be identical for the same host settings
- **AND** the instrument SHALL use the requested sample rate for audio generation

#### Scenario: Host changes its audio settings

- **GIVEN** an instrument has already rendered audio
- **WHEN** the host prepares it again with a different sample rate or maximum block size
- **THEN** subsequent audio SHALL use the new settings and prepared event capacity
- **AND** transient DSP state and previously queued note events SHALL be cleared
- **AND** the loaded instrument's current parameter values and resolved slot identities SHALL remain usable

#### Scenario: Session restoration and failed replacement survive preparation

- **GIVEN** a plugin has restored saved instrument state with non-default public parameter values
- **WHEN** the host prepares the plugin and a later instrument replacement fails
- **THEN** the restored instrument SHALL remain audible at the requested host sample rate with its saved values
- **AND** the fixed host parameter objects, IDs, count, and order SHALL remain unchanged

### Requirement: Sampler example exposes host-modulatable musical controls

The sampler VST3 example SHALL load a prepared drum patch with redistributable sample assets and expose stable public host parameters for live playback pitch, start position, level, pan, and variation. The kick, snare, closed hat, and open hat SHALL have separate pitch, start, level, and pan host parameters, with independent variation controls where the pad has alternates. A host parameter change SHALL reach the running Rust sample player without rebuilding the instrument. Source files, sample maps, voice limits, and choke modes SHALL remain preparation-time structure.

#### Scenario: Sampler VST3 loads a playable drum kit

- **WHEN** the sampler example is instantiated and the host sends mapped MIDI drum notes
- **THEN** the plugin SHALL render the prepared kit samples through its stereo output

#### Scenario: Host modulation changes a playing drum sound

- **WHEN** the host changes a public pitch, start, level, pan, or variation parameter between audio blocks
- **THEN** the corresponding sampler control SHALL change the rendered result without instrument replacement

#### Scenario: Host modulation addresses a single drum pad

- **WHEN** the host changes a public control for one mapped drum pad
- **THEN** the sampler SHALL change that pad's rendered sound without changing another pad's control or replacing the prepared kit

#### Scenario: Sampler structure stays prepared

- **WHEN** a host changes a public playback parameter
- **THEN** the plugin SHALL retain the prepared sample assets, map, voice limit, and choke policy until an explicit reload

#### Scenario: Host sample rate differs from the source file

- **WHEN** the sampler example prepares a 48000 Hz source for a 44100 or 96000 Hz host
- **THEN** it SHALL load before audio processing and play the same musical pitch and duration without realtime file access or allocation

#### Scenario: Modulated drum pitch respects source and host rates

- **WHEN** a 48000 Hz drum map source plays at a 44100 or 96000 Hz host with a valid nondefault pitch control
- **THEN** its signed waveform and duration SHALL follow that musical pitch multiplied by the prepared source-to-host rate ratio
