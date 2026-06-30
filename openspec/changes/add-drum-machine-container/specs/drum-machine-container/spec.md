## ADDED Requirements

### Requirement: Drum-machine pad container declaration
The engine SHALL provide a `drum_machine` container module that declares named trigger pads, trigger selectors, optional pad child chains, and typed public event ports without declaring patterns, sample assets, sequencer clocks, or implicit mix outputs.

#### Scenario: Drum-machine module is accepted
- **WHEN** a patch declares a module with type `drum_machine` and valid trigger pad configuration
- **THEN** patch validation SHALL accept the module and expose the container's public event ports to graph validation

#### Scenario: Drum-machine requires at least one pad
- **WHEN** a patch declares a `drum_machine` module with no pads
- **THEN** validation SHALL fail with a diagnostic identifying the drum-machine module and missing pads

#### Scenario: Duplicate pad identifiers are rejected
- **WHEN** a `drum_machine` module declares two pads with the same pad `id`
- **THEN** validation SHALL fail with a diagnostic identifying the duplicate pad identifier

#### Scenario: Duplicate trigger selectors are rejected
- **WHEN** a `drum_machine` module declares two pads with the same trigger selector
- **THEN** validation SHALL fail with a diagnostic identifying the duplicate selector and both pad IDs

#### Scenario: Sequencer fields are rejected
- **WHEN** a `drum_machine` module declares pattern, tempo, transport, clock, or step-resolution fields
- **THEN** validation SHALL fail with a diagnostic explaining that sequencing belongs outside the drum-machine container

### Requirement: Pad event port contract
Each drum-machine pad SHALL expose named event ports that allow external modules to trigger the pad and downstream modules to receive the pad trigger.

#### Scenario: Pad trigger input is available
- **WHEN** graph validation inspects a valid drum-machine module with a pad named `kick`
- **THEN** it SHALL expose a compatible event input port for the `kick` pad

#### Scenario: Pad trigger output is available
- **WHEN** graph validation inspects a valid drum-machine module with a pad named `kick`
- **THEN** it SHALL expose a compatible event output port for the `kick` pad

#### Scenario: Missing pad port is rejected
- **WHEN** a patch connects to a drum-machine pad port for a pad ID not declared by the module
- **THEN** validation SHALL fail with a diagnostic identifying the missing public pad port

### Requirement: Trigger selector routing
The drum-machine container SHALL route incoming container trigger events to the pad whose trigger selector matches the event and SHALL NOT change event timing or invent new events.

#### Scenario: Matching selector emits pad event
- **WHEN** a compatible incoming event matches the trigger selector for the `kick` pad during rendering
- **THEN** the `kick` pad output SHALL emit the same event at the same render frame offset

#### Scenario: Non-matching selector emits no pad event
- **WHEN** an incoming event does not match the trigger selector for the `kick` pad during a rendered frame range
- **THEN** the `kick` pad output SHALL emit no event for that incoming event

#### Scenario: Same input repeats exactly
- **WHEN** the same drum-machine patch is rendered twice with the same render settings and input events
- **THEN** both renders SHALL produce identical pad trigger outputs and identical downstream audio buffers

### Requirement: Direct pad trigger inputs
Each drum-machine pad SHALL also allow direct compatible event input to trigger that pad without selector matching.

#### Scenario: Direct pad input emits pad output
- **WHEN** a compatible event reaches the direct input for the `kick` pad
- **THEN** the `kick` pad output SHALL emit the same event at the same render frame offset

#### Scenario: Direct input bypasses selector matching
- **WHEN** a compatible event reaches the direct input for the `kick` pad with an event payload that does not match the pad's trigger selector
- **THEN** the `kick` pad output SHALL still emit the event

### Requirement: Drum-machine graph expansion
The engine SHALL expand a valid drum-machine container into deterministic namespaced internal event-routing modules and declared pad child-chain modules before graph processing.

#### Scenario: Container expands before graph processing
- **WHEN** a patch containing a valid `drum_machine` module is prepared for rendering
- **THEN** graph construction SHALL replace the container with namespaced internal event-routing modules and routes that preserve the declared pad trigger behavior

#### Scenario: Multiple drum machines do not collide
- **WHEN** a patch declares two valid `drum_machine` modules
- **THEN** expansion SHALL produce deterministic and distinct internal module IDs for each container instance

#### Scenario: Expansion routes pad trigger events
- **WHEN** a drum-machine pad receives an event through selector matching or direct pad input
- **THEN** the expanded graph SHALL route that event to the corresponding pad output without requiring sampler or audio modules inside the container

### Requirement: Pad child chains
When a drum-machine pad declares an explicit child module chain, the engine SHALL route the pad trigger event into that child chain and expand the child chain as ordinary namespaced graph modules.

#### Scenario: Pad chain receives pad trigger
- **WHEN** a drum-machine pad with a declared child chain receives a matching trigger event
- **THEN** the expanded graph SHALL route the pad trigger event to the declared child chain input

#### Scenario: Pad chain modules are namespaced
- **WHEN** a drum-machine pad declares child modules
- **THEN** expansion SHALL assign deterministic namespaced module IDs that include the drum-machine instance ID and pad ID

#### Scenario: Pad chain uses ordinary validation
- **WHEN** a drum-machine pad child chain contains an invalid route
- **THEN** validation SHALL fail with the same diagnostics used for ordinary graph or composite-module routes

### Requirement: Downstream trigger composition
The drum-machine container SHALL allow pad outputs to trigger ordinary downstream modules through compatible event routes.

#### Scenario: Pad output triggers sampler
- **WHEN** a patch connects a drum-machine pad output to a sampler trigger input
- **THEN** graph validation SHALL accept the route and rendering SHALL trigger the sampler from incoming pad events

#### Scenario: Pad output triggers script module
- **WHEN** a patch connects a drum-machine pad output to a compatible script-module event input
- **THEN** graph validation SHALL accept the route and rendering SHALL deliver incoming pad events to the script module

#### Scenario: Pad output does not generate audio directly
- **WHEN** a patch declares a drum-machine module without downstream audio-generating modules
- **THEN** rendering SHALL NOT produce drum-machine audio solely from the container

### Requirement: Drum-machine example patch
The project SHALL include a minimal YAML patch example that declares a drum-machine container and uses it to trigger downstream modules through the existing CLI render path.

#### Scenario: Drum-machine trigger example renders to WAV
- **WHEN** the drum-machine trigger example patch is rendered through the CLI
- **THEN** the command SHALL succeed and write a non-empty WAV file produced by downstream modules, not by the drum-machine container itself
