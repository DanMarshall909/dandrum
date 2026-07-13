## MODIFIED Requirements

### Requirement: Instrument graph model

An instrument SHALL be represented as a root graph definition whose graph contains modules (node instances of other graph definitions) connected by explicit cables between named ports. The same graph model SHALL describe primitives, composites, and complete instruments.

#### Scenario: Graph is constructed from patch declarations

- **WHEN** a validated patch contains module and connection declarations
- **THEN** the engine SHALL construct a graph whose nodes are modules and whose edges are cable connections

#### Scenario: One validation path for all definition kinds

- **WHEN** a graph mixes primitive instances and composite instances
- **THEN** graph construction and validation SHALL treat them through the same node, port, and cable rules

### Requirement: Named typed ports

Every routable module endpoint SHALL be represented as a named input or output port with a declared signal type, channel count, and input multiplicity. Control input ports SHALL support declared default values with optional range metadata.

#### Scenario: Port direction is validated

- **WHEN** a connection targets an output port or originates from an input port
- **THEN** validation SHALL fail and report the incorrect port direction

#### Scenario: Port existence is validated

- **WHEN** a connection references a missing module or missing port
- **THEN** validation SHALL fail and report the unresolved module or port reference

#### Scenario: Channel counts are validated

- **WHEN** a connection joins ports whose resolved channel counts differ
- **THEN** validation SHALL fail and report both channel counts

#### Scenario: Multiple sources require summing multiplicity

- **WHEN** more than one connection targets a single-source input
- **THEN** validation SHALL fail and direct the author to a summing input or explicit mixer

### Requirement: Signal compatibility validation

The graph validator SHALL reject connections between incompatible signal types before rendering starts. Same-type connections are compatible; control outputs MAY feed audio inputs through compiler-inserted promotion; all other cross-type connections SHALL be rejected.

#### Scenario: Compatible audio output connects to compatible audio input

- **WHEN** a patch connects any audio output port to any compatible audio input port
- **THEN** validation SHALL succeed for that connection

#### Scenario: Control output promotes to audio input

- **WHEN** a patch connects a control output to an audio input
- **THEN** validation SHALL succeed and compilation SHALL insert an explicit rate-promotion step

#### Scenario: Incompatible signal types are rejected

- **WHEN** a patch connects an audio output directly to a MIDI input
- **THEN** validation SHALL fail and report the incompatible source and destination port types
