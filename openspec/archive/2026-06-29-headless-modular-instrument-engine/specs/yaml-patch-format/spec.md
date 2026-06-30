## ADDED Requirements

### Requirement: YAML patch document
Patch files SHALL be human-readable YAML documents that define an instrument's metadata, modules, connections, assets, and render-relevant settings.

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
