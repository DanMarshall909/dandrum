## Purpose

Specify the MIDI drum loop demo that loads a drum-machine asset and exercises the existing engine playback path.

## Requirements

### Requirement: MIDI drum loop integration command

The system SHALL provide a user-invoked integration command that loads a drum-machine patch or preset and plays a repeating drum loop by sending MIDI note events through the existing engine playback path.

#### Scenario: Demo loads drum asset and starts playback

- **WHEN** the user invokes the drum loop integration command from the built executable
- **THEN** the system SHALL load the configured drum-machine patch or preset before starting the loop
- **THEN** the system SHALL start audio playback through the existing JUCE audio output path

#### Scenario: Demo uses MIDI note events

- **WHEN** the drum loop is running
- **THEN** kick, snare, and hat hits SHALL be submitted as note on and note off events to the engine
- **THEN** the demo SHALL NOT bypass the patch's event routing with drum-specific render shortcuts

#### Scenario: Demo exercises integration path

- **WHEN** the drum loop integration command runs with the default asset
- **THEN** it SHALL exercise drum-machine loading, MIDI event scheduling, engine event submission, realtime render preparation, and audio rendering through the existing wrapper path

### Requirement: Built-in simple drum pattern

The system SHALL include a built-in simple drum pattern suitable for quickly auditioning the drum patch.

#### Scenario: Pattern contains basic kit voices

- **WHEN** the built-in drum pattern is scheduled
- **THEN** it SHALL include kick, snare, and hat events over a repeating four-beat loop

#### Scenario: Pattern can be stopped by the user

- **WHEN** the user interrupts the running drum loop demo
- **THEN** the system SHALL stop scheduling new note events and exit cleanly

### Requirement: Broad drum loop integration verification

The system SHALL provide automated verification for the broadest deterministic integration slice that does not require local audio hardware.

#### Scenario: Scheduled loop renders audio

- **WHEN** the built-in drum loop events are rendered against the selected drum-machine patch or preset in tests
- **THEN** rendering SHALL complete without error
- **THEN** the rendered output SHALL contain non-zero audio samples

#### Scenario: Integration verification covers load schedule and render

- **WHEN** the integration verification runs in an automated test environment
- **THEN** it SHALL cover drum-machine asset loading, loop event scheduling, engine preparation, MIDI event submission, and rendered audio output
- **THEN** it SHALL NOT require a physical or virtual audio device to pass

### Requirement: Drum container is the default demo asset

The drum loop demo SHALL use a drum-machine asset as its default loaded instrument rather than loading the raw drum-kit patch directly.

#### Scenario: Default asset is a drum-machine

- **WHEN** the user invokes the drum loop demo without an explicit asset override
- **THEN** the system SHALL select a drum-machine patch or preset as the default asset
- **THEN** the system SHALL NOT select the raw drum-kit patch as the default demo asset
