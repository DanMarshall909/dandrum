## RENAMED Requirements

- FROM: `### Requirement: Instrument authoring is external to the plugin`
- TO: `### Requirement: Instrument authoring uses safe structural replacement`

## MODIFIED Requirements

### Requirement: Plugin runtime loads immutable instrument definitions

The DAW plugin SHALL treat a loaded YAML instrument definition as immutable for the lifetime of that loaded instrument instance. Supported structural edits SHALL automatically replace that instance through the muted off-audio-thread rebuild transaction specified by `plugin-structural-authoring`.

#### Scenario: Loaded instrument establishes structure

- **GIVEN** a plugin instance loads a YAML instrument definition
- **WHEN** the instrument load succeeds
- **THEN** the loaded definition establishes the DSP graph, routing, modules, assets, public preset surface, parameter identities, labels, ranges, defaults, and mappings
- **AND** those structural definitions SHALL remain stable until automatic structural replacement, instrument reload/replacement or plugin recreation
- **AND** automatic structural replacement SHALL preserve existing host parameter and automation identities

#### Scenario: Audio callback does not mutate instrument structure

- **GIVEN** the plugin is processing audio
- **WHEN** the host calls the audio callback
- **THEN** the callback SHALL NOT parse YAML, compile graphs, load samples, change routing, create modules, destroy modules, or mutate the loaded instrument definition
- **AND** callbacks beginning during a structural rebuild SHALL return silence without accessing the replaceable engine or waiting for preparation

### Requirement: Instrument authoring uses safe structural replacement

The plugin SHALL provide supported structural authoring actions only through capability-backed commands that automatically mute, safely hand off engine ownership, validate/rebuild off audio, activate and resume. Unsupported authoring SHALL remain available through the CLI or a dedicated external authoring interface. The plugin SHALL NOT require a draft, Apply or confirmation workflow for supported structural edits.

#### Scenario: User wants to change the instrument graph

- **GIVEN** a user wants to edit modules, routing, YAML structure, assets, scripts, scheduling, feedback, or graph topology
- **WHEN** they are using the DAW plugin editor
- **THEN** supported structural edits SHALL automatically use the safe muted rebuild transaction
- **AND** unsupported edits SHALL remain unavailable in that editor and may be performed through the CLI or a dedicated external authoring interface

### Requirement: Instrument loading is prepared off the audio thread

The plugin SHALL load, validate, compile, and prepare replacement instruments off the audio thread. Structural replacement SHALL use the automatic muted transaction, retain the last working configuration for recovery, and retire engine resources only after a safe callback ownership handoff.

#### Scenario: User reloads an instrument

- **GIVEN** the user requests an instrument reload from the plugin UI
- **WHEN** the reload begins
- **THEN** the plugin SHALL mute its output and safely close callback access to the replaceable engine before validating and preparing the replacement away from the audio callback
- **AND** the plugin SHALL publish the replacement to the audio thread only after preparation succeeds and automatically unmute
- **AND** the previous engine SHALL remain retained for failure recovery without rendering during preparation

#### Scenario: Instrument load fails

- **GIVEN** a replacement instrument load fails validation, compilation, asset preparation, or parsing
- **WHEN** the failure is detected
- **THEN** the current working instrument SHALL resume automatically if one exists, with output unmuted
- **AND** the plugin SHALL expose the failure through editor/status reporting off the audio thread and re-enable structural controls

### Requirement: Sampler example exposes host-modulatable musical controls

The sampler VST3 example SHALL load a prepared drum patch with redistributable sample assets and expose stable public host parameters for live playback pitch, start position, level, pan, and variation. The kick, snare, closed hat, and open hat SHALL have separate pitch, start, level, and pan host parameters, with independent variation controls where the pad has alternates. A host parameter change SHALL reach the running Rust sample player without rebuilding the instrument. Source files, sample maps, voice limits, and choke modes SHALL remain preparation-time structure; supported structural edits SHALL take effect through automatic muted replacement.

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
- **THEN** that change SHALL retain the prepared sample assets, map, voice limit, and choke policy without rebuilding
- **AND** changing those structures SHALL require instrument replacement, automatically triggered by a supported structural edit

#### Scenario: Host sample rate differs from the source file

- **WHEN** the sampler example prepares a 48000 Hz source for a 44100 or 96000 Hz host
- **THEN** it SHALL load before audio processing and play the same musical pitch and duration without realtime file access or allocation

#### Scenario: Modulated drum pitch respects source and host rates

- **WHEN** a 48000 Hz drum map source plays at a 44100 or 96000 Hz host with a valid nondefault pitch control
- **THEN** its signed waveform and duration SHALL follow that musical pitch multiplied by the prepared source-to-host rate ratio
