## ADDED Requirements

### Requirement: Public API boundary
The Rust engine crate SHALL expose a small intentional public API for headless engine use while keeping implementation modules private to the crate unless they are explicitly part of the supported API.

#### Scenario: Crate root exposes facade APIs
- **WHEN** a Rust consumer imports the engine crate
- **THEN** the consumer SHALL be able to access documented facade types for loading/preparing instruments and rendering audio without importing graph processor internals

#### Scenario: Implementation modules remain crate-private
- **WHEN** runtime, graph-processing, or DSP implementation details are reorganized
- **THEN** external Rust consumers SHALL NOT need to update imports for private helper modules

### Requirement: FFI boundary delegates to safe Rust APIs
The C ABI SHALL be implemented as a thin adapter layer that handles unsafe pointer conversion and status-code translation while delegating validation, preparation, and rendering behavior to safe Rust APIs.

#### Scenario: C ABI symbol names remain stable
- **WHEN** the JUCE wrapper links against the Rust static library
- **THEN** the existing exported C ABI entry points SHALL remain available unless a replacement is explicitly specified and tested

#### Scenario: Invalid FFI inputs are contained
- **WHEN** an FFI caller passes a null engine pointer, null buffer pointer, or invalid string pointer
- **THEN** the FFI layer SHALL reject the call without invoking unsafe behavior in the safe engine runtime

### Requirement: Patch preparation pipeline
The engine SHALL prepare instruments through an explicit pipeline from patch document to validated graph to compiled patch to runtime state.

#### Scenario: Valid patch produces prepared runtime
- **WHEN** a valid patch document and its assets are prepared
- **THEN** the engine SHALL produce a runtime-ready representation containing validated routing, compiled execution metadata, module state, and required scratch capacity

#### Scenario: Invalid patch fails before runtime creation
- **WHEN** patch schema validation, graph validation, asset preparation, or compilation fails
- **THEN** the engine SHALL report preparation failure before creating or replacing runtime render state

### Requirement: Compiled patch drives offline and realtime rendering
Offline and realtime rendering SHALL consume the same compiled patch representation for routing, execution order, scope grouping, module kind resolution, and port mapping.

#### Scenario: Offline render uses compiled patch
- **WHEN** an offline render is started from a prepared patch
- **THEN** the offline renderer SHALL use the compiled patch metadata rather than independently rebuilding graph routing or traversal state

#### Scenario: Realtime render uses compiled patch
- **WHEN** a realtime runtime is prepared from a patch
- **THEN** realtime rendering SHALL use the compiled patch metadata rather than independently rebuilding graph routing or traversal state

#### Scenario: Shared compilation preserves parity
- **WHEN** the same patch, render settings, assets, and input events are rendered through offline and realtime block paths
- **THEN** both paths SHALL use equivalent routing and module execution semantics

### Requirement: Runtime dispatch uses typed module kinds
The render path SHALL dispatch built-in module behavior through typed module kinds and configuration resolved before rendering, not by matching raw patch module type strings during audio processing.

#### Scenario: Module kind is resolved during preparation
- **WHEN** a patch declares a built-in module type and parameters
- **THEN** preparation or compilation SHALL resolve it into a typed module kind/configuration used by runtime state creation

#### Scenario: Unknown module type fails before rendering
- **WHEN** a patch declares an unsupported module type
- **THEN** preparation or compilation SHALL fail before any render call can dispatch that module

### Requirement: DSP algorithms are independent from graph concerns
Reusable DSP algorithms SHALL remain independent from patch YAML declarations, module IDs, graph cables, FFI pointers, CLI arguments, and frontend/device APIs.

#### Scenario: DSP can be tested without graph setup
- **WHEN** a DSP algorithm such as a filter, delay, dynamics processor, saturator, convolution, echo, or reverb is unit tested
- **THEN** the test SHALL be able to instantiate and process the DSP without constructing a patch document or routing graph

#### Scenario: Module adapter owns graph translation
- **WHEN** a graph module receives audio, control, or event inputs
- **THEN** module adapter code SHALL translate those inputs into DSP calls without requiring the DSP algorithm to know graph port names or module IDs

### Requirement: Realtime render path preallocates required resources
Realtime rendering SHALL allocate required module state, scratch buffers, output buffers, and event capacity during preparation rather than during the audio render call.

#### Scenario: Prepared runtime records capacity
- **WHEN** a realtime runtime is prepared with a maximum block size and voice allocation
- **THEN** the runtime SHALL allocate or reserve the state and scratch capacity needed to render blocks up to that size

#### Scenario: Render reuses prepared resources
- **WHEN** realtime render is called repeatedly with blocks no larger than the prepared maximum block size
- **THEN** rendering SHALL reuse prepared resources without growing scratch buffers or replacing runtime state
