## ADDED Requirements

### Requirement: Sampler module definition
The engine SHALL provide a built-in `sampler` signal-generator module with an event input port named `trigger`, control inputs for playback rate, sample playback position, and loop behavior, and an audio output port named `audio`.

#### Scenario: Sampler ports are available
- **WHEN** a patch declares a module with type `sampler`
- **THEN** graph validation accepts routes from event outputs to `sampler.trigger` and from `sampler.audio` to compatible audio inputs

#### Scenario: Sampler playback controls are available
- **WHEN** a patch declares a module with type `sampler`
- **THEN** graph validation accepts compatible control routes to sampler playback-control inputs

### Requirement: Sample asset reference validation
Each sampler module SHALL reference a declared sample asset using a text parameter named `asset`.

#### Scenario: Sampler references declared sample asset
- **WHEN** a sampler module parameter `asset` matches an asset declaration with kind `sample`
- **THEN** patch validation accepts the sampler asset reference

#### Scenario: Sampler omits asset reference
- **WHEN** a sampler module omits the `asset` parameter
- **THEN** validation fails with a diagnostic identifying the sampler module and the missing `asset` parameter

#### Scenario: Sampler references missing asset
- **WHEN** a sampler module parameter `asset` names no declared asset
- **THEN** validation fails with a diagnostic identifying the sampler module and missing asset ID

#### Scenario: Sampler references non-sample asset
- **WHEN** a sampler module parameter `asset` names an asset whose kind is not `sample`
- **THEN** validation fails with a diagnostic identifying the sampler module, asset ID, and expected sample kind

### Requirement: Sample file loading
The engine SHALL load sampler sample assets before rendering and SHALL reject unsupported sample files with clear diagnostics.

#### Scenario: Supported WAV sample loads
- **WHEN** a sampler references a readable supported PCM WAV sample asset
- **THEN** render preparation succeeds and sample data is available to the sampler processor

#### Scenario: Missing sample file is reported
- **WHEN** a sampler references a sample asset path that cannot be read
- **THEN** render preparation fails with a diagnostic identifying the asset ID and path

#### Scenario: Unsupported sample format is reported
- **WHEN** a sampler references a file that is not a supported PCM WAV sample
- **THEN** render preparation fails with a diagnostic identifying the asset ID, path, and unsupported format

### Requirement: Event-triggered sample playback
The sampler SHALL start monophonic playback on each routed event received at `trigger` and SHALL output the sample as an audio source without applying MIDI note or velocity semantics.

#### Scenario: Trigger event starts sample audio
- **WHEN** an event reaches `sampler.trigger` during rendering
- **THEN** `sampler.audio` outputs the loaded sample starting at that event's render position

#### Scenario: Trigger payload does not scale sample audio
- **WHEN** two renders trigger the same sampler with events whose payloads carry different MIDI velocity values
- **THEN** the sampler output amplitude is identical before any downstream gain or envelope modules are applied

#### Scenario: Playback rate controls sample pitch
- **WHEN** two renders trigger the same sampler with different routed `sampler.rate` control values
- **THEN** the sampler output uses different playback rates without inspecting MIDI note values

#### Scenario: Later trigger replaces monophonic playback
- **WHEN** an event reaches `sampler.trigger` while a previous sample playback is still active
- **THEN** the sampler restarts monophonic playback for the later event instead of creating an additional voice

#### Scenario: Playback ends after sample length
- **WHEN** a triggered sample reaches its final frame
- **THEN** `sampler.audio` outputs silence until another `NoteOn` event triggers playback

### Requirement: Sampler playback controls
The sampler SHALL expose explicit playback controls for playback rate, sample start position, and loop behavior so playback can be modulated through graph routing instead of hidden sampler-specific policy.

#### Scenario: Rate control changes playback speed
- **WHEN** a sampler receives a valid playback-rate control value before and during playback
- **THEN** playback advances according to the controlled rate

#### Scenario: Start control changes playback position
- **WHEN** a sampler receives a valid start-position control value before a trigger event
- **THEN** playback begins from the controlled sample position

#### Scenario: Loop controls constrain playback range
- **WHEN** a sampler receives valid loop control values and looping is enabled
- **THEN** active playback wraps within the controlled loop range until playback is retriggered or stopped

### Requirement: Trigger and pitch policy remains external
The sampler SHALL NOT decide MIDI note-to-pitch conversion, piano grouping, velocity gain, or multi-voice allocation internally; those policies SHALL be represented by upstream routing/control modules and future generic bus behavior.

#### Scenario: Piano group routing triggers sampler externally
- **WHEN** upstream patch routing sends a trigger event to `sampler.trigger`
- **THEN** the sampler responds to that event without inspecting or enforcing the piano group policy that selected it

#### Scenario: MIDI note-to-pitch conversion remains external
- **WHEN** upstream patch routing converts MIDI note events into a control signal connected to `sampler.rate`
- **THEN** the sampler follows the routed rate control without inspecting MIDI note payloads

#### Scenario: Polyphony is not sampler-specific
- **WHEN** multiple simultaneous notes target sampling behavior before generic per-voice buses exist
- **THEN** the sampler remains monophonic rather than allocating sampler-specific voices

### Requirement: Deterministic sampler rendering
Sampler rendering SHALL be deterministic for the same patch, asset files, render settings, control signals, and input events.

#### Scenario: Same sampler render repeats exactly
- **WHEN** the same sampler patch is rendered twice with the same sample asset and input events
- **THEN** both renders produce identical left and right audio buffers

### Requirement: Sampler example patch
The project SHALL include a minimal YAML patch example that routes event input to a sampler and then to audio output.

#### Scenario: Sampler example renders to WAV
- **WHEN** the sampler example patch is rendered through the CLI
- **THEN** the command succeeds and writes a non-empty WAV file
