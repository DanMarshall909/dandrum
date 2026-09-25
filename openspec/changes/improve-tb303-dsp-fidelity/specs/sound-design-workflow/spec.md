## ADDED Requirements

### Requirement: Sound fixtures reproduce an implementation stimulus

Dandrum SHALL represent a sound-design stimulus as a repository-owned declarative fixture that identifies the patch, render settings, event timeline, and spectral-analysis settings needed to reproduce it.

#### Scenario: Acid fixture exercises representative voice behaviour

- **WHEN** the TB-303 proof-of-concept sound fixture is loaded
- **THEN** it SHALL select the maintained `tb303-acid` patch at 48 kHz
- **AND** its 16-step event timeline SHALL include accented and unaccented notes, rests, a continuous tied note, and an overlapping transition for slide behaviour

### Requirement: Sound analysis reports spectral and level trajectories

Dandrum SHALL analyze rendered sound as a sequence of overlapping Hann-windowed frames so sound implementation can compare level and spectral motion over time.

#### Scenario: Audible and silent frames produce meaningful metrics

- **WHEN** analysis processes audible and silent frames using a declared frequency band
- **THEN** each frame SHALL report its position, time, RMS level, and peak level
- **AND** audible frames SHALL report a finite band-limited spectral centroid
- **AND** silent frames SHALL omit the spectral centroid rather than reporting an arbitrary frequency

### Requirement: Sound workbench emits repeatable implementation artifacts

Dandrum SHALL provide a command that renders a sound fixture and writes an audition WAV plus a frame-by-frame metrics CSV from the same output, and SHALL analyze an aligned external PCM WAV with the fixture's declared analysis settings.

#### Scenario: Workbench renders the acid proof of concept

- **WHEN** the sound workbench renders the TB-303 proof-of-concept fixture twice
- **THEN** both renders SHALL contain identical finite non-silent samples
- **AND** each run SHALL write a playable WAV and a metrics CSV containing time, RMS, peak, and spectral-centroid columns

#### Scenario: Workbench analyzes an external reference recording

- **WHEN** the sound workbench analyzes an aligned PCM WAV whose sample rate matches the selected fixture
- **THEN** it SHALL write the same metrics CSV columns used for a Dandrum render
- **AND** it SHALL reject a reference WAV whose sample rate does not match the fixture
