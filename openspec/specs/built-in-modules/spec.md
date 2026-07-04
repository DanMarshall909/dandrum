## Purpose

Specify the initial built-in module registry, port declarations, parameter declarations, and delay boundary metadata.

## Requirements

### Requirement: Minimum routing and synthesis modules

The engine SHALL provide built-in modules sufficient to prove event input, pitch/control mapping, sound generation,
control routing, audio/control multiplication, mixing, explicit delay boundaries, effects, scripting, sampling, and
audio output.

#### Scenario: Core module registry contains existing MVP modules

- **WHEN** the built-in module registry is initialized
- **THEN** it SHALL continue to include the existing supported modules for MIDI/event input, audio output, oscillator,
  gain/VCA, audio mixer, control mixer, ADSR envelope, LFO, filter, sampler, note-to-rate, dynamics, saturation,
  convolution, echo, reverb, frequency splitter, spectral processor, explicit delay boundary modules, and script modules
  where supported

#### Scenario: Core module registry contains new minimal primitives

- **WHEN** the built-in module registry is initialized after this change is implemented
- **THEN** it SHALL include `noise`, `impulse`, `multiply`, and `note_to_control` module types with typed ports and
  parameter metadata

### Requirement: Built-in modules declare ports

Every built-in module SHALL declare its named input and output ports with signal types and directions.

#### Scenario: VCA module exposes audio and control ports

- **WHEN** the gain/VCA module type is inspected
- **THEN** it SHALL expose an audio input, audio output, and compatible VCA/control input

### Requirement: Built-in modules declare static parameters

Every built-in module type that accepts static configuration SHALL declare its supported parameters in the Rust module
registry.

#### Scenario: Built-in parameter declarations are registered

- **WHEN** the built-in module registry is initialized
- **THEN** each configurable built-in module definition SHALL expose its static parameter declarations alongside its
  port declarations and delay-boundary metadata

#### Scenario: Built-in declaration supports authoring tools

- **WHEN** a future tool or LLM authoring workflow inspects a built-in module definition
- **THEN** the module definition SHALL expose enough parameter metadata to describe valid YAML parameter values without
  reading module DSP implementation code

### Requirement: Built-in parameter declarations are authoritative

Built-in module parameter declarations SHALL be the authoritative source for validating YAML module instance parameters
and CLI override values targeting built-in modules.

#### Scenario: Unknown built-in parameter is rejected

- **WHEN** a YAML module instance or CLI override provides a parameter not declared by the target built-in module type
- **THEN** validation SHALL fail with a structured diagnostic before graph preparation

### Requirement: Built-in module state uses resolved parameters

Built-in module DSP state construction SHALL consume resolved parameter values prepared before rendering rather than
parsing raw YAML values during processing.

#### Scenario: DSP state is prepared from resolved parameters

- **WHEN** a built-in module instance is prepared for offline or realtime rendering
- **THEN** its DSP state SHALL be constructed from validated resolved parameter values

### Requirement: Delay modules are cycle breakers

Built-in delay modules SHALL declare whether they are valid feedback cycle boundaries and which signal types they apply
to.

#### Scenario: One-sample delay breaks audio cycle

- **WHEN** validation analyzes an audio cycle containing a one-sample audio delay module
- **THEN** the validator SHALL treat that module as a valid audio feedback boundary

#### Scenario: Control delay breaks control cycle

- **WHEN** validation analyzes a control cycle containing a control delay module
- **THEN** the validator SHALL treat that module as a valid control feedback boundary

### Requirement: Noise generator module

The engine SHALL provide a `noise` module that outputs deterministic seeded noise for synthesis and modulation.

#### Scenario: Noise module in registry

- **WHEN** the built-in module registry is queried for `noise`
- **THEN** it SHALL report an audio output port and parameter metadata for seed and noise mode where supported

#### Scenario: Noise module render is reproducible

- **WHEN** two renders use the same seed and render settings
- **THEN** the noise output SHALL be identical

### Requirement: Impulse generator module

The engine SHALL provide an `impulse` module that converts trigger events into a documented transient signal.

#### Scenario: Impulse module in registry

- **WHEN** the built-in module registry is queried for `impulse`
- **THEN** it SHALL report an event input port and audio output port

#### Scenario: Impulse timing follows incoming event frame

- **WHEN** an impulse receives a trigger at a block-relative frame
- **THEN** the impulse output SHALL occur at that frame according to the documented impulse shape

### Requirement: Multiply module

The engine SHALL provide a `multiply` module for multiplying audio and/or control signals.

#### Scenario: Multiply module in registry

- **WHEN** the built-in module registry is queried for `multiply`
- **THEN** it SHALL report two input ports and one output port with documented signal compatibility rules

#### Scenario: Multiply performs deterministic product

- **WHEN** the multiply module receives two compatible signals
- **THEN** its output SHALL be the deterministic product of those signals

### Requirement: Note-to-control module

The engine SHALL provide a `note_to_control` module that converts note events into frequency, pitch ratio/CV,
gate/trigger, and velocity control outputs.

#### Scenario: Note-to-control module in registry

- **WHEN** the built-in module registry is queried for `note_to_control`
- **THEN** it SHALL report an event input port and documented control output ports for frequency, pitch ratio/CV,
  gate/trigger, and normalized velocity

### Requirement: Oscillator waveform support is explicit

The oscillator module SHALL document its supported waveform behaviour through parameter metadata.

#### Scenario: Oscillator waveform queried

- **WHEN** the oscillator module metadata is queried
- **THEN** it SHALL report supported waveform values if waveform selection is implemented

#### Scenario: Unsupported waveform rejected

- **WHEN** a patch requests an unsupported oscillator waveform
- **THEN** validation SHALL reject the patch with a structured diagnostic

### Requirement: Deferred modules are not part of this built-in milestone

The engine SHALL fail validation for unavailable deferred module types rather than accepting them silently. Envelope
follower, general delay line, FM operator, resonator, state-variable filter, wavefolder, sample-and-hold, and specialist
drum voice modules are deferred built-ins for this change. The engine MUST report an unknown or unsupported module
diagnostic for unavailable deferred module types.

#### Scenario: Deferred module appears in a patch

- **WHEN** a patch references a deferred module type that has not been implemented
- **THEN** validation SHALL report an unknown module type or unsupported module diagnostic rather than silently
  accepting it
