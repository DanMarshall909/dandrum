## ADDED Requirements

### Requirement: Rhai script modules execute only event/control logic

Script modules using `language: rhai` SHALL accept only event and control input/output ports during the first implementation.

#### Scenario: Event routing script emits events to declared ports

- **GIVEN** a script module with an event input port and declared event output ports
- **AND** the script source emits note events to one of those declared ports
- **WHEN** the graph is rendered
- **THEN** downstream modules connected to the emitted event port receive those events
- **AND** the script does not produce audio directly

#### Scenario: Audio ports are rejected

- **GIVEN** a script module declares an audio input or audio output port
- **WHEN** the patch is validated
- **THEN** validation fails with a structured script diagnostic

### Requirement: Rhai source is prepared off the render path

Rhai script source SHALL be parsed, compiled, and validated during patch preparation, not during per-block rendering.

#### Scenario: Valid Rhai source compiles during preparation

- **GIVEN** a patch contains a script module with valid `language: rhai` source
- **WHEN** the patch is prepared for rendering
- **THEN** the script source is compiled to a prepared runtime representation
- **AND** render-time processing does not parse or compile source text

#### Scenario: Invalid Rhai source fails preparation

- **GIVEN** a patch contains malformed Rhai source
- **WHEN** the patch is prepared
- **THEN** preparation fails
- **AND** a structured script parse diagnostic is reported

### Requirement: Rhai scripts run under deterministic limits

Rhai script execution SHALL be bounded by engine-defined limits for operations, call depth, emitted events, control outputs, and persistent state.

#### Scenario: Infinite loop exceeds operation budget

- **GIVEN** a script contains an unbounded loop
- **WHEN** the script is executed for a render block
- **THEN** execution stops at the configured operation budget
- **AND** a structured budget diagnostic is recorded
- **AND** rendering continues without panicking

#### Scenario: Excess emitted events are bounded

- **GIVEN** a script emits more events than the configured per-port maximum
- **WHEN** the script is executed
- **THEN** output events are capped deterministically
- **AND** a structured bounded-output diagnostic is recorded

### Requirement: Rhai scripts expose only the Dandrum host context

The Rhai runtime SHALL expose only the Dandrum script context API required for event/control processing.

#### Scenario: Unsupported host capabilities are unavailable

- **GIVEN** a script attempts filesystem, network, environment, process, thread, sleep, dynamic import, or logging access
- **WHEN** the script is validated or executed
- **THEN** the capability is unavailable
- **AND** the failure is reported through structured diagnostics

### Requirement: Script failures are deterministic and non-fatal

Script failures SHALL not panic the graph processor or terminate rendering.

#### Scenario: Runtime script failure produces safe output

- **GIVEN** a prepared script fails during block execution
- **WHEN** the graph processor handles the failure
- **THEN** the script produces no further output for that block or truncates output according to the documented limit policy
- **AND** the graph processor continues rendering deterministically
