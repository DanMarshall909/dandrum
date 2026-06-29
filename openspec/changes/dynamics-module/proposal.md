## Why

The headless engine design deferred dynamics processing (limiter, saturator) to "later" as musical safety tools. With FFT infrastructure, frequency splitting, and spectral processing now in the pipeline, dynamics modules are the next essential layer — they cover the gap between raw synthesis and polished, mix-ready output. Sidechain compression enables pumping/ducking effects critical for modern electronic music; convolution brings cabinet/speaker emulation and reverb; and an extensible waveshaping trait gives us a foundation for tube, tape, asymmetric, and other non-linear transfer curves.

Rather than implementing a dozen separate dynamics processors (compressor, limiter, gate, expander, transient shaper), we generalise into a single **dynamics-processor** module whose gain computer has two independently configurable slopes — one below threshold, one above — and two operational modes: level-based and transient-based. This one module replaces five traditional modules while covering combinations no fixed processor can.

## What Changes

- Add a `dynamics-processor` module with a unified gain computer that operates in two modes:
  - **`level` mode**: threshold, `below_ratio`, `above_ratio`, knee, attack, release, makeup gain — covers compressor (above_ratio > 1), limiter (above_ratio = ∞), gate (below_ratio = 0), expander (below_ratio < 1), and upward compressor (above_ratio < 1). Supports feed-forward and feedback detection topologies, and accepts an optional sidechain audio input (falling back to the main input when no sidechain is cabled).
  - **`transient` mode**: `attack_gain` and `sustain_gain` (each ±24 dB) control independent scaling of the attack and sustain phases by tracking the envelope's direction — covers transient shaper/designer use cases.
- Add a `saturator` module with drive, bias, and a pluggable waveshaper curve — ships with `tanh`, `hard_clip`, `soft_clip`, and `sinfold` curves; exposes a `WaveshaperCurve` trait for adding custom shapes without modifying core code
- Add a `convolution` module that loads an impulse response via the sampler asset system (with common audio-loading code extracted) and performs FFT-based partitioned convolution using the existing STFT overlap-add infrastructure
- Extract IR/audio file loading logic from the sampler into a shared utility so it can be reused without code duplication
- No breaking changes to existing patch format or modules
- Individual modules designed to pair cleanly with the frequency splitter for multi-band dynamics chains

## Capabilities

### New Capabilities
- `dynamics-processor`: Unified gain computer with `level` mode (compressor/limiter/gate/expander/upward compressor via below_ratio + above_ratio) and `transient` mode (attack/sustain shaping via envelope direction tracking). Sidechain input, RMS/peak envelope detection, feed-forward/feedback topology, configurable knee, and makeup gain.
- `dynamics-saturator`: Drive-based waveshaper module with extensible `WaveshaperCurve` trait and a set of built-in curves
- `dynamics-convolution`: FFT-based partitioned convolution module for cabinet emulation, reverb, and linear effects, using the sampler asset system for IR loading
- `shared-audio-loading`: Extracted common WAV/audio file loading from the sampler module into a shared utility for reuse by convolution and other future modules

### Modified Capabilities
- (none)

## Impact

- `src/rust-engine/src/dynamics_processor.rs`: new module — unified envelope detector, gain computer with two-slope transfer and two modes, sidechain routing
- `src/rust-engine/src/saturator.rs`: new module — `WaveshaperCurve` trait, built-in curve implementations, per-sample waveshaping
- `src/rust-engine/src/convolution.rs`: new module — partitioned convolution engine, IR management, overlap-add integration
- `src/rust-engine/src/audio_loading.rs`: new shared module — extracted WAV loading utilities from `sample.rs`
- `src/rust-engine/src/sample.rs`: refactor to delegate to `audio_loading.rs` for file I/O
- `src/rust-engine/src/builtins.rs`: add `dynamics_processor_definition()`, `saturator_definition()`, `convolution_definition()`
- `src/rust-engine/src/graph_processor.rs`: add `PerModuleState` variants for each new module type and `process_module` dispatch arms
- `src/rust-engine/src/lib.rs`: add pub mod declarations for new modules
