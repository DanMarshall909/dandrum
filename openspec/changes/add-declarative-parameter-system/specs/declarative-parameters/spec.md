## ADDED Requirements

### Requirement: Parameter categories are distinct
The engine SHALL distinguish module parameters, composite parameters, preset-applied parameter values, and CLI overrides as separate concepts in the parameter system.

#### Scenario: Parameter category is identified
- **WHEN** validation reports a parameter diagnostic
- **THEN** the diagnostic SHALL identify whether the problem concerns a module parameter, composite parameter, preset-applied value, or CLI override when that distinction is applicable

### Requirement: Static scalar parameter declarations
The parameter system SHALL support static scalar parameter declarations with name, type, default value, optional minimum value, optional maximum value, optional enum values, optional unit, optional description, required flag, and static/realtime-preparation metadata.

#### Scenario: Parameter declaration is registered
- **WHEN** a module or composite declares a parameter
- **THEN** the engine SHALL preserve its name, type, default, constraints, unit, description, required flag, and preparation timing metadata for validation and future capability discovery

### Requirement: Supported parameter value types
The parameter system SHALL support number, string, boolean, and enum parameter values for static parameter configuration.

#### Scenario: Enum value is validated as string
- **WHEN** a parameter declaration has type `enum` with an allowed value set
- **THEN** validation SHALL accept only string values present in that allowed value set

### Requirement: Parameter value validation
The engine SHALL validate provided parameter values against their declarations before graph preparation.

#### Scenario: Unknown parameter is rejected
- **WHEN** a module, composite, preset-applied value, or CLI override provides a parameter name that is not declared for the target
- **THEN** validation SHALL fail with a structured diagnostic identifying the unknown parameter

#### Scenario: Wrong parameter type is rejected
- **WHEN** a provided parameter value has a type incompatible with the target declaration
- **THEN** validation SHALL fail with a structured diagnostic identifying the expected type and actual type

#### Scenario: Number below minimum is rejected
- **WHEN** a numeric parameter value is lower than the declaration's minimum value
- **THEN** validation SHALL fail with a structured diagnostic identifying the minimum and actual value

#### Scenario: Number above maximum is rejected
- **WHEN** a numeric parameter value is higher than the declaration's maximum value
- **THEN** validation SHALL fail with a structured diagnostic identifying the maximum and actual value

#### Scenario: Invalid enum value is rejected
- **WHEN** an enum parameter value is not included in the declaration's allowed values
- **THEN** validation SHALL fail with a structured diagnostic identifying the allowed values and actual value

#### Scenario: Missing required parameter is rejected
- **WHEN** a required parameter has no provided value and no default value
- **THEN** validation SHALL fail with a structured diagnostic identifying the missing required parameter

### Requirement: Deterministic default resolution
The resolver SHALL produce a complete resolved parameter map for every prepared module instance by applying declared defaults deterministically where values are omitted.

#### Scenario: Omitted optional value uses default
- **WHEN** a parameter value is omitted and its declaration provides a default
- **THEN** the resolved parameter map SHALL contain that default value

#### Scenario: Defaults are stable across loads
- **WHEN** the same patch is loaded and resolved multiple times
- **THEN** the resolved parameter maps SHALL be identical for equivalent input YAML, presets, assets, render settings, and overrides

### Requirement: Composite public parameters
Composite module definitions SHALL be able to declare public parameters with scalar types, defaults, validation constraints, units, descriptions, and required flags.

#### Scenario: Composite parameter declaration is validated
- **WHEN** a composite definition declares public parameters
- **THEN** validation SHALL verify that each declaration has a supported type, valid default value, and internally consistent constraints

### Requirement: Minimal composite parameter binding
Composite module definitions SHALL bind public parameters to internal module parameters using only literal number, literal string, literal boolean, or direct parameter references of the form `${name}`.

#### Scenario: Direct composite reference resolves
- **WHEN** an internal module parameter is bound to `${tune_hz}` and the composite instance resolves `tune_hz` to `52.0`
- **THEN** the internal module parameter SHALL resolve to `52.0` before graph preparation

#### Scenario: Literal binding resolves
- **WHEN** an internal module parameter is bound to a literal scalar value
- **THEN** the internal module parameter SHALL resolve to that literal value before graph preparation

#### Scenario: Invalid reference is rejected
- **WHEN** a composite binding references a parameter name not declared by the composite
- **THEN** validation SHALL fail with a structured diagnostic identifying the invalid composite parameter reference

#### Scenario: Binding type mismatch is rejected
- **WHEN** a resolved composite binding value is incompatible with the destination module parameter declaration
- **THEN** validation SHALL fail with a structured diagnostic identifying the composite parameter, destination parameter, expected type, and actual type

#### Scenario: Expressions are rejected
- **WHEN** a composite binding contains arithmetic, functions, conditionals, script execution, arbitrary module-state references, or runtime mutation syntax
- **THEN** validation SHALL fail with a structured diagnostic explaining that only literals and direct `${parameter}` references are supported

