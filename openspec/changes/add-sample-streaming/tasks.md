## 1. Scope Definition

- [ ] 1.1 Define DJ-style streaming use cases separately from preloaded sample playback.
- [ ] 1.2 Decide which behaviour belongs in Rust engine versus plugin/host integration.
- [ ] 1.3 Keep streaming-specific behaviour out of the first advanced sampling implementation.

## 2. Buffering And Transport Design

- [ ] 2.1 Design prepared stream asset metadata.
- [ ] 2.2 Design bounded background IO/decode buffering.
- [ ] 2.3 Design audio callback read behaviour and underrun policy.
- [ ] 2.4 Design play/stop/cue/seek/rate control events.
- [ ] 2.5 Decide whether tempo sync, beat grids, and loop points belong in this spec or a later DJ-deck spec.

## 3. Verification

- [ ] 3.1 Add tests proving the audio callback does not perform blocking IO or allocation.
- [ ] 3.2 Add tests proving deterministic transport state transitions.
- [ ] 3.3 Add tests proving explicit underrun behaviour.
- [ ] 3.4 Run `openspec validate add-sample-streaming --strict` when requirements are added.
