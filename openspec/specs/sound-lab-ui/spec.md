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

### Requirement: Sound Lab runs reference matching as a background job

Sound Lab SHALL let the user select a supported reference WAV and start or cancel one spectral match at a time. Reference loading, matching renders, analysis, hashing, and optimization SHALL execute away from both the audio callback and editor message thread.

#### Scenario: User selects a reference and starts matching

- **WHEN** a supported reference has been selected and no Sound Lab job is active
- **THEN** Sound Lab SHALL enter a matching state immediately
- **AND** it SHALL report completed evaluations, total evaluations, and the monotonic best score as progress arrives

#### Scenario: User cancels matching

- **WHEN** the user cancels an active match
- **THEN** Sound Lab SHALL request cooperative cancellation
- **AND** it SHALL present the best completed candidate when cancellation finishes

#### Scenario: User attempts conflicting work

- **WHEN** the user requests another render, match, or AI proposal while a Sound Lab job is active
- **THEN** Sound Lab SHALL reject the conflicting request without replacing the active job

### Requirement: Sound Lab compares and accepts one coherent match

Sound Lab SHALL present the reference and best candidate from one match result with A/B audition, overlaid RMS/peak/centroid trajectories, total and component scores, best public parameter values, provenance, and completion/cancellation status.

#### Scenario: Match completes successfully

- **WHEN** a match finishes with a best candidate
- **THEN** both reference and candidate audio SHALL be playable
- **AND** their plotted trajectories and displayed scores SHALL come from that same result
- **AND** Sound Lab SHALL display the reference fingerprint, seed, and evaluation count

#### Scenario: User accepts the best candidate

- **WHEN** the user accepts a completed or cancelled best candidate
- **THEN** the matching patch SHALL become the active instrument
- **AND** every best public parameter value SHALL be applied through the instrument's public parameter surface
- **AND** the UI SHALL report acceptance without editing the repository fixture

### Requirement: Sound Lab can request a validated topology suggestion

Sound Lab SHALL expose the configured graph-proposal provider only after a match result exists and SHALL distinguish proposing, locally validated, rejected, unavailable, and cancelled outcomes.

#### Scenario: User requests a topology suggestion

- **WHEN** the user requests a suggestion for a completed match
- **THEN** provider work SHALL run on the Sound Lab background worker
- **AND** the UI SHALL identify the provider and show the explanation, suggested search controls, and local validation result
- **AND** it SHALL NOT apply the proposed patch automatically
