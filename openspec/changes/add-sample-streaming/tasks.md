## 1. Scope Definition

- [ ] 1.1 Define DJ-style streaming use cases as long-file `sample_source` behaviour rather than a separate sampler family.
- [ ] 1.2 Decide which behaviour belongs in Rust engine versus plugin/host integration.
- [ ] 1.3 Keep streaming-specific buffering out of the first advanced sampling implementation while preserving shared source metadata compatibility.
- [ ] 1.4 Define how preloaded sources and streaming sources expose common metadata.
- [ ] 1.5 Define tempo/pitch control intent separately from the v1 rendering modes that can actually honour it.

## 2. Buffering And Transport Design

- [ ] 2.1 Design prepared stream asset metadata using the shared sample source model.
- [ ] 2.2 Design bounded background IO/decode buffering.
- [ ] 2.3 Design audio callback read behaviour and underrun policy.
- [ ] 2.4 Design play/stop/cue/seek/rate control events.
- [ ] 2.5 Define source/transport metadata outputs: position, remaining time, buffer fill, loaded range, underrun state, transport state, tempo, beat phase, next beat, and next downbeat.
- [ ] 2.6 Decide whether transport state is a separate primitive, internal component, or both.
- [ ] 2.7 Define `free`, `rate`, `beat_locked_rate`, and future `stretch` tempo modes.
- [ ] 2.8 Define `source_bpm`, `target_bpm`, `tempo_ratio`, `manual_rate`, `nudge_ratio`, `pitch_shift_semitones`, and `pitch_ratio` controls.
- [ ] 2.9 Define explicit fallback/diagnostic behaviour when independent BPM and pitch controls are requested without a supported pitch-preserving stretch path.

## 3. Beat And Cue Metadata

- [ ] 3.1 Support explicit beat-grid metadata declared in YAML or sidecar metadata.
- [ ] 3.2 Design preparation-time beat detection as a bounded analysis step.
- [ ] 3.3 Decide whether background beat analysis can safely publish metadata after source preparation.
- [ ] 3.4 Represent beat analysis provenance as `declared`, `detected`, `missing`, or equivalent.
- [ ] 3.5 Represent analysis confidence and low-confidence/missing analysis diagnostics explicitly.
- [ ] 3.6 Validate cue points, loop points, beat markers, and downbeats against source duration.

## 4. Verification

- [ ] 4.1 Add tests proving the audio callback does not perform blocking IO, beat detection, time-stretch analysis, or allocation.
- [ ] 4.2 Add tests proving deterministic transport state transitions.
- [ ] 4.3 Add tests proving explicit underrun behaviour.
- [ ] 4.4 Add tests proving streaming metadata outputs are accurate and stable where expected.
- [ ] 4.5 Add tests proving explicit beat-grid metadata is parsed, validated, and exposed.
- [ ] 4.6 Add tests for missing/low-confidence beat analysis diagnostics when automatic detection is implemented.
- [ ] 4.7 Add tests proving `beat_locked_rate` derives effective rate from target/source BPM.
- [ ] 4.8 Add tests proving unsupported `stretch` mode and unsupported independent tempo/pitch requests produce explicit validation errors or configured diagnostics.
- [ ] 4.9 Run `openspec validate add-sample-streaming --strict` when requirements are added.
