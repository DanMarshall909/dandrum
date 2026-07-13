## MODIFIED Requirements

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

### Requirement: Script-backed definitions declare their interface

An author-defined script processor SHALL be declared as a named graph definition with explicit ports and construction-time language/source static arguments. Script node instances SHALL obtain their interface from that definition and SHALL NOT add ad-hoc instance ports.

#### Scenario: Script definition has connectable declared ports

- **WHEN** YAML declares a script-backed definition with event/control ports and inline source
- **THEN** ordinary nodes referencing that definition SHALL validate and connect through those declared ports

### Requirement: Built-in modules declare static parameters

Every built-in module type that requires compile-time configuration (channel counts, maximum delay length, FFT size, resource references) SHALL declare typed static parameters in the Rust module registry, distinct from its ports.

#### Scenario: Built-in static declarations are registered

- **WHEN** the built-in module registry is initialized
- **THEN** each built-in definition SHALL expose its static parameter declarations alongside its port declarations

#### Scenario: Built-in declaration supports authoring tools

- **WHEN** a future tool or LLM authoring workflow inspects a built-in module definition
- **THEN** the definition SHALL expose enough port and static-parameter metadata to describe valid YAML declarations without reading module DSP implementation code

### Requirement: Built-in parameter declarations are authoritative

Built-in module port and static-parameter declarations SHALL be the authoritative source for validating YAML `defaults` overrides, `static` arguments, and CLI override values targeting built-in modules.

#### Scenario: Unknown built-in override is rejected

- **WHEN** a YAML module instance or CLI override provides a default override or static argument not declared by the target built-in module type
- **THEN** validation SHALL fail with a structured diagnostic before graph preparation

## REMOVED Requirements

### Requirement: Delay modules are cycle breakers

**Reason**: Per-module cycle-breaker metadata is replaced by the explicit `feedback_delay` primitive as the only legal cycle boundary (see `feedback-routing`).
**Migration**: Route feedback cycles through a `feedback_delay` node; ordinary delay effects no longer legalize cycles.
