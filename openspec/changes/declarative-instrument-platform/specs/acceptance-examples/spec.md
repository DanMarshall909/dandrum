## ADDED Requirements

### Requirement: Acceptance examples validate platform capability incrementally

The engine SHALL provide YAML acceptance examples that demonstrate useful instruments and effects built from primitives, composites, scripts, presets, and explicit graph routing.

Acceptance examples SHALL avoid special-purpose Rust instrument modules. An 808-style kick, for example, should be a YAML composite or patch built from reusable primitives, not a `kick_808` primitive.

#### Scenario: Each completed example loads and renders

- **WHEN** a completed acceptance example YAML patch is loaded by the engine
- **THEN** loading SHALL succeed without validation errors
- **AND** the patch SHALL render deterministically for fixed render settings and fixed inputs

### Requirement: First proof is synthetic 808-style kick

The first acceptance example SHALL demonstrate a synthetic 808-style kick using only reusable primitives and/or composites.

The example may require:

- oscillator or resonant low-frequency body source
- pitch/control envelope behaviour
- impulse or noise click
- gain/VCA-style amplitude control
- mixer
- audio output

#### Scenario: 808 kick composite renders

- **WHEN** the 808 kick composite receives a trigger event
- **THEN** it SHALL produce a deterministic low-frequency decaying kick-like signal with an initial transient

#### Scenario: 808 kick does not require samples

- **WHEN** the 808 kick example is inspected
- **THEN** it SHALL NOT require a sample asset to produce its core sound

### Requirement: Synthetic snare follows after kick primitives are proven

A later acceptance example SHALL demonstrate a synthetic snare using a tone/body source, noise component, explicit envelope/control routing, VCA-style gain control, and mixer.

#### Scenario: Snare composite renders

- **WHEN** the snare composite receives a trigger event
- **THEN** it SHALL produce a deterministic snare-like signal combining tonal body and noise content

### Requirement: Closed/open hi-hat pair follows after noise/filter/envelope routing is proven

A later acceptance example SHALL demonstrate closed and open hi-hat variants using noise or other supported metallic/noisy sources, explicit filtering, and short/long amplitude contours.

#### Scenario: Closed hi-hat renders shorter than open hi-hat

- **WHEN** closed and open hi-hat examples are rendered with the same trigger timing
- **THEN** the closed variant SHALL decay faster than the open variant according to documented parameters

### Requirement: Subtractive synth voice depends on oscillator capability

A subtractive synth voice example SHALL only use oscillator waveforms that the engine explicitly supports.

#### Scenario: Subtractive synth voice produces note

- **WHEN** the subtractive synth composite receives a note-on event with pitch and velocity
- **THEN** it SHALL produce a deterministic pitched tone shaped by amplitude and filter control routing

#### Scenario: Unsupported waveform is not assumed

- **WHEN** the oscillator module does not support a waveform required by the example
- **THEN** the example SHALL be changed or oscillator waveform support SHALL be implemented and tested first

### Requirement: Sampler voice remains explicit and optional

A sampler voice acceptance example SHALL demonstrate sample playback as an explicit graph feature, not as hidden behaviour inside drum, synth, or preset containers.

#### Scenario: Sampler voice plays sample

- **WHEN** the sampler voice composite receives a note-on or trigger event
- **THEN** it SHALL play the configured sample through explicit pitch, amplitude, and output routing

#### Scenario: Sampler asset is declared

- **WHEN** the sampler voice uses a sample
- **THEN** the sample SHALL be declared through the patch asset system

### Requirement: Effects rack uses existing effect modules

An effects rack example SHALL demonstrate routing through existing effect modules such as filter, echo, reverb, saturation, dynamics, convolution, splitter, mixer, and gain where supported.

#### Scenario: Effects rack processes audio

- **WHEN** an audio signal is sent through the effects rack composite
- **THEN** the rendered output SHALL differ from the dry input according to documented parameters

### Requirement: Script event/control mapping example

A script acceptance example SHALL demonstrate event/control transformation without audio-rate DSP.

#### Scenario: Script maps velocity to control

- **WHEN** a note-on event is processed by a script module that maps velocity to a control output range
- **THEN** the control output SHALL reflect the mapped velocity deterministically

#### Scenario: Script example has no audio output port

- **WHEN** the script module in the example is inspected
- **THEN** it SHALL expose event/control outputs only, not audio-rate outputs

### Requirement: Drum machine event mapper drives explicit voices

A drum-machine acceptance example SHALL demonstrate named pad event outputs triggering explicitly declared downstream voice composites.

#### Scenario: Drum machine triggers connected voice

- **WHEN** a configured incoming note/event reaches the drum machine
- **THEN** the matching pad output SHALL emit an event that triggers the connected voice composite

#### Scenario: Drum machine alone produces no audio

- **WHEN** a drum machine module has no downstream audio-generating modules connected
- **THEN** rendering SHALL produce no audio solely because the drum machine received events

### Requirement: Acceptance examples are staged

Acceptance examples SHALL be added in an order that proves the platform incrementally.

#### Scenario: First example completed before broad library

- **WHEN** the first acceptance example is implemented
- **THEN** it SHALL focus on one synthetic 808-style kick before adding broader drum, synth, sampler, or effects libraries