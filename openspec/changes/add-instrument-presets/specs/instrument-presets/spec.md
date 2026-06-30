## ADDED Requirements

### Requirement: Preset YAML document
The engine SHALL support human-readable YAML preset documents that name a preset, identify the compatible instrument, and provide values for that instrument's declared preset targets.

#### Scenario: Preset document is loaded
- **WHEN** the engine loads a preset file with `.yaml` or `.yml` extension
- **THEN** it SHALL parse the file as YAML and validate it against the preset schema before applying any values

#### Scenario: Non-YAML preset is rejected
- **WHEN** the engine is asked to load a preset file whose format is not supported
- **THEN** it SHALL reject the file with an error that identifies the unsupported preset format

### Requirement: Preset compatibility
Preset documents SHALL declare the instrument ID and preset schema version they target, and the engine SHALL apply a preset only when those values match the loaded patch instrument.

#### Scenario: Compatible preset is accepted
- **WHEN** a preset declares the same instrument ID and preset schema version as the loaded patch
- **THEN** preset validation SHALL accept the compatibility check

#### Scenario: Wrong instrument preset is rejected
- **WHEN** a preset declares an instrument ID that differs from the loaded patch instrument ID
- **THEN** preset validation SHALL fail with a diagnostic identifying the expected and actual instrument IDs

#### Scenario: Wrong preset schema version is rejected
- **WHEN** a preset declares a preset schema version that differs from the loaded patch preset schema version
- **THEN** preset validation SHALL fail with a diagnostic identifying the expected and actual preset schema versions

### Requirement: Preset target validation
Preset values SHALL address only targets declared by the loaded patch's public preset surface.

#### Scenario: Preset sets declared target
- **WHEN** a preset provides a value for a target declared in the patch preset surface
- **THEN** preset validation SHALL accept that target if the value satisfies the declared type and constraints

#### Scenario: Preset sets unknown target
- **WHEN** a preset provides a value for a target not declared in the patch preset surface
- **THEN** preset validation SHALL fail with a diagnostic identifying the unknown preset target

#### Scenario: Preset sets incompatible value
- **WHEN** a preset provides a value whose type or range is incompatible with the declared preset target
- **THEN** preset validation SHALL fail with a diagnostic identifying the target and incompatibility

### Requirement: Preset application
The engine SHALL apply validated preset values before graph construction or composite expansion so the resulting instrument graph is deterministic for a given patch, preset, assets, render settings, and input events.

#### Scenario: Preset value reaches graph construction
- **WHEN** a compatible preset sets a declared public parameter target
- **THEN** graph construction SHALL use the preset value instead of the target's default value

#### Scenario: Missing preset value uses default
- **WHEN** a compatible preset omits a declared preset target
- **THEN** graph construction SHALL use the default value declared by the patch preset surface

#### Scenario: Render with preset is deterministic
- **WHEN** the same patch, preset, assets, render settings, and input events are rendered twice
- **THEN** the audio output SHALL be identical within the engine's defined sample format

### Requirement: Presets cannot change graph structure
Preset documents SHALL NOT be able to add, remove, or modify modules, connections, render settings, event sequences, scheduling behavior, scripts, or feedback boundaries.

#### Scenario: Preset declares graph data
- **WHEN** a preset document contains modules, connections, render settings, event sequences, scripts, or scheduling fields
- **THEN** preset validation SHALL fail with a diagnostic explaining that graph structure belongs in the patch

#### Scenario: Preset cannot bypass routing validation
- **WHEN** a preset is applied to a patch
- **THEN** the resulting graph SHALL still pass the same routing, port compatibility, many-to-one, and feedback-boundary validation required for the base patch

### Requirement: Preset metadata
Preset documents SHALL support optional metadata for display and organization without affecting audio rendering.

#### Scenario: Preset declares metadata
- **WHEN** a preset document declares optional metadata such as author, tags, description, or category
- **THEN** preset loading SHALL preserve that metadata without using it as an audio or graph input

#### Scenario: Metadata changes do not affect render
- **WHEN** two presets differ only in metadata
- **THEN** rendering them with the same patch, assets, render settings, and input events SHALL produce identical audio output
