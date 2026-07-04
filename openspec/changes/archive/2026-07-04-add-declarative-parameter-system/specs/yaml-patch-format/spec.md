## ADDED Requirements

### Requirement: Module instance parameters

YAML module instances SHALL support a `parameters` mapping that provides static parameter values for the module
instance.

#### Scenario: Module instance declares parameters

- **WHEN** a YAML patch declares a module with `parameters`
- **THEN** patch loading SHALL preserve those values for validation against the module type's parameter declarations
  before graph preparation

### Requirement: Composite parameter declarations in YAML

YAML composite module definitions SHALL support public parameter declarations with scalar type, default value, optional
minimum value, optional maximum value, optional enum values, optional unit, optional description, and required flag.

#### Scenario: Composite declares public parameters

- **WHEN** a YAML patch declares a composite module definition with public `parameters`
- **THEN** patch loading SHALL preserve the public parameter declarations for composite instance validation and binding
  resolution

### Requirement: Composite parameter bindings in YAML

YAML composite module definitions SHALL support binding internal module parameters to literal scalar values or direct
public parameter references of the form `${name}`.

#### Scenario: Composite binds public parameter to internal module

- **WHEN** a composite internal module parameter is set to `${decay_ms}`
- **THEN** patch loading SHALL preserve that binding so it can resolve to the composite instance's `decay_ms` value
  before graph preparation

#### Scenario: Unsupported binding expression is rejected

- **WHEN** a YAML composite binding contains arithmetic, functions, conditionals, script execution, arbitrary
  module-state references, or runtime mutation syntax
- **THEN** validation SHALL reject the patch before graph preparation

### Requirement: Resolved YAML patch preparation

YAML patch loading SHALL resolve defaults, preset-applied values, CLI overrides, and composite bindings into a
deterministic resolved patch or graph preparation model before rendering.

#### Scenario: Resolved patch contains concrete parameters

- **WHEN** a YAML patch with module parameters and composite parameter bindings is prepared for rendering
- **THEN** the prepared graph SHALL contain concrete validated parameter values for every module instance that requires
  static parameters

### Requirement: Parameter values are parsed deterministically

YAML parameter values SHALL be parsed deterministically according to the declared target parameter type.

#### Scenario: Unparseable value is rejected

- **WHEN** a YAML parameter value cannot be parsed deterministically as the target declaration's type
- **THEN** validation SHALL fail with a structured diagnostic before graph preparation
