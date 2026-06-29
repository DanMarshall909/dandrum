## ADDED Requirements

### Requirement: Frequency splitter splits audio into configurable bands
The system SHALL include a `frequency_splitter` built-in module type that divides a mono audio signal into separate frequency bands using Linkwitz-Riley crossover filters. The splitter SHALL support 2-band (low/high) and 3-band (low/mid/high) configurations via a `bands` parameter.

#### Scenario: 2-band splitter produces low and high outputs
- **WHEN** a `frequency_splitter` is configured with `bands: 2` and `crossover_hz: 1000`
- **THEN** it SHALL expose two audio output ports `low` and `high`

#### Scenario: 3-band splitter produces low, mid, and high outputs
- **WHEN** a `frequency_splitter` is configured with `bands: 3`, `crossover_hz_low: 200`, and `crossover_hz_high: 4000`
- **THEN** it SHALL expose three audio output ports `low`, `mid`, and `high`

#### Scenario: Summed splitter bands reconstruct the original signal
- **WHEN** summing the `low` and `high` outputs of a 2-band splitter driven by any audio signal
- **THEN** the summed output SHALL be within 1 dB RMS of the original signal
- **WHEN** summing all three bands of a 3-band splitter
- **THEN** the summed output SHALL be within 1 dB RMS of the original signal

#### Scenario: Split-and-reconstruct preserves waveform shape
- **WHEN** a simple sinusoidal signal is split through a 2-band crossover and the bands are summed back together
- **THEN** the reconstructed waveform SHALL have a correlation of at least 0.99 with the original input signal

#### Scenario: Crossover transition region has constant power
- **WHEN** measuring the magnitude response of each band of a 2-band splitter at `crossover_hz: 1000`
- **THEN** at 1000 Hz both bands SHALL be at −3 dB relative to their passband, and the sum SHALL be at 0 dB

### Requirement: Frequency splitter has audio_in and crossover frequency control inputs
The module SHALL expose `audio_in` (Audio) and `crossover_hz` (Control) inputs. For 3-band mode, two crossover inputs SHALL be available.

#### Scenario: Crossover modulation routes through control port
- **WHEN** an LFO output is connected to a splitter's `crossover_hz` input
- **THEN** the graph validation SHALL accept the route as compatible Control-to-Control

### Requirement: Frequency splitter handles edge cases
#### Scenario: Crossover at frequency extremes does not produce silence
- **WHEN** a 2-band splitter has `crossover_hz` at 0 (minimum)
- **THEN** the `high` output SHALL pass the full signal and `low` SHALL be silent
- **WHEN** `crossover_hz` at 20000 (maximum)
- **THEN** the `low` output SHALL pass the full signal and `high` SHALL be silent

#### Scenario: Splitter does not produce NaN
- **WHEN** any input signal is processed through a frequency splitter
- **THEN** neither output band SHALL contain NaN values
