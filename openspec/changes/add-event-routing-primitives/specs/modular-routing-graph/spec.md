## ADDED Requirements

### Requirement: Event-routing graph validation

The graph validator SHALL treat event-routing primitive ports as named typed event ports and validate their routes with the same compatibility rules used for ordinary graph modules.

#### Scenario: Event route is accepted

- **WHEN** a patch connects a compatible event output to an event-routing primitive event input
- **THEN** graph validation SHALL accept the connection

#### Scenario: Event output connects downstream

- **WHEN** a patch connects an event-routing primitive output to a compatible event input on another explicit module
- **THEN** graph validation SHALL accept the connection

#### Scenario: Audio route is rejected

- **WHEN** a patch connects an audio output directly to an event-routing primitive event input
- **THEN** graph validation SHALL fail and report the incompatible source and destination port types

### Requirement: Event routing does not imply downstream behavior

Event-routing primitives SHALL NOT cause graph validation or rendering to infer audio, control, sampler, mixer, sequencing, or signal-chain routes.

#### Scenario: Downstream signal chains use ordinary validation

- **WHEN** a patch connects event-routing outputs to explicitly declared downstream modules and routes
- **THEN** graph validation SHALL validate those modules and routes with the same rules used for ordinary patch modules

#### Scenario: Event routing does not hide invalid feedback

- **WHEN** event-routing modules participate in a feedback cycle without an explicit delay or future scheduling boundary
- **THEN** validation SHALL fail with the same cycle diagnostic used for ordinary graph routes
