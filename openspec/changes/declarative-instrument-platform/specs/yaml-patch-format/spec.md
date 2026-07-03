## ADDED Requirements

### Requirement: Preset library section

The YAML patch format SHALL support a `presets` section that references preset files by name and path, loading their parameter values into a base patch.

#### Scenario: Patch references preset library

- **WHEN** a YAML patch contains a `presets` section with preset references
- **THEN** the engine SHALL load the referenced preset files and apply their parameter values

### Requirement: Parameter bindings

The YAML patch format SHALL support a `parameters` section at the patch level that can bind named parameters to internal module parameters, enabling external control without direct module access.

#### Scenario: Patch-level parameter binds to module

- **WHEN** a YAML patch declares a `parameters` section with a binding like `cutoff: filter.cutoff`
- **THEN** external code SHALL be able to set `cutoff` at the patch level without knowing the module ID

### Requirement: Asset bindings

The YAML patch format SHALL support an `assets` section that declares external resource dependencies (sample files, impulse responses) with fallback paths and validation metadata.

#### Scenario: Asset declared in patch

- **WHEN** a YAML patch declares an asset entry with a relative file path
- **THEN** the engine SHALL resolve the asset path during loading and report an error if the asset is missing

#### Scenario: Asset missing produces diagnostic

- **WHEN** an asset file referenced in a YAML patch does not exist
- **THEN** loading SHALL fail with a diagnostic containing the asset name and expected path

### Requirement: Validation metadata section

The YAML patch format SHOULD support a `metadata` section that contains validation hints, authoring information, and version data for tooling and diagnostics.

#### Scenario: Metadata section is optional

- **WHEN** a YAML patch contains a `metadata` section with author and version fields
- **THEN** the engine SHALL parse and store the metadata but SHALL NOT require it for loading

### Requirement: Composite reference syntax

The YAML patch format SHALL support a `type: composite` module declaration with a `composite_id` field that references a named composite definition.

#### Scenario: Composite module declaration

- **WHEN** a YAML patch declares a module with `type: composite` and `composite_id: kick_drum`
- **THEN** the engine SHALL expand the referenced composite into its constituent modules during graph expansion

### Requirement: Drum machine event port mapping

The YAML patch format SHALL support a `pad_map` section in drum machine modules that maps incoming events to named pad output ports.

#### Scenario: Drum machine pad map declared

- **WHEN** a drum machine module in a YAML patch declares a `pad_map` mapping MIDI note 36 to pad `kick`
- **THEN** the drum machine SHALL emit pad `kick` events when note 36 is received
