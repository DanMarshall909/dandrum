## ADDED Requirements

### Requirement: Reverb SHALL use Schroeder/Moorer topology
The reverb SHALL consist of a parallel comb filter bank (minimum 4 combs) followed by series allpass diffusers (minimum 2). Comb delays SHALL be mutually prime to avoid constructive interference and metallic ringing. Each comb SHALL be a pipeline: `DelayLine` → `OnePoleFilter` (damping) → feedback gain.

#### Scenario: Impulse response has dense tail
- **WHEN** a single impulse is sent through the reverb with default settings
- **THEN** the output SHALL produce an initial reflection cluster followed by a smoothly decaying diffuse tail, with no audible metallic ringing at default settings

### Requirement: Reverb SHALL support configurable decay time (RT60)
The reverb SHALL accept a `decay_time` or `rt60` parameter (0.1–30.0 seconds) that controls the time for the reverberation energy to decay by 60 dB. Comb filter feedback gains SHALL be computed from the delay length and target RT60.

#### Scenario: Short decay produces brief tail
- **WHEN** RT60 is set to 0.5 seconds and an impulse is sent
- **THEN** the reverb tail SHALL decay to -60 dB within approximately 500ms

#### Scenario: Long decay produces sustained tail
- **WHEN** RT60 is set to 10.0 seconds and an impulse is sent
- **THEN** the reverb tail SHALL be audibly sustained for approximately 10 seconds

### Requirement: Reverb SHALL support configurable room size
The `room_size` parameter (0.0–1.0) SHALL scale the base comb delay times. Room size of 0.0 SHALL use minimum comb delays; 1.0 SHALL use maximum comb delays. Scaling SHALL be non-linear to map perceptually.

#### Scenario: Room size affects initial delay spread
- **WHEN** room_size is 0.2 vs 0.8
- **THEN** the comb delay times at 0.8 SHALL be longer, producing wider spacing between early reflections

### Requirement: Reverb SHALL support pre-delay
The reverb SHALL accept a `pre_delay` parameter (0–250ms) that delays the onset of the reverberation tail relative to the dry signal, simulating the time before early reflections arrive.

#### Scenario: Pre-delay separates dry from reverb
- **WHEN** pre_delay is 50ms and an impulse is sent
- **THEN** the reverb tail SHALL begin ~50ms after the impulse

### Requirement: Reverb SHALL support damping frequency via OnePoleFilter pipeline stage
The reverb SHALL accept a `damping` parameter (20–20000 Hz) that controls a `OnePoleFilter` (from `filter.rs`) inserted in each comb filter's feedback pipeline: `DelayLine` → `OnePoleFilter` → feedback gain. Higher damping frequencies produce brighter decay. The same `OnePoleFilter` struct SHALL be used as in the echo module.

#### Scenario: Damping controls brightness of tail
- **WHEN** damping is 500 Hz vs 10000 Hz
- **THEN** the high damping (500 Hz) case SHALL produce a darker, more muffled reverb tail; the low damping (10000 Hz) case SHALL produce a brighter tail

### Requirement: Reverb SHALL support diffusion density
The `diffusion` parameter (0.0–1.0) SHALL control the amount of allpass diffusion. Higher diffusion SHALL increase echo density in the early part of the decay, making the reverb sound smoother. At minimum diffusion, the comb resonances SHALL be more audible.

#### Scenario: High diffusion smooths response
- **WHEN** diffusion is 1.0 vs 0.0 with identical decay and room size
- **THEN** the 0.0 diffusion case SHALL produce more distinct comb resonances; the 1.0 case SHALL produce a smoother, denser tail

### Requirement: Reverb SHALL support stereo width
The `stereo_width` parameter (0.0–1.0) SHALL control the stereo image of the reverb tail. At 0.0, the reverb SHALL be mono (identical left and right outputs). At 1.0, the left and right diffuser chains SHALL have slightly different parameters for maximum stereo spread.

#### Scenario: Mono reverb produces identical channels
- **WHEN** stereo_width is 0.0 and a mono impulse is sent
- **THEN** the left and right outputs SHALL be identical

#### Scenario: Full stereo width decorrelates channels
- **WHEN** stereo_width is 1.0 and a mono impulse is sent
- **THEN** the left and right outputs SHALL be audibly decorrelated

### Requirement: Reverb SHALL support wet/dry mix
The reverb SHALL provide independent wet and dry gain control (0.0–1.0). The output SHALL be `dry * input + wet * reverbed_signal`.

#### Scenario: Full wet produces only reverberated signal
- **WHEN** wet is 1.0, dry is 0.0, and an impulse is sent
- **THEN** only the reverb tail SHALL be present in the output; no dry impulse

### Requirement: Reverb SHALL expose built-in module ports
The reverb SHALL be registered as a built-in module with: Audio input L+R, Audio output L+R, Control inputs for `decay_time`, `room_size`, `pre_delay`, `damping`, `diffusion`, `stereo_width`, `wet`, `dry`.

#### Scenario: Module registration has correct ports
- **WHEN** built-in module registry is queried for `reverb`
- **THEN** the port definitions SHALL match the specification above

### Requirement: Reverb SHALL process in global (non-voice) scope
The reverb effect SHALL be a global-scope module — it processes summed mix, not per-voice.

#### Scenario: Reverb is global scope
- **WHEN** a patch contains a reverb module after a voice-scoped oscillator chain
- **THEN** the reverb SHALL process the summed output of all voices

### Requirement: Reverb SHALL have a YAML composite module definition example
A YAML example patch SHALL be created at `examples/patches/composite-reverb.yaml` that defines a reverb effect as a composite module built from primitive modules (delay_line, one_pole_filter, gain, audio_mixer). The composite SHALL demonstrate the comb+allpass signal flow: parallel combs → allpass chain → mix output.

#### Scenario: Composite reverb example loads without error
- **WHEN** the composite reverb example patch is loaded and validated
- **THEN** it SHALL produce no validation errors
