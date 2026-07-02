## ADDED Requirements

### Requirement: velocity_vca composite module
A `velocity_vca` composite module definition SHALL exist that combines `note_to_control`, `multiply`, and `gain` to produce velocity-scaled audio from events, envelope, and audio inputs.

#### Scenario: velocity_vca routes events through note_to_control
- **WHEN** the velocity_vca composite is loaded with events, envelope, and audio inputs wired
- **THEN** the composite SHALL produce audio output that equals `audio × envelope × velocity/127`

### Requirement: impulse_tone composite module
An `impulse_tone` composite module definition SHALL exist that produces a pitched percussive sound using oscillator + ADSR + velocity_vca. The composite SHALL expose an `events` event input and an `audio` audio output.

#### Scenario: impulse_tone triggers and produces audio
- **WHEN** a patch with impulse_tone receives a NoteOn event
- **THEN** it SHALL output audio with fundamental frequency in the 40–200 Hz range, scaled by velocity

### Requirement: impulse_noise composite module
An `impulse_noise` composite module definition SHALL exist that produces a noise-based percussive sound using noise + filter + ADSR + velocity_vca. The composite SHALL expose an `events` event input and an `audio` audio output.

#### Scenario: impulse_noise produces noise-based output
- **WHEN** a patch with impulse_noise receives a NoteOn event
- **THEN** it SHALL output noise-based audio shaped by the envelope and scaled by velocity

### Requirement: impulse_layer composite module
An `impulse_layer` composite module definition SHALL exist that produces a layered percussive sound using oscillator + noise + filter + ADSR + velocity_vca. The composite SHALL expose an `events` event input and an `audio` audio output.

#### Scenario: impulse_layer produces mixed tone and noise
- **WHEN** a patch with impulse_layer receives a NoteOn event
- **THEN** it SHALL output audio containing both tonal and noise components, scaled by velocity

### Requirement: Each impulse composite responds to MIDI velocity
All `impulse_*` composites SHALL scale their output amplitude proportionally to the velocity of the triggering NoteOn event, via the internal `velocity_vca` composite.

#### Scenario: Low velocity produces quieter output
- **WHEN** the same impulse composite is triggered with velocity 30 and then velocity 127
- **THEN** the peak output amplitude at velocity 30 SHALL be lower than at velocity 127

### Requirement: Complete drum kit patch
A `drum-kit` example patch SHALL exist that instantiates multiple `impulse_*` composites routed to different MIDI notes, with shared `midi_input` and `audio_output` modules.

#### Scenario: Drum kit patch loads and renders
- **WHEN** the drum-kit patch is loaded, prepared, and rendered with MIDI events
- **THEN** rendering SHALL complete without error and produce audio on the master output

#### Scenario: Drum kit patch has voice allocation
- **WHEN** the drum-kit patch metadata is inspected
- **THEN** `voice_allocation` SHALL be present and `max_voices` SHALL be at least the number of voice instances
