## MODIFIED Requirements

### Requirement: Patch preparation pipeline

The engine SHALL prepare instruments through an explicit pipeline: parse document → resolve definitions → resolve static arguments and resources → recursively flatten composites to atomic nodes and explicit poly regions → validate ports, rates, channel counts, multiplicity, and cycles → balance latency → schedule and plan channel-span buffers → compiled patch → runtime state. Expansion SHALL be cached keyed by definition identity plus resolved static arguments.

#### Scenario: Valid patch produces prepared runtime

- **WHEN** a valid patch document and its assets are prepared
- **THEN** the engine SHALL produce a runtime-ready representation containing a fully flattened graph, validated routing, compiled execution metadata, module state, and required scratch capacity

#### Scenario: Multichannel ports compile to channel spans

- **WHEN** a resolved graph contains an N-channel port
- **THEN** the compiled patch SHALL contain a contiguous N-buffer span and pre-resolved channel-wise routes for that port

#### Scenario: Invalid patch fails before runtime creation

- **WHEN** schema validation, static-argument resolution, flattening, graph validation, asset preparation, or compilation fails
- **THEN** the engine SHALL report preparation failure before creating or replacing runtime render state

#### Scenario: Repeated expansion is cached

- **WHEN** a definition is instantiated many times with identical static arguments
- **THEN** preparation SHALL reuse one cached expansion per distinct static-argument set rather than re-expanding per instance

### Requirement: Compiled patch drives offline and realtime rendering

Offline and realtime rendering SHALL consume the same compiled flattened patch representation for routing, execution order, module kind resolution, and port mapping. Realtime rendering SHALL derive an explicit render plan from the compiled patch before rendering so the callback uses pre-resolved buffer IDs, event queue IDs, audio/control edge lists, event edge lists, and root-port bus bindings rather than name-based routing lookup.

#### Scenario: Offline render uses compiled patch

- **WHEN** an offline render is started from a prepared patch
- **THEN** the offline renderer SHALL use the compiled patch metadata rather than independently rebuilding graph routing or traversal state

#### Scenario: Realtime render uses compiled render plan

- **WHEN** a realtime runtime is prepared from a patch
- **THEN** realtime preparation SHALL derive a render plan containing execution steps, audio/control input edges, event input edges, output buffer IDs, event queue IDs, MIDI input binding, and named-bus output bindings

#### Scenario: Realtime render avoids port-name lookup

- **WHEN** a realtime block is rendered from a prepared render plan
- **THEN** the render path SHALL route audio, control, and event signals through compiled indices and IDs rather than comparing port names or looking up string-keyed output maps in the callback

#### Scenario: Event routing uses typed event edges

- **WHEN** realtime preparation derives event routes from a compiled patch
- **THEN** the render plan SHALL represent those routes as typed event edges between source and destination event queue IDs rather than as audio/control buffer edges or string-keyed event-port maps

#### Scenario: Shared compilation preserves parity

- **WHEN** the same patch, render settings, assets, and input events are rendered through offline and realtime block paths
- **THEN** both paths SHALL use equivalent routing and module execution semantics

### Requirement: Polyphonic realtime rendering uses prepared storage

Polyphonic realtime rendering SHALL execute each `poly` node through its own runtime region and storage prepared for `max_voices` instances: routing note events to voice instances, processing active voices, summing voice outputs, and retiring voices. It SHALL NOT create per-block voice event vectors, per-voice output maps, or accumulation maps in the audio callback.

#### Scenario: Voice event routing uses prepared queues

- **WHEN** incoming note events are rendered through a `poly` node
- **THEN** the runtime SHALL route them to prepared per-voice event queues without allocating per-block voice event vectors

#### Scenario: Voice outputs accumulate into prepared buffers

- **WHEN** active voice instances produce audio or control output
- **THEN** the runtime SHALL sum their signals into the poly node's prepared accumulation buffers without allocating per-voice output maps or accumulation maps

#### Scenario: Inactive voices do not leak stale output

- **WHEN** a voice instance is inactive or retired
- **THEN** its stale arena buffers SHALL NOT contribute to the poly node's summed output for subsequent blocks
