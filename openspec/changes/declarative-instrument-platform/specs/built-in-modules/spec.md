## MODIFIED Requirements

### Requirement: Minimum routing modules

The initial engine SHALL provide built-in modules sufficient to prove event input, sound generation, VCA/control routing, mixing, effects, scripting, delay boundaries, and audio output.

#### Scenario: Core module registry contains MVP modules

- **WHEN** the built-in module registry is initialized
- **THEN** it SHALL include MIDI/event input, audio output, oscillator, noise generator, impulse/click generator, gain/VCA, math/multiply, audio mixer, control mixer, ADSR envelope, LFO, envelope follower, simple filter, note-to-control mapper, one-sample audio delay, block delay, control delay, delay line, and script module types

## ADDED Requirements

### Requirement: Noise generator module

The engine SHALL provide a noise generator module that outputs white noise with configurable seed for reproducible output.

#### Scenario: Noise module in registry

- **WHEN** the built-in module registry is queried for the noise generator
- **THEN** it SHALL be registered as `noise` with audio output port and seed parameter

### Requirement: Impulse/click generator module

The engine SHALL provide an impulse/click generator module triggered by event inputs.

#### Scenario: Impulse module in registry

- **WHEN** the built-in module registry is queried for the impulse generator
- **THEN** it SHALL be registered as `impulse` with event input and audio output ports

### Requirement: Math/multiply module

The engine SHALL provide a math/multiply module that performs sample-wise multiplication of two input signals.

#### Scenario: Multiply module in registry

- **WHEN** the built-in module registry is queried for the multiply module
- **THEN** it SHALL be registered as `multiply` with two input ports and one output port

### Requirement: Note-to-control mapper module

The engine SHALL provide a note-to-control mapper that converts MIDI note events to frequency, pitch CV, and normalized velocity control outputs.

#### Scenario: Note-to-control module in registry

- **WHEN** the built-in module registry is queried for the note-to-control mapper
- **THEN** it SHALL be registered as `note_to_control` with event input and control output ports

### Requirement: Envelope follower module

The engine SHALL provide an envelope follower module that tracks the amplitude envelope of an audio input signal and outputs a control signal.

#### Scenario: Envelope follower in registry

- **WHEN** the built-in module registry is queried for the envelope follower
- **THEN** it SHALL be registered as `envelope_follower` with audio input and control output ports, plus attack and release time parameters

### Requirement: Delay line module

The engine SHALL provide a delay line module with configurable delay time and feedback.

#### Scenario: Delay line in registry

- **WHEN** the built-in module registry is queried for the delay line
- **THEN** it SHALL be registered as `delay_line` with audio input, audio output, delay time parameter, and feedback parameter
