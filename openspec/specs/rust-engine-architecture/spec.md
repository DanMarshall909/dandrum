## Purpose

Specify the Rust engine crate public API, FFI boundary, patch preparation pipeline, compiled patch representation, runtime dispatch, DSP separation, and preallocation requirements.

## Requirements

### Requirement: Public API boundary

The Rust engine crate SHALL expose a small intentional public API for headless engine use while keeping implementation
modules private to the crate unless they are explicitly part of the supported API.

#### Scenario: Crate root exposes facade APIs

- **WHEN** a Rust consumer imports the engine crate
- **THEN** the consumer SHALL be able to access documented facade types for loading/preparing instruments and rendering
  audio without importing graph processor internals

#### Scenario: Implementation modules remain crate-private

- **WHEN** runtime, graph-processing, or DSP implementation details are reorganized
- **THEN** external Rust consumers SHALL NOT need to update imports for private helper modules

### Requirement: FFI boundary delegates to safe Rust APIs

The C ABI SHALL be implemented as a thin adapter layer that handles unsafe pointer conversion and status-code
translation while delegating validation, preparation, and rendering behavior to safe Rust APIs.

#### Scenario: C ABI symbol names remain stable

- **WHEN** the JUCE wrapper links against the Rust static library
- **THEN** the existing exported C ABI entry points SHALL remain available unless a replacement is explicitly specified
  and tested

#### Scenario: Invalid FFI inputs are contained

- **WHEN** an FFI caller passes a null engine pointer, null buffer pointer, or invalid string pointer
- **THEN** the FFI layer SHALL reject the call without invoking unsafe behavior in the safe engine runtime

### Requirement: Patch preparation pipeline

The engine SHALL prepare instruments through an explicit pipeline from patch document to validated graph to compiled
patch to runtime state.

#### Scenario: Valid patch produces prepared runtime

- **WHEN** a valid patch document and its assets are prepared
- **THEN** the engine SHALL produce a runtime-ready representation containing validated routing, compiled execution
  metadata, module state, and required scratch capacity

#### Scenario: Invalid patch fails before runtime creation

- **WHEN** patch schema validation, graph validation, asset preparation, or compilation fails
- **THEN** the engine SHALL report preparation failure before creating or replacing runtime render state

### Requirement: Compiled patch drives offline and realtime rendering

Offline and realtime rendering SHALL consume the same compiled patch representation for routing, execution order, scope
grouping, module kind resolution, and port mapping. Realtime rendering SHALL derive an explicit render plan from the
compiled patch before rendering so the callback uses pre-resolved buffer IDs, event queue IDs, audio/control edge lists,
and event edge lists rather than name-based routing lookup.

#### Scenario: Offline render uses compiled patch

- **WHEN** an offline render is started from a prepared patch
- **THEN** the offline renderer SHALL use the compiled patch metadata rather than independently rebuilding graph routing
  or traversal state

#### Scenario: Realtime render uses compiled render plan

- **WHEN** a realtime runtime is prepared from a patch
- **THEN** realtime preparation SHALL derive a render plan containing execution steps, audio/control input edges, event
  input edges, output buffer IDs, event queue IDs, MIDI input binding, and audio output binding

#### Scenario: Realtime render avoids port-name lookup

- **WHEN** a realtime block is rendered from a prepared render plan
- **THEN** the render path SHALL route audio, control, and event signals through compiled indices and IDs rather than
  comparing port names or looking up string-keyed output maps in the callback

#### Scenario: Event routing uses typed event edges

- **WHEN** realtime preparation derives event routes from a compiled patch
- **THEN** the render plan SHALL represent those routes as typed event edges between source and destination event queue
  IDs rather than as audio/control buffer edges or string-keyed event-port maps

#### Scenario: Shared compilation preserves parity

- **WHEN** the same patch, render settings, assets, and input events are rendered through offline and realtime block
  paths
- **THEN** both paths SHALL use equivalent routing and module execution semantics

### Requirement: Runtime dispatch uses typed module kinds

The render path SHALL dispatch built-in module behavior through typed module kinds and configuration resolved before
rendering, not by matching raw patch module type strings during audio processing. Module processors used by the realtime
render path SHALL write into prepared output buffers supplied by a process context rather than returning owned output
maps.

#### Scenario: Module kind is resolved during preparation

- **WHEN** a patch declares a built-in module type and parameters
- **THEN** preparation or compilation SHALL resolve it into a typed module kind/configuration used by runtime state
  creation

#### Scenario: Module writes into prepared outputs

- **WHEN** a realtime render step processes a module
- **THEN** the module adapter SHALL read prepared input slices/events and write into prepared output buffers/event queues
  without returning owned `HashMap` or `Vec` output containers

