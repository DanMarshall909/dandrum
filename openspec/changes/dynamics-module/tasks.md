## 1. Shared infrastructure: EnvelopeDetector utility

- [x] 1.1 Create `EnvelopeDetector` struct with configurable attack/release time constants, RMS and peak detection modes
- [x] 1.2 Implement envelope smoothing (two-time-constant exponential) with `process(sample) -> f64` and `reset()`
- [x] 1.3 Write unit tests: attack time accuracy, release time tracking, RMS vs peak envelope shape, reset clears state

## 2. Shared infrastructure: Audio loading extraction

- [x] 2.1 Create `src/rust-engine/src/audio_loading.rs` with `load_pcm_wav()` extracted from `sample.rs`
- [x] 2.2 Add `pub mod audio_loading;` to `src/rust-engine/src/lib.rs`
- [x] 2.3 Refactor `sample.rs` to delegate WAV loading to `audio_loading.rs` and re-export public types
- [x] 2.4 Write unit tests: loaded WAV data matches original, sample rate conversion edge cases, error reporting for unsupported formats

## 3. Dynamics Processor module

- [x] 3.1 Create `src/rust-engine/src/dynamics_processor.rs` with `DynamicsProcessor` struct, `GainComputer` (two-slope transfer for level mode, direction-based for transient mode), envelope detector, feed-forward/feedback topology switching, and sidechain routing
- [x] 3.2 Implement `level` mode: threshold, below_ratio, above_ratio, knee, makeup_gain — covers compressor/limiter/gate/expander/upward compressor
- [x] 3.3 Implement `transient` mode: envelope direction tracking with hysteresis, attack_gain, sustain_gain — covers transient shaper/designer
- [x] 3.4 Implement sidechain routing: use external `sidechain_in` when connected, fall back to `audio_in`
- [x] 3.5 Add `dynamics_processor_definition()` to `builtins.rs` with mode-dependent ports
- [x] 3.6 Add `PerModuleState::DynamicsProcessor` variant in `graph_processor.rs`
- [x] 3.7 Implement `"dynamics-processor"` arms in `PerModuleState::new()` and process dispatch
- [x] 3.8 Add `pub mod dynamics_processor;` to `src/rust-engine/src/lib.rs`
- [x] 3.9 Write unit tests: compressor (unity below threshold, ratio accuracy, RMS/peak, hard/soft knee, makeup gain, feed-forward vs feedback, silence, NaN-free), limiter (brickwall ratio, ceiling constraint), gate (mute below threshold, smooth fade), expander (ratio < 1), upward compressor (ratio < 1 above threshold), transient mode (attack/sustain gain, hysteresis), edge cases (both ratios = 1, extreme parameters)

## 4. Saturator module

- [x] 4.1 Create `src/rust-engine/src/saturator.rs` with `Saturator` struct, `WaveshaperCurve` trait, and built-in implementations: `TanhCurve`, `HardClipCurve`, `SoftClipCurve`, `SinFoldCurve`
- [x] 4.2 Implement drive pre-gain and bias offset before waveshaping
- [x] 4.3 Add `saturator_definition()` to `builtins.rs` with ports: `audio_in`, `drive`, `bias`; output `audio_out`; parameter `curve`
- [x] 4.4 Add `PerModuleState::Saturator` variant in `graph_processor.rs`
- [x] 4.5 Implement `"saturator"` arms in `PerModuleState::new()` and process dispatch
- [x] 4.6 Add `pub mod saturator;` to `src/rust-engine/src/lib.rs`
- [x] 4.7 Write unit tests: tanh curve symmetry and bounds, hard clip clamping, soft clip smooth transition, sinfold produces harmonics, drive scales input, bias creates asymmetry, custom curve trait integration, unity gain at minimum drive, NaN-free

## 5. Convolution module

- [x] 5.1 Create `src/rust-engine/src/convolution.rs` with `ConvolutionEngine` struct, partitioned convolution (FFT overlap-add), IR partitioning at load time
- [x] 5.2 Integrate with `audio_loading.rs` for WAV-based IR loading (kind: `impulse_response`)
- [x] 5.3 Add `convolution_definition()` to `builtins.rs` with ports: `audio_in`, `wet`; output `audio_out`; parameter `asset`
- [x] 5.4 Add `PerModuleState::Convolution` variant in `graph_processor.rs`
- [x] 5.5 Implement `"convolution"` arms in `PerModuleState::new()` and process dispatch
- [x] 5.6 Add `pub mod convolution;` to `src/rust-engine/src/lib.rs`
- [x] 5.7 Write unit tests: impulse input reproduces IR, dry/wet mix, zero input produces zero output, IR shorter than partition size, IR truncated at 4 s, NaN-free

## 6. Acceptance tests

- [x] 6.1 Write FFT-based test for dynamics-processor gain reduction curve (compressor transfer function matches threshold/above_ratio/knee)
- [x] 6.2 Write FFT-based test for dynamics-processor gate transfer function (below_ratio < 1)
- [x] 6.3 Write FFT-based test for dynamics-processor transient mode (attack/sustain envelope response)
- [x] 6.4 Write FFT-based test for saturator harmonic spectrum (each curve produces expected harmonic pattern)
- [x] 6.5 Write FFT-based test for convolution impulse response accuracy
- [x] 6.6 Write end-to-end YAML patch test: dynamics-processor + saturator chain renders to WAV without error
- [x] 6.7 Write end-to-end YAML patch test: dynamics-processor prevents output above threshold (limiter mode)
- [x] 6.8 Write end-to-end YAML patch test: convolution with IR produces expected output
- [x] 6.9 Verify all rust unit tests pass with `cargo test`
