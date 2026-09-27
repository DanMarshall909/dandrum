## ADDED Requirements

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
