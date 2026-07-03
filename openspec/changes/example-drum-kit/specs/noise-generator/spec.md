## ADDED Requirements

### Requirement: Noise module produces white noise

The `noise` module SHALL generate white noise (uniform distribution over [-1, 1]) when its colour control input is at
its default value.

#### Scenario: White noise output is in [-1, 1]

- **WHEN** the noise module is processed for 1024 frames with default colour
- **THEN** every output sample SHALL be in the range [-1.0, 1.0]

#### Scenario: White noise has non-zero variance

- **WHEN** the noise module is processed for 1024 frames with default colour
- **THEN** the RMS of the output SHALL be greater than 0.1

### Requirement: Noise module colour control

The `noise` module SHALL accept a `colour` control input (range 0.0–1.0) that selects the noise colour: 0.0 = white, ~
0.5 = pink, ~1.0 = brownian.

#### Scenario: Colour control switches noise type

- **WHEN** the colour input is set to 0.0 and then to 1.0 for two separate 1024-frame blocks
- **THEN** the output statistics (RMS, spectral centroid) SHALL differ between the two blocks

### Requirement: Noise module amplitude control

The `noise` module SHALL accept an `amplitude` control input that scales the output gain linearly.

#### Scenario: Amplitude scales output

- **WHEN** the amplitude is set to 0.5
- **THEN** the RMS of 1024 output frames SHALL be approximately half that of amplitude 1.0 (within 5%)

### Requirement: Noise module is a pure signal source

The `noise` module SHALL have no event inputs. It SHALL produce continuous output whenever processed, regardless of any
event state. It SHALL NOT accept a gate or trigger input.

#### Scenario: Noise produces output without any event

- **WHEN** the noise module is processed for 256 frames with no events
- **THEN** every output sample SHALL be non-zero (within floating-point tolerance)

### Requirement: Noise module is registered and dispatchable

The noise module SHALL be registered in the built-in module registry, have a corresponding `ModuleKind::Noise` variant,
and be dispatchable at render time.

#### Scenario: Noise module is in built-in registry

- **WHEN** the registry is queried for the `noise` module type
- **THEN** a definition SHALL be returned

#### Scenario: Noise module renders without error

- **WHEN** a patch containing a noise module is loaded, prepared, and rendered
- **THEN** rendering SHALL complete without panic or error
