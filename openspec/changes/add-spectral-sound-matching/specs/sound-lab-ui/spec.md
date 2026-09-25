## ADDED Requirements

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
