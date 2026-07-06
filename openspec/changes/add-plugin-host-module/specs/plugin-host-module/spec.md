## ADDED Requirements

### Requirement: Hosted plugin module

The system SHALL support a plugin-host module that loads a user-selected external plugin and exposes it as a graph module.

#### Scenario: Hosted plugin loads as a module

- **WHEN** a patch declares a hosted plugin module with a valid plugin reference
- **THEN** preparation SHALL load the plugin and create a runtime module instance
- **THEN** the hosted plugin SHALL participate in graph execution like other modules

### Requirement: Hosted plugin is a distinct module boundary

The system SHALL treat hosted plugins as a distinct module boundary separate from built-in DSP modules.

#### Scenario: Built-in registry does not blur plugin loading

- **WHEN** the module registry is inspected
- **THEN** hosted plugins SHALL be identifiable as externally loaded modules rather than built-in engine modules

### Requirement: Hosted plugin exposes typed ports where supported

The system SHALL expose hosted plugin audio and event/control connections through typed ports where the plugin and host support them.

#### Scenario: Hosted plugin can receive and emit audio

- **WHEN** a hosted plugin module is connected in an audio graph
- **THEN** it SHALL be able to receive audio inputs and produce audio outputs through typed ports

#### Scenario: Hosted plugin can receive MIDI or event input where supported

- **WHEN** a hosted plugin declares or supports MIDI/event input
- **THEN** the host SHALL route compatible engine events to the plugin boundary through typed event or MIDI connections

### Requirement: Hosted plugin preparation reports load failure

The system SHALL fail preparation when a hosted plugin cannot be found, loaded, or instantiated.

#### Scenario: Missing plugin fails before render

- **WHEN** a patch references a missing or unsupported plugin
- **THEN** preparation SHALL fail before realtime rendering begins

### Requirement: Hosted plugin state and latency are explicit

The system SHALL treat hosted plugin state and latency as explicit host-managed metadata.

#### Scenario: Plugin state can be prepared and restored

- **WHEN** a hosted plugin instance is prepared from patch data
- **THEN** the host SHALL retain enough metadata to restore the plugin state on subsequent loads where supported

#### Scenario: Plugin latency is reported by the host

- **WHEN** a hosted plugin reports input or output latency
- **THEN** the host SHALL surface that latency in the prepared runtime metadata
