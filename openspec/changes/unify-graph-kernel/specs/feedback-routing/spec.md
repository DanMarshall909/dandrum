## MODIFIED Requirements

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

Control feedback cycles SHALL be valid only when every cycle passes through an explicit `feedback_delay` primitive; the delay is at least one processing block at control rate.

#### Scenario: Control feedback through feedback_delay is valid

- **WHEN** a control output feeds back to an upstream control input through a `feedback_delay` node
- **THEN** graph validation SHALL accept the cycle and deliver the fed-back value on a later block

#### Scenario: Instantaneous control feedback is rejected

- **WHEN** a control feedback cycle contains no `feedback_delay` node
- **THEN** graph validation SHALL fail before rendering starts
