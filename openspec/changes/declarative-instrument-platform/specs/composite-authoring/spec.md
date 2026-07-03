## MODIFIED Requirements

### Requirement: Inline composite module definitions

The YAML patch format SHALL continue to support reusable composite definitions through the existing top-level
`module_definitions` section.

#### Scenario: Composite definition declared inline

- **WHEN** a YAML patch declares a `module_definitions` entry with a `type`, public inputs, public outputs, internal
  modules, and internal connections
- **THEN** patches SHALL be able to instantiate that composite by declaring a module whose `type` matches the composite
  definition type

#### Scenario: Existing composite patches remain valid

- **WHEN** an existing patch uses inline `module_definitions`
- **THEN** this change SHALL NOT require conversion to a separate `type: composite` or `composite_id` syntax

### Requirement: Composite expansion remains deterministic

Composite expansion SHALL produce an identical flat graph for the same patch, composite definitions, parameter values,
and asset bindings.

#### Scenario: Repeated expansion identical

- **WHEN** the same patch is expanded twice
- **THEN** both expansions SHALL produce identical expanded module IDs and connections

### Requirement: Composite module ID prefixing

When a composite instance is expanded, its internal module IDs SHALL be deterministically prefixed or namespaced by the
composite instance ID.

#### Scenario: Expanded module IDs are unique

- **WHEN** two instances of the same composite definition are used in one patch
- **THEN** their expanded internal module IDs SHALL be distinct and deterministic

### Requirement: Composite port mapping

Composite definitions SHALL map public inputs and outputs to internal module ports using the existing `maps_to` and
`maps_from` declarations.

#### Scenario: Public input maps to internal input

- **WHEN** a composite public input maps to one or more internal input ports
- **THEN** connections to the composite public input SHALL expand to connections to the mapped internal ports

#### Scenario: Public output maps from internal output

- **WHEN** a composite public output maps from one or more internal output ports
- **THEN** connections from the composite public output SHALL expand from the mapped internal ports

### Requirement: Composite parameter exposure

Composite definitions MAY expose internal module parameters through the existing `parameters` binding declarations.

#### Scenario: Composite parameter maps to internal parameter

- **WHEN** a patch sets a parameter on a composite instance
- **THEN** the value SHALL be applied only to declared composite parameter bindings

#### Scenario: Undeclared composite parameter rejected

- **WHEN** a patch sets a parameter that is not declared by the composite definition
- **THEN** validation SHALL report a structured diagnostic

### Requirement: Composite asset bindings

Composite definitions MAY expose asset bindings through the existing `asset_bindings` declarations.

#### Scenario: Composite asset binding maps to internal module

- **WHEN** a composite instance sets an asset binding
- **THEN** the binding SHALL resolve to a declared asset and apply to the mapped internal module parameter or asset slot

#### Scenario: Missing composite asset rejected

- **WHEN** a composite instance references an asset ID that does not exist
- **THEN** validation SHALL report a structured diagnostic

### Requirement: Composite diagnostics map to source context

Diagnostics produced from an expanded composite graph SHOULD include both the expanded internal module reference and the
source composite instance/internal path where available.

#### Scenario: Expanded internal port fails validation

- **WHEN** graph validation fails on an expanded internal module port
- **THEN** the diagnostic SHOULD identify the composite instance and internal module path that produced the expanded
  module

### Requirement: External composite libraries are optional future extension

Composite definitions MAY later be loaded from configured external composite directories, but inline
`module_definitions` remain the canonical model for this change.

#### Scenario: External composite support absent

- **WHEN** external composite directories are not configured or not implemented
- **THEN** inline composite definitions SHALL continue to work

#### Scenario: External composite loaded later

- **WHEN** a later implementation loads a composite definition from a configured directory
- **THEN** it SHALL behave identically to an inline `module_definitions` entry after loading

### Requirement: Recursive composites are rejected or depth-limited

Composite recursion SHALL NOT produce infinite expansion.

#### Scenario: Recursive composite detected

- **WHEN** composite definitions recursively reference each other
- **THEN** validation or expansion SHALL fail with a structured diagnostic

#### Scenario: Maximum nesting depth exceeded

- **WHEN** composite nesting exceeds an implementation-defined maximum depth
- **THEN** expansion SHALL fail with a structured diagnostic