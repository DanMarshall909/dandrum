## ADDED Requirements

### Requirement: Peak controller converts audio level into smoothed control

The engine SHALL provide a `peak_controller` primitive that accepts audio input and emits a smoothed control signal derived from the input peak level.

#### Scenario: Rising input follows attack time

- **GIVEN** a `peak_controller` receives a rising audio level
- **WHEN** the graph is rendered
- **THEN** the output control value approaches the new level according to the configured attack behaviour
- **AND** output values remain finite

#### Scenario: Falling input follows decay time

- **GIVEN** a `peak_controller` receives a falling audio level
- **WHEN** the graph is rendered
- **THEN** the output control value falls according to the configured decay behaviour
- **AND** output values remain finite

#### Scenario: Inverted output supports ducking

- **GIVEN** a `peak_controller` has inversion enabled
- **WHEN** the input peak level rises
- **THEN** the output control value falls relative to the uninverted output

### Requirement: Control shaper applies nonlinear curves to control signals

The engine SHALL provide a `control_shaper` primitive that accepts a control input and emits a shaped control output using a selected curve.

#### Scenario: Exponential curve bends control response

- **GIVEN** a `control_shaper` uses the `exponential` curve
- **WHEN** it receives a mid-range control value
- **THEN** the output differs from the linear response in the expected exponential direction
- **AND** the output remains finite

#### Scenario: Step curve quantises control response

- **GIVEN** a `control_shaper` uses the `step` curve with a configured number of steps
- **WHEN** it receives smoothly varying control input
- **THEN** the output is quantised into the configured number of levels

### Requirement: Audio-derived control belongs in Rust primitives

Audio-derived control generation SHALL be implemented as realtime-safe Rust primitives rather than Rhai script logic.

#### Scenario: Peak controller is usable for modulation routing

- **GIVEN** a patch routes audio into `peak_controller`
- **AND** routes the resulting control through `control_shaper` into a downstream control input
- **WHEN** the patch is rendered
- **THEN** the downstream module receives deterministic finite control values
- **AND** no script module is required
