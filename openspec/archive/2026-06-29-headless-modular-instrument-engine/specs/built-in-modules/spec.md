## ADDED Requirements

### Requirement: Minimum routing modules
The initial engine SHALL provide built-in modules sufficient to prove event input, sound generation, VCA/control routing, mixing, effects, scripting, delay boundaries, and audio output.

#### Scenario: Core module registry contains MVP modules
- **WHEN** the built-in module registry is initialized
- **THEN** it SHALL include MIDI/event input, audio output, oscillator or sample player, gain/VCA, audio mixer, control mixer, ADSR envelope, LFO, simple filter, one-sample audio delay, block delay, control delay, and script module types

### Requirement: Built-in modules declare ports
Every built-in module SHALL declare its named input and output ports with signal types and directions.

#### Scenario: VCA module exposes audio and control ports
- **WHEN** the gain/VCA module type is inspected
- **THEN** it SHALL expose an audio input, audio output, and compatible VCA/control input

### Requirement: Delay modules are cycle breakers
Built-in delay modules SHALL declare whether they are valid feedback cycle boundaries and which signal types they apply to.

#### Scenario: One-sample delay breaks audio cycle
- **WHEN** validation analyzes an audio cycle containing a one-sample audio delay module
- **THEN** the validator SHALL treat that module as a valid audio feedback boundary

#### Scenario: Control delay breaks control cycle
- **WHEN** validation analyzes a control cycle containing a control delay module
- **THEN** the validator SHALL treat that module as a valid control feedback boundary