#### Scenario: Unknown module type fails before rendering

- **WHEN** a patch declares an unsupported module type
- **THEN** preparation or compilation SHALL fail before any render call can dispatch that module

### Requirement: DSP algorithms are independent from graph concerns

Reusable DSP algorithms SHALL remain independent from patch YAML declarations, module IDs, graph cables, FFI pointers,
CLI arguments, and frontend/device APIs.

#### Scenario: DSP can be tested without graph setup

- **WHEN** a DSP algorithm such as a filter, delay, dynamics processor, saturator, convolution, echo, or reverb is unit
  tested
- **THEN** the test SHALL be able to instantiate and process the DSP without constructing a patch document or routing
  graph

#### Scenario: Module adapter owns graph translation

- **WHEN** a graph module receives audio, control, or event inputs
- **THEN** module adapter code SHALL translate those inputs into DSP calls without requiring the DSP algorithm to know
  graph port names or module IDs

### Requirement: Realtime render path preallocates required resources

Realtime rendering SHALL allocate required module state, scratch buffers, audio/control buffers, output buffers, and
event capacity during preparation rather than during the audio render call. For blocks no larger than the prepared
maximum block size, realtime rendering SHALL NOT allocate, grow collections, rebuild routing maps, or replace runtime
state.

#### Scenario: Prepared runtime records capacity

- **WHEN** a realtime runtime is prepared with a maximum block size and voice allocation
- **THEN** the runtime SHALL allocate or reserve the state, audio/control arena storage, event queue capacity, output
  binding capacity, and scratch capacity needed to render blocks up to that size

#### Scenario: Render reuses prepared resources

- **WHEN** realtime render is called repeatedly with blocks no larger than the prepared maximum block size
- **THEN** rendering SHALL reuse prepared resources without growing scratch buffers, growing event buffers, growing module
  output storage, or replacing runtime state

#### Scenario: Prepared-size realtime render does not allocate

- **WHEN** realtime render is called after preparation with a block no larger than the prepared maximum block size
- **THEN** the render path SHALL avoid callback-time allocation for audio buffers, control buffers, module outputs, input
  gathering, event delivery, voice routing, and output collection

#### Scenario: Oversized render is chunked through prepared storage

- **WHEN** realtime render is called with a block larger than the prepared maximum block size
- **THEN** the runtime SHALL process the request in prepared-size chunks without requiring larger callback-time scratch
  allocation

### Requirement: Realtime event delivery is bounded

Realtime event delivery SHALL use prepared fixed-capacity storage. Callback-time event routing SHALL NOT allocate or grow
event collections. Event overflow SHALL be explicit, observable to the runtime, and handled according to documented
priority or drop/coalescing rules.

#### Scenario: Pending events enter prepared queues

- **WHEN** note or control events are submitted before a render block
- **THEN** realtime rendering SHALL transfer them into prepared event queues without allocating in the render callback

#### Scenario: Module events route through prepared event edges

- **WHEN** a module emits events to a declared event output during realtime rendering
- **THEN** realtime rendering SHALL deliver those events to downstream event input queues through compiled event queue
  IDs without consulting event port names or growing string-keyed event maps in the callback

#### Scenario: Event-producing module uses bounded writer

- **WHEN** a module or script emits events during realtime rendering
- **THEN** it SHALL write through a bounded event writer that either accepts the event or reports overflow without
  allocating

#### Scenario: Event overflow is deterministic

- **WHEN** a prepared event queue is full during realtime rendering
- **THEN** the runtime SHALL apply documented overflow behaviour, preserve critical events where required, and record an
  overflow condition without growing the queue

### Requirement: Polyphonic realtime rendering uses prepared storage

Polyphonic realtime rendering SHALL route voice events, process active voices, accumulate voice outputs, process global
nodes, collect output, and retire voices using prepared storage. It SHALL NOT create per-block voice event vectors,
per-voice output maps, or accumulation maps in the audio callback.

#### Scenario: Voice event routing uses prepared queues

- **WHEN** incoming note events are rendered in a polyphonic runtime
- **THEN** the runtime SHALL route them to prepared per-voice event queues without allocating per-block voice event
  vectors

#### Scenario: Voice outputs accumulate into prepared buffers

- **WHEN** active voices produce audio or control output
- **THEN** the runtime SHALL accumulate their signals into prepared accumulation buffers without allocating per-voice
  output maps or accumulation maps

#### Scenario: Inactive voices do not leak stale output

- **WHEN** a voice is inactive or retired
- **THEN** its stale arena buffers SHALL NOT contribute to the accumulated output for subsequent blocks
