## Purpose

Specify the YAML root graph document shape for definitions, ports, static arguments, preset aliases, and connections.

## Requirements

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

### Requirement: Metadata extension is compatible

The existing `metadata` section MAY be extended with optional authoring or validation metadata, but the change SHALL NOT
introduce a second metadata concept.

#### Scenario: Existing metadata parsed

- **WHEN** a patch contains the existing required metadata fields
- **THEN** the engine SHALL parse them with the same semantics as before

#### Scenario: Optional metadata extension parsed

- **WHEN** a patch contains supported optional metadata extension fields
- **THEN** the engine SHALL parse and preserve them for tooling or diagnostics where applicable

### Requirement: Script and custom port declarations

The YAML patch format SHALL support named script-backed definitions with declared input and output ports.

#### Scenario: Script ports are declared in YAML

- **WHEN** a script-backed definition declares input and output ports in the YAML patch
- **THEN** those ports SHALL be available for connection validation and graph construction

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

### Requirement: Defined-module references use existing module definition semantics

Defined-module instances SHALL reference either an inline `module_definitions` type or a macro-qualified, version-pinned external package entry YAML file (see `module-library`). Both resolve to ordinary graph definitions before recursive flattening. A `type` beginning with a `$` macro SHALL be treated as an external module reference.

#### Scenario: Inline defined-module declaration

- **WHEN** a patch declares a module whose `type` matches an inline `module_definitions` entry
- **THEN** the engine SHALL resolve that graph definition and flatten its internal nodes and connections

#### Scenario: External module reference by macro path

- **WHEN** a patch declares a module whose `type` is a macro-qualified pinned path such as `$LIB/1.3.9/drum_voice/drum_voice.yaml`
- **THEN** the engine SHALL load the referenced external module package
- **AND** flatten it identically to the same definition authored inline

#### Scenario: Unsupported composite_id syntax

- **WHEN** a patch uses `type: composite` with `composite_id` before such syntax is explicitly introduced by a migration spec
- **THEN** validation SHALL reject or ignore it according to the current schema rather than treating it as canonical

### Requirement: Resolved YAML patch preparation

YAML patch loading SHALL resolve port defaults, instance overrides, preset-applied values, and CLI overrides into a deterministic resolved graph — concrete static arguments and effective port defaults for every node — before compilation.

#### Scenario: Resolved patch contains concrete values

- **WHEN** a YAML patch with defaults, overrides, and preset values is prepared for rendering
- **THEN** the prepared graph SHALL contain a concrete validated effective default for every control input port and a resolved value for every static parameter

### Requirement: Static arguments and defaults are parsed deterministically

YAML `static` arguments and `defaults` overrides SHALL be parsed deterministically according to their declared static or control-port types.

#### Scenario: Unparseable value is rejected

- **WHEN** a YAML static argument or control default cannot be parsed as its declaration's type
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

### Requirement: Presets alter declared values, not graph structure

Presets SHALL apply values only through declared root-port or resource-static aliases. They SHALL NOT add hidden modules, connections, resources, or realtime behavior.

#### Scenario: Preset applies declared aliases

- **WHEN** a patch references or selects a preset
- **THEN** the preset SHALL replace only the effective defaults or resource arguments of its declared aliases

#### Scenario: Preset cannot hide graph changes

- **WHEN** a preset attempts to add modules, connections, or a resource selection outside its declared public asset aliases
- **THEN** validation SHALL reject the structural change or unknown asset target; selecting a declared resource alias SHALL remain valid

### Requirement: Instrument preset identity

Patch YAML SHALL declare a stable instrument ID and preset schema version when it supports external presets.

#### Scenario: Patch declares preset-compatible identity

- **WHEN** a YAML patch declares an instrument ID and preset schema version
- **THEN** patch loading SHALL preserve those values for preset compatibility validation

#### Scenario: Patch without preset identity rejects external preset

- **WHEN** the engine loads a patch with an external preset and the patch does not declare preset-compatible identity
- **THEN** validation SHALL fail with a diagnostic explaining that the patch does not support external presets

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

### Requirement: Preset surface is explicit

Patch YAML SHALL NOT expose internal control ports or resource static arguments to presets unless aliased in the public preset surface.

#### Scenario: Internal port is not automatically presettable

- **WHEN** a patch contains an internal control port that is not aliased as a preset target
- **THEN** external preset validation SHALL reject attempts to set that port

#### Scenario: Public target hides internal path

- **WHEN** a preset sets a declared public target
- **THEN** diagnostics and preset files SHALL refer to the public target name rather than requiring the internal module
  path

### Requirement: Script-backed module definitions

Patch YAML SHALL allow a named module definition to select the Rust script implementation while declaring explicit public ports and string static arguments for language and source. Script instances SHALL use the ordinary `type`, `static`, and `defaults` node shape.

#### Scenario: Script definition preserves the unified node shape

- **WHEN** a patch declares a script-backed definition and instantiates it more than once
- **THEN** each instance SHALL use the definition's declared ports with no instance-level `inputs` or `outputs` fields

### Requirement: Root port declarations

Patch YAML SHALL declare the root graph's public input and output ports — name, signal type, channel count, and defaults for control inputs — as the instrument's external interface. Audio output SHALL be expressed only through root output ports.

#### Scenario: Patch declares named output ports

- **WHEN** a patch declares a 2-channel audio output port `master` mapped from internal module outputs
- **THEN** loading SHALL expose `master` as a bindable root port with two channels

#### Scenario: Patch without root output ports is rejected

- **WHEN** a patch declares no root output ports
- **THEN** validation SHALL fail with a diagnostic explaining the instrument has no observable output

### Requirement: Sample assets are declared beside the root graph

Patch and defined-module YAML SHALL accept an `assets` section containing `sample_sources` and `sample_maps`. A source SHALL declare a stable ID and relative resource path and SHALL accept named regions, simple loop metadata, explicit slices, cues, and optional timing metadata. A map SHALL declare a stable ID and zones with region references, key and velocity ranges, selection metadata, and optional choke groups. Loading SHALL preserve the declarations and each source path's document or package provenance for preparation. The schema SHALL reject unknown sampling fields.

#### Scenario: Patch declares source and map assets

- **WHEN** a valid patch declares a source with a region, loop, slice, and cue plus a map with velocity zones and a choke group
- **THEN** YAML loading SHALL accept and preserve those declarations for preparation

#### Scenario: Packaged source keeps its resource origin

- **WHEN** a packaged defined module declares a relative sample source path
- **THEN** YAML loading SHALL retain the package-version root with that path

#### Scenario: Unknown sampling field fails schema validation

- **WHEN** a source, region, slice, map, or zone declares an unsupported field
- **THEN** patch loading SHALL reject the document before graph construction

#### Scenario: Other sampling families are deferred

- **WHEN** a source or zone declares streaming, granular, time-stretch, keyswitch, or release-trigger settings
- **THEN** patch loading SHALL reject those declarations rather than silently accepting unsupported workstation or creative sampling behaviour

#### Scenario: Empty sample map is rejected

- **WHEN** a sample map declares no zones
- **THEN** schema validation SHALL reject the map before preparation
