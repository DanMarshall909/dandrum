## MODIFIED Requirements

### Requirement: External module libraries extend inline module definitions

Inline `module_definitions` SHALL remain the canonical model for defining YAML-assembled modules. In addition, the engine SHALL support loading a defined module from an external module package referenced by a macro-qualified, version-pinned file path (see the `module-library` capability). A definition loaded from an external package SHALL behave identically to an inline `module_definitions` entry after loading.

> Note: this delta lives under the legacy `composite-authoring` capability folder only to preserve OpenSpec continuity while the terminology is being migrated. User-facing and implementation terminology for this change is **module** / **defined module**.

#### Scenario: External module reference absent

- **WHEN** no external module reference is used in a patch
- **THEN** inline `module_definitions` SHALL continue to work unchanged

#### Scenario: External module loaded from a package

- **WHEN** a patch references a module by a macro-qualified pinned path to an external module package
- **THEN** the engine SHALL load that package's definition and expand it
- **AND** it SHALL behave identically to an inline `module_definitions` entry after loading