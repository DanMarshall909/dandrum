## Context

The engine currently has no dynamics processing. The Moog ladder's per-stage `tanh()` is the only nonlinear element in the codebase. The headless engine design explicitly deferred dynamics (limiter, saturator) to "later" as musical safety tools. With the graph processor, FFT infrastructure (`fft.rs`, `spectral.rs`), crossover/splitter (`crossover.rs`), and sampler asset system now in place, the foundations for building dynamics modules exist.

The project pattern is:
- DSP primitives live in dedicated modules (`filter.rs`, `crossover.rs`) with a trait interface (`FilterAlgorithm { process(), reset() }`)
- Module definitions are registered in `builtins.rs` with typed named ports
- Per-instance state is stored in `PerModuleState` enum variants in `graph_processor.rs`
- Processing dispatch matches on module type string and reads inputs through a `ModuleInputProvider` trait
- Sample-level audio file loading lives in `sample.rs` (`load_pcm_wav`, `PreparedSamplerAssets`)

## Goals / Non-Goals

**Goals:**
- Implement `dynamics-processor` module with a unified gain computer supporting two modes:
  - **`level` mode**: two independent slopes (`below_ratio`, `above_ratio`) around a configurable threshold — covers compressor, limiter, gate, expander, upward compressor in one. Configurable attack/release/knee/detection/topology, optional sidechain input.
  - **`transient` mode**: attack/sustain phase shaping via envelope direction tracking — covers transient shaper/designer.
- Implement `saturator` module with drive, bias, and a `WaveshaperCurve` trait; ship `tanh`, `hard_clip`, `soft_clip`, and `sinfold` built-in curves
- Implement `convolution` module for FFT-based partitioned convolution (cabinet emulation, reverb); load IRs via sampler asset system
- Extract WAV/audio loading from `sample.rs` into shared `audio_loading.rs` so convolution and other future modules reuse it
- Wire all three modules into `builtins.rs`, `PerModuleState`, and `graph_processor.rs` dispatch
- Write unit tests (impulse response, gain reduction curves, waveshaper transfer, gate/expander curves, transient shapes) and FFT-based acceptance tests for each module

**Non-Goals:**
- Multi-band dynamics (user pairs individual modules with frequency splitter for multiband chains)
- Dynamic EQ (can be built later from filter + dynamics-processor)
- De-esser (can be built via frequency splitter + dynamics-processor sidechain)
- Dedicated clipper (clipper mode is `hard_clip` saturator curve with the dynamics-processor for envelope protection)
- Real-time visualization or GUI
- Modifying existing patch YAML format
- Custom IR import beyond the sampler asset system

## Decisions

### Dynamics Processor: unified two-slope gain computer with two modes

Compressor, limiter, gate, expander, and upward compressor are all the same device — a gain computer that maps envelope level to gain reduction — differing only in which side of the threshold is attenuated and by how much:

| Device | Below threshold | Above threshold |
|--------|----------------|-----------------|
| Compressor | 1:1 (pass) | N:1 (reduce) |
| Limiter | 1:1 (pass) | ∞:1 (brickwall) |
| Gate | 0:1 (mute) | 1:1 (pass) |
| Downward expander | M:1, M<1 (reduce more) | 1:1 (pass) |
| Upward compressor | N:1, N>1 (boost) | 1:1 (pass) |

A single **`dynamics-processor`** module exposes both slopes as independent parameters:

```
gain_db(scaled_envelope) =
    if envelope_db > threshold:
        (envelope_db - threshold) * (1 - 1/above_ratio)
    else:
        (envelope_db - threshold) * (1 - 1/below_ratio)
```

With `below_ratio` defaulting to 1:1 and `above_ratio` to e.g. 4:1, it's a conventional compressor. Set `above_ratio` to 40:1+ for a limiter. Set `below_ratio` to 0:1 for a gate. Set `below_ratio` to 0.5:1 (attenuate below threshold) for an expander. Set `above_ratio` to 0.5:1 and `below_ratio` to 1:1 for upward compression.

A `topology` parameter switches between feed-forward (clean, precise) and feedback (classic character, one-sample delayed detection).

