## MODIFIED Requirements

### Requirement: Module port metadata

The discovery API SHALL return port metadata for each graph definition — primitive, defined module, or patch — including port name, direction, signal type, channel count (literal or static-parameter reference), multiplicity, and for control inputs the default value, range, and unit where declared. One metadata schema SHALL describe all definition kinds.

#### Scenario: Query module ports

- **WHEN** a specific definition is queried for ports
- **THEN** the API SHALL return each port's name, direction, signal type, channel count, multiplicity, and any default/range/unit metadata

#### Scenario: Primitive and defined module share the schema

- **WHEN** a Rust primitive and a YAML defined module are both queried
- **THEN** their port metadata SHALL be returned in the same schema with no kind-specific fields required to interpret it

### Requirement: Capability discovery is metadata-driven

The engine SHALL expose capability discovery through graph-definition, port, static-parameter, and preset-alias metadata.
Discovery SHALL NOT inspect or mutate realtime render state.

#### Scenario: Discovery uses metadata

- **WHEN** capability discovery is queried
- **THEN** it SHALL return information from registered metadata rather than constructing an audio renderer or running a
  patch

### Requirement: Module category classification

The discovery API SHALL classify graph definitions as Rust primitives or authored definitions. Script-backed and packaged definitions SHALL use the authored-definition category while retaining their implementation and origin metadata.

#### Scenario: Query module category

- **WHEN** a specific module type is queried for its category
- **THEN** the API SHALL distinguish primitive from authored definitions without requiring a separate node schema

### Requirement: Module type enumeration

The discovery API SHALL enumerate available module types, including built-in Rust primitives, inline/external defined modules where loaded, and script module support where available.

#### Scenario: Enumerate module types

- **WHEN** the capability discovery API is queried for module types
- **THEN** it SHALL return a deterministic list of available module type identifiers

### Requirement: Composite discovery preserves source model

Defined module discovery SHALL read inline and externally loaded graph definitions through the same metadata model as primitives, without requiring a separate runtime defined module type.

#### Scenario: Inline defined module discovered

- **WHEN** a patch contains an inline defined module
- **THEN** discovery MAY expose that definition's public ports and static parameters through the ordinary schema

### Requirement: Discovery supports future tooling but does not implement it

Capability discovery SHALL be suitable for future CLI, GUI, documentation, and LLM-authoring tools, but this change
SHALL NOT implement the LLM authoring layer.

#### Scenario: Discovery returns tool-friendly metadata

- **WHEN** discovery metadata is serialized for tooling
- **THEN** it SHALL contain enough stable identifiers for tools to reference definitions, ports, and static parameters without
  parsing human-readable documentation

## ADDED Requirements

### Requirement: Static parameter metadata

The discovery API SHALL return static parameter metadata for each definition: name, type (integer, enumeration, string, resource reference), default where declared, and enumeration values where applicable.

#### Scenario: Query static parameters

- **WHEN** a definition with static parameters is queried
- **THEN** the API SHALL return each static parameter's name, type, default, and allowed values without instantiating the definition

### Requirement: Prepared root enumeration reuses definition metadata

Prepared-host root-port enumeration SHALL use the same port metadata representation as capability discovery rather than maintaining a separate FFI-only schema.

#### Scenario: Discovered root matches prepared enumeration

- **WHEN** a root definition is discovered and then prepared
- **THEN** its prepared FFI enumeration SHALL report matching names, directions, signal types, and resolved channel counts

## REMOVED Requirements

### Requirement: Module parameter metadata

**Reason**: Parameters no longer exist as a separate concept; tunables are control input ports with defaults (covered by port metadata) and shape-affecting values are static parameters (covered by static parameter metadata).
**Migration**: Read default/range/unit from control-input port metadata and compile-time values from static parameter metadata.
