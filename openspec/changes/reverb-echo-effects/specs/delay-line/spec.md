## ADDED Requirements

### Requirement: DelayLine SHALL support configurable maximum delay time
The `DelayLine` struct SHALL be initialized with a maximum delay time in samples (based on `sample_rate` and `max_delay_ms`). It SHALL allocate a power-of-two circular buffer of sufficient size. It SHALL provide the actual allocated capacity for inspection.

#### Scenario: Initialize with max delay time
- **WHEN** a `DelayLine` is created with `sample_rate=48000`, `max_delay_ms=2000`
- **THEN** the buffer SHALL hold at least `48000 * 2 = 96000` samples and SHALL be power-of-two sized (next power of two: 131072)

#### Scenario: Query max delay samples
- **WHEN** `max_delay_samples()` is called
- **THEN** it SHALL return the maximum number of samples that can be stored

### Requirement: DelayLine SHALL support write and fractional read
The `DelayLine` SHALL provide `write(sample: f32)` to push a sample into the buffer, advancing the write head. It SHALL provide `read(delay_samples: f32) -> f32` to read a sample from `delay_samples` samples in the past, with fractional interpolation.

#### Scenario: Write then read at integer delay
- **WHEN** sample `1.0` is written, then sample `2.0` is written, then `read(1.0)` is called
- **THEN** the result SHALL be approximately `1.0` (the sample written one write ago)

#### Scenario: Read at fractional delay with linear interpolation
- **WHEN** sample `1.0` and `2.0` are written consecutively, then `read(0.5)` is called
- **THEN** the result SHALL be approximately `1.5` (midpoint between newest two samples)

### Requirement: DelayLine SHALL support modulation input
The `DelayLine` SHALL accept a modulation offset applied to the read position. Modulation SHALL be bipolar and additive to the base delay time. The resulting read position SHALL be clamped to valid range.

#### Scenario: Modulation shifts read position
- **WHEN** base delay is `100` samples and modulation is `10.0` samples
- **THEN** the effective read position SHALL be `110.0` samples

#### Scenario: Modulation clamped
- **WHEN** base delay is `5` samples and modulation is `-10.0` samples
- **THEN** the effective read position SHALL be clamped to a minimum of `1.0` sample (no read at current write position)

### Requirement: DelayLine SHALL support interpolation mode switching
The `DelayLine` SHALL support switching between `Linear` and `Cubic` interpolation modes. Cubic mode SHALL use 4-point interpolation for higher quality.

#### Scenario: Cubic interpolation at fractional delay
- **WHEN** cubic mode is selected and `read(0.5)` is called with four distinct samples written
- **THEN** the result SHALL use 4-point cubic interpolation (about 1.4375 for samples [0,0,1,0])

### Requirement: DelayLine SHALL reset to zero
The `DelayLine` SHALL provide a `reset()` method that clears the buffer to zero and resets the write head.

#### Scenario: Reset clears buffer
- **WHEN** samples are written then `reset()` is called
- **THEN** all subsequent reads SHALL return `0.0` until new samples are written
