## ADDED Requirements

### Requirement: Poly node instantiates a wrapped definition per voice

The engine SHALL provide a `poly` structural node that references a graph definition and a static `max_voices` count. It SHALL expose an event input for note events, forward its remaining inputs to every voice instance, and expose the wrapped definition's audio and control outputs as summed outputs. Each compiled poly node SHALL own an independent runtime region containing its allocator, voice state, queues, schedule, and accumulators. Polyphony SHALL be expressed only through `poly`; there SHALL be no graph-wide voice execution scope after migration completes.

#### Scenario: Poly node wraps a voice definition

- **WHEN** a graph instantiates `poly` with a voice definition and `max_voices: 8`
- **THEN** validation SHALL accept note-event connections into the poly node and audio/control connections out of it, with no voice-scope annotations anywhere in the graph

#### Scenario: Poly nodes nest

- **WHEN** a definition wrapped by `poly` itself contains a `poly` node
- **THEN** compilation SHALL expand both levels and rendering SHALL behave as independent nested voice pools

#### Scenario: Voice cannot reach outside the poly node

- **WHEN** the flattened graph is inspected
- **THEN** every connection out of a voice instance SHALL pass through the poly node's summed outputs, making direct voice-to-host routing structurally impossible

### Requirement: Voice instances are preallocated

Preparation SHALL preallocate flattened state, buffers, and event capacity for all `max_voices` instances. Voice activation, routing, processing, mixing, and retirement SHALL NOT allocate in the render callback.

#### Scenario: Full polyphony renders without allocation

- **WHEN** `max_voices` voices are activated and rendered at the prepared block size
- **THEN** the render path SHALL perform no heap allocation

### Requirement: Voice allocation and stealing policy

The poly node SHALL route incoming note-on events to voice instances using a declared allocation policy. The supported policies SHALL include `oldest-steal`, where a note-on beyond capacity steals the longest-active voice, and `reject-new`, where a note-on beyond capacity is ignored. The policy SHALL be declared as an enumerated static parameter.

#### Scenario: Note events route to free voices

- **WHEN** note-on events arrive while free voices exist
- **THEN** each note SHALL activate a distinct free voice carrying that note's pitch and velocity

#### Scenario: Voice stealing at capacity

- **WHEN** a note-on arrives while all `max_voices` voices are active
- **THEN** the oldest active voice SHALL be retired and reused for the new note

#### Scenario: New note is rejected when stealing is disabled

- **WHEN** a note-on arrives at capacity under the `reject-new` policy
- **THEN** existing voices SHALL continue unchanged and no new voice SHALL activate

#### Scenario: Sibling poly nodes allocate independently

- **WHEN** two poly nodes receive different note streams in one graph
- **THEN** each SHALL allocate, steal, retire, and mix voices through its own runtime region without affecting the other

### Requirement: Per-voice intrinsic ports

Inside a definition instantiated by `poly`, the engine SHALL provide intrinsic input ports carrying the activating note's pitch (control), velocity (control), and gate (event) for each voice instance.

#### Scenario: Voice reads note and velocity

- **WHEN** a voice definition connects its oscillator pitch to the note intrinsic and an amplitude path to the velocity intrinsic
- **THEN** each active voice SHALL render with its own activating note's pitch and velocity

#### Scenario: Gate reflects note lifetime

- **WHEN** a note-off arrives for an active voice
- **THEN** that voice's gate intrinsic SHALL deliver the release event while other voices are unaffected

### Requirement: Voice completion detection

A voice definition MAY expose a designated `done` output; when present, the poly node SHALL retire the voice when `done` signals completion. When absent, the poly node SHALL retire a voice after gate release once its audio outputs remain below a documented silence threshold for a documented duration. Retired voices SHALL NOT contribute stale output to later blocks.

#### Scenario: Explicit done retires voice

- **WHEN** a voice definition signals its `done` output
- **THEN** the poly node SHALL retire that voice and make it available for reuse

#### Scenario: Silence tracking retires voice

- **WHEN** a voice without a `done` output receives gate release and its output stays below the silence threshold for the documented duration
- **THEN** the poly node SHALL retire that voice

#### Scenario: Retired voice output is clean

- **WHEN** a retired voice's instance is inspected in subsequent blocks
- **THEN** its buffers SHALL NOT contribute to the poly node's summed outputs

### Requirement: Poly output mixing

The poly node SHALL sum each wrapped audio output across active voices into a single output port of the same channel count. Per-voice level shaping (e.g. velocity scaling) SHALL be composed inside the voice definition rather than built into the poly node.

#### Scenario: Two voices sum on one output

- **WHEN** two voices produce audio simultaneously
- **THEN** the poly node's output SHALL be their sample-wise sum on the matching output port

#### Scenario: No hidden velocity scaling

- **WHEN** a voice definition applies no velocity shaping internally
- **THEN** notes of different velocities SHALL produce identical poly output levels apart from the voice's own behavior
