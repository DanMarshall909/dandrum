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

### Requirement: Script and custom port declarations

The YAML patch format SHALL support script modules with declared input and output ports.

#### Scenario: Script ports are declared in YAML

- **WHEN** a script module declares custom input and output ports in the YAML patch
- **THEN** those ports SHALL be available for connection validation and graph construction

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
