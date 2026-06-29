## Why

The filter module type is registered in `builtins.rs` with `audio_in` and `cutoff` ports but has no DSP processing. Adding a unified multi-algorithm filter module covers all common subtractive synthesis and audio processing needs with minimal module surface area. FFT-based testing ensures frequency response correctness in CI.

## What Changes

- Implement a single `filter` module with selectable algorithm: `biquad` (LP/HP/PEQ), `moog` (4-pole ladder low-pass), or `comb` (feedback/feedforward delay comb)
- The filter module exposes common control inputs (`cutoff`, `resonance`, `gain`) whose semantics adapt to the active algorithm
- Build a `frequency_splitter` utility module that internally uses two filter instances (LP + HP) to split audio into bands — no custom filter logic
- Add a `spectral_processor` module for STFT-based spectral effects (gate, freeze)
- Extract filter algorithms into `filter.rs` with unit-tested per-sample processing
- Add an FFT analysis utility (`fft.rs`) for magnitude response measurement
- Write FFT-based spec tests verifying frequency response for each algorithm
- No breaking changes to existing patch format or modules

## Capabilities

### New Capabilities
- `filter-module`: Unified DSP filter module supporting biquad (low-pass, high-pass, parametric EQ), Moog ladder (4-pole resonant low-pass), and comb (feedback/feedforward) algorithms via a single module type with `algorithm` parameter and adaptive control inputs
- `frequency-splitter`: Lightweight crossover utility module (no custom filter logic — delegates to internal filter instances)
- `spectral-processing`: Overlap-add STFT processing module for spectral-domain effects
- `fft-analysis`: FFT-based frequency response analysis utility for verifying filter and DSP behavior

### Modified Capabilities
- (none)

## Impact

- `src/rust-engine/src/filter.rs`: new module — biquad (RBJ LP/HP/PEQ), Moog ladder (4-pole resonant), and comb (feedback/feedforward) filter algorithms with common interface
- `src/rust-engine/src/fft.rs`: new module — FFT magnitude response analysis
- `src/rust-engine/src/spectral.rs`: new module — STFT overlap-add spectral processor
- `src/rust-engine/src/crossover.rs`: new module — Linkwitz-Riley crossover pair (reused by filter engine and splitter)
- `src/rust-engine/src/builtins.rs`: update `filter_definition()` with `algorithm` parameter and new ports; add `frequency_splitter_definition()` and `spectral_processor_definition()`
- `src/rust-engine/src/graph_processor.rs`: add `PerModuleState::Filter` (multi-algorithm), `PerModuleState::FrequencySplitter` (thin wrapper around crossover LP+HP), `PerModuleState::SpectralProcessor`
- `src/rust-engine/src/lib.rs`: add pub mod declarations
- `src/rust-engine/Cargo.toml`: add `rustfft` dependency