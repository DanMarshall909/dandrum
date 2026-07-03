## ADDED Requirements

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
