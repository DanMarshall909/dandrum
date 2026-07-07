## ADDED Requirements

### Requirement: Wavetable oscillator primitive

The engine SHALL provide a `wavetable_oscillator` primitive that renders audio from prepared wavetable data using deterministic phase accumulation and table lookup.

#### Scenario: Wavetable oscillator is registered

- **WHEN** the built-in module registry is initialized
- **THEN** it SHALL include `wavetable_oscillator`
- **THEN** the module definition SHALL expose typed ports for frequency input, wavetable position input, optional phase reset/sync input where supported, and audio output
- **THEN** the module definition SHALL expose parameter metadata for wavetable selection, interpolation mode, anti-aliasing mode, level, phase reset mode, and initial phase where supported

#### Scenario: Wavetable oscillator renders deterministically

- **WHEN** two renders use the same wavetable asset, render settings, module parameters, and input signals
- **THEN** the rendered audio SHALL be identical

#### Scenario: Wavetable oscillator follows pitch input

- **WHEN** the oscillator receives a constant frequency control signal
- **THEN** its phase increment SHALL correspond to that frequency and the current sample rate

#### Scenario: Wavetable oscillator morphs between frames

- **WHEN** the oscillator receives a wavetable position between two frames
- **THEN** it SHALL interpolate between the neighbouring frames according to the selected interpolation mode
- **THEN** a smooth position sweep SHALL NOT introduce discontinuities beyond those inherent in the source tables

### Requirement: Wavetable assets are prepared before render

The engine SHALL treat wavetable data as prepared assets that are resolved, validated, normalized, and converted into render-ready state before audio rendering begins.

#### Scenario: Valid wavetable asset prepares successfully

- **WHEN** a patch references a supported wavetable asset with valid frame data
- **THEN** preparation SHALL produce a render-ready wavetable model before rendering begins

#### Scenario: Missing wavetable asset is rejected

- **WHEN** a patch references a wavetable asset that cannot be resolved
- **THEN** preparation SHALL fail before rendering with a structured diagnostic containing the asset ID

#### Scenario: Malformed wavetable asset is rejected

- **WHEN** a wavetable asset has zero frames, inconsistent frame sizes, unsupported sample data, or unsupported metadata
- **THEN** preparation SHALL fail before rendering with a structured diagnostic identifying the invalid asset field

#### Scenario: Wavetable rendering performs no steady-state allocation

- **WHEN** a prepared wavetable oscillator renders audio blocks after preparation
- **THEN** it SHALL NOT allocate heap memory during steady-state rendering

### Requirement: Wavetable anti-aliasing strategy is explicit

The engine SHALL make the wavetable oscillator's anti-aliasing strategy explicit through parameter metadata and preparation behaviour.

#### Scenario: Supported anti-aliasing mode is accepted

- **WHEN** a patch requests a supported anti-aliasing mode for a wavetable oscillator
- **THEN** preparation SHALL select or prepare the corresponding render data before rendering begins

#### Scenario: Unsupported anti-aliasing mode is rejected

- **WHEN** a patch requests an unsupported anti-aliasing mode for a wavetable oscillator
- **THEN** validation or preparation SHALL fail before rendering with a structured diagnostic

### Requirement: Stereo pan primitive

The engine SHALL provide or reuse a `stereo_pan` primitive that converts mono audio into deterministic equal-power stereo placement.

#### Scenario: Stereo pan is registered

- **WHEN** the built-in module registry is initialized
- **THEN** it SHALL include `stereo_pan` or an explicitly equivalent existing primitive
- **THEN** the module definition SHALL expose typed audio input, pan control input, and left/right audio outputs

#### Scenario: Stereo pan produces deterministic channels

- **WHEN** the primitive receives the same mono input and pan control twice
- **THEN** it SHALL produce identical left and right outputs for both renders

### Requirement: Slew limiter primitive

The engine SHALL provide or reuse a `slew_limiter` primitive for smoothing control-signal changes with bounded rise and fall rates.

#### Scenario: Slew limiter is registered

- **WHEN** the built-in module registry is initialized
- **THEN** it SHALL include `slew_limiter` or an explicitly equivalent existing primitive
- **THEN** the module definition SHALL expose a control input, control output, and parameter metadata for rise and fall timing

#### Scenario: Slew limiter bounds upward and downward movement

- **WHEN** the input control signal steps from one value to another
- **THEN** the output SHALL approach the target according to the configured rise or fall timing rather than jumping instantly

### Requirement: Sample-and-hold primitive

The engine SHALL provide or reuse a `sample_and_hold` primitive for triggered stepped control modulation.

