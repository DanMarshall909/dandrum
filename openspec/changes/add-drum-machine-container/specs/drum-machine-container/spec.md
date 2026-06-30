## ADDED Requirements

### Requirement: Drum-machine event module declaration
The engine SHALL provide a `drum_machine` event-only module that declares named trigger pads, trigger selectors, per-pad emitted-event configuration, one container event input, and typed public pad event ports without declaring patterns, sample assets, signal chains, sequencer clocks, audio outputs, or implicit mix outputs.

#### Scenario: Drum-machine module is accepted
- **WHEN** a patch declares a module with type `drum_machine` and valid pad event configuration
- **THEN** patch validation SHALL accept the module and expose the container and pad event ports to graph validation

#### Scenario: Drum-machine requires at least one pad
- **WHEN** a patch declares a `drum_machine` module with no pads
- **THEN** validation SHALL fail with a diagnostic identifying the drum-machine module and missing pads

#### Scenario: Duplicate pad identifiers are rejected
- **WHEN** a `drum_machine` module declares two pads with the same pad `id`
- **THEN** validation SHALL fail with a diagnostic identifying the duplicate pad identifier

#### Scenario: Duplicate trigger selectors are rejected
- **WHEN** a `drum_machine` module declares two pads with the same trigger selector
- **THEN** validation SHALL fail with a diagnostic identifying the duplicate selector and both pad IDs

#### Scenario: Signal-chain fields are rejected
- **WHEN** a `drum_machine` module declares child modules, internal connections, sample assets, audio outputs, or mix outputs
- **THEN** validation SHALL fail with a diagnostic explaining that signal chains must be modeled by external patch modules

#### Scenario: Sequencer fields are rejected
- **WHEN** a `drum_machine` module declares pattern, tempo, transport, clock, or step-resolution fields
- **THEN** validation SHALL fail with a diagnostic explaining that sequencing belongs outside the drum-machine module

### Requirement: Drum-machine event port contract
The drum-machine module SHALL expose a standard event input for selector routing and named event ports for each declared pad.

#### Scenario: Container event input is available
- **WHEN** graph validation inspects a valid drum-machine module
- **THEN** it SHALL expose a compatible event input port named `events`

#### Scenario: Pad trigger input is available
- **WHEN** graph validation inspects a valid drum-machine module with a pad named `kick`
- **THEN** it SHALL expose a compatible event input port for directly triggering the `kick` pad

#### Scenario: Pad event output is available
- **WHEN** graph validation inspects a valid drum-machine module with a pad named `kick`
- **THEN** it SHALL expose a compatible event output port for the `kick` pad

#### Scenario: Missing pad port is rejected
- **WHEN** a patch connects to a drum-machine pad port for a pad ID not declared by the module
- **THEN** validation SHALL fail with a diagnostic identifying the missing public pad port

### Requirement: Event-transformer behavior
The drum-machine module SHALL behave as an event transformer: it consumes event inputs, produces event outputs, preserves deterministic frame timing, and performs no audio, control, or signal-chain processing.

#### Scenario: Event transformer accepts event stream
- **WHEN** a compatible event stream is connected to a drum-machine event input
- **THEN** graph validation SHALL treat the drum-machine module as an event-to-event transformer

#### Scenario: Event transformer emits only events
- **WHEN** a drum-machine module emits pad output during rendering
- **THEN** every emitted output SHALL be an event and SHALL NOT include audio, control, or hidden module output

#### Scenario: Event transformer is deterministic
- **WHEN** the same drum-machine patch receives the same input events at the same render frame offsets
- **THEN** it SHALL emit the same output events at the same render frame offsets

### Requirement: Trigger selector routing
The drum-machine module SHALL route incoming events on its standard `events` input to the pad whose trigger selector matches the incoming event.

#### Scenario: Matching selector emits configured pad event
- **WHEN** a compatible incoming event on `events` matches the trigger selector for the `kick` pad during rendering
- **THEN** the `kick` pad output SHALL emit the event configured for that pad at the same render frame offset

