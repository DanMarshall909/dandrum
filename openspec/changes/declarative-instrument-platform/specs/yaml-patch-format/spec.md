## MODIFIED Requirements

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

### Requirement: Patch-level parameter bindings are compatible

The YAML patch format SHALL allow compatible patch-level `parameters` where implemented, and those parameters SHALL NOT
conflict with existing module-level parameters or composite parameter bindings.

#### Scenario: Patch parameter binds to module parameter

- **WHEN** a patch declares a patch-level parameter that maps to a module parameter or composite parameter
- **THEN** external code SHALL be able to set the patch-level parameter without knowing the internal module ID

#### Scenario: Patch parameter binding target missing

- **WHEN** a patch-level parameter binding references a missing module, composite, or parameter
- **THEN** validation SHALL report a structured diagnostic

### Requirement: Presets are parameter sets, not new graph semantics

The YAML patch format SHALL support presets as named parameter sets when presets are implemented. Presets SHALL NOT add
hidden modules, hidden connections, hidden assets, or hidden realtime behaviour.

#### Scenario: Preset applies parameter values

- **WHEN** a patch references or selects a preset
- **THEN** the preset SHALL apply documented parameter values to existing patch, module, or composite parameters

#### Scenario: Preset cannot hide graph changes

- **WHEN** a preset attempts to add modules, connections, or assets
- **THEN** validation SHALL reject the preset unless a separate spec explicitly defines graph-altering presets

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

### Requirement: Event routing remains event-only

Generic event-routing module declarations SHALL describe event routing only when used for drum-machine-style,
poly-synth, keyboard-split, articulation, and velocity-layer use cases.

#### Scenario: Event routing declared

- **WHEN** a routing module declares a selector for incoming note/event data
- **THEN** the router SHALL emit matching events on explicit event outputs without changing graph topology

#### Scenario: Event routing rejects hidden audio fields

- **WHEN** an event-routing declaration contains embedded sample chains, audio outputs, mix outputs, patterns, transport,
  or sequencing fields
- **THEN** validation SHALL reject those fields with structured diagnostics
