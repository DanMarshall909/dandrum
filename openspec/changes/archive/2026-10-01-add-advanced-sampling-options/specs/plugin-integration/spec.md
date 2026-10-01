## ADDED Requirements

### Requirement: Sampler example exposes host-modulatable musical controls

The sampler VST3 example SHALL load a prepared drum patch with redistributable sample assets and expose stable public host parameters for live playback pitch, start position, level, pan, and variation. The kick, snare, closed hat, and open hat SHALL have separate pitch, start, level, and pan host parameters, with independent variation controls where the pad has alternates. A host parameter change SHALL reach the running Rust sample player without rebuilding the instrument. Source files, sample maps, voice limits, and choke modes SHALL remain preparation-time structure.

#### Scenario: Sampler VST3 loads a playable drum kit

- **WHEN** the sampler example is instantiated and the host sends mapped MIDI drum notes
- **THEN** the plugin SHALL render the prepared kit samples through its stereo output

#### Scenario: Host modulation changes a playing drum sound

- **WHEN** the host changes a public pitch, start, level, pan, or variation parameter between audio blocks
- **THEN** the corresponding sampler control SHALL change the rendered result without instrument replacement

#### Scenario: Host modulation addresses a single drum pad

- **WHEN** the host changes a public control for one mapped drum pad
- **THEN** the sampler SHALL change that pad's rendered sound without changing another pad's control or replacing the prepared kit

#### Scenario: Sampler structure stays prepared

- **WHEN** a host changes a public playback parameter
- **THEN** the plugin SHALL retain the prepared sample assets, map, voice limit, and choke policy until an explicit reload

#### Scenario: Host sample rate differs from the source file

- **WHEN** the sampler example prepares a 48000 Hz source for a 44100 or 96000 Hz host
- **THEN** it SHALL load before audio processing and play the same musical pitch and duration without realtime file access or allocation

#### Scenario: Modulated drum pitch respects source and host rates

- **WHEN** a 48000 Hz drum map source plays at a 44100 or 96000 Hz host with a valid nondefault pitch control
- **THEN** its signed waveform and duration SHALL follow that musical pitch multiplied by the prepared source-to-host rate ratio
