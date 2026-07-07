## MODIFIED Requirements

### Requirement: Composite references use existing module definition semantics

Composite instances SHALL use the existing `module_definitions` mechanism. A module instance references a composite
either by setting its `type` to an inline composite definition's `type`, or by setting its `type` to a macro-qualified,
version-pinned path to an external module package's entry YAML file (see the `module-library` capability). A `type`
beginning with a `$` macro SHALL be treated as an external module reference; any other `type` SHALL be treated as a
built-in type name or an inline composite type as before.

#### Scenario: Composite module declaration

- **WHEN** a patch declares a module whose `type` matches an inline composite definition
- **THEN** the engine SHALL expand the referenced composite into its constituent internal modules and connections

#### Scenario: External module reference by macro path

- **WHEN** a patch declares a module whose `type` is a macro-qualified pinned path such as `$LIB/1.3.9/drum_voice/drum_voice.yaml`
- **THEN** the engine SHALL load the referenced external module package and expand it identically to an inline composite

#### Scenario: Unsupported composite_id syntax

- **WHEN** a patch uses `type: composite` with `composite_id` before such syntax is explicitly introduced by a migration
  spec
- **THEN** validation SHALL reject or ignore it according to the current schema rather than treating it as canonical
