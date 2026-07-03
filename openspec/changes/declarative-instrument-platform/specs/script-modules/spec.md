## MODIFIED Requirements

### Requirement: Script execution is bounded

Script execution SHALL be bounded in time, memory, and graph recursion. Scripts SHALL NOT recursively execute the graph, create unbounded same-tick event loops, allocate heap memory during execution, access the filesystem, access the network, or perform blocking calls.

#### Scenario: Script output feedback is queued

- **WHEN** a script output is routed back to an upstream script or event input
- **THEN** the engine SHALL queue that feedback to a future tick or block rather than executing recursively in the same processing step

#### Scenario: Script cannot access filesystem during render

- **WHEN** a script attempts to read a file from the filesystem during render-time execution
- **THEN** the script runtime SHALL reject the operation with a diagnostic error

#### Scenario: Script cannot allocate heap during execution

- **WHEN** a script attempts to allocate memory on the heap during render-time execution
- **THEN** the script runtime SHALL reject the operation with a diagnostic error

#### Scenario: Script cannot perform blocking calls

- **WHEN** a script calls a function that would block the audio thread
- **THEN** the script runtime SHALL reject the operation with a diagnostic error

## ADDED Requirements

### Requirement: Script pre-validation

Script modules SHALL be parsed, validated, and compiled or interpreted off the audio thread before rendering begins.

#### Scenario: Script validated before render

- **WHEN** a patch containing a script module is loaded
- **THEN** the script SHALL be parsed and validated before graph expansion and rendering start

#### Scenario: Invalid script is rejected at load time

- **WHEN** a script contains syntax errors, references to disallowed APIs, or exceeds bounded execution limits
- **THEN** loading SHALL fail with a diagnostic error

### Requirement: Script deterministic execution

Script module execution SHALL be deterministic: the same script, same inputs, and same internal state SHALL produce identical outputs.

#### Scenario: Script produces identical output

- **WHEN** a script is executed twice with identical inputs and initial state
- **THEN** both executions SHALL produce identical outputs

### Requirement: Script stable error reporting

Script runtime errors SHALL be reported through the structured diagnostics system with stable error codes and source location information.

#### Scenario: Script error has stable error code

- **WHEN** a script runtime error occurs
- **THEN** the diagnostic SHALL include a stable error code in the `script.*` namespace

### Requirement: Script scope excludes audio-rate DSP

Script modules SHALL NOT be used for sample-rate audio signal processing in the initial implementation. Their scope is limited to event transformation, control-value mapping, conditional routing, and modulation logic.

#### Scenario: Audio-rate output from script is prevented

- **WHEN** a script module is configured with an audio-rate output port
- **THEN** engine validation SHALL reject the configuration with a diagnostic indicating that audio-rate script outputs are not supported
