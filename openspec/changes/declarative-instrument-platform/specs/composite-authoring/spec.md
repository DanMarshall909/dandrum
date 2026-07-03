## ADDED Requirements

### Requirement: Composite module type

The YAML patch format SHALL support a `composite` module type that references a named composite definition containing its own module and connection declarations.

#### Scenario: Composite declared in patch

- **WHEN** a YAML patch declares a module with `type: composite` and a `composite_id` referencing a composite definition
- **THEN** the engine SHALL expand the composite into its constituent primitive graph during loading

### Requirement: Composite definition format

Composite definitions SHALL be YAML documents containing `modules` and `connections` sections identical in structure to patch files, plus a `ports` section that declares the composite's external interface.

#### Scenario: Composite has external ports

- **WHEN** a composite definition declares an `input` port in its `ports` section
- **THEN** that port SHALL be available for connection in any patch that uses the composite

### Requirement: Composite port mapping

Composite definitions SHALL map their external ports to internal module ports using explicit `connect` entries.

#### Scenario: External port mapped to internal module

- **WHEN** a composite declares an external input port `audio_in`
- **THEN** the composite SHALL contain a connection from `audio_in` to a target internal module's input port

### Requirement: Composite parameter exposure

Composite definitions MAY expose internal module parameters as composite-level parameters using a `parameters` section that maps parameter names through to internal module parameter paths.

#### Scenario: Parameter exposed at composite level

- **WHEN** a composite exposes a parameter `cutoff` that maps to `filter.frequency`
- **THEN** patches using the composite SHALL be able to set `cutoff` without knowledge of the internal module structure

### Requirement: Composite expansion is deterministic

Composite expansion SHALL produce an identical flat graph for the same composite definition and parameter values on every expansion.

#### Scenario: Repeated expansion identical

- **WHEN** the same composite with the same parameter values is expanded twice
- **THEN** both expansions SHALL produce identical flat graphs

### Requirement: Composite nesting

Composites MAY reference other composites, up to a configurable maximum nesting depth.

#### Scenario: Nested composite expansion

- **WHEN** a composite contains another composite as one of its internal modules
- **THEN** the engine SHALL recursively expand all nested composites into the flat primitive graph

#### Scenario: Maximum nesting depth exceeded

- **WHEN** composite nesting exceeds the configured maximum depth (default 16)
- **THEN** expansion SHALL fail with a diagnostic indicating the exceeded nesting depth

### Requirement: Composite module ID prefixing

When a composite is expanded, its internal module IDs SHALL be prefixed with the composite instance ID to ensure uniqueness in the flat graph.

#### Scenario: Expanded module IDs are unique

- **WHEN** two instances of the same composite are used in a patch
- **THEN** their expanded internal module IDs SHALL be distinct

### Requirement: Composite reuse across patches

Composite definitions SHOULD be stored in separate YAML files under a composites directory that the engine searches during loading.

#### Scenario: Composite loaded from composites directory

- **WHEN** a patch references a composite_id that is not defined inline
- **THEN** the engine SHALL search configured composite directories for a matching definition file