A `detection` parameter switches between RMS and peak envelope following.

#### Transient mode: attack/sustain shaping

In `transient` mode, the gain computer switches from threshold-based to direction-based:

- Track whether the envelope is rising (attack phase) or falling (sustain/release phase)
- Apply `attack_gain` (dB boost/cut) during the attack phase
- Apply `sustain_gain` (dB boost/cut) during the sustain phase
- A short hysteresis window prevents chatter at the attack/sustain transition

This covers transient shaper/designer: punch up drums by boosting attack and cutting sustain, or soften transients by cutting attack and boosting sustain.

Ports: `audio_in`, `sidechain_in` (Control type), `audio_out`.

Parameters:
- `mode`: level | transient
- Level mode: `threshold` (-80 to 0 dB), `below_ratio` (0:1 to 20:1), `above_ratio` (1:1 to 40:1), `knee` (0 to 12 dB), `attack` (0.1 to 100 ms), `release` (10 ms to 3 s), `makeup_gain` (0 to 24 dB), `detection` (rms | peak), `topology` (feedforward | feedback)
- Transient mode: `attack_gain` (-24 to +24 dB), `sustain_gain` (-24 to +24 dB), `attack` (0.1 to 100 ms), `release` (10 ms to 3 s), `detection` (rms | peak)

Implementation approach:
- `EnvelopeDetector` struct with separate attack/release time constants (exponential smoothing, two-time-constant), RMS and peak modes
- `GainComputer` struct: in `level` mode computes gain reduction from two-slope transfer with optional soft knee; in `transient` mode computes direction from envelope first derivative (with hysteresis) and applies phase gains
- `DynamicsProcessor` struct composes envelope detector, gain computer, optional sidechain input routing
- Follows `FilterAlgorithm`-style trait pattern: `process(&mut self, input: f32) -> f32`

### Saturator: trait-based waveshaper with built-in curves

(Same as before — unchanged.)

Separate the waveshaping function from the module so curves are pluggable:

```
pub trait WaveshaperCurve: Send {
    fn process(&self, sample: f64) -> f64;
    fn name(&self) -> &'static str;
}
```

Built-in implementations:
- `TanhCurve` — `tanh(drive * sample)`, smooth symmetric soft clip
- `HardClipCurve` — clamp to [-1, 1] after drive
- `SoftClipCurve` — polynomial soft clip (3rd or 5th order), warmer than tanh
- `SinFoldCurve` — `sin(drive * sample)`, wavefolding for extreme harmonics

The saturator module holds a `Box<dyn WaveshaperCurve>` selected by the `curve` parameter. Drive is a control input (0–24 dB pre-gain). Bias shifts the curve center for asymmetric clipping (tube-style odd harmonics).

Ports: `audio_in`, `audio_out`. Parameters: `drive` (0 to 24 dB), `bias` (-1 to 1), `curve` (tanh | hard_clip | soft_clip | sinfold).

### Convolution: FFT-based partitioned convolution

(Same as before — unchanged.)

For cabinet emulation and linear effects, use partitioned convolution with the overlap-add method from `spectral.rs`. The impulse response is loaded from a WAV file via the sampler asset system.

Partitioned convolution splits the IR into fixed-size blocks, processes each block with FFT, and accumulates the overlap-add output. This keeps latency low compared to full FFT convolution of long IRs.

Design:
- `ConvolutionEngine` struct owns the partitioned IR, FFT plans, and overlap-add state
- `PartitionedIR` loads an IR, partitions it (default 256-sample partitions), computes FFT of each partition
- Process: for each audio block, FFT the input, multiply with each partition's frequency response, IFFT, overlap-add
- Reuses `SpectralProcessor`'s Hann windowing and overlap-add approach
- IR assets loaded through `PreparedConvolutionAssets` (parallel to `PreparedSamplerAssets`), using extracted `audio_loading.rs` for WAV parsing

Ports: `audio_in`, `audio_out`. Parameters: `asset` (text, IR asset ID), `wet` (0–100%).

### Shared audio loading extraction

(Same as before — unchanged.)

