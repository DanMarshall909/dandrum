## MODIFIED Requirements

### Requirement: External composite libraries are optional future extension

Composite definitions SHALL continue to support inline `module_definitions` as a canonical model. In addition, the engine
SHALL support loading a module (composite) definition from an external module package referenced by a macro-qualified,
version-pinned file path (see the `module-library` capability). A definition loaded from an external package SHALL behave
identically to an inline `module_definitions` entry after loading.

#### Scenario: External composite support absent

- **WHEN** no external module reference is used in a patch
- **THEN** inline composite definitions SHALL continue to work unchanged

#### Scenario: External composite loaded from a package

- **WHEN** a patch references a module by a macro-qualified pinned path to an external module package
- **THEN** the engine SHALL load that package's definition and expand it
- **AND** it SHALL behave identically to an inline `module_definitions` entry after loading
