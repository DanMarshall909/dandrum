## 1. Specification And Validation Surface

- [ ] 1.1 Define the `wavetable-synthesis` capability and its module/asset requirements.
- [ ] 1.2 Add `wavetable_oscillator` to the built-in module registry with typed ports and parameter metadata.
- [ ] 1.3 Add or confirm supporting primitive registry entries for `unison_voice`, `stereo_pan`, `chorus`, `phase_distortion`, `oscillator_sync`, `ring_modulator`, `sample_and_hold`, and `slew_limiter`.
- [ ] 1.4 Extend YAML/schema validation to accept supported wavetable asset declarations and reject unsupported ones.
- [ ] 1.5 Add structured diagnostics for missing wavetable assets, malformed table data, invalid frame sizes, and unsupported interpolation/anti-aliasing modes.
- [ ] 1.6 Reuse existing primitives where they already cover one of the required behaviours instead of adding duplicate module types.

## 2. Wavetable Preparation

- [ ] 2.1 Load and validate supported wavetable assets off the audio thread.
- [ ] 2.2 Normalize/prepare table frames into a deterministic render-ready representation.
- [ ] 2.3 Prepare bandlimited table banks or an explicitly documented v1 anti-aliasing strategy before render.
- [ ] 2.4 Ensure all buffers/scratch state needed by wavetable rendering are allocated during preparation, not during steady-state rendering.

## 3. Realtime Oscillator Rendering

- [ ] 3.1 Implement deterministic phase accumulation from frequency and optional pitch ratio inputs.
- [ ] 3.2 Implement table lookup and interpolation for a fixed wavetable position.
- [ ] 3.3 Implement smooth frame-to-frame wavetable position morphing under control modulation.
- [ ] 3.4 Implement phase reset behaviour where note/trigger reset mode is enabled.
- [ ] 3.5 Verify oversized block splitting still produces continuous oscillator phase and identical audio to equivalent smaller blocks.

## 4. Supporting Primitive Rendering

- [ ] 4.1 Implement or confirm equal-power `stereo_pan` rendering with deterministic left/right outputs.
- [ ] 4.2 Implement or confirm `slew_limiter` control smoothing with separate rise/fall behaviour.
- [ ] 4.3 Implement or confirm deterministic `sample_and_hold` behaviour for triggered control sampling and seeded random mode where supported.
- [ ] 4.4 Implement or confirm `ring_modulator` audio-rate multiplication and optional dry/wet mix.
- [ ] 4.5 Implement or confirm `phase_distortion` phase/control shaping for digital edge and pulse-like movement.
- [ ] 4.6 Implement or confirm `chorus` as a prepared, bounded modulated-delay primitive with no render-time allocation.
- [ ] 4.7 Implement or confirm `oscillator_sync` as an explicit phase-reset/sync utility.
- [ ] 4.8 Implement or confirm `unison_voice` only if explicit module composition is too noisy or too inefficient for the target patches.

## 5. Patch And Preset Examples

- [ ] 5.1 Add a minimal wavetable oscillator patch that renders a stable tone.
- [ ] 5.2 Add a wide virtual-analogue lead patch using wavetable/unison-style composition, filter, envelope, and effects.
- [ ] 5.3 Add an evolving pad patch with slow wavetable position modulation.
- [ ] 5.4 Add a sync/ring-mod style patch for sharper metallic or aggressive digital timbres.
- [ ] 5.5 Add preset surfaces exposing musical controls without exposing internal module IDs.

## 6. Verification

- [ ] 6.1 Add registry tests proving `wavetable_oscillator` exposes the expected ports and parameters.
- [ ] 6.2 Add registry tests proving each supporting primitive exposes the expected ports and parameters, or explicitly maps to an existing primitive.
- [ ] 6.3 Add render tests proving deterministic output for identical render settings and wavetable assets.
- [ ] 6.4 Add render tests proving frequency-to-pitch behaviour across representative notes.
- [ ] 6.5 Add tests proving wavetable position interpolation/morphing behaves continuously.
- [ ] 6.6 Add tests proving each supporting primitive's externally observable behaviour.
- [ ] 6.7 Add tests proving invalid/missing wavetable assets fail before rendering with structured diagnostics.
- [ ] 6.8 Add tests or instrumentation proving the steady-state render path performs no heap allocation.
- [ ] 6.9 Run `openspec validate add-wavetable-synthesis --strict` and fix validation errors.
