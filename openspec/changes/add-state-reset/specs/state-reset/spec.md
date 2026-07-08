## ADDED Requirements

### Requirement: Voice-scoped module state SHALL be reset on note retrigger

When a voice is allocated for a new note-on, the engine SHALL reset the stateful
per-module state of every voice-scoped module in that voice (e.g. `filter`,
`envelope_follower`) before the note is rendered, so the note does not inherit
state from the voice's previous note. Modules without meaningful carried state
(e.g. pure functions) SHALL be unaffected.

#### Scenario: Retriggered resonant filter starts clean

- **WHEN** a voice running a high-resonance `filter` is stolen and re-triggered by a
  new note-on
- **THEN** the filter's internal state SHALL be cleared before the new note renders,
  so the note's onset does not contain the decaying tail of the previous note

#### Scenario: Global effect tails are not reset by note activity

- **WHEN** any note-on or note-off occurs
- **THEN** global (non-voice-scoped) effects such as `reverb` and `echo` SHALL NOT be
  reset, and their tails SHALL continue uninterrupted

### Requirement: The engine SHALL provide an explicit reset that clears all state

The engine SHALL provide a reset/panic operation that stops all active voices and
cascades `reset()` to every stateful module, including global effects, clearing
reverb/echo/delay tails and processor state. After reset, rendering silence SHALL
produce silence.

#### Scenario: Engine reset clears reverb tail

- **WHEN** a reverb is excited by an impulse and then the engine reset is invoked
- **THEN** subsequent silent input SHALL render silence (the reverb tail SHALL be
  cleared) rather than the decaying tail continuing

#### Scenario: Engine reset stops active voices

- **WHEN** one or more voices are active and the engine reset is invoked
- **THEN** all voices SHALL become inactive and no further audio SHALL be produced from
  them without a new note-on

### Requirement: Host callers SHALL be able to trigger an engine reset

The engine reset SHALL be reachable from the FFI boundary so a host can implement
panic / all-notes-off / patch reload. The export SHALL be additive and SHALL NOT change
existing FFI symbols.

#### Scenario: Host invokes reset over FFI

- **WHEN** the host calls the engine reset export on a loaded engine
- **THEN** the engine SHALL clear voice and effect state as specified above and remain
  usable for subsequent note-on and render calls
