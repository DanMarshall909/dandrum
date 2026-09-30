## MODIFIED Requirements

### Requirement: Preset target validation

Preset values SHALL address only targets declared by the loaded patch's public preset surface, where each target aliases a root graph control port (values) or a resource static parameter (assets).

#### Scenario: Preset sets declared target

- **WHEN** a preset provides a value for a target declared in the patch preset surface
- **THEN** preset validation SHALL accept that target if the value satisfies the aliased port's or static parameter's type and constraints

#### Scenario: Preset sets unknown target

- **WHEN** a preset provides a value for a target not declared in the patch preset surface
- **THEN** preset validation SHALL fail with a diagnostic identifying the unknown preset target

#### Scenario: Preset sets incompatible value

- **WHEN** a preset provides a value whose type or range is incompatible with the aliased port or static parameter
- **THEN** preset validation SHALL fail with a diagnostic identifying the target and incompatibility

### Requirement: Preset application

The engine SHALL apply validated preset values before compilation: value targets become the effective defaults of their aliased root ports, and asset targets become the resolved static arguments of their aliased resource parameters, so the compiled instrument is deterministic for a given patch, preset, assets, render settings, and input events.

#### Scenario: Preset value reaches compilation

- **WHEN** a compatible preset sets a declared value target
- **THEN** compilation SHALL use the preset value as the aliased port's effective default instead of the declared default

#### Scenario: Root preset default reaches mapped internal ports

- **WHEN** a preset replaces a root control input's default and that root port maps to one or more internal control inputs
- **THEN** flattening SHALL propagate the preset value to every mapped destination unless an incoming connection takes precedence

#### Scenario: Missing preset value uses default

- **WHEN** a compatible preset omits a declared preset target
- **THEN** compilation SHALL use the default declared by the aliased port or static parameter

#### Scenario: Render with preset is deterministic

- **WHEN** the same patch, preset, assets, render settings, and input events are rendered twice
- **THEN** the audio output SHALL be identical within the engine's defined sample format
