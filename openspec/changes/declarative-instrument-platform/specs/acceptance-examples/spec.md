## ADDED Requirements

### Requirement: Acceptance examples validate platform capability

The engine SHALL provide YAML patch files as acceptance examples that demonstrate each category of instrument expressible through primitives, composites, scripts, and presets.

#### Scenario: Each example loads and renders

- **WHEN** an acceptance example YAML patch is loaded by the engine
- **THEN** loading SHALL succeed without errors and the patch SHALL render deterministically

### Requirement: Synthetic 808-style kick

A YAML composite or patch SHALL demonstrate a synthetic 808-style kick drum using oscillator (sine, pitch envelope), noise (attack click), and VCA modules.

**Required primitives**: oscillator (sine), noise, gain/VCA, ADSR (pitch envelope via note-to-control/multiply), mixer, audio output

#### Scenario: 808 kick composite renders

- **WHEN** the 808 kick composite receives a trigger event
- **THEN** it SHALL produce a low-frequency sine sweep with an initial attack click, mixed to a single audio output

### Requirement: Synthetic 909-style kick

A YAML composite SHALL demonstrate a synthetic 909-style kick drum using oscillator (sine, pitch envelope), noise, and distortion/saturation modules.

**Required primitives**: oscillator (sine), noise, gain/VCA, ADSR, saturation, mixer, audio output

#### Scenario: 909 kick composite renders

- **WHEN** the 909 kick composite receives a trigger event
- **THEN** it SHALL produce a punchier low-frequency sine sweep with saturation and a noise attack component

### Requirement: Synthetic snare

A YAML composite SHALL demonstrate a synthetic snare drum using noise (tonal body + noise snap), oscillator (for body tone), VCA, and mixer modules.

**Required primitives**: oscillator (sine/triangle), noise, gain/VCA, ADSR (×2 for body and noise envelopes), mixer, audio output

#### Scenario: Snare composite renders

- **WHEN** the snare composite receives a trigger event
- **THEN** it SHALL produce a tonal body with a noise component, mixed to audio output

### Requirement: Closed/open hi-hat pair

A YAML composite SHALL demonstrate closed and open hi-hat voices using noise, VCA with short/long envelopes, and a filter module.

**Required primitives**: noise, gain/VCA, ADSR (short for closed, longer for open), filter (high-pass), mixer, audio output

#### Scenario: Hi-hat composite renders

- **WHEN** the closed hi-hat composite receives a trigger event
- **THEN** it SHALL produce a short filtered noise burst

#### Scenario: Open hi-hat renders with longer decay

- **WHEN** the open hi-hat composite receives a trigger event
- **THEN** it SHALL produce a filtered noise burst with a longer decay than the closed variant

### Requirement: Basic subtractive synth voice

A YAML composite SHALL demonstrate a subtractive synthesis voice using oscillator (saw/pulse), filter, ADSR (amp and filter envelopes), VCA, and note-to-control modules.

**Required primitives**: oscillator (saw/pulse), filter (low-pass), ADSR (×2), gain/VCA, note-to-control, audio output

#### Scenario: Subtractive synth voice produces note

- **WHEN** the subtractive synth composite receives a note-on event with pitch and velocity
- **THEN** it SHALL produce a filtered oscillator tone with amplitude and filter envelope contours

### Requirement: Basic sampler voice

A YAML composite SHALL demonstrate a sampler voice using the sampler module with pitch mapping and amplitude envelope.

**Required primitives**: sampler, gain/VCA, ADSR, note-to-control, audio output

#### Scenario: Sampler voice plays sample

- **WHEN** the sampler voice composite receives a note-on event
- **THEN** it SHALL play the configured sample at the mapped pitch with an amplitude envelope

### Requirement: Drum machine event mapper

A YAML patch SHALL demonstrate a drum machine module mapping MIDI notes to named pad event outputs that trigger explicit downstream voice composites.

**Required primitives**: drum machine, oscillator, noise, gain/VCA, ADSR, mixer, audio output

#### Scenario: Drum machine triggers voice

- **WHEN** a MIDI note-on event is received by the drum machine on a configured pad
- **THEN** the drum machine SHALL emit a pad event that triggers the connected voice composite

### Requirement: Effects rack

A YAML composite SHALL demonstrate an effects chain using delay, reverb, filter, mixer, and gain modules.

**Required primitives**: delay, reverb, filter, gain/VCA, mixer, audio output

#### Scenario: Effects rack processes audio

- **WHEN** an audio signal is sent through the effects rack composite
- **THEN** it SHALL produce a processed output with delay, reverb, and filtering applied

### Requirement: Script event/control mapping

A YAML patch SHALL demonstrate a script module transforming MIDI events into control values for modulation or velocity mapping.

**Required primitives**: script module, note-to-control, gain/VCA, oscillator, audio output

#### Scenario: Script maps velocity to modulation

- **WHEN** a note-on event is processed by a script module that maps velocity to a control output range
- **THEN** the control output SHALL reflect the mapped velocity value (e.g., velocity 64 → 0.5)
