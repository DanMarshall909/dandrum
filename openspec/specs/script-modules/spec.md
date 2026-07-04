## Purpose

Specify script modules as bounded graph modules with declared ports and module-local state.

## Requirements

### Requirement: Script modules are graph modules

Script modules SHALL be first-class modules in the routing graph with declared ports and module identifiers.

#### Scenario: Script module participates in routing

- **WHEN** a script module declares an event input and control output
- **THEN** other compatible modules SHALL be able to connect to those ports using normal patch connections

### Requirement: Script modules process events and control signals

Script modules SHALL be able to receive events and control values and emit events and control values through declared
ports.

#### Scenario: Script transforms MIDI event to control output

- **WHEN** a script receives a note event and emits an accent control value
- **THEN** that control value SHALL be available to downstream connected control inputs according to graph scheduling
  rules

### Requirement: Script state is retained safely

Script modules SHALL be able to maintain module-local state between processing calls without sharing mutable engine
internals.

#### Scenario: Script remembers previous note

- **WHEN** a script stores the last received note during one processing call
- **THEN** the script SHALL be able to read that state during a later processing call for the same module instance

### Requirement: Script execution is bounded

Script execution SHALL be bounded in time, memory, and graph recursion. Scripts SHALL NOT recursively execute the graph,
create unbounded same-tick event loops, allocate heap memory during execution, access the filesystem, access the
network, or perform blocking calls.

#### Scenario: Script output feedback is queued

- **WHEN** a script output is routed back to an upstream script or event input
- **THEN** the engine SHALL queue that feedback to a future tick or block rather than executing recursively in the same
  processing step

#### Scenario: Script cannot access filesystem during render

- **WHEN** a script attempts to read a file from the filesystem during render-time execution
- **THEN** the script runtime SHALL reject the operation with a diagnostic error

#### Scenario: Script cannot allocate heap during execution

- **WHEN** a script attempts to allocate memory on the heap during render-time execution
- **THEN** the script runtime SHALL reject the operation with a diagnostic error

#### Scenario: Script cannot perform blocking calls

- **WHEN** a script calls a function that would block the audio thread
- **THEN** the script runtime SHALL reject the operation with a diagnostic error

### Requirement: Script pre-validation

Script modules SHALL be parsed, validated, and compiled or interpreted off the audio thread before rendering begins.

#### Scenario: Script validated before render

- **WHEN** a patch containing a script module is loaded
- **THEN** the script SHALL be parsed and validated before graph expansion and rendering start

#### Scenario: Invalid script is rejected at load time

- **WHEN** a script contains syntax errors, references to disallowed APIs, or exceeds bounded execution limits
- **THEN** loading SHALL fail with a diagnostic error

### Requirement: Script deterministic execution

Script module execution SHALL be deterministic: the same script, same inputs, and same internal state SHALL produce
identical outputs.

#### Scenario: Script produces identical output

- **WHEN** a script is executed twice with identical inputs and initial state
- **THEN** both executions SHALL produce identical outputs

### Requirement: Script stable error reporting

Script runtime errors SHALL be reported through the structured diagnostics system with stable error codes and source
location information.

#### Scenario: Script error has stable error code

- **WHEN** a script runtime error occurs
- **THEN** the diagnostic SHALL include a stable error code in the `script.*` namespace

### Requirement: Script scope excludes audio-rate DSP

Script modules SHALL NOT be used for sample-rate audio signal processing in the initial implementation. Their scope is
limited to event transformation, control-value mapping, conditional routing, and modulation logic.

#### Scenario: Audio-rate output from script is prevented

- **WHEN** a script module is configured with an audio-rate output port
- **THEN** engine validation SHALL reject the configuration with a diagnostic indicating that audio-rate script outputs
  are not supported

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
