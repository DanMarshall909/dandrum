## ADDED Requirements

### Requirement: Echo SHALL be a stereo effect with independent left/right delay times
The echo effect SHALL process left and right channels independently, each with its own delay line. Left and right delay times SHALL be independently configurable in milliseconds.

#### Scenario: Stereo echo with different delay times
- **WHEN** left delay is 200ms, right delay is 300ms, and a mono impulse is sent to both channels
- **THEN** the left output SHALL have its first echo at ~200ms and the right output at ~300ms

### Requirement: Echo SHALL support feedback with configurable gain
The echo effect SHALL feed back a configurable percentage (0%–99%) of each delayed output into its respective delay line input. Feedback gain SHALL be independently settable per channel.

#### Scenario: Feedback produces decaying repeats
- **WHEN** feedback is 50% and a single impulse is sent
- **THEN** each subsequent echo SHALL be approximately 6 dB quieter than the previous

#### Scenario: Feedback of 0% produces single repeat
- **WHEN** feedback is 0% and delay time is 100ms
- **THEN** only one repeat SHALL be audible at 100ms; no further repeats

### Requirement: Echo SHALL support ping-pong mode
In ping-pong mode, the delayed left signal SHALL be routed to the right delay line input and vice versa, creating a stereo bouncing effect. Feedback SHALL still apply.

#### Scenario: Ping-pong alternates channels
- **WHEN** ping-pong is enabled with 100ms delay, 100% feedback, and a left-only impulse
- **THEN** the first repeat SHALL be at 100ms on the right channel, the second at 200ms on the left, alternating

### Requirement: Echo SHALL support damping via OnePoleFilter in the feedback pipeline
The echo effect SHALL compose the damping filter as a separate `OnePoleFilter` stage in each channel's feedback pipeline: `DelayLine` → `OnePoleFilter` → feedback gain → mix. The `OnePoleFilter` SHALL be configurable as low-pass with settable cutoff frequency (20–20000 Hz). The `OnePoleFilter` SHALL be the same struct from `filter.rs`.

#### Scenario: Damping filter darkens repeats
- **WHEN** damping cutoff is 1000 Hz and an impulse is sent
- **THEN** the first repeat SHALL show high-frequency roll-off above 1000 Hz relative to the dry signal; successive repeats SHALL become progressively darker

### Requirement: Echo SHALL support tempo sync via note division
When tempo sync mode is active, the echo SHALL accept a beats-per-minute (BPM) input and a note division parameter (whole, half, quarter, eighth, sixteenth, triplet variants). Delay time SHALL be computed as `(60.0 / bpm) * note_length_in_beats * 1000.0` milliseconds.

#### Scenario: Quarter note sync at 120 BPM
- **WHEN** sync is enabled, BPM is 120, division is quarter note
- **THEN** delay time SHALL be 500ms

#### Scenario: Eighth note triplet sync at 120 BPM
- **WHEN** sync is enabled, BPM is 120, division is eighth note triplet
- **THEN** delay time SHALL be `(60.0 / 120.0) * (1.0 / 3.0) * 1000.0 = 166.67ms`

### Requirement: Echo SHALL support wet/dry mix
The echo effect SHALL provide independent wet and dry gain control (0.0–1.0). The output SHALL be `dry * input + wet * delayed_signal`.

#### Scenario: Full wet, no dry
- **WHEN** wet is 1.0, dry is 0.0, and an impulse is sent
- **THEN** the direct impulse SHALL NOT appear in the output; only the delayed repeats SHALL be present

### Requirement: Echo SHALL expose built-in module ports
The echo SHALL be registered as a built-in module with: Audio input L+R, Audio output L+R, Control input `time_left_ms`, `time_right_ms`, `feedback`, `damping_cutoff`, `wet`, `dry`, Event input (for BPM clock), and parameters for `sync_division`, `ping_pong` (bool).

#### Scenario: Module registration has correct ports
- **WHEN** built-in module registry is queried for `echo`
- **THEN** the port definitions SHALL match the specification above

### Requirement: Echo SHALL process in global (non-voice) scope
The echo effect SHALL be a global-scope module — it processes summed mix, not per-voice. Voice-scoped delay/echo modules SHALL NOT be supported in this change.

#### Scenario: Echo is global scope
- **WHEN** a patch contains an echo module after a voice-scoped oscillator chain
- **THEN** the echo SHALL process the summed output of all voices

### Requirement: Echo SHALL have a YAML composite module definition example
A YAML example patch SHALL be created at `examples/patches/composite-echo.yaml` that defines an echo effect as a composite module built from primitive modules (delay_line, one_pole_filter, gain, audio_mixer). The composite SHALL demonstrate the same signal flow as the built-in echo: delay line → damping filter → feedback → wet/dry mix.

#### Scenario: Composite echo example loads without error
- **WHEN** the composite echo example patch is loaded and validated
- **THEN** it SHALL produce no validation errors
