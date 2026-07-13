## ADDED Requirements

### Requirement: Unified graph definition model

The engine SHALL represent all DSP structure through a single recursive model: a graph definition declares static parameters, public ports, internal nodes, and connections; a node is an instance of a graph definition inside another graph definition. Primitives are graph definitions implemented in Rust; composites are graph definitions implemented in YAML. Both SHALL expose the same public interface shape (ports and static parameters) and SHALL be instantiable interchangeably.

#### Scenario: Primitive and composite instantiate identically

- **WHEN** a graph definition instantiates one node referencing a Rust primitive and one node referencing a YAML composite
- **THEN** both nodes SHALL be declared, connected, validated, and discovered through the same node and port model with no composite-specific or primitive-specific declaration shape

#### Scenario: Composite is replaced by a primitive without patch changes

- **WHEN** a definition name previously implemented as a YAML composite is re-implemented as a Rust primitive with the same ports and static parameters
- **THEN** graph definitions instantiating it SHALL load and validate without modification

### Requirement: Patch is the root graph definition

A patch SHALL be a graph definition like any other: its public input and output ports are the host boundary, and any graph definition with ports MAY serve as the root. There SHALL be no patch-specific document concepts beyond optional metadata.

#### Scenario: Patch loads as root definition

- **WHEN** the engine loads a patch document
- **THEN** it SHALL construct a root graph definition whose public ports define the instrument's external interface

#### Scenario: Patch is reusable as a module

- **WHEN** a graph definition instantiates another complete patch document as a node
- **THEN** expansion and validation SHALL treat it identically to any other composite definition

### Requirement: Port model

Every port SHALL declare a name, direction, signal type (`audio`, `control`, or `event`), channel count, and input multiplicity (single-source or summing). Input ports of `control` type SHALL support a default value with optional minimum, maximum, and unit metadata. Signal rate SHALL be determined by signal type: audio ports carry per-sample streams, control ports carry one value held for a processing block, event ports carry timestamped event queues.

#### Scenario: Port declares channel count

- **WHEN** a graph definition declares an audio port with `channels: 2`
- **THEN** the port SHALL be validated and routed as a single two-channel port rather than two mono ports

#### Scenario: Control port declares default and range

- **WHEN** a primitive declares a control input port with a default value and range metadata
- **THEN** validation and discovery SHALL expose that default and range without a separate parameter declaration

#### Scenario: Summing input accepts multiple sources

- **WHEN** two compatible outputs connect to an input declared with summing multiplicity
- **THEN** validation SHALL accept both connections and compilation SHALL sum them without consulting legacy module metadata

### Requirement: Unconnected inputs read defaults

An unconnected control input port SHALL produce its effective default value: the node's declared default, unless overridden by the instantiating definition or by a preset targeting a root port alias.

#### Scenario: Unconnected input uses declared default

- **WHEN** a node's control input port has no incoming connection and no override
- **THEN** rendering SHALL use the port's declared default value

#### Scenario: Instance override replaces default

- **WHEN** a node declaration overrides a control input port's default value
- **THEN** rendering SHALL use the override, and a later incoming connection to that port SHALL take precedence over both

#### Scenario: Override of unknown port is rejected

- **WHEN** a node declaration overrides a default for a port the referenced definition does not declare
- **THEN** validation SHALL fail with a structured diagnostic naming the unknown port

### Requirement: Channel count compatibility

A connection SHALL be valid only when source and destination ports have the same signal type and the same resolved channel count.

#### Scenario: Matching channel counts connect

- **WHEN** a two-channel audio output is connected to a two-channel audio input
- **THEN** validation SHALL accept the connection and route both channels through one connection

#### Scenario: Mismatched channel counts are rejected

- **WHEN** a two-channel audio output is connected to a one-channel audio input
- **THEN** validation SHALL fail with a structured diagnostic reporting both resolved channel counts

### Requirement: Control-to-audio promotion

A `control` output MAY connect to an `audio` input; the compiler SHALL insert an explicit sample-and-hold promotion step that fills each audio block with the block's control value. Audio-to-control and any implicit conversion involving `event` ports SHALL be rejected.

#### Scenario: Control output feeds audio input

- **WHEN** a control output port is connected to an audio input port
- **THEN** compilation SHALL succeed and the flattened graph SHALL contain an inspectable sample-and-hold promotion step for that connection

### Requirement: Compiled ports use channel-aware buffer spans

Each resolved audio or control port SHALL compile to a contiguous physical buffer span containing one buffer per resolved channel. Logical connections SHALL expand to channel-wise compiled edges without name lookup in the render callback.

#### Scenario: Six-channel connection compiles to six physical routes

- **WHEN** a compatible six-channel output connects to a six-channel input
- **THEN** compilation SHALL assign six source buffers and six destination buffers and route corresponding channels by pre-resolved indices

#### Scenario: Audio output cannot feed control input implicitly

- **WHEN** an audio output port is connected directly to a control input port
- **THEN** validation SHALL fail with a diagnostic directing the author to an explicit downsampling or follower module

#### Scenario: Event ports never convert

- **WHEN** an event port is connected to an audio or control port in either direction
- **THEN** validation SHALL fail with an incompatible-signal-type diagnostic

### Requirement: Recursive flattening

Compilation SHALL recursively expand composite nodes until only atomic Rust nodes remain, producing a flat execution graph. Expansion SHALL be deterministic: the same definitions, static arguments, and overrides SHALL produce identical flattened node identities and connections.

#### Scenario: Nested composites flatten to atomic nodes

- **WHEN** a root definition instantiates a composite that itself instantiates other composites
- **THEN** the compiled graph SHALL contain only atomic Rust nodes with deterministic namespaced identities

#### Scenario: Flattening is repeatable

- **WHEN** the same root definition is compiled twice
- **THEN** both compilations SHALL produce identical flattened graphs

### Requirement: Runtime is statically typed

The flattened runtime SHALL dispatch each node through a statically resolved kind with buffers typed per signal type and rate. The render path SHALL NOT perform runtime variant switching, string-keyed lookup, or dynamic type inspection per block.

#### Scenario: Compiled graph dispatches statically

- **WHEN** a realtime block renders a flattened graph
- **THEN** every node SHALL execute through its compile-time-resolved kind and pre-resolved buffer indices
