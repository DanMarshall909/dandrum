# Dandrum Nomenclature

Use this vocabulary consistently in code comments, examples, user-facing documentation, and OpenSpec changes.

## Core graph concepts

| Concept | Preferred term | Avoid in user-facing docs |
|---|---|---|
| Graph building block | Module | node, unit, processor |
| Built-in Rust implementation | Primitive | native module, DSP module |
| Reusable YAML graph | Defined module | composite, macro, subpatch unless referring to external systems |
| Complete instrument/effect definition | Patch | preset, graph |
| Saved parameter variation | Preset | patch |
| Runtime behaviour module | Script module | script node |
| Graph connection | Cable | wire, edge |
| Input/output endpoint | Port | pin |

Internal Rust types may keep existing names such as `ModuleNode` where they already describe implementation detail. New user-facing names should follow the preferred terms.

## Signal types

Dandrum uses three signal types:

- `audio` — audio-rate sample streams.
- `control` — continuous modulation/control signals.
- `event` — note, trigger, and other discrete events.

Use **control signal** in documentation. Avoid **CV** unless explicitly comparing Dandrum to modular synthesizer terminology.

## Layering terms

Use these responsibility boundaries:

- **Primitive**: tested Rust module for realtime-safe DSP/control behaviour.
- **Defined module**: reusable YAML graph built from primitives and other defined modules.
- **Script module**: event/control-rate policy logic only.
- **Patch**: complete instrument/effect graph that can be validated and rendered.
- **Preset**: named parameter values applied to a compatible patch or module surface.

## Naming style

Module type names should be lower snake case noun phrases:

- `envelope_follower`
- `curve_mapper`
- `note_to_control`
- `frequency_splitter`
- `spectral_processor`

Prefer general engine names over product-specific feature names. For example, use `envelope_follower` rather than `peak_controller`, even when the intended behaviour is similar to FL Studio's Peak Controller.

Use verbs for functions and commands, not module types.
