## ADDED Requirements

### Requirement: Patch declares voice allocation
The patch format SHALL support an optional patch-level voice allocation declaration that defines the maximum number of simultaneous voices and the voice-stealing policy.

#### Scenario: Missing voice allocation defaults to monophonic
- **WHEN** a valid patch omits the voice allocation declaration
- **THEN** the engine SHALL treat the patch as having one maximum voice and no stealing

#### Scenario: Polyphonic voice allocation is loaded
- **WHEN** a YAML patch declares a positive `max_voices` value greater than one
- **THEN** the loader SHALL make that voice limit available to graph preparation and rendering

#### Scenario: Invalid voice limit is rejected
- **WHEN** a YAML patch declares `max_voices` as zero or a negative value
- **THEN** patch validation SHALL fail with a diagnostic identifying the invalid voice allocation setting

### Requirement: MIDI notes allocate and release voices
The engine SHALL allocate one voice for each note-on event up to the declared voice limit and release the matching active voice when a note-off event for the same note is received.

#### Scenario: Overlapping notes use independent voices
- **WHEN** a patch with at least two voices receives note-on events for two different notes before either note is released
- **THEN** both notes SHALL remain active and be processed as independent voices

#### Scenario: Note-off releases matching note
- **WHEN** a patch receives note-on events for two notes and then receives a note-off for one of those notes
- **THEN** the released note SHALL stop or enter release processing while the other note remains active

#### Scenario: Repeated note-on can allocate another voice
- **WHEN** a patch receives repeated note-on events for the same MIDI note while voice capacity remains available
- **THEN** each note-on SHALL allocate a separate voice until the voice limit is reached

### Requirement: Voice stealing is deterministic
When the declared voice limit is reached and voice stealing is enabled, the engine SHALL choose a voice to steal using a deterministic policy.

#### Scenario: Oldest active voice is stolen
- **WHEN** all voice slots are active and a new note-on event arrives with oldest-active stealing enabled
- **THEN** the engine SHALL replace the oldest active voice with the new note

#### Scenario: Full allocator without stealing ignores new note
- **WHEN** all voice slots are active and a new note-on event arrives with stealing disabled
- **THEN** the engine SHALL leave existing voices active and ignore the new note-on event

#### Scenario: Voice stealing render is repeatable
- **WHEN** the same patch, settings, assets, and note sequence are rendered twice with voice stealing enabled
- **THEN** both renders SHALL produce identical audio samples

### Requirement: Voices are independent sub-synths
Modules classified as voice-scoped SHALL form a voice-local sub-synth with independent per-voice state, local routing, local modulation, and adjustable output processing so one active note does not overwrite another active note's oscillator, sampler, envelope, mixer, VCA, filter, or note-derived control state.

#### Scenario: Sampler voices overlap instead of replacing playback
- **WHEN** a polyphonic sampler patch receives two note-on events before the first sample playback finishes
- **THEN** the rendered output SHALL include both overlapping sample playbacks rather than replacing the first playback with the second

#### Scenario: Per-voice envelopes release independently
- **WHEN** two voices are active and a note-off releases one note
- **THEN** only the matching voice envelope SHALL enter release processing

#### Scenario: Per-voice controls adjust only their voice
- **WHEN** a voice-local control path modulates a VCA or filter inside a voice sub-synth
- **THEN** that modulation SHALL affect only the matching voice instance before voices are mixed globally

### Requirement: Voice-to-global routing remains explicit
The graph validator SHALL require explicit routing boundaries when voice sub-synth outputs feed global processing, and it SHALL NOT implicitly sum per-voice outputs into ordinary single-source inputs.

#### Scenario: Voice has local sub-synth before global mix
- **WHEN** a polyphonic patch routes multiple voice-scoped audio outputs through voice-scoped mixer, control, and output-shaping modules before crossing into global processing
- **THEN** each active voice SHALL have an independent instance of that sub-synth before the resulting voice outputs are mixed with other voices

#### Scenario: Per-voice audio routes through explicit mixer
- **WHEN** a polyphonic patch routes voice-scoped audio into a mixer input that explicitly accepts multiple sources before the audio output
- **THEN** graph validation SHALL succeed for the voice-to-global routing

#### Scenario: Per-voice audio cannot route directly to single-source input
- **WHEN** a polyphonic patch routes voice-scoped outputs directly into a global input that does not accept multiple sources
- **THEN** graph validation SHALL fail with a diagnostic naming the voice-to-global boundary and requiring an explicit mixer or summing module

### Requirement: Mixed signal families require explicit interaction modules
The graph validator SHALL allow different signal families to interact only at modules that explicitly declare conversion, merge, or polymorphic input behavior.

#### Scenario: Same signal type mixes through matching mixer
- **WHEN** a patch routes multiple audio signals into an audio mixer or multiple control signals into a control mixer
- **THEN** graph validation SHALL accept the same-type mixing behavior declared by that module

#### Scenario: Event to control uses converter module
- **WHEN** a patch routes MIDI events into a note-to-rate or velocity-to-control module and then routes the resulting control output onward
- **THEN** graph validation SHALL accept the event-to-control interaction because the converter module declares the conversion

#### Scenario: Audio to control uses detector module
- **WHEN** a patch routes audio into an envelope follower, gate extractor, or threshold detector module and then routes the resulting control or event output onward
- **THEN** graph validation SHALL accept the audio-derived control or event interaction because the module declares the conversion

#### Scenario: Implicit mixed-type routing is rejected
- **WHEN** a patch connects audio, control, and event signals directly to a port that does not explicitly declare polymorphic or conversion behavior
- **THEN** graph validation SHALL fail with a diagnostic identifying the incompatible signal families and requiring an explicit conversion, merge, or polymorphic module

### Requirement: Polyphonic rendering is deterministic
The offline renderer SHALL produce deterministic output for polyphonic patches when the patch, assets, render settings, and input events are unchanged.

#### Scenario: Same polyphonic render repeats exactly
- **WHEN** a polyphonic patch is rendered twice with the same events, assets, and settings
- **THEN** both renders SHALL produce identical left and right sample buffers
