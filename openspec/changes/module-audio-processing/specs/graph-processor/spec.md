## ADDED Requirements

### Requirement: Graph processes modules in topological order
The graph processor SHALL dispatch modules in topological order respecting delay-boundary cycle cuts, so every module's inputs are computed before it runs.

#### Scenario: Processing order follows dependency order
- **WHEN** a module graph is constructed and processed
- **THEN** each module SHALL receive input signals computed by its upstream dependencies in the same block

### Requirement: Signal bus routes audio, control, and event signals per block
The graph processor SHALL route module outputs to downstream module inputs through typed buses per block.

#### Scenario: Audio output of one module is routed to audio input of another
- **WHEN** module A writes to an audio output port and module B reads from a connected audio input port
- **THEN** module B SHALL receive the samples written by module A for the same block
