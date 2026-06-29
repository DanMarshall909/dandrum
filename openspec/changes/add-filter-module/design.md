## Context

The filter module type is registered in `builtins.rs` with `audio_in` and `cutoff` ports but no processing logic. The project uses a modular graph architecture where each module type maps to a `PerModuleState` variant with per-sample `process()`. Audio is processed in topological order.

**Design philosophy: least modules, most flexibility.** Instead of separate modules for each filter type (biquad-only, Moog-only, comb-only), the `filter` module supports multiple algorithms via an `algorithm` parameter. Control inputs (`cutoff`, `resonance`, `gain`) adapt their semantics to the active algorithm. This way one module type serves as the universal audio processing workhorse, and new algorithms can be added by extending `filter.rs` without adding new module types.

## Goals / Non-Goals

**Goals:**
- Implement a unified `filter` module with three algorithms: biquad (LP/HP/PEQ), Moog ladder (4-pole resonant LP), and comb (feedback/feedforward)
- Add `resonance` and `gain` control inputs with algorithm-adaptive semantics
- Extract all filter algorithms into `filter.rs` with common trait/interface and isolated unit tests
- Implement `frequency_splitter` as a thin utility that delegates to two internal filter/crossover instances (no custom filter logic)
- Implement `spectral_processor` for STFT-based effects
- Add `fft.rs` for magnitude response analysis and FFT-based spec tests
- Write FFT-based tests verifying response for biquad modes, Moog resonance, comb notches/peaks, crossover summing

**Non-Goals:**
- Adding new module types for each filter algorithm (all algorithms live under one `filter` module type)
- Real-time FFT visualization or GUI
- Modifying existing patch YAML format

## Decisions

### Unified filter module (algorithm parameter)

The `filter` module type gains an `algorithm` parameter with values `biquad`, `moog`, `comb`. The `mode` parameter is only meaningful for `biquad` (values `lowpass`, `highpass`, `peaking`). Control inputs map as:

| Control    | Biquad              | Moog Ladder            | Comb                    |
|------------|---------------------|------------------------|-------------------------|
| `cutoff`   | Filter frequency    | Filter frequency       | Delay time (ms)         |
| `resonance`| Q (0.1–10.0)        | Feedback/resonance     | Feedback gain (0–1)     |
| `gain`     | PEQ boost/cut (dB)  | (unused)               | (unused)                |

### Biquad filter (RBJ cookbook)
Direct-form I biquad with coefficient computation for lowpass, highpass, and peaking EQ modes. Most widely documented structure, known stability criteria.

### Moog ladder filter (4-pole resonant low-pass)
Digital model based on the classic 4-pole transistor ladder topology:
- 4 cascaded one-pole low-pass sections
- Feedback from output to first stage input for resonance
- Nonlinear saturation per stage (tanh clamps at each pole for the characteristic overdrive)
- Cutoff and resonance are the two primary parameters
- Self-oscillation at high resonance (Q > ~0.95)

Implementation approach: use the Huovilainen-style digital Moog model (or a simplified variant) with state variables per pole stage.

### Comb filter (feedback/feedforward delay)
Delay-line-based comb filter:
- `comb_type` parameter: `feedback` or `feedforward`
- `cutoff` maps to delay time in milliseconds (1–100 ms range)
- `resonance` maps to feedback/feedforward gain (0–1, with polarity)
- Frequency response shows equally spaced notches (feedforward) or peaks (feedback)
- Used for flanging, chorusing, resonator effects

### Frequency splitter delegates to crossover filter pair
Rather than building custom filter logic for the splitter, it internally instantiates two filter modules configured as Linkwitz-Riley LP and HP. The splitter module is purely a routing utility — it owns the cutoff frequency control and routes it to both internal filters. The crossover module (`crossover.rs`) provides a 4th-order Linkwitz-Riley pair (two cascaded biquad LP + two cascaded biquad HP) ensuring flat summed response at the crossover point.

### Spectral processor (separate module — fundamentally different paradigm)
STFT overlap-add with:
- 50% overlap, Hann window, 2048-frame FFT
- Real FFT → magnitude/phase decomposition → magnitude modification → overlap-add resynthesis
- Effects: passthrough, spectral gate (bin-level noise gate by magnitude threshold)
- Separate module because FFT-based processing is architecturally distinct from sample-by-sample filters

### FFT analysis module
`rustfft` crate provides real FFT. The analysis module zero-pads to next power of two, optionally windows (Hann), computes magnitude spectrum in dB, and returns `Vec<(freq_hz, magnitude_db)>` pairs.

### Frame-by-frame processing in graph processor
Biquad and Moog ladder process one sample per call. Comb uses a delay line internally but exposes a sample-by-sample interface. The spectral processor buffers internally and processes in STFT frames but outputs one sample per call via an internal ring buffer.

### Control mapping
- Cutoff (biquad/Moog): 0–1 → 20–20000 Hz
- Cutoff (comb): 0–1 → 1–100 ms delay
- Resonance/Q: 0–1 → 0.1–10.0 Q (biquad), 0–0.99 feedback (Moog, comb)
- PEQ gain: 0–1 → −24 to +24 dB

## Risks / Trade-offs

- **Moog digital model accuracy**: Simplified Huovilainen model may not perfectly match analog behavior. Acceptable for a useful digital filter; can be refined with more accurate models later.
- **Comb filter delay line memory**: At 48 kHz, 100 ms delay = 4800 samples. Each comb instance allocates a delay line. Acceptable for polyphonic use (16 voices × 4800 = ~300 KB).
- **Parameter ambiguity**: `cutoff` means different things per algorithm. Documented per-algorithm in patch YAML.
- **Biquad precision at high Q**: Mitigated by using `f64` for coefficient computation.
- **STFT latency**: 2048-frame at 48 kHz = ~43 ms. Acceptable for offline rendering. 
- **Crossover phase response**: Linkwitz-Riley has constant phase shift but not linear-phase. Acceptable for multiband processing.