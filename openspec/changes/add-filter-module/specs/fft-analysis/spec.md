## ADDED Requirements

### Requirement: FFT analysis computes magnitude response from impulse response
The system SHALL provide an FFT analysis function that takes an impulse response (Vec<f32>) and computes the magnitude spectrum in dB at each frequency bin.

#### Scenario: Impulse response produces flat magnitude spectrum
- **WHEN** computing the FFT magnitude response of a unit impulse (single 1.0 sample followed by zeroes)
- **THEN** the magnitude SHALL be within 0.1 dB of flat (0 dB) across all frequency bins

#### Scenario: Magnitude response returns frequency and magnitude pairs
- **WHEN** computing the FFT magnitude response of a 480-sample impulse at 48000 Hz sample rate
- **THEN** the result SHALL contain a Vec of (frequency_hz, magnitude_db) pairs with frequency values matching FFT bin centers

#### Scenario: FFT handles arbitrary-length impulse responses
- **WHEN** computing the FFT magnitude response of a 1000-sample impulse response
- **THEN** it SHALL be zero-padded to 1024 samples (next power of two) before the FFT

### Requirement: FFT analysis uses windowing
The FFT analysis SHALL apply a Hann window to the impulse response before computing the FFT to reduce spectral leakage.

#### Scenario: Windowed FFT reduces sidelobe amplitude
- **WHEN** comparing the magnitude response of a windowed vs unwindowed analysis of a short impulse
- **THEN** the windowed analysis SHALL show lower magnitude at bins far from the main lobe

### Requirement: Filter frequency response can be verified via FFT
The system SHALL provide a test helper that computes the frequency response of a filter by passing a unit impulse through the filter and analyzing the output with FFT.

#### Scenario: Low-pass filter FFT response matches specification
- **WHEN** computing the FFT magnitude response of a low-pass filter (cutoff=0.1, resonance=0.0) via impulse response analysis
- **THEN** the magnitude at normalized frequency 0.1 SHALL be within 3 dB of the DC magnitude, and the magnitude at 0.4 SHALL be at least 24 dB below the DC magnitude

#### Scenario: High-pass filter FFT response
- **WHEN** computing the FFT magnitude response of a high-pass filter (cutoff=0.3, resonance=0.0)
- **THEN** the magnitude at 0.5 normalized frequency SHALL be within 3 dB of unity, and the magnitude at 0.05 SHALL be at least 12 dB below unity

#### Scenario: Parametric EQ FFT response shows boost
- **WHEN** computing the FFT magnitude response of a peaking EQ (cutoff=0.2, Q=2.0, gain=+12 dB)
- **THEN** the peak magnitude at 0.2 normalized frequency SHALL be at least +9 dB

### Requirement: Crossover frequency response can be verified via FFT
The system SHALL provide a test helper that computes the frequency response of each splitter band via impulse response analysis.

#### Scenario: 2-band crossover low and high passband sum is flat
- **WHEN** computing FFT magnitude responses of both bands of a 2-band splitter with crossover at 1000 Hz
- **THEN** the summed magnitude at the crossover frequency SHALL be within 1 dB of the passband magnitude, and each band SHALL be within 1 dB of −3 dB at the crossover

### Requirement: Spectral processor round-trip can be verified via FFT
The system SHALL provide a test helper that verifies spectral processor passthrough accuracy via FFT comparison.

#### Scenario: Spectral passthrough FFT difference is below threshold
- **WHEN** computing the FFT magnitude response of both the input and output of a spectral passthrough processor
- **THEN** the magnitude difference at each bin SHALL be below −60 dB
