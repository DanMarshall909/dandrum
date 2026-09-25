## Purpose

Define the embedded Sound Lab workflow for rendering the maintained TB-303 sound-design fixture away from realtime audio, then auditioning and inspecting one coherent audio-and-analysis artifact.

## Requirements

### Requirement: Sound Lab renders the maintained fixture without blocking realtime audio

The instrument editor SHALL provide an explicit Sound Lab action that renders the maintained TB-303 proof-of-concept fixture through the Rust sound workbench away from both the audio callback and the editor message thread.

#### Scenario: User starts a Sound Lab render

- **WHEN** the user requests a Sound Lab render while no render is active
- **THEN** the panel SHALL enter a rendering state immediately
- **AND** the fixture render and analysis SHALL execute on a background worker
- **AND** normal plugin audio processing SHALL NOT perform fixture loading, offline rendering, FFT analysis, or WAV construction

#### Scenario: User starts another render while one is active

- **WHEN** the user requests a Sound Lab render while a render is already active
- **THEN** the system SHALL reject the duplicate request without starting a second worker
- **AND** the active render SHALL remain valid

### Requirement: Sound Lab presents one coherent analysis artifact

The Sound Lab SHALL present audition audio and measured trajectories from the same completed fixture render.

#### Scenario: Fixture render completes successfully

- **WHEN** the maintained fixture finishes rendering and analysis succeeds
- **THEN** the panel SHALL expose playable PCM WAV audio from that render
- **AND** it SHALL plot time-aligned RMS, peak, and finite spectral-centroid measurements from that same render
- **AND** it SHALL identify the sample rate and duration

#### Scenario: Fixture render fails

- **WHEN** fixture loading, rendering, analysis, or WAV construction fails
- **THEN** the panel SHALL enter an error state with a diagnostic message
- **AND** it SHALL permit a later retry

### Requirement: Sound Lab state crosses the existing editor bridge

The embedded editor SHALL invoke Sound Lab rendering through a named JUCE native function and SHALL receive state updates through the browser event bridge.

#### Scenario: Editor opens before a Sound Lab render

- **WHEN** the editor page initializes
- **THEN** it SHALL query and display the current Sound Lab state
- **AND** it SHALL subscribe to later Sound Lab state-change events
