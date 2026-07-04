## 1. Envelope Follower Primitive

- [ ] 1.1 Add failing unit tests for level detection from positive and negative audio samples.
- [ ] 1.2 Add failing unit tests for attack smoothing when the input level rises.
- [ ] 1.3 Add failing unit tests for release smoothing when the input level falls.
- [ ] 1.4 Promote or adapt the existing `EnvelopeFollower` utility for graph primitive use.
- [ ] 1.5 Add tests for `amount`, `offset`, and `invert` behaviour.
- [ ] 1.6 Add tests proving output remains finite for extreme inputs and parameters.

## 2. Curve Mapper Primitive

- [ ] 2.1 Add failing unit tests for `linear`, `exponential`, `logarithmic`, `s_curve`, `soft_clip`, `hard_clip`, and `step` curves.
- [ ] 2.2 Implement `CurveMapper` processing.
- [ ] 2.3 Add tests for `amount` blend behaviour.
- [ ] 2.4 Add tests for `scale` and `offset` behaviour.
- [ ] 2.5 Add tests proving invalid inputs produce finite bounded outputs.

## 3. Built-in Registry

- [ ] 3.1 Add `envelope_follower` and `curve_mapper` module type constants.
- [ ] 3.2 Add built-in module definitions with typed ports.
- [ ] 3.3 Add parameter metadata for detection mode, curve selection, and step count.
- [ ] 3.4 Add registry tests proving both modules expose expected ports and metadata.

## 4. Graph Processor Integration

- [ ] 4.1 Add `ModuleKind` variants for `EnvelopeFollower` and `CurveMapper`.
- [ ] 4.2 Add `PerModuleState` support where state is required.
- [ ] 4.3 Add dispatch functions for both primitives.
- [ ] 4.4 Add deterministic render tests for `audio -> envelope_follower -> curve_mapper -> gain/filter` routing.

## 5. Examples

- [ ] 5.1 Add a ducking example using `envelope_follower` inverted into a downstream gain control.
- [ ] 5.2 Add a modulation example using `envelope_follower -> curve_mapper -> filter.cutoff`.
- [ ] 5.3 Document that this is the preferred path for audio-derived control signals, not scripts.

## 6. Verification

- [ ] 6.1 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [ ] 6.2 Run CMake/CTest verification if CMake configure/build is available.
- [ ] 6.3 Run `openspec validate add-envelope-follower-control-primitives --strict` if the OpenSpec command is available.
- [ ] 6.4 Update task checkboxes only after the related tests and verification pass, or document the verification gap.
