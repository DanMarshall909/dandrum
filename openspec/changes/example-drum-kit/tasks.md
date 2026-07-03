## 1. Noise Generator Module

- [ ] 1.1 Add `NOISE` constant to `builtins/module_types.rs`
- [ ] 1.2 Add `Noise` variant to `ModuleKind` enum with `from_str` and `is_render_supported` in
  `builtins/module_kind.rs`
- [ ] 1.3 Add `noise_definition()` to `builtins.rs` registering the noise module with audio output and colour/amplitude
  control inputs
- [ ] 1.4 Create `src/rust-engine/src/noise.rs` with `NoiseGenerator` struct supporting white, pink, and brownian noise
- [ ] 1.5 Add `process_noise()` to `graph_processor/processing.rs`
- [ ] 1.6 Add `ModuleKind::Noise` dispatch arm to `graph_processor/dispatch.rs`
- [ ] 1.7 Export `noise` module from `src/rust-engine/src/lib.rs`
- [ ] 1.8 Add unit tests for noise generator (all colour types, amplitude scaling, continuous output without events)

## 2. note_to_control Module

- [ ] 2.1 Add `NOTE_TO_CONTROL` constant to `builtins/module_types.rs`
- [ ] 2.2 Add `NoteToControl` variant to `ModuleKind` enum
- [ ] 2.3 Add `note_to_control_definition()` to `builtins.rs` with events input and control output
- [ ] 2.4 Create `src/rust-engine/src/note_to_control.rs` with velocity-extraction processing
- [ ] 2.5 Add `process_note_to_control()` to `graph_processor/processing.rs`
- [ ] 2.6 Add `ModuleKind::NoteToControl` dispatch arm to `graph_processor/dispatch.rs`
- [ ] 2.7 Export `note_to_control` module from `src/rust-engine/src/lib.rs`
- [ ] 2.8 Add unit tests for velocity extraction (multiple notes, NoteOff reset, constant output between events)

## 3. multiply Module

- [ ] 3.1 Add `MULTIPLY` constant to `builtins/module_types.rs`
- [ ] 3.2 Add `Multiply` variant to `ModuleKind` enum
- [ ] 3.3 Add `multiply_definition()` to `builtins.rs` with two control inputs and one control output
- [ ] 3.4 Create `src/rust-engine/src/multiply.rs` with per-sample multiplication
- [ ] 3.5 Add `process_multiply()` to `graph_processor/processing.rs`
- [ ] 3.6 Add `ModuleKind::Multiply` dispatch arm to `graph_processor/dispatch.rs`
- [ ] 3.7 Export `multiply` module from `src/rust-engine/src/lib.rs`
- [ ] 3.8 Add unit tests for multiply (constant, zero, negative, frame-length matching)

## 4. delay_line Module

- [ ] 4.1 Add `DELAY_LINE` constant to `builtins/module_types.rs`
- [ ] 4.2 Add `DelayLine` variant to `ModuleKind` enum
- [ ] 4.3 Add `delay_line_definition()` to `builtins.rs` with audio input and delay_samples control input
- [ ] 4.4 Wrap existing `delay_line.rs` as a built-in module with state initialization from sample rate
- [ ] 4.5 Add `process_delay_line()` to `graph_processor/processing.rs`
- [ ] 4.6 Add `ModuleKind::DelayLine` dispatch arm to `graph_processor/dispatch.rs`
- [ ] 4.7 Add unit tests for delay_line module (integer delay, fractional delay, no-output-on-zero-input)

## 5. envelope_follower Module

- [ ] 5.1 Add `ENVELOPE_FOLLOWER` constant to `builtins/module_types.rs`
- [ ] 5.2 Add `EnvelopeFollower` variant to `ModuleKind` enum
- [ ] 5.3 Add `envelope_follower_definition()` to `builtins.rs` with audio input and attack/release control inputs
- [ ] 5.4 Wrap existing `envelope_follower.rs` as a built-in module with state initialization
- [ ] 5.5 Add `process_envelope_follower()` to `graph_processor/processing.rs`
- [ ] 5.6 Add `ModuleKind::EnvelopeFollower` dispatch arm to `graph_processor/dispatch.rs`
- [ ] 5.7 Add unit tests for envelope_follower (attack time, release time, constant input tracking)

## 6. Composite Module Definitions

- [ ] 6.1 Create `composite-velocity-vca.yaml` with note_to_control + multiply + gain
- [ ] 6.2 Create `composite-impulse-tone.yaml` with oscillator + ADSR + velocity_vca
- [ ] 6.3 Create `composite-impulse-noise.yaml` with noise + filter + ADSR + velocity_vca
- [ ] 6.4 Create `composite-impulse-layer.yaml` with oscillator + noise + filter + ADSR + velocity_vca

## 7. Drum Kit Example Patch

- [ ] 7.1 Create `drum-kit.yaml` example patch wiring impulse_* composites with MIDI input and master output
- [ ] 7.2 Configure voice allocation in drum kit patch

## 8. Verification

- [ ] 8.1 Run `cargo test` and fix any regressions
- [ ] 8.2 Run CMake build and CTest
- [ ] 8.3 Run `openspec validate example-drum-kit --strict`
- [ ] 8.4 Render drum-kit patch with CLI and verify audio output
