## Purpose

Specify the YAML patch document shape used to declare instruments, modules, ports, assets, render settings, and
connections.

## Requirements

### Requirement: YAML patch document

Patch files SHALL be human-readable YAML documents that define an instrument's metadata, modules, connections, assets,
and render-relevant settings.

#### Scenario: YAML patch is loaded

- **WHEN** the engine loads a patch file with `.yaml` or `.yml` extension
- **THEN** it SHALL parse the file as YAML and validate it against the patch schema before graph construction

#### Scenario: Non-YAML patch is rejected

- **WHEN** the engine is asked to load a patch file whose format is not supported
- **THEN** it SHALL reject the file with an error that identifies the unsupported patch format

### Requirement: Modules and connections are separate declarations

The patch format SHALL declare modules separately from connections so routing is explicit and inspectable.

#### Scenario: Patch declares modules and connections

- **WHEN** a YAML patch contains `modules` and `connections` sections
- **THEN** the loader SHALL create module definitions first and then resolve connections between named ports

### Requirement: Stable module identifiers

Every module in a patch SHALL have a stable unique identifier used by connections and diagnostics.

#### Scenario: Duplicate module identifiers are rejected

- **WHEN** a YAML patch declares two modules with the same `id`
- **THEN** validation SHALL fail and report the duplicated module identifier

### Requirement: Existing patch sections remain canonical

The YAML patch format SHALL preserve the existing top-level patch shape unless a separate migration spec explicitly
changes it.

The canonical patch sections are:

- `metadata`
- `render`
- `assets`
- `module_definitions`
- `modules`
- `connections`
- `voice_allocation`

#### Scenario: Existing patch remains valid

- **WHEN** a patch valid under the current schema is loaded after this change
- **THEN** it SHALL remain valid unless it relies on behaviour explicitly deprecated by a separate migration spec

### Requirement: Metadata extension is compatible

The existing `metadata` section MAY be extended with optional authoring or validation metadata, but the change SHALL NOT
introduce a second metadata concept.

#### Scenario: Existing metadata parsed

- **WHEN** a patch contains the existing required metadata fields
- **THEN** the engine SHALL parse them with the same semantics as before

#### Scenario: Optional metadata extension parsed

- **WHEN** a patch contains supported optional metadata extension fields
- **THEN** the engine SHALL parse and preserve them for tooling or diagnostics where applicable

### Requirement: Asset validation extension

The existing `assets` section SHALL remain the canonical way to declare external resources. This change MAY add
validation metadata such as expected kind, fallback path, checksum, or authoring hint if those fields are explicitly
specified.

#### Scenario: Asset declared in patch

- **WHEN** a patch declares an asset entry
- **THEN** the engine SHALL validate the asset ID, kind, path, and any supported validation metadata

#### Scenario: Missing asset produces diagnostic

- **WHEN** an asset file referenced by a patch cannot be resolved
- **THEN** loading or preparation SHALL fail with a structured diagnostic containing the asset ID and expected path

### Requirement: Script and custom port declarations

The YAML patch format SHALL support script modules with declared input and output ports.

#### Scenario: Script ports are declared in YAML

- **WHEN** a script module declares custom input and output ports in the YAML patch
- **THEN** those ports SHALL be available for connection validation and graph construction

### Requirement: Module instance parameters

YAML module instances SHALL support a `parameters` mapping that provides static parameter values for the module
instance.

#### Scenario: Module instance declares parameters

- **WHEN** a YAML patch declares a module with `parameters`
- **THEN** patch loading SHALL preserve those values for validation against the module type's parameter declarations
  before graph preparation

### Requirement: Patch-level parameter bindings are compatible

The YAML patch format SHALL allow compatible patch-level `parameters` where implemented, and those parameters SHALL NOT
conflict with existing module-level parameters or composite parameter bindings.

#### Scenario: Patch parameter binds to module parameter

- **WHEN** a patch declares a patch-level parameter that maps to a module parameter or composite parameter
- **THEN** external code SHALL be able to set the patch-level parameter without knowing the internal module ID

#### Scenario: Patch parameter binding target missing