### Requirement: Preset-applied parameter values
Preset-applied parameter values SHALL override declared defaults only through declared module, composite, or preset-surface targets and SHALL be validated through the same parameter declarations as YAML values.

#### Scenario: Preset-applied value overrides default
- **WHEN** a preset or patch instance provides a valid value for a declared composite parameter
- **THEN** the resolved graph SHALL use that value instead of the composite parameter default

### Requirement: CLI parameter overrides
CLI parameter overrides SHALL act as temporary developer experiment-time replacements addressed by module ID plus parameter name and applied after YAML parsing but before validation and resolved graph preparation.

#### Scenario: CLI override replaces YAML value
- **WHEN** a render command provides `--set kick.tune_hz=48` for a module instance `kick` with a declared `tune_hz` parameter
- **THEN** validation and graph preparation SHALL use `48` for `kick.tune_hz` without mutating the source YAML file

#### Scenario: Repeated CLI override is deterministic
- **WHEN** a render command provides multiple CLI overrides for the same module ID and parameter name
- **THEN** the last override in command-line order SHALL be the value used for validation and resolved graph preparation

#### Scenario: Unknown CLI module is rejected
- **WHEN** a CLI override references a module ID that does not exist in the patch
- **THEN** validation SHALL fail with a structured diagnostic identifying the unknown module ID

#### Scenario: Unknown CLI parameter is rejected
- **WHEN** a CLI override references a parameter not declared for the target module or composite
- **THEN** validation SHALL fail with a structured diagnostic identifying the unknown parameter

### Requirement: Structured parameter diagnostics
Parameter validation SHALL produce structured diagnostics with stable code, severity, YAML path where applicable, module ID where applicable, parameter name where applicable, expected type/range/value where applicable, actual type/value where applicable, message, and suggested fix where safe.

#### Scenario: Diagnostic contains stable fields
- **WHEN** parameter validation fails
- **THEN** each diagnostic SHALL include a stable code, severity, message, and all applicable parameter context fields

### Requirement: Deterministic resolved graph preparation
Parameter declaration, validation, default resolution, composite binding, CLI override application, and graph preparation SHALL be deterministic for equivalent inputs.

#### Scenario: Same resolved patch renders the same output
- **WHEN** the same resolved patch, assets, render settings, and input events are rendered twice
- **THEN** the rendered output SHALL be identical within the engine's defined sample format

### Requirement: Realtime safety boundary
Static parameter resolution SHALL happen before realtime rendering begins.

#### Scenario: Audio callback receives prepared state
- **WHEN** realtime rendering is active
- **THEN** the audio callback SHALL NOT parse YAML, resolve parameter bindings, allocate due to parameter lookup, validate parameters, format parameter diagnostics, or evaluate scripts or expressions for static parameters

### Requirement: Parameter capability discovery foundation
Parameter declarations SHALL be stored in a machine-readable form reusable for future capability discovery by tools and LLM-assisted authoring workflows.

#### Scenario: Declaration exposes discovery metadata
- **WHEN** a future tool or LLM authoring workflow inspects registered module or composite parameter declarations
- **THEN** the declaration model SHALL make parameter names, types, defaults, ranges, enums, units, descriptions, static timing metadata, and example-ready values available without parsing DSP implementation code

#### Scenario: Declaration supports safe authoring guidance
- **WHEN** a future LLM authoring workflow needs to generate or repair a patch
- **THEN** the declaration model SHALL provide enough typed constraints and descriptions to identify valid parameter names and safe value ranges without executing DSP code

### Requirement: LLM-repairable parameter diagnostics
Parameter diagnostics SHALL be structured so future LLM repair loops can identify the invalid YAML location, target parameter, expected constraint, actual value, and safe correction where one is known.

#### Scenario: Diagnostic supports repair loop
- **WHEN** a parameter value fails validation because it is unknown, mistyped, out of range, or outside an enum set
- **THEN** the diagnostic SHALL include stable machine-readable fields sufficient for an automated repair loop to propose a safe YAML edit

### Requirement: Synthetic 808 kick acceptance example
The change SHALL include an acceptance example using a `synthetic_808_kick` composite with public parameters `tune_hz`, `decay_ms`, `punch`, and `click` bound to internal module parameters.

#### Scenario: Kick is tuned from YAML
- **WHEN** a YAML patch instantiates `synthetic_808_kick` and sets `tune_hz`, `decay_ms`, `punch`, and `click`
- **THEN** the resolved graph SHALL use those values without requiring Rust DSP code changes

#### Scenario: Kick is tuned from CLI override
- **WHEN** a render command overrides one of the kick parameters using `--set kick.tune_hz=48`
- **THEN** the resolved graph SHALL use the CLI value for that render without modifying the YAML preset

#### Scenario: Invalid kick value is diagnosed
- **WHEN** the kick patch sets `tune_hz` outside the declared range
- **THEN** validation SHALL fail with a structured range diagnostic before rendering

#### Scenario: Kick parameter changes affect output deterministically
- **WHEN** two renders differ only by a valid `synthetic_808_kick` parameter value
- **THEN** the outputs SHALL differ in a deterministic way that proves the parameter affects prepared module state
