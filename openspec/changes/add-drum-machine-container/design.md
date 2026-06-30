## Context

The engine already has a modular graph model with named typed ports, explicit connection validation, sampler modules, composite module expansion, and compiled render work in progress. Drum-style patches still need a small primitive for grouping named trigger pads such as kick, snare, and hat without embedding sequencing, audio-generation, or signal-chain policy in that primitive.

Bitwig's Drum Machine is useful inspiration here because its pad-oriented workflow maps incoming note signals to named drum pads. Dandrum should take the pad-trigger mapping idea while keeping the module event-only: pads emit configured events, and all sound generation, synthesis, sample playback, effects, and mixing remain ordinary graph modules connected by named typed ports.

## Goals / Non-Goals

**Goals:**
- Introduce a `drum_machine` event-only module type with named trigger pads.
- Let each pad declare a trigger selector, usually a note number or event label, and the event it emits when triggered.
- Expand each valid drum-machine module into deterministic internal event-routing modules before graph processing/compilation.
- Expose typed public event ports for each declared pad.
- Allow patches to connect pad outputs to explicitly declared samplers, synth voices, envelopes, script modules, or other downstream event consumers.
- Keep diagnostics stable and specific for invalid pads, duplicate selectors, missing ports, and incompatible routes.
- Cover behavior with Rust unit tests close to YAML parsing, validation, expansion, and event routing.

**Non-Goals:**
- Building a GUI drum-machine editor.
- Implementing step patterns, sequencing, tempo, clocking, transport, swing, probability, ratchets, or humanization.
- Adding sampler-specific velocity, choke-group, sample binding, fixed audio output, or polyphony policy.
- Hosting signal chains, generating audio, mixing audio, or owning downstream module behavior.
- Bypassing the existing event, sampler, mixer, or graph-validation model.
- Changing the C FFI or JUCE wrapper unless an example CLI path already requires it.

## Decisions

### 1. Event-module expansion instead of a bespoke render path

**Decision:** Parse `drum_machine` as an event-only module and expand it into ordinary event-routing graph nodes before graph processor construction.

**Rationale:** Expansion keeps routing inspectable and lets existing sampler, synth, effect, mixer, and compiled-patch work remain the execution substrate. A bespoke render path would incorrectly make the drum machine responsible for behaviors that belong to downstream signal chains.

**Alternative considered:** Implement `drum_machine` as a monolithic audio or sequencing module. This is broader than the intended primitive and would hide important graph behavior.

### 2. Pads are named event mappings

**Decision:** Each pad has a stable `id`, a trigger selector, and an emitted-event declaration. Pad IDs derive public event port names and appear in diagnostics. Trigger selectors decide which incoming events trigger each pad; the emitted-event declaration decides what event the pad outputs.

**Rationale:** Named pads are readable in YAML, stable for connections, and suitable for future UI mapping. The module's job is to say "this incoming note/event emits the configured kick event" rather than decide what the kick sounds like or when it plays.

**Alternative considered:** Use array position only. This is compact, but error messages and external routing become brittle after pad reorderings.

### 3. Pitch/event selector routing, no internal scheduler

**Decision:** The first version accepts trigger selectors and emitted-event declarations but does not accept pattern data, tempo, transport, or clock configuration. It routes incoming events to matching pad event outputs.

**Rationale:** This follows the useful Bitwig-inspired shape of note/event routing to pads, not step generation. Keeping the drum machine clockless prevents it from becoming a hidden step sequencer and keeps trigger timing externally observable in the graph.

**Alternative considered:** Add minimal boolean patterns. That was rejected because the container is intended as a primitive trigger surface, not a sequencer.

### 4. No built-in signal chains, audio, samples, or implicit mix output

**Decision:** The module owns event mapping only, and it does not host signal chains or create sound by itself. Patch authors connect pad outputs to sampler, synth, envelope, script, or other modules when they want sound.

**Rationale:** This keeps sample selection, synthesis policy, velocity handling, effects, and summing explicit in the patch. It also avoids special drum-specific DSP that would duplicate existing modules.

**Alternative considered:** Include per-pad sample assets and a fixed mixed audio output. That would be convenient for a kit preset, but it turns the container into a sampler/mixer rather than a primitive trigger container.

## Risks / Trade-offs

- **[Risk] The primitive may feel small compared with a full drum machine** -> Keep the name but define the contract tightly: it is a named event mapper that composes with external sequencers and sound generators.
- **[Risk] Users may expect built-in patterns or device chains** -> Diagnostics and examples should show external event sources driving pad inputs and explicit downstream signal chains producing audio.
- **[Risk] Expanded graphs add indirection** -> Use deterministic namespacing and build expansion once during patch preparation/compilation, not per audio block.
- **[Risk] Port naming can become awkward** -> Derive stable public port names from pad IDs and validate pad ID syntax up front.
