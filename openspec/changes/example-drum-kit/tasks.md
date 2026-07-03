## 1. Platform Primitive Prerequisites

- [x] 1.1 Confirm `declarative-instrument-platform` supplies registered `noise`, `impulse`, `note_to_control`, `gain`,
  and `multiply` primitives with compatible metadata and render support before implementing drum-kit examples.
- [x] 1.2 Confirm `add-event-routing-primitives` supplies generic event routing with compatible metadata and render
  support before implementing drum-kit routing.
- [x] 1.3 Add failing example-level tests that document the exact primitive ports consumed by the drum-kit composites.

## 2. Composite Module Definitions

- [x] 2.1 Create `composite-velocity-vca.yaml` with note_to_control + gain stages
- [x] 2.2 Create `composite-impulse-tone.yaml` with oscillator + ADSR + velocity VCA pattern
- [x] 2.3 Create `composite-impulse-noise.yaml` with noise + filter + ADSR + velocity VCA pattern
- [x] 2.4 Create `composite-impulse-layer.yaml` with oscillator + noise + filter + ADSR + velocity VCA pattern

## 3. Drum Kit Example Patch

- [x] 3.1 Create `drum-kit.yaml` example patch wiring MIDI input through generic event routing into impulse_* composites
  and master output
- [x] 3.2 Configure voice allocation in drum kit patch

## 4. Verification

- [x] 4.1 Run `cargo test` and fix any regressions
- [x] 4.2 Run CMake build and CTest
- [x] 4.3 Run `openspec validate example-drum-kit --strict`
- [x] 4.4 Render drum-kit patch with CLI and verify audio output
