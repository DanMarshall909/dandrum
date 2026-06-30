## ADDED Requirements

### Requirement: Dynamics Processor module definition
The engine SHALL provide a built-in `dynamics-processor` module with an `audio_in` (Audio) input, optional `sidechain_in` (Control) input, and `audio_out` (Audio) output. When no sidechain is connected, the processor SHALL derive its envelope from `audio_in`.

The processor SHALL operate in one of two modes, selected by the `mode` parameter:
- **`level` mode**: threshold-based gain computer with two independently configurable slopes (`below_ratio`, `above_ratio`)
- **`transient` mode**: direction-based gain computer that applies `attack_gain` and `sustain_gain` based on envelope direction

#### Scenario: Level mode ports are registered
- **WHEN** querying the built-in registry for the `dynamics-processor` module type with `mode` set to `level`
- **THEN** the definition SHALL have input ports `audio_in`, `sidechain_in`, `threshold`, `below_ratio`, `above_ratio`, `attack`, `release`, `knee`, `makeup_gain`, `detection`, `topology` and output port `audio_out`

#### Scenario: Transient mode ports are registered
- **WHEN** querying the built-in registry for the `dynamics-processor` module type with `mode` set to `transient`
- **THEN** the definition SHALL have input ports `audio_in`, `sidechain_in`, `attack_gain`, `sustain_gain`, `attack`, `release`, `detection` and output port `audio_out`

#### Scenario: Sidechain falls back to main input
- **WHEN** a dynamics-processor module has no cable connected to `sidechain_in`
- **THEN** the envelope detector SHALL use `audio_in` as the sidechain source

### Requirement: Level mode gain computer with two independent slopes
In `level` mode, the gain computer SHALL compute gain reduction from the envelope using a two-slope transfer function:

```
gain_db(envelope_db) =
    if envelope_db > threshold:
        (envelope_db - threshold) * (1 - 1/above_ratio)
    else:
        (envelope_db - threshold) * (1 - 1/below_ratio)
```

Gain reduction SHALL then be `db_to_linear(gain_db)` applied to the audio signal, with makeup gain added to the result.

#### Scenario: Signal below threshold passes at unity (compressor behavior)
- **WHEN** `below_ratio` is 1:1, `above_ratio` is 4:1, and the sidechain envelope is below `threshold`
- **THEN** the output SHALL be at unity gain (no gain reduction, no expansion)

#### Scenario: Signal above threshold is attenuated by above_ratio (compressor behavior)
- **WHEN** `above_ratio` is 4:1 and the sidechain envelope exceeds `threshold` by 10 dB
- **THEN** the output level SHALL be approximately 2.5 dB above threshold (10 dB input excess reduced to 10/4 = 2.5 dB)

#### Scenario: Signal above threshold is brickwall limited (limiter behavior)
- **WHEN** `above_ratio` is 40:1 and the sidechain envelope exceeds `threshold` by 10 dB
- **THEN** the output level SHALL be approximately 0.25 dB above threshold (near-brickwall limiting)

#### Scenario: Signal below threshold is gated (gate behavior)
- **WHEN** `below_ratio` is 0:1 and the sidechain envelope is 10 dB below `threshold`
- **THEN** the output SHALL be attenuated such that the residual is inaudible (≥60 dB reduction below threshold), with exponential fade following the release time constant

#### Scenario: Signal below threshold is expanded (expander behavior)
- **WHEN** `below_ratio` is 0.5:1 and the sidechain envelope is 10 dB below `threshold`
- **THEN** the output level SHALL be approximately 10 dB below the threshold-scaled envelope (10 dB input deficit expanded to 10/0.5 = 20 dB deficit)

#### Scenario: Signal above threshold is boosted (upward compressor behavior)
- **WHEN** `above_ratio` is 0.5:1 and the sidechain envelope exceeds `threshold` by 10 dB
- **THEN** the output level SHALL be approximately 20 dB above threshold (10 dB excess boosted to 10/0.5 = 20 dB)

#### Scenario: Unity gain below threshold for compressor default
- **WHEN** `below_ratio` is 1:1 (default for compressor operation)
- **THEN** signals below threshold pass unaffected regardless of level

#### Scenario: Makeup gain compensates for gain reduction
- **WHEN** the processor applies X dB of gain reduction
- **THEN** the output SHALL be boosted by the configured `makeup_gain` in dB

### Requirement: Transient mode shapes attack and sustain phases
In `transient` mode, the gain computer SHALL track whether the envelope is rising (attack phase) or falling (sustain phase), applying `attack_gain` dB during attack and `sustain_gain` dB during sustain. A hysteresis window SHALL prevent rapid oscillation between phases on steady signals.

#### Scenario: Attack gain boosts attack phase
- **WHEN** `mode` is `transient`, `attack_gain` is +6 dB, `sustain_gain` is 0 dB, and a percussive signal with a sharp transient is processed
- **THEN** the initial transient peak SHALL be approximately 6 dB higher than the input (within envelope tracking tolerances)

#### Scenario: Sustain gain shapes sustain phase
- **WHEN** `mode` is `transient`, `attack_gain` is 0 dB, `sustain_gain` is -6 dB, and a percussive signal is processed
- **THEN** the sustain tail SHALL be approximately 6 dB lower than the input

#### Scenario: Hysteresis prevents chatter
- **WHEN** the envelope hovers near the attack/sustain transition point
- **THEN** the phase SHALL not oscillate more than once per 10 ms period

### Requirement: Processor supports feed-forward and feedback topologies
#### Scenario: Feed-forward topology detects from input
- **WHEN** `topology` is set to `feedforward`
- **THEN** the envelope detector SHALL measure the sidechain signal before gain reduction is applied

#### Scenario: Feedback topology detects from output
- **WHEN** `topology` is set to `feedback`
- **THEN** the envelope detector SHALL measure the sidechain signal after gain reduction (one-sample delayed feedback)

### Requirement: Processor supports RMS and peak detection
#### Scenario: RMS detection averages signal energy
- **WHEN** `detection` is `rms`
- **THEN** the envelope SHALL follow the RMS level of the sidechain signal with the configured attack/release time constants

#### Scenario: Peak detection follows instantaneous level
- **WHEN** `detection` is `peak`
- **THEN** the envelope SHALL follow the peak level of the sidechain signal with the configured attack/release time constants

### Requirement: Processor supports soft and hard knee
#### Scenario: Hard knee engages abruptly at threshold
- **WHEN** `knee` is 0 dB (hard knee)
- **THEN** gain reduction starts exactly at the threshold with no transition zone

#### Scenario: Soft knee smooths the threshold transition
- **WHEN** `knee` is 6 dB
- **THEN** gain reduction begins gradually 3 dB below threshold and reaches full ratio 3 dB above threshold

### Requirement: Processor handles edge cases
#### Scenario: No NaN for any parameter combination
- **WHEN** a dynamics-processor processes any input signal with any combination of parameters
- **THEN** the output SHALL not contain NaN or infinite values

#### Scenario: Both ratios at unity passes signal unchanged
- **WHEN** `below_ratio` is 1:1 and `above_ratio` is 1:1 (minimum)
- **THEN** the processor SHALL pass the input signal unaffected regardless of level

### Requirement: Deterministic processor rendering
#### Scenario: Same dynamics-processor patch repeats exactly
- **WHEN** the same dynamics-processor patch is rendered twice with the same input signal and parameters
- **THEN** both renders produce identical audio output
