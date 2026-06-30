## ADDED Requirements

### Requirement: Drum-machine event module YAML
Patch YAML SHALL support `drum_machine` module declarations with named pads, trigger selectors, optional pad metadata, and per-pad emitted-event configuration, but without embedded sequencing or signal-chain configuration.

#### Scenario: YAML declares drum-machine pads
- **WHEN** a YAML patch declares a `drum_machine` module with a `pads` collection containing pad IDs, trigger selectors, and emitted-event configuration
- **THEN** patch loading SHALL preserve the pad declarations for validation and graph expansion

#### Scenario: YAML declares pad metadata
- **WHEN** a YAML patch declares optional metadata for a drum-machine pad
- **THEN** patch loading SHALL preserve the metadata for validation without treating it as a sequencer pattern or signal-chain declaration

#### Scenario: YAML rejects drum-machine signal-chain data
- **WHEN** a YAML patch declares child modules, internal connections, sample assets, audio outputs, or mix outputs inside a drum-machine module
- **THEN** validation SHALL fail with a diagnostic explaining that signal chains must be modeled by external patch modules

#### Scenario: YAML rejects drum-machine pattern data
- **WHEN** a YAML patch declares `pattern`, `patterns`, `steps`, `tempo`, `transport`, or `clock` configuration inside a drum-machine module
- **THEN** validation SHALL fail with a diagnostic explaining that sequencing must be modeled by external modules

### Requirement: Drum-machine event port references
Patch YAML SHALL allow connections to reference the drum-machine standard event input and public pad event ports using stable port names.

#### Scenario: YAML connects drum-machine event input
- **WHEN** a YAML patch connects a compatible event source to a drum-machine module's `events` input
- **THEN** patch loading and graph validation SHALL resolve the standard event input

#### Scenario: YAML connects drum-machine pad input
- **WHEN** a YAML patch connects a compatible event source to a declared drum-machine pad input
- **THEN** patch loading and graph validation SHALL resolve the pad public event input

#### Scenario: YAML connects drum-machine pad output
- **WHEN** a YAML patch connects a declared drum-machine pad output to a compatible event input
- **THEN** patch loading and graph validation SHALL resolve the pad public event output

#### Scenario: YAML rejects drum-machine audio port reference
- **WHEN** a YAML patch connects from a drum-machine `mix` or pad audio output
- **THEN** validation SHALL fail with a diagnostic identifying the missing public port
