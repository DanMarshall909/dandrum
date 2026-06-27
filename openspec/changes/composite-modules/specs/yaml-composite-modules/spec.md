## ADDED Requirements

### Requirement: YAML composite module definitions
The engine SHALL allow patch YAML to declare reusable composite module definitions with a unique type name, explicit public typed input ports, explicit public typed output ports, optional public parameters, optional asset bindings, internal modules, and internal connections.

#### Scenario: Patch declares composite module type
- **WHEN** a patch YAML document declares a composite module definition named `drum_voice`
- **THEN** patch loading exposes `drum_voice` as an instantiable module type for that patch

#### Scenario: Duplicate composite type name is rejected
- **WHEN** a patch YAML document declares two composite module definitions with the same type name
- **THEN** validation fails with a diagnostic identifying the duplicate composite type name

### Requirement: Composite instance public port contract
Composite module instances SHALL expose only the public ports declared by their YAML definition, and graph validation SHALL use those public port names and signal types when validating external routes.

#### Scenario: External route uses public input
- **WHEN** a patch connects an event output to a composite instance public event input
- **THEN** graph validation accepts the route if the public input signal type is compatible

#### Scenario: External route uses public output
- **WHEN** a patch connects a composite instance public audio output to an audio input
- **THEN** graph validation accepts the route if the public output signal type is compatible

#### Scenario: External route reaches hidden internal port
- **WHEN** a patch connects directly to an internal module port inside a composite instance
- **THEN** validation fails with a diagnostic identifying the missing public port

### Requirement: Composite internal graph mapping
Composite definitions SHALL map each public input and output to one or more explicit internal module ports using YAML mappings, and those mappings SHALL be type-compatible.

#### Scenario: Public input maps to internal input
- **WHEN** a composite public control input maps to an internal module control input
- **THEN** validation accepts the mapping

#### Scenario: Public output maps from internal output
- **WHEN** a composite public audio output maps from an internal module audio output
- **THEN** validation accepts the mapping

#### Scenario: Public mapping has incompatible signal type
- **WHEN** a composite public event input maps to an internal control input
- **THEN** validation fails with a diagnostic identifying the composite definition, public port, internal port, and incompatible signal types

### Requirement: Composite expansion
The engine SHALL expand composite instances into deterministic namespaced internal modules and cables before graph processing, preserving the behavior of the declared internal graph.

#### Scenario: Composite instance expands before render
- **WHEN** a patch instantiates a valid composite module
- **THEN** graph construction expands the instance into namespaced internal modules and routes before offline or realtime graph processor construction

#### Scenario: Multiple instances do not collide
- **WHEN** a patch instantiates the same composite definition twice
- **THEN** the expanded internal module IDs are deterministic and distinct for each instance

### Requirement: Composite parameter and asset bindings
Composite definitions SHALL expose public parameters and asset bindings explicitly, and instances SHALL only set declared public bindings.

#### Scenario: Instance sets declared parameter binding
- **WHEN** a composite instance sets a declared public parameter
- **THEN** expansion applies the value to the mapped internal module parameter

#### Scenario: Instance sets undeclared parameter binding
- **WHEN** a composite instance sets a parameter not declared by the composite definition
- **THEN** validation fails with a diagnostic identifying the composite instance and undeclared parameter

#### Scenario: Instance binds declared sample asset
- **WHEN** a composite instance binds a declared public asset binding to a patch sample asset
- **THEN** expansion applies the asset ID to the mapped internal sampler parameter before sampler asset validation

### Requirement: Composite validation preserves graph safety
Composite definitions and expanded composite instances SHALL obey the same graph safety rules as ordinary patches, including explicit mixers for many-to-one routing and explicit delay or future scheduling boundaries for feedback.

#### Scenario: Composite hides implicit many-to-one route
- **WHEN** a composite internal graph connects multiple sources to a non-mixing input
- **THEN** validation fails with the same many-to-one diagnostic used for ordinary graph routes

#### Scenario: Composite hides instantaneous audio feedback
- **WHEN** a composite internal graph contains an audio feedback cycle without an explicit delay boundary
- **THEN** validation fails with the same cycle diagnostic used for ordinary graph routes

### Requirement: Recursive composites are invalid
Composite module definitions SHALL NOT recursively instantiate themselves directly or indirectly.

#### Scenario: Direct recursive composite
- **WHEN** a composite definition instantiates its own type inside its internal modules
- **THEN** validation fails with a diagnostic identifying the recursive composite type

#### Scenario: Indirect recursive composite
- **WHEN** composite definition `a` instantiates `b` and `b` instantiates `a`
- **THEN** validation fails with a diagnostic identifying the recursive composite dependency path
