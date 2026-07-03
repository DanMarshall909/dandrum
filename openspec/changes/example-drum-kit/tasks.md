## 1. Platform Primitive Prerequisites

- [ ] 1.1 Confirm `declarative-instrument-platform` supplies registered `noise`, `impulse`, `note_to_control`, and
  `multiply` primitives with compatible metadata and render support before implementing drum-kit examples.
- [ ] 1.2 Confirm `add-event-routing-primitives` supplies generic event routing with compatible metadata and render
  support before implementing drum-kit routing.
- [ ] 1.3 Add failing example-level tests that document the exact primitive ports consumed by the drum-kit composites.

## 2. Composite Module Definitions

- [ ] 2.1 Create `composite-velocity-vca.yaml` with note_to_control + multiply + gain
- [ ] 2.2 Create `composite-impulse-tone.yaml` with oscillator + ADSR + velocity_vca
- [ ] 2.3 Create `composite-impulse-noise.yaml` with noise + filter + ADSR + velocity_vca
- [ ] 2.4 Create `composite-impulse-layer.yaml` with oscillator + noise + filter + ADSR + velocity_vca

## 3. Drum Kit Example Patch

- [ ] 3.1 Create `drum-kit.yaml` example patch wiring MIDI input through generic event routing into impulse_* composites
  and master output
- [ ] 3.2 Configure voice allocation in drum kit patch

## 4. Verification

- [ ] 4.1 Run `cargo test` and fix any regressions
- [ ] 4.2 Run CMake build and CTest
- [ ] 4.3 Run `openspec validate example-drum-kit --strict`
- [ ] 4.4 Render drum-kit patch with CLI and verify audio output
