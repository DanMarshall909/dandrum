## ADDED Requirements

### Requirement: Oscillator generates sawtooth audio from gate events
The oscillator module SHALL generate a sawtooth waveform at the pitch extracted from the most recent MIDI note-on event, starting when a gate event is received.

#### Scenario: Oscillator produces non-silent output when gated
- **WHEN** a note-on gate event is received
- **THEN** the oscillator output SHALL contain non-zero audio samples at the corresponding pitch

### Requirement: ADSR envelope follows gate events
The ADSR module SHALL produce a control envelope that rises on gate-on and decays to sustain, then releases on gate-off.

#### Scenario: ADSR output is non-zero after gate-on
- **WHEN** a gate-on event is received
- **THEN** the ADSR output SHALL produce a positive control value for a period after the event

### Requirement: VCA multiplies audio by control input
The gain/VCA module SHALL multiply its audio input sample-by-sample by its control input value.

#### Scenario: VCA passes audio when gain is non-zero
- **WHEN** audio and a non-zero control signal are present at VCA inputs
- **THEN** the VCA output SHALL be the element-wise product of audio and control