#### Scenario: Non-matching selector emits no pad event
- **WHEN** an incoming event on `events` does not match the trigger selector for the `kick` pad during a rendered frame range
- **THEN** the `kick` pad output SHALL emit no event for that incoming event

#### Scenario: Same input repeats exactly
- **WHEN** the same drum-machine patch is rendered twice with the same render settings and input events
- **THEN** both renders SHALL produce identical pad event outputs and identical downstream audio buffers

### Requirement: Direct pad trigger inputs
Each drum-machine pad SHALL also allow direct compatible event input to trigger that pad without selector matching.

#### Scenario: Direct pad input emits configured pad event
- **WHEN** a compatible event reaches the direct input for the `kick` pad
- **THEN** the `kick` pad output SHALL emit the event configured for that pad at the same render frame offset

#### Scenario: Direct input bypasses selector matching
- **WHEN** a compatible event reaches the direct input for the `kick` pad with an event payload that does not match the pad's trigger selector
- **THEN** the `kick` pad output SHALL still emit the event configured for that pad

### Requirement: Per-pad emitted events
Each drum-machine pad SHALL declare what event it emits when triggered, and the module SHALL emit that configured event rather than owning downstream signal-chain behavior.

#### Scenario: Pad emits configured event payload
- **WHEN** a pad declares an emitted event payload and the pad is triggered
- **THEN** the pad output SHALL emit an event carrying the configured payload

#### Scenario: Pad can preserve incoming event
- **WHEN** a pad declares that it preserves the incoming event and the pad is triggered
- **THEN** the pad output SHALL emit the incoming event unchanged except for graph routing identity

#### Scenario: Invalid emitted event configuration is rejected
- **WHEN** a pad declares an emitted-event configuration that is unsupported or malformed
- **THEN** validation SHALL fail with a diagnostic identifying the drum-machine module, pad ID, and invalid emitted-event configuration

### Requirement: Drum-machine graph expansion
The engine SHALL expand a valid drum-machine module into deterministic namespaced internal event-routing modules before graph processing.

#### Scenario: Module expands before graph processing
- **WHEN** a patch containing a valid `drum_machine` module is prepared for rendering
- **THEN** graph construction SHALL replace the module with namespaced internal event-routing modules and routes that preserve the declared pad event behavior

#### Scenario: Multiple drum machines do not collide
- **WHEN** a patch declares two valid `drum_machine` modules
- **THEN** expansion SHALL produce deterministic and distinct internal module IDs for each module instance

#### Scenario: Expansion routes only event data
- **WHEN** a drum-machine module expands
- **THEN** the expanded graph SHALL contain event-compatible routes for pad triggering and SHALL NOT include sampler, audio, control, or mixer modules on behalf of the drum-machine module

### Requirement: Downstream signal-chain composition
The drum-machine module SHALL allow pad outputs to trigger ordinary downstream modules through compatible event routes, and those downstream modules SHALL be declared explicitly outside the drum-machine module.

#### Scenario: Pad output triggers sampler
- **WHEN** a patch connects a drum-machine pad output to an explicitly declared sampler trigger input
- **THEN** graph validation SHALL accept the route and rendering SHALL trigger the sampler from emitted pad events

#### Scenario: Pad output triggers script module
- **WHEN** a patch connects a drum-machine pad output to a compatible explicitly declared script-module event input
- **THEN** graph validation SHALL accept the route and rendering SHALL deliver emitted pad events to the script module

#### Scenario: Drum-machine module does not generate audio directly
- **WHEN** a patch declares a drum-machine module without downstream audio-generating modules
- **THEN** rendering SHALL NOT produce drum-machine audio solely from the module

### Requirement: Drum-machine example patch
The project SHALL include a minimal YAML patch example that declares a drum-machine module and an explicit downstream signal chain that consumes emitted pad events.

#### Scenario: Drum-machine trigger example renders to WAV
- **WHEN** the drum-machine trigger example patch is rendered through the CLI
- **THEN** the command SHALL succeed and write a non-empty WAV file produced by explicitly declared downstream modules, not by the drum-machine module itself
