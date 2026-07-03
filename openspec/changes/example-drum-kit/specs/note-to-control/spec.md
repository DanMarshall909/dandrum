## ADDED Requirements

### Requirement: note_to_control extracts velocity from NoteOn events

The `note_to_control` module SHALL accept an `events` event input. When a `NoteOn` event is received, it SHALL output
`velocity / 127.0` as a control signal. When a `NoteOff` event is received, it SHALL output 0.0.

#### Scenario: NoteOn produces velocity-proportional control signal

- **WHEN** a NoteOn event with velocity 64 is received
- **THEN** the output control value SHALL be approximately 0.504 (64/127, within 1%)

#### Scenario: NoteOff resets to zero

- **WHEN** a NoteOn event has been received and then a NoteOff event for the same note is received
- **THEN** the output control value SHALL be 0.0

#### Scenario: Multiple notes track latest velocity

- **WHEN** a NoteOn with velocity 100 is followed by a different-note NoteOn with velocity 50
- **THEN** the output SHALL reflect velocity 50 (50/127) while the second note is active

### Requirement: note_to_control produces frame-rate control output

The `note_to_control` module SHALL output a control signal with `frames` samples, where each sample equals the current
velocity-scaled value.

#### Scenario: Output length matches frame count

- **WHEN** the module is processed for 128 frames
- **THEN** the output control vector SHALL have exactly 128 samples

#### Scenario: Output is constant while no event occurs

- **WHEN** a NoteOn has been received and the module is processed for multiple subsequent blocks with no new events
- **THEN** the output SHALL remain at the same velocity-scaled value

### Requirement: note_to_control is registered and dispatchable

The note_to_control module SHALL be registered in the built-in module registry, have a corresponding
`ModuleKind::NoteToControl` variant, and be dispatchable at render time.

#### Scenario: note_to_control is in built-in registry

- **WHEN** the registry is queried for the `note_to_control` module type
- **THEN** a definition SHALL be returned

#### Scenario: note_to_control renders without error

- **WHEN** a patch containing a note_to_control module is loaded, prepared, and rendered
- **THEN** rendering SHALL complete without panic or error
