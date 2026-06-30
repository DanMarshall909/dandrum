## ADDED Requirements

### Requirement: Prepared realtime render state
The engine SHALL prepare patch validation, graph construction, asset loading, and processor state before a realtime audio callback renders audio.

#### Scenario: Patch load prepares render state outside callback
- **WHEN** a valid patch is loaded for realtime instrument use
- **THEN** the engine SHALL validate the patch, prepare assets, construct the graph processor, and expose a render-ready state before the audio callback starts rendering it

#### Scenario: Realtime render does not parse patch data
- **WHEN** the realtime audio callback renders an audio block
- **THEN** it SHALL NOT parse YAML, access the filesystem, or prepare sample assets during that callback

### Requirement: Bounded non-blocking event handoff
MIDI and control input callbacks SHALL hand events to the realtime renderer through a bounded non-blocking queue or equivalent wait-free handoff.

#### Scenario: MIDI note event is rendered by later audio block
- **WHEN** the MIDI callback submits a note-on event before the next audio block is rendered
- **THEN** the realtime renderer SHALL consume the event and produce the same note behavior as a direct engine note-on call

#### Scenario: Full event queue reports dropped input
- **WHEN** the MIDI or control callback submits an event and the bounded queue has no capacity
- **THEN** the submit call SHALL return a dropped-event status without blocking the callback

### Requirement: Audio callback avoids blocking primitives
The realtime audio callback path SHALL NOT wait on mutexes, critical sections, condition variables, logging streams, allocation-heavy patch loading, or other blocking primitives.

#### Scenario: JUCE render callback does not lock engine state
- **WHEN** `RustEngineSource::getNextAudioBlock` is called by JUCE
- **THEN** it SHALL render or clear the requested block without acquiring the shared engine critical section

#### Scenario: MIDI callback does not perform console IO
- **WHEN** `MidiToRustEngine::handleIncomingMidiMessage` receives note events
- **THEN** it SHALL submit events without writing to `std::cout` or `std::cerr`

### Requirement: Realtime render uses bounded scratch state
The realtime graph renderer SHALL allocate required per-block scratch storage during preparation or explicit resizing, not during steady-state audio callback rendering.

#### Scenario: Repeated fixed-size blocks reuse scratch buffers
- **WHEN** the same prepared realtime graph renders multiple audio blocks with the prepared maximum block size
- **THEN** it SHALL reuse prepared scratch buffers for module outputs and event routing rather than allocating new vectors or maps per block

#### Scenario: Oversized callback block is handled explicitly
- **WHEN** the audio callback receives a block larger than the prepared maximum block size
- **THEN** the engine SHALL either split the render into prepared-size chunks or return a clear diagnostic/fallback without unbounded allocation in the callback

### Requirement: Realtime output remains deterministic
The realtime instrument renderer SHALL produce deterministic output for the same prepared patch, input event sequence, sample rate, and block partitioning.

#### Scenario: Same queued events produce identical realtime blocks
- **WHEN** two fresh realtime engines prepare the same patch and receive the same queued events before rendering the same block sequence
- **THEN** their rendered audio buffers SHALL be identical within the engine's sample format
