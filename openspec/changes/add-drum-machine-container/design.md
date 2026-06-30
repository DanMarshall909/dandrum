## Context

The engine already has a modular graph model with named typed ports, explicit connection validation, sampler modules, composite module expansion, and compiled render work in progress. Drum-style patches still need a small primitive for grouping named trigger pads such as kick, snare, and hat without embedding sequencing or audio-generation policy in that primitive.

Bitwig's Drum Machine is useful inspiration here because its core shape is not a step sequencer: incoming note signals are routed by pitch to specific chains, and those chains contain the actual devices. Dandrum should take that container-and-pad-chain idea while preserving explicit graph safety: all sound generation, synthesis, sample playback, effects, and mixing remain ordinary graph modules connected by named typed ports.

## Goals / Non-Goals

**Goals:**
- Introduce a `drum_machine` container module type with named trigger pads.
- Let each pad declare a trigger selector, usually a note number or event label, and an optional child module chain.
- Expand each valid container into deterministic internal event-routing modules and pad child-chain modules before graph processing/compilation.
- Expose typed public event ports for each declared pad.
- Allow patches to connect pad outputs to samplers, synth voices, envelopes, script modules, or other downstream event consumers.
- Keep diagnostics stable and specific for invalid pads, duplicate selectors, missing ports, and incompatible routes.
- Cover behavior with Rust unit tests close to YAML parsing, validation, expansion, and event routing.

**Non-Goals:**
- Building a GUI drum-machine editor.
- Implementing step patterns, sequencing, tempo, clocking, transport, swing, probability, ratchets, or humanization.
- Adding sampler-specific velocity, choke-group, sample binding, fixed audio output, or polyphony policy.
- Generating audio, mixing pad chains, or owning downstream module behavior.
- Bypassing the existing event, sampler, mixer, or graph-validation model.
- Changing the C FFI or JUCE wrapper unless an example CLI path already requires it.

## Decisions

### 1. Pad-chain container expansion instead of a bespoke render path

**Decision:** Parse `drum_machine` as a container module and expand it into ordinary event-routing graph nodes plus any declared pad child-chain modules before graph processor construction.

**Rationale:** Expansion keeps routing inspectable and lets existing sampler, synth, effect, mixer, and compiled-patch work remain the execution substrate. A bespoke render path would incorrectly make the drum machine responsible for behaviors that belong to child-chain or downstream modules.

**Alternative considered:** Implement `drum_machine` as a monolithic audio or sequencing module. This is broader than the intended primitive and would hide important graph behavior.

### 2. Pads are named trigger chains

**Decision:** Each pad has a stable `id` and a trigger selector. Pad IDs derive public event port names and appear in diagnostics. Trigger selectors decide which incoming events enter each pad chain.

**Rationale:** Named pads are readable in YAML, stable for connections, and suitable for future UI mapping. The container's job is to say "this incoming note/event triggers the kick pad" rather than decide what the kick sounds like or when it plays.

**Alternative considered:** Use array position only. This is compact, but error messages and external routing become brittle after pad reorderings.

### 3. Pitch/event selector routing, no internal scheduler

**Decision:** The first version accepts trigger selectors but does not accept pattern data, tempo, transport, or clock configuration. It routes incoming events to matching pad event outputs and pad child-chain inputs.

**Rationale:** This follows the useful Bitwig-inspired shape: note/event routing into per-pad chains, not step generation. Keeping the drum machine clockless prevents it from becoming a hidden step sequencer and keeps trigger timing externally observable in the graph.

**Alternative considered:** Add minimal boolean patterns. That was rejected because the container is intended as a primitive trigger surface, not a sequencer.

### 4. No built-in audio, samples, or implicit mix output

**Decision:** The container owns trigger routing and child-chain structure, but it does not create sound by itself. Patch authors put sampler, synth, envelope, script, or other modules in pad chains or connect pad outputs externally when they want sound.

**Rationale:** This keeps sample selection, synthesis policy, velocity handling, effects, and summing explicit in the patch. It also avoids special drum-specific DSP that would duplicate existing modules.

**Alternative considered:** Include per-pad sample assets and a fixed mixed audio output. That would be convenient for a kit preset, but it turns the container into a sampler/mixer rather than a primitive trigger container.

## Risks / Trade-offs

- **[Risk] The primitive may feel small compared with a full drum machine** -> Keep the name but define the contract tightly: it is a named trigger container that composes with sequencers and sound generators.
- **[Risk] Users may expect built-in patterns** -> Diagnostics and examples should show an external sequencer or event source driving the pad inputs.
- **[Risk] Expanded graphs add indirection** -> Use deterministic namespacing and build expansion once during patch preparation/compilation, not per audio block.
- **[Risk] Port naming can become awkward** -> Derive stable public port names from pad IDs and validate pad ID syntax up front.
