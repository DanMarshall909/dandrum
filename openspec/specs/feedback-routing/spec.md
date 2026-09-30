## Purpose

Specify feedback cycle validation and scheduling boundaries for audio, control, event, and script routing.

## Requirements

### Requirement: Graph cycles are detected

The graph validator SHALL detect routing cycles before rendering starts.

#### Scenario: Cycle path is reported

- **WHEN** validation finds a routing cycle
- **THEN** diagnostics SHALL include the modules and ports participating in the cycle path

### Requirement: Audio feedback requires delay boundary

Audio-rate feedback cycles SHALL be valid only when every cycle passes through an explicit `feedback_delay` primitive with a declared delay amount. Implicit delay-boundary attributes on other modules SHALL NOT satisfy the cycle rule.

#### Scenario: Audio feedback through feedback_delay is valid

- **WHEN** an audio feedback cycle includes a `feedback_delay` node
- **THEN** graph validation SHALL accept the cycle and scheduling SHALL cut the cycle at that node

#### Scenario: Instantaneous audio feedback is rejected

- **WHEN** an audio feedback cycle contains no `feedback_delay` node
- **THEN** graph validation SHALL fail before rendering starts, naming the cycle path and the required primitive

#### Scenario: Ordinary delay module does not legalize a cycle

- **WHEN** an audio feedback cycle passes through a delay-bearing effect module but no `feedback_delay` node
- **THEN** graph validation SHALL fail with the cycle diagnostic

### Requirement: Control feedback requires scheduling boundary

Control feedback cycles SHALL be valid only when every cycle passes through an explicit `feedback_delay` primitive; the delay is at least one processing block of control samples.

#### Scenario: Control feedback through feedback_delay is valid

- **WHEN** a control output feeds back to an upstream control input through a `feedback_delay` node
- **THEN** graph validation SHALL accept the cycle and deliver the fed-back value on a later block

#### Scenario: Instantaneous control feedback is rejected

- **WHEN** a control feedback cycle contains no `feedback_delay` node
- **THEN** graph validation SHALL fail before rendering starts

### Requirement: Event and script feedback is future scheduled

Event and script feedback SHALL be queued to a future tick or processing block and SHALL NOT execute recursively in the
same processing step.

#### Scenario: Event feedback is queued

- **WHEN** an event output is routed back to an upstream event input
- **THEN** the engine SHALL schedule the feedback for a future tick or block according to the event scheduler
