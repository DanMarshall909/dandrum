## MODIFIED Requirements

### Requirement: YAML patch document

Patch files SHALL be human-readable YAML documents that declare a root graph definition: metadata and instrument identity, static parameters, public input/output ports, preset aliases, module definitions, module instances, and connections. Render settings SHALL NOT appear in patch documents.

#### Scenario: YAML patch is loaded

- **WHEN** the engine loads a patch file with `.yaml` or `.yml` extension
- **THEN** it SHALL parse the file as YAML and validate it against the patch schema before graph construction

#### Scenario: Non-YAML patch is rejected

- **WHEN** the engine is asked to load a patch file whose format is not supported
- **THEN** it SHALL reject the file with an error that identifies the unsupported patch format

#### Scenario: Render settings are rejected

- **WHEN** a patch document declares a `render` section
- **THEN** validation SHALL fail with a diagnostic explaining that sample rate, block size, and duration are host or render-invocation settings

### Requirement: Module instance parameters

YAML module instances SHALL support a `static` mapping supplying static arguments for the referenced definition and a `defaults` mapping overriding control input port defaults. There SHALL be no other per-instance value mechanism.

#### Scenario: Module instance supplies static arguments

- **WHEN** a YAML module declares `static: { channels: 2 }` for a definition with a `channels` static parameter
- **THEN** patch loading SHALL preserve the arguments for compile-time resolution

#### Scenario: Module instance overrides port defaults

- **WHEN** a YAML module declares `defaults: { cutoff_hz: 800 }` for a definition with a `cutoff_hz` control input port
- **THEN** patch loading SHALL preserve the override for validation against the port's declared type and range

#### Scenario: Unknown static or default name rejected

- **WHEN** a `static` or `defaults` entry names something the referenced definition does not declare
- **THEN** validation SHALL fail with a structured diagnostic before graph preparation

### Requirement: Resolved YAML patch preparation

YAML patch loading SHALL resolve port defaults, instance overrides, preset-applied values, and CLI overrides into a deterministic resolved graph — concrete static arguments and effective port defaults for every node — before compilation.

#### Scenario: Resolved patch contains concrete values

- **WHEN** a YAML patch with defaults, overrides, and preset values is prepared for rendering
- **THEN** the prepared graph SHALL contain a concrete validated effective default for every control input port and a resolved value for every static parameter

### Requirement: Public preset surface

Patch YAML SHALL declare the public preset surface as stable named aliases onto root graph ports (for values) and resource static parameters (for assets), preserving value types, defaults, and constraints from the aliased declarations.

#### Scenario: Patch declares preset parameter target

- **WHEN** a YAML patch declares a preset target aliasing a root graph control port
- **THEN** patch loading SHALL preserve the target name and the aliased port's type, default, and constraints

#### Scenario: Patch declares preset asset target

- **WHEN** a YAML patch declares a preset target aliasing a resource static parameter
- **THEN** patch loading SHALL preserve the target name, allowed asset kind, default, and aliased destination

#### Scenario: Duplicate preset targets are rejected

- **WHEN** a YAML patch declares two preset targets with the same target name
- **THEN** validation SHALL fail with a diagnostic identifying the duplicated preset target

#### Scenario: Preset target maps to missing destination

- **WHEN** a YAML patch declares a preset target whose aliased port or static parameter does not exist
- **THEN** validation SHALL fail with a diagnostic identifying the unresolved preset target destination

### Requirement: Script-backed module definitions

Patch YAML SHALL allow a named module definition to select the Rust script implementation while declaring explicit public ports and string static arguments for language and source. Script instances SHALL use the ordinary `type`, `static`, and `defaults` node shape.

#### Scenario: Script definition preserves the unified node shape

- **WHEN** a patch declares a script-backed definition and instantiates it more than once
- **THEN** each instance SHALL use the definition's declared ports with no instance-level `inputs` or `outputs` fields

## ADDED Requirements

### Requirement: Root port declarations

Patch YAML SHALL declare the root graph's public input and output ports — name, signal type, channel count, and defaults for control inputs — as the instrument's external interface. Audio output SHALL be expressed only through root output ports.

#### Scenario: Patch declares named output ports

- **WHEN** a patch declares a 2-channel audio output port `master` mapped from internal module outputs
- **THEN** loading SHALL expose `master` as a bindable root port with two channels

#### Scenario: Patch without root output ports is rejected

- **WHEN** a patch declares no root output ports
- **THEN** validation SHALL fail with a diagnostic explaining the instrument has no observable output

## REMOVED Requirements

### Requirement: Existing patch sections remain canonical

**Reason**: The kernel document shape replaces the legacy patch shape; `render` and `voice_allocation` leave the document (host settings and `poly` nodes respectively), and root `ports` are added.
**Migration**: Move render settings to the host/render invocation; replace `voice_allocation` with a `poly` node wrapping the voice definition; declare root `ports` in place of `audio_output` modules.

### Requirement: Patch-level parameter bindings are compatible

**Reason**: Superseded by root graph ports and the preset-surface aliases onto them; a separate patch-level parameter concept is redundant.
**Migration**: Expose tunable values as root control input ports (or preset-surface aliases onto internal ports promoted to root ports).

### Requirement: Composite parameter declarations in YAML

**Reason**: Composite public parameters are replaced by public control input ports with defaults and range metadata (see `composite-authoring`).
**Migration**: Redeclare each public parameter as a public control input port with `default`, optional `min`/`max`/`unit`, mapped to internal ports.

### Requirement: Composite parameter bindings in YAML

**Reason**: The `${name}` binding syntax is replaced by port mapping (`maps_to`) and static-argument name pass-through (see `static-parameters`).
**Migration**: Bind tunables by mapping public control ports to internal ports; pass shape-affecting values as static arguments.
