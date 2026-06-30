## ADDED Requirements

### Requirement: Instrument preset identity
Patch YAML SHALL declare a stable instrument ID and preset schema version when it supports external presets.

#### Scenario: Patch declares preset-compatible identity
- **WHEN** a YAML patch declares an instrument ID and preset schema version
- **THEN** patch loading SHALL preserve those values for preset compatibility validation

#### Scenario: Patch without preset identity rejects external preset
- **WHEN** the engine loads a patch with an external preset and the patch does not declare preset-compatible identity
- **THEN** validation SHALL fail with a diagnostic explaining that the patch does not support external presets

### Requirement: Public preset surface
Patch YAML SHALL declare the public preset surface for an instrument as stable named targets with value types, default values, and optional validation constraints.

#### Scenario: Patch declares preset parameter target
- **WHEN** a YAML patch declares a preset target mapped to a public module or composite parameter
- **THEN** patch loading SHALL preserve the target name, value type, default value, constraints, and mapped destination

#### Scenario: Patch declares preset asset target
- **WHEN** a YAML patch declares a preset target mapped to a public asset binding
- **THEN** patch loading SHALL preserve the target name, allowed asset kind, default asset value, and mapped destination

#### Scenario: Duplicate preset targets are rejected
- **WHEN** a YAML patch declares two preset targets with the same target name
- **THEN** validation SHALL fail with a diagnostic identifying the duplicated preset target

#### Scenario: Preset target maps to missing destination
- **WHEN** a YAML patch declares a preset target whose mapped module, composite parameter, or asset binding does not exist
- **THEN** validation SHALL fail with a diagnostic identifying the unresolved preset target destination

### Requirement: Preset surface is explicit
Patch YAML SHALL NOT expose internal module parameters or asset bindings to presets unless they are declared in the public preset surface.

#### Scenario: Internal parameter is not automatically presettable
- **WHEN** a patch contains an internal module parameter that is not declared as a preset target
- **THEN** external preset validation SHALL reject attempts to set that parameter

#### Scenario: Public target hides internal path
- **WHEN** a preset sets a declared public target
- **THEN** diagnostics and preset files SHALL refer to the public target name rather than requiring the internal module path
