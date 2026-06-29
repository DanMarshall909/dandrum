## Context

The dandrum engine currently has delay primitives (`audio_delay_one_sample`, `block_delay`) registered as built-in modules and a `CombFilter` in `filter.rs`, but no general-purpose delay line abstraction or end-user delay/echo/reverb effects. The modular graph architecture demands that new effects follow the established pattern: standalone DSP struct → built-in registration (ports, scope) → graph processor integration (state variant, process dispatch arm).

All existing DSP is hand-rolled without external dependencies (no `fundsp`, `dasp` etc.). Filters, crossovers, and spectral processing follow a sample-by-sample `process(input: f32) -> f32` API. This design extends that pattern. The engine also supports composite modules — YAML-defined subsystems that expand to primitive modules at load time — enabling effects to be built either as optimized Rust built-ins or as user-visible composite examples.

## Goals / Non-Goals

**Goals:**
- Shared `DelayLine` submodule usable by echo, reverb, and future modulation effects (chorus, flanger, phaser)
- Shared `OnePoleFilter` in `filter.rs` implementing `FilterAlgorithm`, used for damping in both echo and reverb
- Stereo `Echo` built-in module composed of pipeline stages: delay + damping filter + feedback + wet/dry
- Stereo `Reverb` built-in module composed of pipeline stages: comb bank (delay + feedback + damping per comb) + allpass diffusers + stereo spread + wet/dry
- YAML composite module examples showing echo and reverb built from primitive modules in patches
- Full test coverage: impulse response, parameter sweeps, edge cases

**Non-Goals:**
- Not implementing convolution reverb (belongs in `dynamics-module` change where `ConvolutionEngine` is planned)
- Not implementing modulation effects (chorus, flanger, phaser) — these will build on the `DelayLine` submodule in a future change
- Not making the delay-line a standalone built-in module (it's an internal DSP component; standalone delay belongs to the echo module)

## Decisions

### 1. Circular buffer delay line over VecDeque
- **Decision**: Fixed-capacity `Vec<f32>` with head cursor and mask-based wrapping (power-of-two buffer sizes)
- **Rationale**: Predictable allocation at initialization, O(1) read/write, cache-friendly contiguous memory. `VecDeque` has overhead for random-access reads at arbitrary offsets.
- **Alternatives considered**: `VecDeque` (remove/append overhead), ring buffer from `ringbuf` crate (external dependency)

### 2. Fractional interpolation: linear (default) + cubic (optional)
- **Decision**: Linear interpolation as the default low-latency mode; 4-point cubic interpolation for high-quality mode switchable per `DelayLine` instance
- **Rationale**: Linear is adequate for most delay modulation and minimal CPU. Cubic eliminates audible aliasing when delay time changes rapidly (e.g., chorusing, flanging).
- **Alternatives considered**: All-pass interpolation (more CPU, subtle audible artifacts at extreme modulation rates)

### 3. Decomposed pipeline architecture over monolithic effect structs
- **Decision**: Echo and reverb are composed from separate pipeline stages (DelayLine, OnePoleFilter, gain, mixer) rather than monolithic structs with hard-coded damping. Each stage implements `FilterAlgorithm` or a dedicated trait.
- **Rationale**: Each stage is independently testable, reusable across effects, and replaceable. The damping filter in the reverb is the same `OnePoleFilter` used in the echo. Composing from stages also mirrors the modular graph philosophy.
- **Alternatives considered**: Monolithic echo/reverb structs with damping built in — simpler initially, but duplicated code when adding new effects, harder to test individual stages

### 4. OnePoleFilter as a FilterAlgorithm implementation
- **Decision**: The one-pole LP/HP filter is added to `filter.rs` as a new struct implementing `FilterAlgorithm` with `process(input: f32) -> f32`, alongside `BiquadFilter`, `MoogLadder`, and `CombFilter`.
- **Rationale**: Follows the established pattern. One-pole is the minimum-phase choice for damping — cheap (1 multiply, 1-2 adds), no ringing, adequate 6 dB/oct roll-off.
- **Alternatives considered**: Built-in inline one-pole in each effect — duplicated code, harder to test, can't be swapped for other filter types

### 5. Schroeder/Moorer reverb topology with damped combs
- **Decision**: Parallel comb filter bank (4–8 combs) → series allpass diffusers (2–4). Each comb is a pipeline: `DelayLine` delay → `OnePoleFilter` damping → feedback gain. Comb delays are mutually prime.
- **Rationale**: Mathematically well-characterized, efficient, parameter knobs map intuitively to perceptual qualities. Decomposing the comb into separate delay/damping/gain stages means the damping filter can be omitted, replaced, or tuned independently.
- **Alternatives considered**: FDN (feedback delay network) — more complex, harder to tune; convolution — requires IR loading infrastructure (planned elsewhere)

### 6. Per-module state stored in engine graph processor
- **Decision**: `DelayLine` and `OnePoleFilter` are pure DSP structs. `Echo` and `Reverb` are composite structs containing `DelayLine`, `OnePoleFilter`, and gain stage instances. Engine state wraps them in `PerModuleState::Echo(...)` and `PerModuleState::Reverb(...)`.
- **Rationale**: Standard existing pattern. Keeps DSP code testable in isolation without engine dependencies.

### 7. Tempo sync via external control input
- **Decision**: Echo accepts a `sync` control input with note division enum (1/2, 1/4, 1/4T, 1/8, 1/8T, 1/16, etc.) or a `time` control input for free milliseconds. When `sync` is active and a clock control input is present, delay time is computed from the clock interval.
- **Rationale**: Decouples tempo from the audio rate — the host or sequencer sends clock events. Follows existing modular graph pattern where modules are driven by control inputs.

### 8. YAML composite module examples show user-facing modular composition
- **Decision**: In addition to the optimized Rust built-in echo and reverb modules, create `examples/patches/composite-echo.yaml` and `examples/patches/composite-reverb.yaml`. These composite module definitions build echo/reverb from primitive modules (delay_line, one_pole_filter, gain, audio_mixer).
- **Rationale**: Demonstrates the composability of the engine, serves as documentation, and gives users a template for building their own effect chains. The built-in versions exist for performance (single `PerModuleState` dispatch vs. many sub-modules).
- **Alternatives considered**: Only built-in modules — misses the educational/demo value of showing modular composition

## Risks / Trade-offs

- **Risk**: Power-of-two buffer sizes waste memory for non-power-of-two delay times → **Mitigation**: Round up to next power of two; acceptable trade-off for O(1) mask-based wrapping vs. conditional branch wrapping
- **Risk**: High-feedback settings cause runaway oscillation → **Mitigation**: Clamp feedback to [0, 0.99] at the API level; consider optional hard limiter in feedback path
- **Risk**: Schroeder reverb can sound metallic at low diffusion → **Mitigation**: Default to 8 combs + 4 allpasses; expose diffusion parameter that modulates allpass coefficients; document the perceptual trade-off
- **Risk**: Modulation of delay read position (for future chorus/flanger) can cause zipper noise → **Mitigation**: Always interpolate read position; only apply modulation at sample boundaries in the interpolation function
- **Risk**: Pipeline stage overhead vs. monolithic implementation → **Mitigation**: Accept minimal overhead; `OnePoleFilter` is a single multiply-add; `DelayLine::read` is a memory load. Combined overhead is negligible (< 5% of a comb filter's work).
