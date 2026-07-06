## ADDED Requirements

### Requirement: Drum voices are authored from reusable synthesis primitives first

Dandrum SHALL prefer 808/909-style drum voices authored as ordinary instrument graphs using reusable DSP primitives before introducing bespoke `808_kick`, `909_snare`, or similar special-case module types.

#### Scenario: Author creates an 808-style kick instrument

- **GIVEN** Dandrum has primitives such as oscillator, decay/envelope, gain/multiply, filter, saturator, impulse, mixer, and output modules
- **WHEN** an 808-style kick is authored
- **THEN** the voice SHALL be represented as a normal YAML instrument graph composed from those primitives
- **AND** the public controls SHALL be declared through `preset_surface.parameters`
- **AND** no dedicated 808 kick module SHALL be introduced unless the primitive graph proves impractical, too slow, or sonically inadequate

#### Scenario: Author creates a 909-style instrument set

- **GIVEN** 909 voices include both analog-style synthesized voices and sample/ROM-like voices
- **WHEN** a 909-style instrument is authored
- **THEN** kick, snare, tom, and clap voices SHOULD be attempted with reusable synthesis primitives first
- **AND** hats, crash, and ride MAY use sampler-backed assets where synthesized primitives are not an accurate or practical model

### Requirement: Core drum synthesis gaps are explicit and primitive-oriented

The engine SHALL close drum-synthesis gaps by improving reusable primitives rather than adding machine-specific voice modules as the first option.

#### Scenario: A synthesized drum voice requires stable oscillator tuning

- **GIVEN** a drum voice needs a tuned body oscillator independent of keyboard tracking
- **WHEN** the instrument graph is prepared
- **THEN** the oscillator primitive SHOULD support an explicit frequency-oriented control path or a clearly documented pitch-ratio mapping
- **AND** the oscillator SHOULD support waveform selection at least for sine and the existing ramp/saw behaviour

#### Scenario: A synthesized drum voice requires tweakable decay

- **GIVEN** a drum voice exposes decay as a public parameter
- **WHEN** the public parameter changes at runtime
- **THEN** the mapped decay/envelope target SHOULD update the runtime control value without rewriting or reparsing YAML
- **AND** decay shaping SHOULD support at least linear and exponential curves

#### Scenario: A synthesized drum voice requires transient excitation

- **GIVEN** a drum voice needs a click, snap, or impulse excitation
- **WHEN** the graph is authored
- **THEN** the author SHOULD be able to combine impulse, noise, filter, gain, and decay primitives to create the transient
- **AND** any later transient-specific primitive SHALL remain general-purpose rather than 808/909-specific

### Requirement: Drum parameter defaults may be seeded from free/open reference instruments

Dandrum SHALL allow authored drum instruments to use parameter defaults derived from documented, free, or open reference instruments as initial seed values, provided those values are treated as tuning hints rather than copied implementation assets.

#### Scenario: Author seeds an 808-style kick control surface

- **GIVEN** a free/open 808-style synth, public documentation, or permissively inspectable preset provides representative values for tune, decay, pitch sweep, click, drive, or tone
- **WHEN** those values are converted into a Dandrum YAML instrument
- **THEN** the Dandrum instrument MAY use them as initial `preset_surface.parameters[*].default` values
- **AND** the values SHALL be converted into Dandrum's own parameter ranges and units
- **AND** the resulting graph SHALL remain a Dandrum-authored implementation, not copied source code or copied sample content

#### Scenario: Reference values come from multiple sources

- **GIVEN** different free/open synths or reference documents disagree on a drum parameter value
- **WHEN** a default is selected
- **THEN** the selected value SHOULD be conservative and musically useful
- **AND** the source and rationale SHOULD be documented near the authored instrument or in accompanying implementation notes
- **AND** later spectral analysis MAY replace the seed value with a fitted value

### Requirement: Drum presets are tuned later through offline spectral comparison

Dandrum SHALL treat free/open reference parameter values as starting points and use offline spectral/envelope comparison against reference samples as a later tuning step.

#### Scenario: A candidate drum preset is rendered for analysis

- **GIVEN** a Dandrum drum instrument has seed parameters
- **WHEN** the offline tuning workflow runs
- **THEN** it SHOULD render the candidate voice to audio
- **AND** compare it with one or more reference samples using spectral, amplitude-envelope, transient, and decay-tail metrics
- **AND** propose or apply parameter adjustments outside the realtime audio callback

#### Scenario: Spectral fitting updates a preset

- **GIVEN** spectral analysis identifies a better parameter set for a drum voice
- **WHEN** the preset is updated
- **THEN** only public parameter values or authored YAML defaults SHALL change
- **AND** the fitting process SHALL NOT introduce hidden graph mutations that bypass `preset_surface.parameters` or immutable instrument semantics
