## ADDED Requirements

### Requirement: Generic event-filter primitive

The engine SHALL provide an event-only `event_filter` primitive that accepts an event stream, applies explicit selector configuration, and emits only matching events without changing event timing or owning downstream signal-chain behavior.

#### Scenario: Note selector passes matching event

- **WHEN** an `event_filter` is configured to pass note `36` and receives a note event with note number `36`
- **THEN** it SHALL emit that event at the same render frame offset

#### Scenario: Note selector blocks non-matching event

- **WHEN** an `event_filter` is configured to pass note `36` and receives a note event with note number `38`
- **THEN** it SHALL emit no event for that input

#### Scenario: Event filter is deterministic

- **WHEN** the same `event_filter` patch is rendered twice with the same render settings and input events
- **THEN** both renders SHALL produce identical event outputs and downstream audio buffers

### Requirement: Event-routing primitives are event-only

Event-routing primitives SHALL consume and emit typed events only and SHALL NOT generate audio, emit control signals, own samples, allocate voices, mix audio, schedule patterns, or hide signal-chain behavior.

#### Scenario: Audio route to event router is rejected

- **WHEN** a patch connects an audio output to an event-routing primitive input
- **THEN** graph validation SHALL fail with an incompatible port type diagnostic

#### Scenario: Event router alone produces no audio

- **WHEN** a patch contains event-routing modules but no downstream audio-generating modules
- **THEN** rendering SHALL NOT produce audio solely because event-routing modules received events

### Requirement: Event routing is metadata-discoverable

Event-routing primitives SHALL expose module metadata, typed ports, selector parameters, defaults, allowed selector forms, realtime notes, and short YAML examples through capability discovery.

#### Scenario: Capability discovery describes event filter

- **WHEN** capability discovery lists built-in modules
- **THEN** `event_filter` SHALL include event input/output ports and selector parameter metadata

### Requirement: Drum-machine dogfood uses generic routing

A drum-machine-style example SHALL route note events to explicit kick, snare, and hat voice composites using generic event-routing primitives rather than a `drum_machine` primitive.

#### Scenario: Drum-machine example routes kick note

- **WHEN** the drum-machine dogfood patch receives a kick note event
- **THEN** generic event routing SHALL deliver the event to the explicit kick voice composite

#### Scenario: Drum-machine example has explicit signal chain

- **WHEN** the drum-machine dogfood patch is inspected
- **THEN** voice modules, sample assets, mixers, effects, and audio outputs SHALL be ordinary explicit graph declarations

### Requirement: Simple poly-synth dogfood uses generic routing

A simple polyphonic synth example SHALL use generic event routing, explicit voice allocation, note-to-control conversion, oscillator/filter/envelope/VCA composition, and presets rather than a `poly_synth` primitive.

#### Scenario: Poly-synth example renders note event

- **WHEN** the simple poly-synth dogfood patch receives a note-on event
- **THEN** it SHALL render deterministic pitched audio through explicit synth voice and output routing

#### Scenario: Poly-synth example is not a hardcoded instrument

- **WHEN** the simple poly-synth dogfood patch is inspected
- **THEN** it SHALL be expressed as YAML modules, composites, parameters, presets, and connections without dedicated Rust instrument code