- **WHEN** a patch-level parameter binding references a missing module, composite, or parameter
- **THEN** validation SHALL report a structured diagnostic

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

### Requirement: Composite references use existing module definition semantics

Composite instances SHALL use the existing `module_definitions` mechanism for this change. A module instance references
a composite by setting its `type` to the composite definition's `type`.

#### Scenario: Composite module declaration

- **WHEN** a patch declares a module whose `type` matches an inline composite definition
- **THEN** the engine SHALL expand the referenced composite into its constituent internal modules and connections

#### Scenario: Unsupported composite_id syntax

- **WHEN** a patch uses `type: composite` with `composite_id` before such syntax is explicitly introduced by a migration
  spec
- **THEN** validation SHALL reject or ignore it according to the current schema rather than treating it as canonical

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

### Requirement: Event-routing module YAML

Patch YAML SHALL support readable declarations for generic event-routing primitives, including typed event ports and explicit selector configuration.

#### Scenario: YAML declares event filter

- **WHEN** a YAML patch declares an `event_filter` module with selector configuration
- **THEN** patch loading SHALL preserve the selector for validation and render preparation

#### Scenario: YAML avoids instrument-specific routing containers

- **WHEN** a YAML patch models drum-pad or synth-input routing
- **THEN** it SHALL be able to use generic event-routing modules and explicit connections rather than requiring a `drum_machine`, `drum_pad`, or `poly_synth` module type

### Requirement: Event-routing YAML rejects hidden signal-chain fields

Event-routing modules SHALL reject embedded signal-chain, sample, sequencing, transport, or mixer configuration.

#### Scenario: YAML rejects hidden audio fields

- **WHEN** an event-routing module declares child modules, internal connections, sample assets, audio outputs, or mix outputs
- **THEN** validation SHALL fail with a diagnostic explaining that signal chains must be modeled by external patch modules

#### Scenario: YAML rejects sequencing fields

- **WHEN** an event-routing module declares `pattern`, `patterns`, `steps`, `tempo`, `transport`, or `clock` configuration
- **THEN** validation SHALL fail with a diagnostic explaining that sequencing must be modeled by explicit external modules

### Requirement: Presets are parameter sets, not new graph semantics

The YAML patch format SHALL support presets as named parameter sets when presets are implemented. Presets SHALL NOT add
hidden modules, hidden connections, hidden assets, or hidden realtime behaviour.

#### Scenario: Preset applies parameter values

- **WHEN** a patch references or selects a preset
- **THEN** the preset SHALL apply documented parameter values to existing patch, module, or composite parameters

#### Scenario: Preset cannot hide graph changes

- **WHEN** a preset attempts to add modules, connections, or assets
- **THEN** validation SHALL reject the preset unless a separate spec explicitly defines graph-altering presets

### Requirement: Instrument preset identity

Patch YAML SHALL declare a stable instrument ID and preset schema version when it supports external presets.

#### Scenario: Patch declares preset-compatible identity

- **WHEN** a YAML patch declares an instrument ID and preset schema version
- **THEN** patch loading SHALL preserve those values for preset compatibility validation

#### Scenario: Patch without preset identity rejects external preset

- **WHEN** the engine loads a patch with an external preset and the patch does not declare preset-compatible identity
- **THEN** validation SHALL fail with a diagnostic explaining that the patch does not support external presets

### Requirement: Public preset surface

Patch YAML SHALL declare the public preset surface for an instrument as stable named targets with value types, default
values, and optional validation constraints.

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

- **WHEN** a YAML patch declares a preset target whose mapped module, composite parameter, or asset binding does not
  exist
- **THEN** validation SHALL fail with a diagnostic identifying the unresolved preset target destination

### Requirement: Preset surface is explicit

Patch YAML SHALL NOT expose internal module parameters or asset bindings to presets unless they are declared in the
public preset surface.

#### Scenario: Internal parameter is not automatically presettable

- **WHEN** a patch contains an internal module parameter that is not declared as a preset target
- **THEN** external preset validation SHALL reject attempts to set that parameter

#### Scenario: Public target hides internal path

- **WHEN** a preset sets a declared public target
- **THEN** diagnostics and preset files SHALL refer to the public target name rather than requiring the internal module
  path
