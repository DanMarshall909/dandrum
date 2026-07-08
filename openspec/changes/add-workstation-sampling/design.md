## Context

Workstation sampling is deliberately separate from the first advanced sampling slice. The initial sampler should remain primitive-first and cover drum-machine playback, explicit slicing, and modest chromatic instruments. Workstation sampling can later compose those primitives into richer instruments.

## Scope Boundary

This future capability may cover:

- articulations,
- key switches,
- release triggers,
- crossfaded velocity layers,
- richer sample groups,
- per-articulation envelopes/filters/modulation,
- import/export formats such as an SFZ-like subset if justified,
- larger module-library instrument packages.

It should not redefine the low-level sample playback primitive. `sample_player`, zone selection, voice/choke behaviour, envelopes, filters, and modulation should stay reusable.

## Design Principle

Prefer orchestration primitives and module-library templates over one huge `workstation_sampler` primitive. Add a primitive only when the behaviour is stateful, reusable, realtime-sensitive, and awkward to express by composition.

## Non-Goals For Now

- No implementation in the advanced sampling v1 work.
- No proprietary sample-library compatibility target.
- No large importer before the native model is stable.
