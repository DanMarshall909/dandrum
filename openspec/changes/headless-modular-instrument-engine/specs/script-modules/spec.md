## ADDED Requirements

### Requirement: Script modules are graph modules
Script modules SHALL be first-class modules in the routing graph with declared ports and module identifiers.

#### Scenario: Script module participates in routing
- **WHEN** a script module declares an event input and control output
- **THEN** other compatible modules SHALL be able to connect to those ports using normal patch connections

### Requirement: Script modules process events and control signals
Script modules SHALL be able to receive events and control values and emit events and control values through declared ports.

#### Scenario: Script transforms MIDI event to control output
- **WHEN** a script receives a note event and emits an accent control value
- **THEN** that control value SHALL be available to downstream connected control inputs according to graph scheduling rules

### Requirement: Script state is retained safely
Script modules SHALL be able to maintain module-local state between processing calls without sharing mutable engine internals.

#### Scenario: Script remembers previous note
- **WHEN** a script stores the last received note during one processing call
- **THEN** the script SHALL be able to read that state during a later processing call for the same module instance

### Requirement: Script execution is bounded
Script modules SHALL NOT recursively execute the graph or create unbounded same-tick event loops.

#### Scenario: Script output feedback is queued
- **WHEN** a script output is routed back to an upstream script or event input
- **THEN** the engine SHALL queue that feedback to a future tick or block rather than executing recursively in the same processing step
