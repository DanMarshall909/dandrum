## ADDED Requirements

### Requirement: Drum-machine event port validation
The graph validator SHALL treat drum-machine public ports as named typed event ports derived from the container's declared pads.

#### Scenario: Drum-machine pad input accepts event output
- **WHEN** a patch connects a compatible event output to a declared drum-machine pad input
- **THEN** graph validation SHALL accept the connection

#### Scenario: Drum-machine pad output connects to event input
- **WHEN** a patch connects a declared drum-machine pad output to a compatible event input
- **THEN** graph validation SHALL accept the connection

#### Scenario: Drum-machine event port rejects audio route
- **WHEN** a patch connects an audio output directly to a drum-machine pad input
- **THEN** validation SHALL fail and report the incompatible source and destination port types

### Requirement: Drum-machine expansion preserves graph safety
Expanded drum-machine event routes SHALL obey the same graph validation rules as ordinary patch routes and SHALL NOT introduce implicit audio, control, sequencing, or mixing behavior.

#### Scenario: Expanded routes remain event typed
- **WHEN** a drum-machine container expands its pad trigger routing
- **THEN** the expanded graph SHALL contain event-compatible routes for pad triggers and no implicit audio or control routes

#### Scenario: Pad chain routes use ordinary graph validation
- **WHEN** a drum-machine pad child chain expands into graph modules and routes
- **THEN** graph validation SHALL validate those expanded modules and routes with the same rules used for ordinary patch modules

#### Scenario: Expanded routes do not hide invalid feedback
- **WHEN** a drum-machine container participates in a feedback cycle without an explicit delay or future scheduling boundary
- **THEN** validation SHALL fail with the same cycle diagnostic used for ordinary graph routes
