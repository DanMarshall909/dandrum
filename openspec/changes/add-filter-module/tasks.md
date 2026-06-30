## 1. Dependency: Add rustfft crate

- [x] 1.1 Add `rustfft` dependency to `src/rust-engine/Cargo.toml`

## 2. FFT analysis module

- [x] 2.1 Create `src/rust-engine/src/fft.rs` with magnitude response analysis (zero-pad to power of two, real FFT, frequency/magnitude in dB)
- [x] 2.2 Add `pub mod fft;` to `src/rust-engine/src/lib.rs`
- [x] 2.3 Write unit tests: flat response from unit impulse, frequency bin centers, zero-padding behavior

## 3. Crossover filter pair (shared foundation)

- [x] 3.1 Create `src/rust-engine/src/crossover.rs` with 4th-order Linkwitz-Riley LP + HP pair (two cascaded biquad sections each)
- [x] 3.2 Add `pub mod crossover;` to `src/rust-engine/src/lib.rs`
- [x] 3.3 Write unit tests: flat summed response at crossover, −3 dB at crossover per band, edge cases

## 4. Filter DSP module (multi-algorithm engine)

- [x] 4.1 Create `src/rust-engine/src/filter/` with a common `FilterAlgorithm` trait and implementations:
      - `BiquadFilter` — direct-form I biquad with RBJ coefficient computation (lowpass, highpass, peaking)
      - `MoogLadder` — 4-pole resonant low-pass with per-stage nonlinearity (tanh) and feedback
      - `CombFilter` — delay-line comb (feedback/feedforward) with configurable delay time and gain
- [x] 4.2 Add `pub mod filter;` to `src/rust-engine/src/lib.rs`
- [x] 4.3 Write unit tests for biquad: LP attenuation, HP attenuation, PEQ boost/cut, resonance peak, coefficient bounds
- [ ] 4.4 Write unit tests for Moog: resonance peak, rolloff slope (≥18 dB/oct), self-oscillation at high resonance, NaN-free
- [x] 4.5 Write unit tests for comb: periodic notch/peak spacing matches delay time, gain stability bounds, NaN-free

## 5. Update filter built-in definition with algorithm parameter and new ports

- [ ] 5.1 Add `resonance` and `gain` input ports to `filter_definition()` in `builtins.rs`
- [ ] 5.2 Update `filter_definition()` to accept `algorithm` and `mode` parameters via the module's parameter map
- [ ] 5.3 Update built-in registry test to verify `resonance` and `gain` ports exist

## 6. Implement filter processing in graph processor

- [x] 6.1 Add `PerModuleState::Filter` variant with algorithm enum and per-algorithm state
- [ ] 6.2 Implement `"filter"` arm in `PerModuleState::new()` reading `algorithm`, `mode`, `comb_type`, and initial parameter values
- [x] 6.3 Implement `"filter"` arm in per-sample process dispatch: read audio_in + control inputs, dispatch to active algorithm, write audio_out
- [ ] 6.4 Coefficient update: recompute biquad coefficients or Moog pole coefficients when cutoff/resonance/gain control inputs change

## 7. Frequency splitter utility module

- [ ] 7.1 Add `frequency_splitter_definition()` to `builtins.rs` with `audio_in` input, `crossover_hz` control, and `low`/`mid`/`high` audio outputs
- [ ] 7.2 Add `PerModuleState::FrequencySplitter` variant that instantiates two internal crossover LP+HP pairs
- [ ] 7.3 Implement `"frequency_splitter"` arms in `new()` and process dispatch

## 8. Spectral processing module

- [x] 8.1 Create `src/rust-engine/src/spectral.rs` with STFT overlap-add (Hann window, 256 frame, 50% overlap), real FFT, magnitude/phase decomposition, and spectral gate effect
- [x] 8.2 Add `pub mod spectral;` to `src/rust-engine/src/lib.rs`
- [ ] 8.3 Add `spectral_processor_definition()` to `builtins.rs`
- [ ] 8.4 Add `PerModuleState::SpectralProcessor` variant in `graph_processor.rs`
- [ ] 8.5 Implement `"spectral_processor"` arms in `new()` and process dispatch
- [x] 8.6 Write unit tests: passthrough round-trip (output within −60 dB of input), spectral gate attenuation, all-zero in → all-zero out

## 9. FFT-based frequency response tests

- [x] 9.1 Write FFT test for biquad low-pass frequency response
- [x] 9.2 Write FFT test for biquad high-pass frequency response
- [x] 9.3 Write FFT test for biquad peaking EQ boost/cut response
- [ ] 9.4 Write FFT test for Moog ladder resonance peak and rolloff slope
- [x] 9.5 Write FFT test for comb filter notch/peak spacing
- [x] 9.6 Write FFT test for 2-band crossover summed flatness
- [x] 9.7 Write FFT test for spectral processor passthrough accuracy