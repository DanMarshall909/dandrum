## MODIFIED Requirements

### Requirement: Sound Lab renders the maintained fixture without blocking realtime audio

When a demo configuration enables Sound Lab, the instrument editor SHALL provide an explicit action that renders that demo's maintained fixture through the Rust sound workbench away from both the audio callback and the editor message thread.

#### Scenario: User starts a Sound Lab render

- **GIVEN** the configured fixture matches the active instrument
- **WHEN** the user requests a Sound Lab render while no render is active
- **THEN** the panel SHALL enter a rendering state immediately
- **AND** the configured fixture render and analysis SHALL execute on a background worker
- **AND** normal plugin audio processing SHALL NOT perform fixture loading, offline rendering, FFT analysis, or WAV construction

#### Scenario: User starts another render while one is active

- **WHEN** the user requests a Sound Lab render while a render is already active
- **THEN** the system SHALL reject the duplicate request without starting a second worker
- **AND** the active render SHALL remain valid

### Requirement: Sound Lab compares and accepts one coherent match

Sound Lab SHALL present the reference and best candidate from one match result with A/B audition, overlaid RMS/peak/centroid trajectories, total and component scores, best public parameter values, provenance, and completion/cancellation status.

#### Scenario: Match completes successfully

- **WHEN** a match finishes with a best candidate
- **THEN** both reference and candidate audio SHALL be playable
- **AND** their plotted trajectories and displayed scores SHALL come from that same result
- **AND** Sound Lab SHALL display the reference fingerprint, seed, and evaluation count

#### Scenario: User accepts the best candidate

- **GIVEN** the active instrument matches the configured Sound Lab fixture
- **WHEN** the user accepts a completed or cancelled best candidate
- **THEN** the matching patch from the configured instrument source SHALL become the active instrument
- **AND** every best public parameter value SHALL be applied through the instrument's public parameter surface
- **AND** the UI SHALL report acceptance without editing the repository fixture

### Requirement: Sound Lab state crosses the existing editor bridge

When Sound Lab is enabled for a demo, the embedded editor SHALL invoke Sound Lab rendering through a named JUCE native function and SHALL receive state updates through the browser event bridge.

#### Scenario: Editor opens before a Sound Lab render

- **WHEN** a Sound Lab-enabled editor page initializes
- **THEN** it SHALL query and display the current Sound Lab state
- **AND** it SHALL subscribe to later Sound Lab state-change events
