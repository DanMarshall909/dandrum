## MODIFIED Requirements

### Requirement: Inline composite module definitions

The YAML patch format SHALL support reusable composite definitions through the top-level `module_definitions` section. A composite definition SHALL be a full graph definition — static parameters, public ports, internal modules, internal connections — exposing the same interface shape as a Rust primitive.

#### Scenario: Composite definition declared inline

- **WHEN** a YAML patch declares a `module_definitions` entry with a `type`, public inputs, public outputs, internal modules, and internal connections
- **THEN** patches SHALL be able to instantiate that composite by declaring a module whose `type` matches the composite definition type

#### Scenario: Composite declares static parameters

- **WHEN** a composite definition declares static parameters used in its port channel counts or internal static arguments
- **THEN** instances SHALL supply static arguments resolved before expansion

### Requirement: Composite parameter exposure

Composite definitions SHALL expose tunable values as public control input ports with default values. Instantiating graphs tune a composite by overriding port defaults or connecting cables to those ports; there SHALL be no separate composite parameter-binding concept.

#### Scenario: Public control port carries a default

- **WHEN** a composite declares a public control input port with a default value mapped to internal ports
- **THEN** an instance with no override and no incoming cable SHALL render using that default

#### Scenario: Instance overrides a public port default

- **WHEN** a module instance overrides a composite's public control port default
- **THEN** expansion SHALL apply the override to the mapped internal ports

#### Scenario: Undeclared override rejected

- **WHEN** a module instance overrides a port the composite definition does not declare
- **THEN** validation SHALL report a structured diagnostic

## ADDED Requirements

### Requirement: Patch and composite are the same definition shape

Any patch document SHALL be usable as a composite definition, and any composite definition with bindable ports SHALL be loadable as a root patch. There SHALL be no structural distinction between the two.

#### Scenario: Patch instantiated as module

- **WHEN** a graph definition instantiates a complete patch document as a node
- **THEN** expansion SHALL treat the patch's public ports as the node's ports with no conversion step

## REMOVED Requirements

### Requirement: Composite asset bindings

**Reason**: Assets become resource-typed static parameters on graph definitions (see `static-parameters`); a dedicated `asset_bindings` mechanism is redundant.
**Migration**: Declare a resource-typed static parameter on the composite and pass the asset reference as a static argument at instantiation.
