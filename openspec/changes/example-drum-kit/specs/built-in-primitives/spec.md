## ADDED Requirements

### Requirement: delay_line stores and reads back audio samples
The `delay_line` module SHALL write incoming audio to an internal circular buffer and read it back after a configurable number of samples. It SHALL support fractional delay via linear interpolation.

#### Scenario: delay_line delays audio by integer sample count
- **WHEN** an impulse (1.0 at frame 0, silence thereafter) is sent through a delay_line with `delay_samples` = 2.0
- **THEN** the output SHALL have a single non-zero sample at frame 2

#### Scenario: delay_line supports fractional delay
- **WHEN** an impulse is sent through a delay_line with `delay_samples` = 1.5
- **THEN** the output SHALL have non-zero samples at frames 1 and 2 (linear interpolation between adjacent buffer reads)

#### Scenario: delay_line is registered and dispatchable
- **WHEN** the built-in registry is queried for the `delay_line` module type
- **THEN** a definition SHALL be returned

#### Scenario: delay_line renders without error
- **WHEN** a patch containing a delay_line module is loaded, prepared, and rendered
- **THEN** rendering SHALL complete without panic or error

### Requirement: envelope_follower tracks signal amplitude
The `envelope_follower` module SHALL accept an `audio_in` audio input and output a control signal tracking the signal amplitude with configurable `attack` and `release` time constants.

#### Scenario: envelope_follower responds to constant input
- **WHEN** a constant signal (0.5) is fed into the envelope_follower with fast attack and release
- **THEN** the envelope output SHALL rise from 0 toward 0.5 and settle within a tolerance of 5%

#### Scenario: envelope_follower attack and release differ
- **WHEN** the attack control is set much faster than release and the input jumps from 0 to 1
- **THEN** the envelope output SHALL rise faster than it falls when the input drops back to 0

#### Scenario: envelope_follower is registered and dispatchable
- **WHEN** the built-in registry is queried for the `envelope_follower` module type
- **THEN** a definition SHALL be returned

#### Scenario: envelope_follower renders without error
- **WHEN** a patch containing an envelope_follower module is loaded, prepared, and rendered
- **THEN** rendering SHALL complete without panic or error