#### Scenario: Sample-and-hold is registered

- **WHEN** the built-in module registry is initialized
- **THEN** it SHALL include `sample_and_hold` or an explicitly equivalent existing primitive
- **THEN** the module definition SHALL expose typed control input, trigger/event input, and control output ports

#### Scenario: Sample-and-hold updates only on trigger

- **WHEN** the control input changes without a trigger event
- **THEN** the output SHALL keep the previously held value
- **WHEN** a trigger event arrives
- **THEN** the output SHALL update to the input value observed at the trigger frame

### Requirement: Ring modulator primitive

The engine SHALL provide or reuse a `ring_modulator` primitive for audio-rate multiplication of carrier and modulator signals.

#### Scenario: Ring modulator is registered

- **WHEN** the built-in module registry is initialized
- **THEN** it SHALL include `ring_modulator` or an explicitly equivalent existing primitive
- **THEN** the module definition SHALL expose typed carrier audio input, modulator audio input, optional mix control input, and audio output

#### Scenario: Ring modulator multiplies audio inputs

- **WHEN** the primitive receives carrier and modulator audio signals
- **THEN** its wet output SHALL be the deterministic product of the two signals

### Requirement: Phase distortion primitive

The engine SHALL provide or reuse a `phase_distortion` primitive that reshapes phase or audio/control input for sharper digital oscillator movement.

#### Scenario: Phase distortion is registered

- **WHEN** the built-in module registry is initialized
- **THEN** it SHALL include `phase_distortion` or an explicitly equivalent existing primitive
- **THEN** the module definition SHALL expose typed input, amount control, optional symmetry control, and typed output according to its selected mode

#### Scenario: Phase distortion is deterministic

- **WHEN** the primitive receives the same input and modulation controls twice
- **THEN** it SHALL produce identical output for both renders

### Requirement: Chorus primitive

The engine SHALL provide or reuse a `chorus` primitive based on bounded prepared modulated delay for wide synth ensemble tone.

#### Scenario: Chorus is registered

- **WHEN** the built-in module registry is initialized
- **THEN** it SHALL include `chorus` or an explicitly equivalent existing primitive
- **THEN** the module definition SHALL expose typed audio input, modulation controls, mix control, and stereo audio outputs

#### Scenario: Chorus prepares bounded delay memory before render

- **WHEN** a chorus module is prepared
- **THEN** delay memory SHALL be allocated according to the declared maximum delay before rendering begins
- **THEN** steady-state rendering SHALL NOT allocate heap memory

### Requirement: Oscillator sync primitive

The engine SHALL provide or reuse an `oscillator_sync` primitive or explicit oscillator sync input so patches can create hard-sync style phase reset behaviour without a branded synth module.

#### Scenario: Oscillator sync capability is available

- **WHEN** the built-in module registry is initialized
- **THEN** patches SHALL be able to model oscillator sync using either `oscillator_sync` or a documented sync/phase-reset input on a compatible oscillator primitive

#### Scenario: Sync reset changes slave phase deterministically

- **WHEN** a sync trigger occurs at a block-relative frame
- **THEN** the slave oscillator phase or reset signal SHALL update deterministically at that frame

### Requirement: Unison voice primitive is optional and justified

The engine MAY provide a `unison_voice` primitive when explicit graph composition is too noisy or inefficient for practical wide synth patches, but it SHALL NOT replace the general oscillator/filter/envelope/effects graph model.

#### Scenario: Unison support is available to patches

- **WHEN** a patch needs a detuned wide oscillator stack
- **THEN** it SHALL be able to express that stack either as a composite of existing primitives or through a registered `unison_voice` primitive

#### Scenario: Unison voice reports voice controls

- **WHEN** `unison_voice` is registered
- **THEN** it SHALL expose parameter metadata for voice count, detune, spread, level compensation, and phase mode

### Requirement: Product-inspired patches remain product-neutral

Example patches and presets SHALL demonstrate the intended bright, wide, evolving synth sound family without claiming compatibility with or copying proprietary commercial synth presets.

#### Scenario: Example patch uses reusable primitives

- **WHEN** an example patch demonstrates a Nord/Virus-like lead, pad, bass, or pluck
- **THEN** the patch SHALL be built from reusable Dandrum primitives and composites rather than a branded monolithic module

#### Scenario: Preset surface hides graph internals

- **WHEN** a wavetable synth preset exposes controls to the plugin or external preset file
- **THEN** it SHALL expose musical controls such as wavetable, position, detune, spread, filter, envelope, modulation, and effects mix rather than requiring callers to know internal module IDs
