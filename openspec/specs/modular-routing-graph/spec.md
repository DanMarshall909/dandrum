## Purpose

Specify graph construction and validation for explicit named-port module routing.

## Requirements

### Requirement: Instrument graph model
An instrument SHALL be represented as a graph of modules connected by explicit cables between named ports.

#### Scenario: Graph is constructed from patch declarations
- **WHEN** a validated patch contains module and connection declarations
- **THEN** the engine SHALL construct a graph whose nodes are modules and whose edges are cable connections

### Requirement: Named typed ports
Every routable module endpoint SHALL be represented as a named input or output port with a declared signal type.

#### Scenario: Port direction is validated
- **WHEN** a connection targets an output port or originates from an input port
- **THEN** validation SHALL fail and report the incorrect port direction

#### Scenario: Port existence is validated
- **WHEN** a connection references a missing module or missing port
- **THEN** validation SHALL fail and report the unresolved module or port reference

### Requirement: Signal compatibility validation
The graph validator SHALL reject connections between incompatible signal types before rendering starts.

#### Scenario: Compatible audio output connects to compatible audio input
- **WHEN** a patch connects any audio output port to any compatible audio input port
- **THEN** validation SHALL succeed for that connection

#### Scenario: Incompatible signal types are rejected
- **WHEN** a patch connects an audio output directly to a MIDI input
- **THEN** validation SHALL fail and report the incompatible source and destination port types

### Requirement: Explicit many-to-one routing
The graph SHALL NOT implicitly sum multiple outputs connected to the same single-value input unless that input explicitly supports mixing.

#### Scenario: Multiple sources require a mixer
- **WHEN** two control outputs are connected to a control input that does not support multi-source mixing
- **THEN** validation SHALL fail and instruct the patch author to use an explicit mixer or summing module
