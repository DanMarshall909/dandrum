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
| Patch or defined module interface | Root port | output module, implicit stereo sink |
| Host connection to a root audio port | Named bus | fixed left/right output |
| Repeated note processing area | Poly region | graph-wide voice scope |

Internal Rust types may keep existing names such as `ModuleNode` where they already describe implementation detail. New user-facing names should follow the preferred terms.

## Signal types

Dandrum uses three signal types:

- `audio` — audio-rate sample streams.
- `control` — per-sample modulation/control signals; constant sources may repeat one value across a block.
- `event` — note, trigger, and other discrete events.

Use **control signal** in documentation. Avoid **CV** unless explicitly comparing Dandrum to modular synthesizer terminology.

## Layering terms

Use these responsibility boundaries:

- **Primitive**: tested Rust module for realtime-safe DSP/control behaviour.
- **Defined module**: reusable YAML graph built from primitives and other defined modules.
- **Script module**: block-scheduled event/control policy logic only.
- **Patch**: complete instrument/effect graph that can be validated and rendered.
- **Preset**: named parameter values applied to a compatible patch or module surface.

A patch and a defined module have the same graph shape: ports, modules, and cables.
The patch's root ports form its public interface. A root audio output connects to
a host bus of the same name and channel count; a stereo output can be one
two-channel `master` port. A root control input can expose a live public value.

Use **static argument** for a value resolved when the graph is prepared, such
as channel count, waveform, maximum delay length, or a sample resource. Use
**control port default** for a tunable value that a cable or host can replace
while the patch runs. A summing input has **summing multiplicity** and accepts
multiple cables; ordinary inputs accept one.

`poly` creates a **poly region** with a fixed maximum voice count and its own
note allocation. `feedback_delay` is the explicit boundary required in a
feedback cycle; an ordinary delay effect does not make a cycle legal.

## Naming style

Module type names should be lower snake case noun phrases:

- `envelope_follower`
- `curve_mapper`
- `note_to_control`
- `frequency_splitter`
- `spectral_processor`

Prefer general engine names over product-specific feature names. For example, use `envelope_follower` rather than `peak_controller`, even when the intended behaviour is similar to FL Studio's Peak Controller.

Use verbs for functions and commands, not module types.
