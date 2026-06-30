## ADDED Requirements

### Requirement: Graph cycles are detected
The graph validator SHALL detect routing cycles before rendering starts.

#### Scenario: Cycle path is reported
- **WHEN** validation finds a routing cycle
- **THEN** diagnostics SHALL include the modules and ports participating in the cycle path

### Requirement: Audio feedback requires delay boundary
Audio-rate feedback cycles SHALL be valid only when every cycle contains an explicit audio delay-bearing boundary.

#### Scenario: Audio feedback through one-sample delay is valid
- **WHEN** an audio feedback cycle includes a one-sample delay module
- **THEN** graph validation SHALL accept the cycle

#### Scenario: Instantaneous audio feedback is rejected
- **WHEN** an audio feedback cycle contains no delay-bearing module
- **THEN** graph validation SHALL fail before rendering starts

### Requirement: Control feedback requires scheduling boundary
Control/VCA feedback cycles SHALL be valid only when the cycle contains a control delay, smoothing stage, or explicit tick/block boundary.

#### Scenario: Control feedback through control delay is valid
- **WHEN** a control output feeds back to an upstream control input through a control delay module
- **THEN** graph validation SHALL accept the cycle

#### Scenario: Instantaneous control feedback is rejected
- **WHEN** a control feedback cycle contains no control delay, smoothing stage, or tick/block boundary
- **THEN** graph validation SHALL fail before rendering starts

### Requirement: Event and script feedback is future scheduled
Event and script feedback SHALL be queued to a future tick or processing block and SHALL NOT execute recursively in the same processing step.

#### Scenario: Event feedback is queued
- **WHEN** an event output is routed back to an upstream event input
- **THEN** the engine SHALL schedule the feedback for a future tick or block according to the event scheduler
