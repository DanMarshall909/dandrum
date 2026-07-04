## ADDED Requirements

### Requirement: Module behaviour classification

Every proposed engine behaviour SHALL be classified before implementation as one of: Rust primitive, YAML composite,
script, preset, future tooling, or out-of-scope.

#### Scenario: New behaviour request

- **WHEN** a contributor proposes a new module type, instrument feature, effect, control behaviour, or authoring feature
- **THEN** the proposal SHALL classify the behaviour before implementation begins
- **AND** the proposal SHALL explain why lower-level engine support is necessary if the behaviour is not expressed as
  YAML, script, preset, or tooling

### Requirement: Primitive gate criteria

A built-in Rust primitive SHALL be evaluated against all five criteria before implementation:

1. **Performance-critical**: The operation requires audio-rate or near-audio-rate processing that cannot achieve
   acceptable performance as YAML composition or script.
2. **Reusable**: The behaviour is useful across multiple instruments, effects, or routing patterns.
3. **Realtime-sensitive state**: The behaviour owns mutable state that must be updated in the realtime path without
   allocation, locking, or blocking.
4. **Awkward or unsafe as YAML composition**: Expressing the behaviour with existing modules would be substantially
   fragile, unclear, inefficient, or unsafe.
5. **Testable DSP/control behaviour**: The input/output behaviour can be specified and tested deterministically.

#### Scenario: Primitive passes gate

- **WHEN** a proposed primitive satisfies all five criteria
- **THEN** it is a valid candidate for implementation as a built-in Rust module

#### Scenario: Primitive requires exception

- **WHEN** a proposed primitive fails one or more criteria but is still considered necessary
- **THEN** the spec SHALL document the failed criteria, rejected alternatives, and concrete acceptance example that
  requires the primitive

#### Scenario: Primitive fails gate without exception

- **WHEN** a proposed primitive fails one or more criteria and has no documented exception
- **THEN** the proposal SHALL recommend an alternative category: YAML composite, script, preset, future tooling, or
  out-of-scope

### Requirement: Composite eligibility

A behaviour SHALL be implemented as a YAML composite when it can be expressed as a deterministic graph of existing
primitives, scripts, and other composites without compromising performance, validation, or realtime safety.

#### Scenario: Instrument voice expressible as composite

- **WHEN** an instrument voice can be assembled from existing primitives with explicit connections
- **THEN** it SHALL be implemented as a YAML composite rather than as a new Rust primitive

### Requirement: Script eligibility

A behaviour SHALL be eligible for a script module when it involves event transformation, control-value mapping,
conditional routing, note remapping, or simple deterministic modulation logic that is awkward to express as fixed
connections but does not require audio-rate DSP.

#### Scenario: Event/control behaviour suitable for script

- **WHEN** a proposed behaviour transforms events, maps control values, or implements conditional routing
- **THEN** it MAY be implemented as a script module rather than a Rust primitive or composite

### Requirement: Preset eligibility

A behaviour SHALL be expressed as a preset when it is a named configuration of existing patches, composites, modules,
and parameter values.

#### Scenario: Specific sound configuration

- **WHEN** a specific sound or effect configuration is useful as a starting point
- **THEN** it SHALL be expressed as a preset rather than a new module type

### Requirement: Future tooling eligibility

A behaviour SHALL be classified as future tooling when it helps humans, GUIs, LLMs, or repair workflows author patches
but is not required for rendering audio.

#### Scenario: LLM authoring support

- **WHEN** a proposed behaviour helps generate, repair, summarize, or explain YAML patches
- **THEN** it SHALL be implemented outside the realtime engine unless it is also needed by loading, validation, or
  rendering

### Requirement: Minimal primitive roadmap

This change SHALL implement or immediately plan only the primitives needed to prove the declarative platform with the
first acceptance examples.

#### Scenario: Required primitive set reviewed

- **WHEN** the primitive roadmap is evaluated for this change
- **THEN** `noise`, `impulse`, `multiply`, and `note_to_control` SHALL be classified as the minimal new primitive
  candidates

### Requirement: Noise generator

A noise generator SHALL be added as a Rust primitive. It is reusable across drum synthesis, subtractive synthesis,
modulation, and effects, and it requires deterministic realtime sample generation.

#### Scenario: Noise module produces deterministic seeded white noise

- **WHEN** the noise module is configured with a fixed seed and rendered twice with identical settings
- **THEN** both renders SHALL produce identical output buffers

#### Scenario: Noise module output range

- **WHEN** the noise module renders white noise
- **THEN** output samples SHALL remain within the documented range

### Requirement: Impulse generator

An impulse/click generator SHALL be added as a Rust primitive. It provides sample-accurate transient generation for
percussion and deterministic trigger timing tests.

#### Scenario: Impulse triggered by event

- **WHEN** the impulse module receives a trigger event at a block-relative frame
- **THEN** it SHALL output a documented impulse shape at that frame and silence elsewhere unless configured otherwise

### Requirement: Multiply module

A multiply module SHALL be added as a Rust primitive. It provides reusable audio/control multiplication for VCA-style
gain control, modulation scaling, tremolo, and ring-mod-style composites.

#### Scenario: Multiply two audio signals

- **WHEN** the multiply module receives two audio inputs
- **THEN** its output SHALL be the sample-wise product of the inputs

#### Scenario: Multiply audio by control

- **WHEN** the multiply module receives an audio input and a control input
- **THEN** its output SHALL be the audio input scaled by the control signal according to the documented control-rate
  behaviour

### Requirement: Note-to-control mapper

A note-to-control mapper SHALL be added as a Rust primitive. It converts note events into reusable control signals for
pitch, frequency, velocity, and gate/trigger behaviour.

#### Scenario: Note maps to frequency

- **WHEN** the note-to-control module receives a note-on event with note number 69
- **THEN** its frequency output SHALL be 440.0 Hz

#### Scenario: Note maps to normalized velocity

- **WHEN** the note-to-control module receives a note-on event with velocity 100
- **THEN** its velocity output SHALL equal 100.0 / 127.0 within the documented tolerance

### Requirement: Oscillator waveform gap is explicit

Acceptance examples SHALL NOT assume oscillator waveforms that the engine does not support.

#### Scenario: Acceptance example requires sine oscillator

- **WHEN** an acceptance example requires sine, saw, pulse, or triangle oscillator output
- **THEN** the implementation SHALL either add minimal waveform support with tests or rewrite the example to use
  supported oscillator behaviour

### Requirement: Deferred candidates

The following candidates SHALL NOT be added as Rust primitives in this change unless a later spec documents a concrete
acceptance example and failed alternatives:

- envelope follower
- general delay line
- FM operator
- resonator
- state-variable filter
- wavefolder
- sample-and-hold
- soft clipper beyond existing saturation
- specialist 808/909 kick, snare, hat, clap, tom, or cymbal modules

#### Scenario: Deferred primitive revisited

- **WHEN** a deferred primitive is proposed later
- **THEN** the proposal SHALL document why existing primitives, composites, scripts, or presets are insufficient
