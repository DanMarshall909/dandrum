## ADDED Requirements

### Requirement: Module type classification

Every module in the engine SHALL be classified into exactly one of five categories: Rust primitive, YAML composite, script, preset, or out-of-scope.

#### Scenario: New module type request

- **WHEN** a contributor proposes a new module type
- **THEN** the module SHALL be classified using the five-category framework before implementation begins

### Requirement: Primitive gate criteria

A built-in Rust primitive SHALL be added only when it meets ALL of the following criteria:

1. **Performance-critical**: The operation cannot achieve acceptable performance as a YAML composite or script composition of existing primitives
2. **Reusable**: The module is useful across multiple instruments, not a single-purpose convenience wrapper
3. **Realtime-safe internal state**: The module maintains mutable state that must be updated in the audio callback without allocation or locking
4. **Awkward or unsafe as YAML composition**: Expressing the behaviour as a composite of existing modules would be significantly more complex, fragile, or error-prone
5. **Testable DSP behaviour**: The module's input/output behaviour can be precisely specified and tested

#### Scenario: Primitive passes gate

- **WHEN** a proposed primitive satisfies all five criteria
- **THEN** it is a valid candidate for implementation as a built-in Rust module

#### Scenario: Primitive fails gate

- **WHEN** a proposed primitive fails one or more criteria
- **THEN** the proposal SHALL document which criteria are not met and recommend an alternative category (composite, script, or preset)

### Requirement: Composite eligibility

A behaviour SHOULD be implemented as a YAML composite when it can be expressed as a deterministic graph of existing primitives, composites, or script modules without compromising performance or realtime safety.

#### Scenario: Behaviour expressible as composite

- **WHEN** a proposed instrument voice or effect can be assembled from existing primitives with explicit connections
- **THEN** it SHALL be implemented as a YAML composite, not as a new Rust primitive

### Requirement: Script eligibility

A behaviour SHOULD be implemented as a script module when it involves event transformation, control-value mapping, conditional routing, or simple modulation logic that is awkward to express as fixed connections but does not require audio-rate DSP.

#### Scenario: Behaviour suitable for script

- **WHEN** a proposed behaviour transforms MIDI events, maps control values, or implements conditional routing
- **THEN** it MAY be implemented as a script module rather than a new primitive or composite

### Requirement: Preset eligibility

A behaviour SHOULD be expressed as a YAML preset when it is a specific configuration of existing modules and composites with named parameter values, intended as a reusable starting point.

#### Scenario: Usable instrument configuration

- **WHEN** a specific instrument sound or effect configuration is useful as a starting point
- **THEN** it SHALL be expressed as a preset (`.yaml` file in a presets directory) rather than a new module type

### Requirement: Primitive roadmap classification

Module type proposals SHALL be classified using the following outcomes: implement now, defer, avoid, implement as composite, implement as script.

#### Scenario: Primitive roadmap reviewed

- **WHEN** the primitive roadmap is evaluated
- **THEN** each candidate SHALL receive a classification with documented rationale

### Requirement: Noise generator

A noise generator module SHALL be added as a Rust primitive (implement now). It is performance-critical (sample-rate white/pink noise generation), reusable across synthesis and effects, and more efficient than composite alternatives.

#### Scenario: Noise module produces white noise

- **WHEN** the noise module is configured for white noise and processed for one block
- **THEN** each output sample SHALL be an independent random value in the range [-1.0, 1.0] with approximately uniform spectral distribution

#### Scenario: Noise module seed reproducibility

- **WHEN** the noise module is configured with a fixed seed and rendered twice
- **THEN** both renders SHALL produce identical output

### Requirement: Impulse/click generator

An impulse/click generator SHALL be added as a Rust primitive (implement now). It is performance-critical (sample-accurate trigger-to-output timing), reusable across percussion synthesis, and requires realtime-safe trigger state.

#### Scenario: Impulse triggered by event

- **WHEN** the impulse module receives a trigger event
- **THEN** it SHALL output a single sample with value 1.0 followed by zero-valued samples for the remaining block

### Requirement: Math/multiply module

A math/multiply module SHALL be added as a Rust primitive (implement now). It is performance-critical for modulation and ring modulation, reusable across synthesis and effects.

#### Scenario: Multiply two audio signals

- **WHEN** the multiply module receives two audio inputs A and B
- **THEN** its output SHALL be the sample-wise product of A and B

#### Scenario: Multiply control and audio signals

- **WHEN** the multiply module receives a control input and an audio input
- **THEN** its output SHALL be the sample-wise product, with the control signal interpolated to audio rate

### Requirement: Note-to-control mapper

A note-to-control mapper SHALL be added as a Rust primitive (implement now). It converts MIDI note events to control values (frequency, pitch CV, velocity) and is reusable across all voice modules.

#### Scenario: Note maps to frequency

- **WHEN** the note-to-control module receives a MIDI note-on event with note number 69
- **THEN** its frequency output SHALL be 440.0 Hz

#### Scenario: Note maps to velocity

- **WHEN** the note-to-control module receives a MIDI note-on event with velocity 100
- **THEN** its velocity output SHALL be 100.0 / 127.0

### Requirement: Envelope follower

An envelope follower SHALL be added as a Rust primitive (implement now). It tracks amplitude envelopes from audio signals for modulation and ducking, requires realtime-safe state, and cannot be efficiently composed from existing modules.

#### Scenario: Envelope follower tracks amplitude

- **WHEN** the envelope follower receives a steady-state audio signal at 0 dB
- **THEN** its control output SHALL approach 1.0 within its configured attack/release time constants

### Requirement: Delay line (block-length)

A block-length delay line SHALL be added as a Rust primitive (implement now). It provides a configurable delay buffer for effects and feedback, requires realtime-safe state, and is reusable across echo, reverb, and modulation effects.

#### Scenario: Delay line produces delayed output

- **WHEN** the delay line module receives an audio input with delay time set to 44100 samples
- **THEN** the output at sample N SHALL equal the input at sample N - 44100

### Requirement: Candidates deferred or implemented as composites or scripts

The following candidates SHALL NOT be added as Rust primitives in this change:

- **Multi-wave oscillator**: Defer. Requirement still unclear; existing oscillator may suffice. Revisit when a specific use case demands additional wave types.
- **Pitch envelope**: Implement as composite. An ADSR feeding a note-to-control mapper and multiply module achieves the same result.
- **Simpler envelope generator**: Avoid. ADSR covers the use case; adding simpler variants creates confusion.
- **Sample-and-hold**: Implement as composite or script. Can be expressed as a script with state or a composite of existing modules.
- **Constant/control-value source**: Implement as composite. A script or module with fixed parameter output is sufficient.
- **Event filter/router**: Implement as script. Event filtering and routing are natural script use cases.
- **Ring modulation**: Implement as composite. The multiply primitive plus existing modules achieves ring modulation.
- **FM operator**: Defer. Requires careful design for phase modulation and feedback; not needed for current acceptance examples.
- **Resonator**: Defer. Useful but not required for the current platform scope; composite approaches should be explored first.
- **State-variable filter**: Implement as composite of existing filter primitives if multi-mode output is needed.
- **Band-pass filter**: Implement as composite. Existing filter module with band-pass mode selection.
- **Wavefolder**: Defer. Useful but not required for current acceptance examples.
- **Soft clipper**: Implement as composite or reuse saturation module.

#### Scenario: Deferred primitive revisited

- **WHEN** a deferred primitive is proposed for implementation in a later change
- **THEN** the proposal SHALL document which acceptance example requires it and why composite or script approaches are insufficient
