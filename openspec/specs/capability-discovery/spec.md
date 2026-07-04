## Purpose

Specify capability discovery through module, composite, script, preset, port, and parameter metadata.

## Requirements

### Requirement: Capability discovery is metadata-driven

The engine SHALL expose capability discovery through module, composite, script, preset, port, and parameter metadata.
Discovery SHALL NOT inspect or mutate realtime render state.

#### Scenario: Discovery uses metadata

- **WHEN** capability discovery is queried
- **THEN** it SHALL return information from registered metadata rather than constructing an audio renderer or running a
  patch

### Requirement: Module type enumeration

The discovery API SHALL enumerate available module types, including built-in Rust primitives, inline/external composites
where loaded, and script module support where available.

#### Scenario: Enumerate module types

- **WHEN** the capability discovery API is queried for module types
- **THEN** it SHALL return a deterministic list of available module type identifiers

### Requirement: Module category classification

The discovery API SHALL report a category for each discoverable item: Rust primitive, YAML composite, script, preset,
future tooling, or out-of-scope where applicable.

#### Scenario: Query module category

- **WHEN** a specific module type is queried for its category
- **THEN** the API SHALL return its classification using the platform decision framework vocabulary

### Requirement: Module port metadata

The discovery API SHALL return port metadata for each module type where known, including port name, direction, signal
type, multiplicity, and any realtime notes relevant to the port.

#### Scenario: Query module ports

- **WHEN** a specific module type is queried for ports
- **THEN** the API SHALL return each port's name, direction, signal type, and supported multiplicity

### Requirement: Module parameter metadata

The discovery API SHALL return parameter metadata for each module type where known, including name, type, default value,
range, unit, enum values, and realtime-safety note.

#### Scenario: Query module parameters

- **WHEN** a specific module type with configurable parameters is queried
- **THEN** the API SHALL return parameter metadata without requiring a patch instance

### Requirement: Composite discovery preserves source model

Composite discovery SHALL understand the existing `module_definitions` model and any later external composite library
model without requiring a separate runtime composite module type.

#### Scenario: Inline composite discovered

- **WHEN** a patch contains an inline composite definition
- **THEN** discovery MAY expose that composite's public ports and parameters from the definition

### Requirement: Discovery supports future tooling but does not implement it

Capability discovery SHALL be suitable for future CLI, GUI, documentation, and LLM-authoring tools, but this change
SHALL NOT implement the LLM authoring layer.

#### Scenario: Discovery returns tool-friendly metadata

- **WHEN** discovery metadata is serialized for tooling
- **THEN** it SHALL contain enough stable identifiers for tools to reference module types, ports, and parameters without
  parsing human-readable documentation

### Requirement: Capability discovery is separate from rendering

Capability discovery SHALL be a separate query interface from the realtime and offline rendering paths.

#### Scenario: Discovery does not affect render

- **WHEN** the capability discovery API is queried during engine operation
- **THEN** the rendering path SHALL NOT be blocked, mutated, or degraded
