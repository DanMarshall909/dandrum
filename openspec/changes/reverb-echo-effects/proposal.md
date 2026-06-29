## Why

The current engine has primitive delay boundaries (one-sample, block-delay) and a comb filter, but no usable delay-based effects. Reverb and echo are foundational to any instrument or mix, and building them from shared submodules (delay lines, one-pole filters, allpass filters, diffusion networks) establishes a reusable DSP library for all future time-based and modulation effects (chorus, flanger, phaser). Decomposing the effects into pipeline stages means each component is independently testable, reusable, and composable in YAML patches.

## What Changes

- Introduce a `delay-line` submodule: a circular-buffer delay line with fractional interpolation (linear/cubic), write/read cursor, and modulation input
- Introduce a `one-pole-filter` primitive in `filter.rs`: configurable LP/HP one-pole filter implementing `FilterAlgorithm`, shared by echo and reverb
- Introduce an `echo` module built from pipeline stages: `DelayLine` → `OnePoleFilter` (damping) → feedback gain → wet/dry mix. Supports ping-pong and tempo-sync.
- Introduce a `reverb` module built from pipeline stages: comb filter bank (pure `DelayLine` + feedback gain + `OnePoleFilter` per comb) → allpass diffuser chain → stereo spread → wet/dry mix
- Create YAML composite module definitions showing echo and reverb built from primitive modules
- Register all new modules as built-in module types in the modular graph engine

## Capabilities

### New Capabilities

- `delay-line`: Core delay-line abstraction — circular buffer with read/write cursor, fractional sample interpolation (linear, cubic), and modulation input. Designed as the foundational submodule for all time-based effects.
- `one-pole-filter`: One-pole low-pass and high-pass filter implementing `FilterAlgorithm` trait. Shares the sample-by-sample `process(input: f32) -> f32` API from `filter.rs`. Used as the damping filter in echo and reverb feedback pipelines.
- `echo`: Stereo/mono delay effect composed of pipeline stages — `DelayLine` → `OnePoleFilter` → feedback gain → mix. Independent left/right delay times, ping-pong mode, tempo-synced note divisions, and wet/dry mix.
- `reverb`: Room reverberator using Schroeder/Moorer topology — parallel comb filter bank (each comb is `DelayLine` + feedback gain + `OnePoleFilter` damping) feeding series allpass diffusers, with room size, decay time (RT60), pre-delay, diffusion, stereo width, and wet/dry mix.

### Modified Capabilities

<!-- No existing specs are changing -->

## Impact

- **New DSP source files**: `src/rust-engine/src/delay_line.rs`, `src/rust-engine/src/echo.rs`, `src/rust-engine/src/reverb.rs`
- **OnePoleFilter added to existing**: `src/rust-engine/src/filter.rs`
- **New built-in module registrations**: `builtins.rs` gets delay_line, echo, reverb definitions
- **Graph processor changes**: `graph_processor.rs` gets `PerModuleState` variants and `process_module` dispatch arms for delay_line, echo, reverb
- **Example YAML patches**: `examples/patches/composite-echo.yaml`, `examples/patches/composite-reverb.yaml`
- **Test files**: Unit tests for each submodule; integration tests via YAML patch rendering
- **No new external crate dependencies** — all DSP is hand-rolled (existing pattern)
