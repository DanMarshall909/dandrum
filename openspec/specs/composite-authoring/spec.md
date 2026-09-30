## Purpose

Specify defined modules for reusable instrument and effect graphs.

## Requirements

### Requirement: Inline composite module definitions

The YAML patch format SHALL support reusable defined modules through the top-level `module_definitions` section. A defined module SHALL be a full graph definition — static parameters, public ports, internal modules, internal connections — exposing the same interface shape as a Rust primitive.

#### Scenario: Defined module declared inline

- **WHEN** a YAML patch declares a `module_definitions` entry with a `type`, public inputs, public outputs, internal modules, and internal connections
- **THEN** patches SHALL be able to instantiate that defined module by declaring a module whose `type` matches the defined module type

#### Scenario: Defined module declares static parameters

- **WHEN** a defined module declares static parameters used in its port channel counts or internal static arguments
- **THEN** instances SHALL supply static arguments resolved before expansion

### Requirement: Composite expansion remains deterministic

Defined module expansion SHALL produce an identical flat graph for the same definitions, resolved static arguments, control defaults, and resource origins.

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

Defined module definitions SHALL expose tunable values as public control input ports with default values. Instantiating graphs tune a defined module by overriding port defaults or connecting cables to those ports; there SHALL be no separate parameter-binding concept.

#### Scenario: Public control port carries a default

- **WHEN** a defined module declares a public control input port with a default value mapped to internal ports
- **THEN** an instance with no override and no incoming cable SHALL render using that default

#### Scenario: Instance overrides a public port default

- **WHEN** a module instance overrides a defined module's public control port default
- **THEN** expansion SHALL apply the override to the mapped internal ports

#### Scenario: Undeclared override rejected

- **WHEN** a module instance overrides a port the defined module does not declare
- **THEN** validation SHALL report a structured diagnostic

### Requirement: Composite diagnostics map to source context

Diagnostics produced from an expanded composite graph SHALL include the expanded internal module reference, and SHALL
include the source composite instance/internal path where available.

#### Scenario: Expanded internal port fails validation

- **WHEN** graph validation fails on an expanded internal module port
- **THEN** the diagnostic SHOULD identify the composite instance and internal module path that produced the expanded
  module

### Requirement: External module libraries extend inline module definitions

Inline `module_definitions` and external module packages SHALL both provide graph definitions to the same kernel parser and recursive flattening path. External packages SHALL be referenced by macro-qualified, version-pinned file paths (see the `module-library` capability).

> Note: this requirement lives under the legacy `composite-authoring` capability folder only to preserve OpenSpec continuity while the terminology is being migrated. User-facing and implementation terminology is **module** / **defined module**.

#### Scenario: External module reference absent

- **WHEN** no external module reference is used in a patch
- **THEN** inline `module_definitions` SHALL continue to work unchanged

#### Scenario: External module loaded from a package

- **WHEN** a patch references a module by a macro-qualified pinned path to an external module package
- **THEN** the engine SHALL load that package's graph definition and flatten it
- **AND** it SHALL behave identically to the same definition authored inline

### Requirement: Recursive composites are rejected or depth-limited

Composite recursion SHALL NOT produce infinite expansion.

#### Scenario: Recursive composite detected

- **WHEN** composite definitions recursively reference each other
- **THEN** validation or expansion SHALL fail with a structured diagnostic

#### Scenario: Maximum nesting depth exceeded

- **WHEN** composite nesting exceeds an implementation-defined maximum depth
- **THEN** expansion SHALL fail with a structured diagnostic

### Requirement: Patch and defined module are the same definition shape

Any patch document SHALL be usable as a defined module, and any defined module with bindable ports SHALL be loadable as a root patch. There SHALL be no structural distinction between the two.

#### Scenario: Patch instantiated as module

- **WHEN** a graph definition instantiates a complete patch document as a node
- **THEN** expansion SHALL treat the patch's public ports as the node's ports with no conversion step
