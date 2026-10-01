## Purpose

Specify the built-in module registry, typed ports, static parameters, and prepared DSP state.

## Requirements

### Requirement: Minimum routing and synthesis modules

The engine SHALL provide built-in modules sufficient to prove event input, pitch/control mapping, sound generation, control routing, audio/control multiplication, mixing, explicit feedback boundaries, polyphony, effects, scripting, and sampling. Audio output SHALL be expressed through root graph ports rather than an output module.

#### Scenario: Core module registry contains kernel modules

- **WHEN** the built-in module registry is initialized after this change is implemented
- **THEN** it SHALL include the existing supported modules for MIDI/event input, oscillator, gain/VCA, audio mixer, control mixer, ADSR envelope, LFO, filter, sampler, note-to-rate, dynamics, saturation, convolution, echo, reverb, frequency splitter, spectral processor, noise, impulse, multiply, note-to-control, and script modules where supported, plus the `poly` and `feedback_delay` structural primitives

#### Scenario: audio_output type is rejected

- **WHEN** a patch declares a module of type `audio_output`
- **THEN** validation SHALL fail with a diagnostic directing the author to root graph output ports

### Requirement: Built-in modules declare ports

Every built-in module SHALL declare its named input and output ports with signal type, direction, and channel count. Control input ports SHALL declare default values and range metadata where meaningful; every tunable SHALL be a control input port rather than a separate parameter.

#### Scenario: VCA module exposes audio and control ports

- **WHEN** the gain/VCA module type is inspected
- **THEN** it SHALL expose an audio input, audio output, and a compatible control input with a declared default value

#### Scenario: Tunables are ports

- **WHEN** any built-in module's tunable value (e.g. filter cutoff, echo feedback) is inspected
- **THEN** it SHALL be declared as a control input port with a default rather than as a non-connectable parameter

#### Scenario: Generic builtin resolves arbitrary channel count

- **WHEN** a channel-independent builtin such as gain or mixer is instantiated with six channels
- **THEN** preparation SHALL allocate and process six channel buffers through the same logical ports

#### Scenario: Intrinsically stereo builtin rejects unsupported width

- **WHEN** echo or reverb is instantiated with a channel count greater than two
- **THEN** static resolution SHALL fail with a diagnostic listing the supported mono and stereo channel counts

### Requirement: Built-in modules declare static parameters

Every built-in module type that requires compile-time configuration (channel counts, maximum delay length, FFT size, resource references) SHALL declare typed static parameters in the Rust module registry, distinct from its ports.

#### Scenario: Built-in static declarations are registered

- **WHEN** the built-in module registry is initialized
- **THEN** each built-in definition SHALL expose its static parameter declarations alongside its port declarations

#### Scenario: Built-in declaration supports authoring tools

- **WHEN** a future tool or LLM authoring workflow inspects a built-in module definition
- **THEN** the definition SHALL expose enough port and static-parameter metadata to describe valid YAML declarations without reading module DSP implementation code

#### Scenario: Resource-consuming builtins declare resource kind

- **WHEN** sampler and convolution definitions are inspected
- **THEN** sampler SHALL require a `sample` resource static argument and convolution SHALL require an `impulse_response` resource static argument

### Requirement: Built-in parameter declarations are authoritative

Built-in module port and static-parameter declarations SHALL be the authoritative source for validating YAML `defaults` overrides, `static` arguments, and CLI override values targeting built-in modules.

#### Scenario: Unknown built-in override is rejected

- **WHEN** a YAML module instance or CLI override provides a default override or static argument not declared by the target built-in module type
- **THEN** validation SHALL fail with a structured diagnostic before graph preparation

### Requirement: Built-in module state uses resolved construction values

Built-in DSP state construction SHALL consume resolved typed static arguments, resource handles, and effective control defaults prepared before rendering rather than parsing raw YAML values during processing.

#### Scenario: DSP state is prepared from resolved values

- **WHEN** a built-in module instance is prepared for offline or realtime rendering
- **THEN** its DSP state SHALL be constructed from validated typed arguments and control defaults

### Requirement: Noise generator module

The engine SHALL provide a `noise` module that outputs deterministic seeded noise for synthesis and modulation.

#### Scenario: Noise module in registry

- **WHEN** the built-in module registry is queried for `noise`
- **THEN** it SHALL report an audio output port and typed static metadata for its seed

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

The oscillator module SHALL document its supported waveform behaviour through typed static-parameter metadata.

#### Scenario: Oscillator waveform queried

- **WHEN** the oscillator module metadata is queried
- **THEN** it SHALL report the allowed waveform enum values

#### Scenario: Unsupported waveform rejected

- **WHEN** a patch requests an unsupported oscillator waveform
- **THEN** validation SHALL reject the patch with a structured diagnostic

### Requirement: Script-backed definitions declare their interface

An author-defined script processor SHALL be declared as a named graph definition marked `implementation: script`, with explicit ports and construction-time language/source static arguments. Script node instances SHALL obtain their interface from that definition and SHALL NOT add ad-hoc instance ports.

#### Scenario: Script definition has connectable declared ports

- **WHEN** YAML declares a script-backed definition with event/control ports and inline source
- **THEN** ordinary nodes referencing that definition SHALL validate and connect through those declared ports

### Requirement: Advanced sampling primitives declare connectable registry surfaces

The built-in registry SHALL expose `sample_player`, `sample_zone_selector`, `sample_slicer`, and a thin `sample_map_player` that composes selection, playback, and bounded voice/choke behavior. Source, region, slice-table, and sample-map identities SHALL be typed static arguments. Performance values SHALL be control input ports and triggers, gates, and selected-zone messages SHALL be event ports. Audio outputs SHALL declare their channel count. An existing `sampler` patch SHALL keep its current registry contract while migration to the shared playback implementation proceeds.

#### Scenario: Region player surface is discoverable

- **WHEN** an authoring tool inspects `sample_player`
- **THEN** it SHALL find source, region, mode, interpolation, and channel static arguments; trigger and gate event inputs; pitch ratio, start offset, level, and pan control inputs; and a typed audio output

#### Scenario: Map selection surface is discoverable

- **WHEN** an authoring tool inspects `sample_zone_selector`
- **THEN** it SHALL find a sample-map static argument, note event input, variation control input, and selected-zone event output

#### Scenario: Slice player surface is discoverable

- **WHEN** an authoring tool inspects `sample_slicer`
- **THEN** it SHALL find source and slice-table static arguments, a trigger event input, slice index, pitch ratio, and level control inputs, and a typed audio output

#### Scenario: Map player declares bounded voice and choke options

- **WHEN** an authoring tool inspects `sample_map_player`
- **THEN** it SHALL find a sample-map static argument, bounded voice-count and voice-stealing arguments, a declared choke mode, a note event input, live playback control inputs, and a typed audio output
