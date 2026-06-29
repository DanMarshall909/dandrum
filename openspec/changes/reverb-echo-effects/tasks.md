## 1. Delay Line Submodule

- [x] 1.1 Create `src/rust-engine/src/delay_line.rs` with `DelayLine` struct: circular buffer, write head, read with fractional interpolation
- [x] 1.2 Implement `write(sample: f32)` and `read(delay_samples: f32) -> f32` with linear interpolation
- [x] 1.3 Implement cubic (4-point) interpolation mode switchable per instance
- [x] 1.4 Implement modulation offset input for read position
- [x] 1.5 Implement `reset()` method (zero buffer, reset write head)
- [x] 1.6 Write unit tests: integer delay, fractional delay, modulation, interpolation modes, reset, power-of-two buffer sizing
- [x] 1.7 Add `pub mod delay_line` to `src/rust-engine/src/lib.rs`

## 2. One-Pole Filter Primitive

- [x] 2.1 Add `OnePoleFilter` struct to `src/rust-engine/src/filter.rs` implementing `FilterAlgorithm`: one-pole low-pass with configurable cutoff, `process(input: f32) -> f32`
- [x] 2.2 Add `OnePoleMode` enum with `Lowpass` and `Highpass` variants
- [x] 2.3 Implement `set_cutoff(hz: f64, sample_rate: f64)` method using standard one-pole coefficient formula `a = exp(-2*pi*cutoff/sample_rate)`
- [x] 2.4 Implement `reset()` (zero internal state)
- [x] 2.5 Write unit tests: DC gain, impulse response, frequency roll-off, highpass attenuation, cutoff sweep, no NaN at extreme cutoff
- [x] 2.6 Add `use` exports if needed so `crate::filter::OnePoleFilter` is accessible

## 3. Echo Module (Pipeline Composition)

- [x] 3.1 Create `src/rust-engine/src/echo.rs` with `Echo` struct containing stereo pair of: `DelayLine` + `OnePoleFilter` (damping) + feedback gain stage
- [x] 3.2 Implement stereo process function: per-channel pipeline of `DelayLine::write` → `DelayLine::read` → `OnePoleFilter::process` → feedback gain → mix with dry input
- [x] 3.3 Implement ping-pong mode (cross-channel feedback routing: left delay output feeds right delay input and vice versa, plus crossed output taps)
- [x] 3.4 Implement tempo sync mode: compute delay time from BPM + note division
- [x] 3.5 Implement wet/dry mix output
- [x] 3.6 Implement parameter setters: delay time (ms), feedback (0–0.99), damping cutoff, wet/dry, ping-pong toggle, sync division
- [x] 3.7 Write unit tests: impulse response, feedback decay, ping-pong routing, damping filter effect, tempo sync calculation, wet/dry mix, edge cases (zero delay, max feedback)
- [x] 3.8 Add `pub mod echo` to `src/rust-engine/src/lib.rs`

## 4. Reverb Module (Pipeline Composition)

- [ ] 4.1 Create `src/rust-engine/src/reverb.rs` with `Reverb` struct containing mono comb filter bank, allpass diffuser chain, and pre-delay `DelayLine`
- [ ] 4.2 Implement comb stage as a pipeline: `DelayLine` delay → `OnePoleFilter` damping → feedback gain, with mutually prime delay lengths
- [ ] 4.3 Implement allpass diffuser sub-struct using a `DelayLine` with configurable delay and coefficient
- [ ] 4.4 Combine into Schroeder topology: parallel comb bank → series allpass chain
- [ ] 4.5 Implement stereo expansion: duplicate diffuser chain with slightly offset parameters, controlled by `stereo_width`
- [ ] 4.6 Implement parameter setters: decay_time/RT60, room_size, pre_delay, damping, diffusion, stereo_width, wet/dry
- [ ] 4.7 Compute comb feedback gains from RT60 target and delay length
- [ ] 4.8 Implement pre-delay via a `DelayLine` before the reverb core
- [ ] 4.9 Write unit tests: impulse response shape, RT60 decay envelope, room size scaling, damping effect on spectrum, diffusion density, stereo decorrelation, wet/dry mix
- [ ] 4.10 Add `pub mod reverb` to `src/rust-engine/src/lib.rs`

## 5. Built-in Module Registrations

- [ ] 5.1 Add `echo_definition()` to `builtins.rs` with audio L/R input, audio L/R output, control inputs (time_left_ms, time_right_ms, feedback, damping_cutoff, wet, dry, sync division), declared as global scope
- [ ] 5.2 Add `reverb_definition()` to `builtins.rs` with audio L/R input, audio L/R output, control inputs (decay_time, room_size, pre_delay, damping, diffusion, stereo_width, wet, dry), declared as global scope
- [ ] 5.3 Register both in `BuiltInModuleRegistry::new()` and `module_types` constants
- [ ] 5.4 Write port assertion tests for both definitions

## 6. Graph Processor Integration

- [ ] 6.1 Add `PerModuleState::Echo(Echo)` and `PerModuleState::Reverb(Reverb)` variants in `graph_processor.rs`
- [ ] 6.2 Add `"echo"` and `"reverb"` arms in `PerModuleState::new()` to initialize from `ModuleNode` parameters
- [ ] 6.3 Add `process_echo()` function: read inputs via `input_provider`, call `Echo::process()`, write audio outputs
- [ ] 6.4 Add `process_reverb()` function: same pattern for reverb
- [ ] 6.5 Add `"echo"` and `"reverb"` arms in `process_module()` match dispatch
- [ ] 6.6 Build and verify no compile errors

## 7. YAML Composite Module Examples

- [ ] 7.1 Create `examples/patches/composite-echo.yaml`: composite module `composite_echo` built from `delay_line` + `one_pole_filter` + `gain` + `audio_mixer`, demonstrating feedback loop, damping, and wet/dry mix
- [ ] 7.2 Create `examples/patches/composite-reverb.yaml`: composite module `composite_reverb` built from parallel `delay_line` + `one_pole_filter` + `gain` combs feeding series `delay_line` allpass diffusers, with `audio_mixer` summing
- [ ] 7.3 Verify both examples load without validation errors via patch loading tests

## 8. Integration Tests

- [ ] 8.1 Write offline render test with YAML patch using `echo` module: mono impulse → echo → audio output, verify repeat timing and feedback decay
- [ ] 8.2 Write offline render test with YAML patch using `reverb` module: mono impulse → reverb → audio output, verify tail length and stereo spread
- [ ] 8.3 Write compiled-vs-raw render parity test for echo module
- [ ] 8.4 Write compiled-vs-raw render parity test for reverb module
- [ ] 8.5 Run `cargo test` and confirm all tests pass
