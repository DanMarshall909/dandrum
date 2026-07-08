## Why

Some sampled instruments eventually need workstation-sampler features: articulations, release triggers, key switches, crossfaded layers, richer zone/group metadata, and import/export compatibility. Those features should not be folded into the primitive-first advanced sampling slice because they would turn the initial sampler into a monolith.

This change is a separate future capability for richer instrument-style sampling built on top of the lower-level sample primitives.

## What Changes

- Add a future `workstation-sampling` capability for complex multi-sample instruments.
- Build on `sample_player`, `sample_zone_selector`, voice/choke behaviour, envelopes, filters, and module-library composition rather than replacing them with one opaque sampler.
- Consider articulation selection, key switches, release triggers, crossfaded velocity layers, sample groups, richer region metadata, and optional import formats.
- Keep this capability separate from drum-machine/break-slicer/chromatic v1 sampling.

## Capabilities

### New Capabilities

- `workstation-sampling`: Future capability for richer multi-sample instrument behaviour built from lower-level sampling primitives.

### Modified Capabilities

- `advanced-sampling-options`: Provides the primitive base but does not include workstation features.
- `yaml-patch-format`: May later support articulation/group metadata once this capability is specified.
- `module-library`: May later ship reusable workstation-style instrument modules composed from primitives.

## Impact

- This is intentionally not part of the first advanced sampling implementation.
- Requires separate design before implementation because it affects event modelling, preset surfaces, module-library packaging, and validation complexity.
- Non-goal: do not introduce Kontakt/SFZ-scale behaviour into the primitive sampling spec by accident.
