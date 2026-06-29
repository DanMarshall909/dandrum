## ADDED Requirements

### Requirement: Spectral processor applies FFT-based effects
The system SHALL include a `spectral_processor` built-in module type that processes audio through an overlap-add STFT (short-time Fourier transform) pipeline with configurable frame size and hop size, allowing spectral-domain modifications.

#### Scenario: Spectral processor passes audio through with no modification
- **WHEN** a `spectral_processor` is configured with `mode: passthrough`
- **THEN** the output audio SHALL be within −60 dB of the input audio (round-trip transparency)

#### Scenario: Spectral gate suppresses bins below threshold
- **WHEN** a `spectral_processor` is configured with `mode: gate` and `threshold_db: −40`
- **THEN** frequency bins with magnitude below −40 dB SHALL be attenuated by at least 20 dB

#### Scenario: Spectral processor exposes control inputs for parameters
- **WHEN** querying the built-in registry for the `spectral_processor` module type
- **THEN** it SHALL have an `audio_in` (Audio) input and an `audio_out` (Audio) output

### Requirement: Spectral processing handles real signals correctly
#### Scenario: Spectral processor does not produce artifacts in passthrough
- **WHEN** a sinusoidal signal is processed through a `spectral_processor` in `mode: passthrough` with 2048-frame STFT and 50% overlap
- **THEN** the output THD+N SHALL be below −60 dB relative to the input

#### Scenario: Spectral processor handles edge case inputs
- **WHEN** a silent (all-zero) signal is processed through a spectral processor
- **THEN** the output SHALL be all zeros

### Requirement: Implementation approach
The spectral processing SHALL use real FFT, Hann windowing, 50% overlap, and overlap-add resynthesis.

#### Scenario: STFT uses 50% overlap
- **WHEN** the spectral processor is initialized with frame size N
- **THEN** the hop size SHALL be N/2
