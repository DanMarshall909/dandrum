## 1. Specification And Validation Surface

- [ ] 1.1 Define the `wavetable-synthesis` capability and its module/asset requirements.
- [ ] 1.2 Add `wavetable_oscillator` to the built-in module registry with typed ports and parameter metadata.
- [ ] 1.3 Extend YAML/schema validation to accept supported wavetable asset declarations and reject unsupported ones.
- [ ] 1.4 Add structured diagnostics for missing wavetable assets, malformed table data, invalid frame sizes, and unsupported interpolation/anti-aliasing modes.

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

## 4. Patch And Preset Examples

- [ ] 4.1 Add a minimal wavetable oscillator patch that renders a stable tone.
- [ ] 4.2 Add a wide virtual-analogue lead patch using wavetable/unison-style composition, filter, envelope, and effects.
- [ ] 4.3 Add an evolving pad patch with slow wavetable position modulation.
- [ ] 4.4 Add preset surfaces exposing musical controls without exposing internal module IDs.

## 5. Verification

- [ ] 5.1 Add registry tests proving `wavetable_oscillator` exposes the expected ports and parameters.
- [ ] 5.2 Add render tests proving deterministic output for identical render settings and wavetable assets.
- [ ] 5.3 Add render tests proving frequency-to-pitch behaviour across representative notes.
- [ ] 5.4 Add tests proving wavetable position interpolation/morphing behaves continuously.
- [ ] 5.5 Add tests proving invalid/missing wavetable assets fail before rendering with structured diagnostics.
- [ ] 5.6 Add tests or instrumentation proving the steady-state render path performs no heap allocation.
- [ ] 5.7 Run `openspec validate add-wavetable-synthesis --strict` and fix validation errors.
