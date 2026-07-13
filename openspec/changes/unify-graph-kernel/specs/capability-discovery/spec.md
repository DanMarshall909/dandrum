## MODIFIED Requirements

### Requirement: Module port metadata

The discovery API SHALL return port metadata for each graph definition — primitive, composite, or patch — including port name, direction, signal type, channel count (literal or static-parameter reference), multiplicity, and for control inputs the default value, range, and unit where declared. One metadata schema SHALL describe all definition kinds.

#### Scenario: Query module ports

- **WHEN** a specific definition is queried for ports
- **THEN** the API SHALL return each port's name, direction, signal type, channel count, multiplicity, and any default/range/unit metadata

#### Scenario: Primitive and composite share the schema

- **WHEN** a Rust primitive and a YAML composite are both queried
- **THEN** their port metadata SHALL be returned in the same schema with no kind-specific fields required to interpret it

## ADDED Requirements

### Requirement: Static parameter metadata

The discovery API SHALL return static parameter metadata for each definition: name, type (integer, enumeration, string, resource reference), default where declared, and enumeration values where applicable.

#### Scenario: Query static parameters

- **WHEN** a definition with static parameters is queried
- **THEN** the API SHALL return each static parameter's name, type, default, and allowed values without instantiating the definition

## REMOVED Requirements

### Requirement: Module parameter metadata

**Reason**: Parameters no longer exist as a separate concept; tunables are control input ports with defaults (covered by port metadata) and shape-affecting values are static parameters (covered by static parameter metadata).
**Migration**: Read default/range/unit from control-input port metadata and compile-time values from static parameter metadata.
