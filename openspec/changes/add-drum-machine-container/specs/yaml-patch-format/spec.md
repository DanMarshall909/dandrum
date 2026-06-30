## ADDED Requirements

### Requirement: Drum-machine pad container YAML
Patch YAML SHALL support `drum_machine` module declarations with named pads, trigger selectors, optional pad metadata, and optional pad child chains, but without embedded sequencing configuration.

#### Scenario: YAML declares drum-machine pads
- **WHEN** a YAML patch declares a `drum_machine` module with a `pads` collection containing pad IDs and trigger selectors
- **THEN** patch loading SHALL preserve the pad declarations for validation and graph expansion

#### Scenario: YAML declares pad metadata
- **WHEN** a YAML patch declares optional metadata for a drum-machine pad
- **THEN** patch loading SHALL preserve the metadata for validation without treating it as a sequencer pattern

#### Scenario: YAML declares pad child chain
- **WHEN** a YAML patch declares child modules and internal connections for a drum-machine pad
- **THEN** patch loading SHALL preserve the pad child chain for validation and graph expansion

#### Scenario: YAML rejects drum-machine pattern data
- **WHEN** a YAML patch declares `pattern`, `patterns`, `steps`, `tempo`, `transport`, or `clock` configuration inside a drum-machine module
- **THEN** validation SHALL fail with a diagnostic explaining that sequencing must be modeled by external modules

### Requirement: Drum-machine event port references
Patch YAML SHALL allow connections to reference drum-machine public event ports using stable container port names derived from declared pads.

#### Scenario: YAML connects drum-machine pad input
- **WHEN** a YAML patch connects a compatible event source to a declared drum-machine pad input
- **THEN** patch loading and graph validation SHALL resolve the pad public event input

#### Scenario: YAML connects drum-machine pad output
- **WHEN** a YAML patch connects a declared drum-machine pad output to a compatible event input
- **THEN** patch loading and graph validation SHALL resolve the pad public event output

#### Scenario: YAML rejects drum-machine audio port reference
- **WHEN** a YAML patch connects from a drum-machine `mix` or pad audio output that was not explicitly declared by ordinary child-chain routing
- **THEN** validation SHALL fail with a diagnostic identifying the missing public port
