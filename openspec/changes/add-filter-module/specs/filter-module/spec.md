## ADDED Requirements

### Requirement: Filter module supports multiple algorithms
The system SHALL include a `filter` built-in module type with a configurable `algorithm` parameter (`biquad`, `moog`, `comb`). The module SHALL expose a single `audio_in` (Audio) input and `audio_out` (Audio) output. Control inputs (`cutoff`, `resonance`, `gain`) SHALL adapt their semantics to the active algorithm. The `mode` parameter is specific to the `biquad` algorithm.

#### Scenario: Filter module with biquad algorithm provides LP/HP/PEQ modes
- **WHEN** a filter module is configured with `algorithm: biquad` and `mode: lowpass`
- **THEN** the filter SHALL apply a low-pass biquad filter
- **WHEN** `mode: highpass`
- **THEN** the filter SHALL apply a high-pass biquad filter
- **WHEN** `mode: peaking`
- **THEN** the filter SHALL apply a peaking EQ filter

#### Scenario: Moog ladder algorithm provides 4-pole resonant low-pass
- **WHEN** a filter module is configured with `algorithm: moog`
- **THEN** the filter SHALL apply a 4-pole low-pass ladder filter

#### Scenario: Comb algorithm provides delay-based filtering
- **WHEN** a filter module is configured with `algorithm: comb` and `comb_type: feedback`
- **THEN** the filter SHALL apply a feedback comb filter
- **WHEN** `comb_type: feedforward`
- **THEN** the filter SHALL apply a feedforward comb filter

### Requirement: Biquad low-pass filter passes frequencies below cutoff
#### Scenario: Low-pass attenuates high frequencies
- **WHEN** a biquad filter configured with `mode: lowpass`, `cutoff` at 0.1, `resonance` at 0.0
- **THEN** the magnitude response at 0.1 normalized frequency SHALL be within 3 dB of unity, and at 0.5 normalized frequency SHALL be at least 12 dB below the passband

### Requirement: Biquad high-pass filter attenuates low frequencies
#### Scenario: High-pass attenuates low frequencies
- **WHEN** a filter configured with `mode: highpass`, `cutoff` at 0.3, `resonance` at 0.0
- **THEN** the magnitude at 0.5 normalized freq SHALL be within 3 dB of unity, and at 0.05 SHALL be at least 12 dB below the passband

### Requirement: Biquad parametric EQ boosts or cuts a frequency band
#### Scenario: PEQ boosts at center frequency
- **WHEN** a filter configured with `mode: peaking`, `cutoff` at 0.2, `resonance` at 0.5, `gain` at 0.75 (≈ +12 dB)
- **THEN** the magnitude at 0.2 normalized freq SHALL be at least 6 dB above unity

#### Scenario: PEQ cuts at center frequency
- **WHEN** a filter configured with `mode: peaking`, `cutoff` at 0.2, `resonance` at 0.5, `gain` at 0.25 (≈ −12 dB)
- **THEN** the magnitude at 0.2 normalized freq SHALL be at least 6 dB below unity

#### Scenario: Q controls PEQ bandwidth
- **WHEN** a peaking EQ with +12 dB has `resonance` at 0.3 (wide) vs 0.8 (narrow)
- **THEN** the narrow Q response SHALL have a narrower −3 dB bandwidth

### Requirement: Moog ladder filter produces resonant 4-pole low-pass
#### Scenario: Moog resonance emphasizes cutoff
- **WHEN** a Moog filter with `cutoff` at 0.15 has `resonance` at 0.8
- **THEN** the magnitude at cutoff SHALL be at least 3 dB higher than with `resonance` at 0.0

#### Scenario: Moog filter has 24 dB/oct rolloff
- **WHEN** a Moog filter with `cutoff` at 0.1 and `resonance` at 0.0 is measured via impulse response
- **THEN** the rolloff above cutoff SHALL be at least 18 dB/octave (4-pole characteristic)

#### Scenario: Moog self-oscillates at high resonance
- **WHEN** a Moog filter with `resonance` at 0.98 and no audio input is processed
- **THEN** the output SHALL contain a sustained oscillation at the cutoff frequency

### Requirement: Comb filter produces periodic notches or peaks
#### Scenario: Feedforward comb produces notches
- **WHEN** a feedforward comb filter with delay 2 ms at 48 kHz (96 samples) and gain 0.7
- **THEN** the magnitude response SHALL show notches at 250 Hz, 500 Hz, 750 Hz, ... (multiples of 1/delay_time)

#### Scenario: Feedback comb produces peaks
- **WHEN** a feedback comb filter with delay 2 ms and gain 0.7
- **THEN** the magnitude response SHALL show peaks at 250 Hz, 500 Hz, 750 Hz, ...

### Requirement: Filter has algorithm-adaptive control ports
The filter module SHALL expose `audio_in` (Audio), `cutoff` (Control), `resonance` (Control), and `gain` (Control) input ports. Port interpretation depends on algorithm.

#### Scenario: Filter ports are registered in the built-in registry
- **WHEN** querying the built-in registry for the `filter` module type
- **THEN** the definition SHALL have input ports `audio_in`, `cutoff`, `resonance`, `gain` and output port `audio_out`

#### Scenario: Cutoff modulation routes through the cutoff port
- **WHEN** an LFO output is connected to a filter's `cutoff` input
- **THEN** graph validation SHALL accept the route as compatible Control-to-Control

### Requirement: Filter algorithms handle edge cases
#### Scenario: No NaN for extreme parameters
- **WHEN** a biquad filter with `cutoff` at 0.0 processes any input
- **THEN** output SHALL not contain NaN values
- **WHEN** a Moog filter with `resonance` at 1.0 processes any input
- **THEN** output SHALL be finite (bounded)
- **WHEN** a comb filter with `gain` at 1.0 processes any input
- **THEN** output SHALL be finite (no unbounded growth)

#### Scenario: Filter state is isolated per voice
- **WHEN** two voice-scoped filter instances process different input signals
- **THEN** their internal state (biquad delay, Moog pole states, comb delay line) SHALL be independent