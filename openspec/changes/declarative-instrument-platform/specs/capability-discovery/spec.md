## ADDED Requirements

### Requirement: Module type enumeration

The engine SHALL expose an API to enumerate all available module types, including primitives, composites, and script modules registered in the engine.

#### Scenario: Enumerate module types

- **WHEN** the capability discovery API is queried for module types
- **THEN** it SHALL return a list of all registered module type identifiers

### Requirement: Module port metadata

The capability discovery API SHALL return port metadata for each module type, including port name, direction (input/output), and signal type.

#### Scenario: Query module ports

- **WHEN** a specific module type is queried for its ports
- **THEN** the API SHALL return each port's name, direction, and signal type

### Requirement: Module parameter metadata

The capability discovery API SHALL return parameter metadata for each module type that has configurable parameters, including name, type, range, default value, unit, and enum values where applicable.

#### Scenario: Query module parameters

- **WHEN** a specific module type with configurable parameters is queried
- **THEN** the API SHALL return each parameter's name, type, default, range, and unit

### Requirement: Module category classification

The capability discovery API SHALL return the category (primitive, composite, script, or built-in) for each module type.

#### Scenario: Query module category

- **WHEN** a specific module type is queried for its category
- **THEN** the API SHALL return its classification as primitive, composite, script, or built-in

### Requirement: Realtime safety notes

The capability discovery API SHOULD include realtime safety notes for each module type, indicating any constraints or considerations for realtime use.

#### Scenario: Realtime note returned

- **WHEN** a module type has realtime safety considerations
- **THEN** the API SHALL include a human-readable note describing the consideration

### Requirement: Capability discovery is a separate interface

The capability discovery API SHALL be a separate query interface from the rendering engine. It SHALL NOT affect realtime performance or rendering paths.

#### Scenario: Discovery does not affect render

- **WHEN** the capability discovery API is queried during engine operation
- **THEN** the rendering path SHALL NOT be blocked or degraded
