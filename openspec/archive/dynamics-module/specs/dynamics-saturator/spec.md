## ADDED Requirements

### Requirement: Saturator module definition
The engine SHALL provide a built-in `saturator` module with `audio_in` (Audio) input and `audio_out` (Audio) output. The saturator SHALL apply per-sample waveshaping with configurable drive, bias, and curve selection.

#### Scenario: Saturator ports are registered
- **WHEN** querying the built-in registry for the `saturator` module type
- **THEN** the definition SHALL have input ports `audio_in`, `drive`, `bias` and output port `audio_out`, and parameters `curve`

### Requirement: Saturator applies waveshaping curve
The saturator SHALL apply the selected waveshaping curve to each input sample after drive gain and bias offset.

#### Scenario: Tanh curve produces soft clipping
- **WHEN** `curve` is `tanh` and `drive` is 12 dB
- **THEN** the output SHALL be a hyperbolic tangent function of the driven input, symmetric about zero

#### Scenario: Hard clip curve clamps to unity
- **WHEN** `curve` is `hard_clip`
- **THEN** the output SHALL clamp to the [-1, 1] range (before any module output gain)

#### Scenario: Soft clip curve rounds the transition
- **WHEN** `curve` is `soft_clip`
- **THEN** the output SHALL use a polynomial transfer function with a smooth transition into clipping, producing a warmer saturation than hard clip

#### Scenario: SinFold curve produces wavefolding
- **WHEN** `curve` is `sinfold`
- **THEN** the output SHALL be `sin(drive * input)` applied to the biased signal, creating foldover distortion rich in odd harmonics

### Requirement: Drive parameter controls pre-gain
#### Scenario: Unity drive at minimum
- **WHEN** `drive` is 0 dB
- **THEN** the signal SHALL be passed at unity gain before the waveshaper

#### Scenario: High drive increases saturation
- **WHEN** `drive` is 24 dB
- **THEN** the input SHALL be amplified by 24 dB before waveshaping, producing heavier saturation

### Requirement: Bias shifts the transfer curve
#### Scenario: Positive bias creates asymmetric clipping
- **WHEN** `bias` is +0.5
- **THEN** the waveshaping curve SHALL be shifted, producing asymmetric (even-order harmonic) distortion

### Requirement: Extensible WaveshaperCurve trait
The saturator SHALL expose a public `WaveshaperCurve` trait that allows external code to define custom waveshaping functions without modifying the saturator module.

#### Scenario: Custom curve can be registered
- **WHEN** a struct implements `WaveshaperCurve` and is passed to the saturator
- **THEN** the saturator SHALL apply the custom curve's `process()` function to each sample

### Requirement: Saturator handles edge cases
#### Scenario: No NaN for any parameter combination
- **WHEN** a saturator processes any input with any combination of drive, bias, and curve
- **THEN** the output SHALL not contain NaN or infinite values

#### Scenario: Zero drive passes clean signal
- **WHEN** `drive` is 0 dB and `bias` is 0.0
- **THEN** the saturator output SHALL match the input within floating-point precision

### Requirement: Deterministic saturator rendering
#### Scenario: Same saturator patch repeats exactly
- **WHEN** the same saturator patch is rendered twice with the same input signal and parameters
- **THEN** both renders produce identical audio output