The WAV loading and asset preparation in `sample.rs` applies to both sampler and convolution modules. Extract:
- `load_pcm_wav()` (currently in `sample.rs`)
- `AssetKind` and `ParameterValue` dispatch (currently in `sample.rs::prepare_sampler_assets`)
- Into `audio_loading.rs` as a general `load_pcm_wav()` + `PrepareAudioAssets` that can be used for any audio-file-based module

`sample.rs` re-exports or delegates to `audio_loading.rs` for backward compatibility. `PreparedSamplerAssets` stays in `sample.rs` but wraps the shared asset loading.

### EnvelopeDetector as a shared utility

The dynamics processor's envelope detector is the same primitive needed for other dynamics modules. Extract `EnvelopeDetector` to a shared utility (alongside, or within, the dynamics_processor module) with:
- Configurable attack/release time constants (converted to per-sample coefficients)
- RMS mode (moving average of squared signal) and peak mode (instantaneous with programmable hold)
- `process(sample) -> envelope` and `reset()`

### Module integration pattern

Following established pattern from filter/splitter work:
- Each module type registered in `builtins.rs` with port definitions
- `PerModuleState` enum gains variants: `DynamicsProcessor(DynamicsProcessorState)`, `Saturator(SaturatorState)`, `Convolution(ConvolutionState)`
- `PerModuleState::new()` constructs the appropriate variant from module parameters
- `process_module()` dispatch matches on module type, reads inputs, calls process, writes outputs
- Convolution assets prepared during `render_offline()`/`render_offline_compiled()` initialization (parallel to `prepare_sampler_assets`)

### Control mapping conventions

| Module            | Control        | Normalized 0–1 mapping |
|-------------------|----------------|-------------------------|
| DynamicsProcessor | threshold      | 0→−80 dB, 1→0 dB |
| DynamicsProcessor | below_ratio    | 0→0:1, 1→20:1 |
| DynamicsProcessor | above_ratio    | 0→1:1, 1→40:1 |
| DynamicsProcessor | attack         | 0→0.1 ms, 1→100 ms (log) |
| DynamicsProcessor | release        | 0→10 ms, 1→3 s (log) |
| DynamicsProcessor | knee           | 0→0 dB, 1→12 dB |
| DynamicsProcessor | makeup_gain    | 0→0 dB, 1→24 dB |
| DynamicsProcessor | attack_gain    | 0→−24 dB, 1→+24 dB |
| DynamicsProcessor | sustain_gain   | 0→−24 dB, 1→+24 dB |
| Saturator         | drive          | 0→0 dB, 1→24 dB |
| Saturator         | bias           | 0→−1, 1→+1 |
| Convolution       | wet            | 0→0%, 1→100% |

## Risks / Trade-offs

- **Sidechain latency**: Feed-forward topology creates a one-sample delay between envelope detection and gain application. Acceptable — equivalent to a lookahead of 1 sample at the sample rate.
- **Transient mode hysteresis**: The attack/sustain transition needs hysteresis to avoid rapid flickering on sustained but noisy signals. Adds state complexity but essential for musical behavior.
- **Below_ratio at zero (gate)**: A below_ratio of 0:1 means zero gain below threshold — effectively muting. Need to ensure smooth fade-out rather than hard cut to avoid zipper noise. Exponential fade follows release time constant.
- **Convolution CPU cost**: Partitioned convolution with 256-sample partitions at 48 kHz is ~2× realtime on modern CPUs per convolution module for IRs up to 1 s. Mitigated by making it optional and documenting that heavy convolution chains should be used in offline rendering.
- **Waveshaper alias**: Non-linear waveshaping produces harmonics above Nyquist, causing aliasing. Mitigated by 2x oversampling option in the saturator (can be added post-MVP if needed). The basic version operates at sample rate.
- **Asset system extraction**: Moving `load_pcm_wav` to shared module could break existing sampler integration if not done carefully. Mitigated by keeping the same public API via re-exports.
- **FFT-based convolution IR length**: Very long IRs (>2 s) blow up partition count. Mitigated by clamping IR length at load time (max 4 s, flag if longer found).
