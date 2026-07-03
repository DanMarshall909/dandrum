## Context

Dandrum is aiming at a generic, declarative modular instrument/effect system that is readable by humans and future LLM tooling. Standard DAW devices such as drum machines and polyphonic synths are important targets, but they should be expressed as YAML graphs/composites over reusable engine capabilities rather than as special-purpose Rust instrument modules.

The previous drum-machine-container direction proved that event routing is the missing capability. This change extracts that need into general event-routing primitives.

## Goals / Non-Goals

**Goals:**

- Provide generic event-only modules for routing musical events by note and event properties.
- Keep primitives small, metadata-rich, deterministic, and reusable across drum machines, synth splits, articulations, velocity layers, and future sequencing/control tools.
- Make drum-machine and simple polyphonic synth examples dogfood targets for the generic system.
- Preserve explicit graph routing: sound generation, sample playback, mixing, effects, presets, and voice allocation remain ordinary graph/composite behavior.
- Keep YAML readable enough that a human or future LLM can inspect available metadata and write a correct patch.

**Non-Goals:**

- Adding a `drum_machine`, `drum_pad`, `poly_synth`, or other instrument-specific primitive.
- Adding a sequencer, timeline, clip launcher, DAW channel model, pattern editor, transport, or UI-facing pad rack.
- Hiding samples, synth chains, effects, mixers, or output busses inside event-routing modules.
- Allowing event-routing modules to emit audio/control signals in the initial implementation.

## Decisions

### 1. Event routing is a primitive family, not a drum-machine container

Event filtering and routing are general graph capabilities. Drum pads, keyboard splits, articulation switching, velocity layers, and poly-synth input conditioning are all event-routing use cases. The engine should provide general modules that compose into those products.

### 2. Dogfood targets drive requirements

The first dogfood targets are:

- a DAW-style drum machine built from event routing, sampler/synth voices, velocity mapping, explicit mixers, effects, and presets
- a simple polyphonic subtractive synth built from event routing, voice allocation, note-to-control, oscillator/filter/envelope/VCA composites, modulation, and presets

If either target needs a missing behavior, the first question is whether the missing behavior is a reusable primitive, a composite pattern, or a preset/schema problem.

### 3. Readability and metadata are part of the feature

The routing modules must expose typed ports, parameter metadata, defaults, allowed selector forms, and short examples through capability discovery. The YAML should favor explicit names over compact but opaque encodings.

### 4. Event routing remains event-only

Event-routing modules consume and emit events. They do not generate audio, own samples, apply effects, mix outputs, allocate voices, or schedule future events unless a later event-scheduling primitive explicitly defines that boundary.

## Risks / Trade-offs

- A generic router may be less convenient than a drum-machine module -> keep convenience in composites/examples/tooling, not Rust primitives.
- Selector syntax can become too broad -> start with note-number matching and explicit event-field equality before adding ranges, labels, or expressions.
- Dogfood examples may pressure the engine toward product-shaped modules -> reject primitive names that only make sense for one device type.
- Readable YAML can become verbose -> use composites and examples to package repeated patterns without hiding graph behavior.
