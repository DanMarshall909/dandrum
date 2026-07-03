## Context

Dandrum already supports much of the declarative engine direction:

- Rust DSP engine and headless renderer
- JUCE wrapper for realtime audio/MIDI IO
- YAML patch loading
- typed modules and ports
- inline `module_definitions` for reusable composites
- deterministic composite expansion into namespaced internal modules
- graph validation for missing modules, missing ports, incompatible signal types, multiple-source inputs, invalid
  cycles, and voice/global routing boundaries
- built-in primitives including oscillator, gain, mixer, ADSR, filter, sampler, saturation, dynamics, convolution, echo,
  reverb, splitter, and spectral processor

This change should not redesign those foundations. It should define how to extend them carefully.

The platform goal is:

```text
Intentional YAML definition -> validated graph -> deterministic Rust DSP render
```

Future LLM tooling should be able to target this model, but only after the platform is stable.

## Goals / Non-Goals

**Goals:**

- Define the layer model: Rust primitives -> YAML composites -> Script glue -> Presets -> Future tooling.
- Preserve the existing inline composite mechanism based on `module_definitions`.
- Add only the minimum new primitives needed for the first useful synthetic instrument examples.
- Define when a proposed behaviour becomes a primitive, composite, script, preset, future tooling, or out-of-scope.
- Define script constraints for deterministic event/control transformation.
- Define structured validation/diagnostics suitable for humans now and repair tooling later.
- Define staged acceptance examples that prove platform capability without hardcoded Rust instrument modules.
- Define capability discovery as a non-realtime query surface built on module/parameter metadata.

**Non-Goals:**

- Building an LLM generation layer.
- Building a GUI editor.
- Replacing existing `module_definitions` with a parallel `type: composite` system.
- Adding every plausible synth primitive.
- Hardcoding 808/909 voices as Rust modules.
- Making the drum machine a sampler, mixer, sequencer, or hidden signal-chain host.
- Allowing unrestricted scripting in the audio callback.
- Hiding sound-generation policy inside opaque containers.

## Decisions

### D1: Four implementation layers plus future tooling

Dandrum uses these layers:

1. **Rust primitives** own audio-rate DSP, realtime-safe mutable state, and performance-critical operations.
2. **YAML composites** own reusable voices, instruments, effects, and higher-level building blocks assembled from
   primitives and other composites.
3. **Scripts** own bounded event/control transformation and small amounts of deterministic stateful glue.
4. **Presets** own named usable configurations of patches, composites, and parameters.
5. **Future tooling** owns LLM authoring, GUI editing, documentation generation, and patch repair workflows.

Rust primitives should remain small and general. Composites should carry musical structure. Scripts should fill
control/event gaps without becoming an audio DSP sandbox. Presets should demonstrate useful sounds without changing
runtime semantics.

### D2: Primitive gate is strict by default, explicit by exception

A new Rust primitive SHOULD satisfy all five criteria:

1. Performance-critical.
2. Reusable across multiple instruments/effects.
3. Needs realtime-safe internal state or audio-rate processing.
4. Awkward, fragile, or unsafe as YAML composition.
5. Has clear testable DSP behaviour.

If a proposed primitive does not satisfy all five, it MAY still be accepted only when the spec documents the failed
criteria, the rejected alternatives, and the concrete acceptance example that requires it.

### D3: Preserve existing inline composites

Composites continue to use the existing `module_definitions` model. A patch can define reusable module types inline,
then instantiate them by using the definition's type as a normal module type.

This change may harden the existing model by adding:

- parameter metadata
- better source-mapped diagnostics after expansion
- optional external composite library loading
- deterministic expansion tests
- improved documentation and examples

It must not introduce a conflicting second composite model unless a separate migration spec explicitly deprecates the
current one.

### D4: Composite expansion remains a load/compile-time graph transformation

Composite expansion occurs before rendering. The renderer should see a flat graph of renderable primitive/script modules
with deterministic, namespaced IDs.

Validation should be able to report errors against both the expanded internal node and the original YAML/composite
source where possible.

### D5: Scripts are event/control glue, not audio-rate DSP

Scripts are parsed, validated, and prepared off the audio thread. The runtime must enforce:

- no filesystem APIs
- no network APIs
- no blocking calls
- bounded execution cost
- deterministic execution
- no recursive same-tick graph execution
- no heap allocation during render-time execution
- no audio-rate output ports in the initial implementation

Scripts are appropriate for velocity mapping, note remapping, conditional event routing, event-to-control conversion,
and simple control modulation logic.

### D6: Drum machine remains an event mapper

The drum machine module is an event transformer. It maps incoming note/event triggers to named pad event outputs. It
does not contain samples, synthesis chains, sequencers, clocks, mixers, effects, or hidden audio behaviour.

Pad outputs should trigger explicitly connected downstream voice composites or modules.

### D7: Diagnostics become structured before repair loops

Validation and runtime errors should use structured records containing:

- stable error code
- severity
- YAML file path and source range when available
- module ID and port name where applicable
- expected type/value
- actual type/value
- message
- suggested fix when safe

This is valuable for humans immediately and enables future LLM repair loops later without string matching.

### D8: Capability discovery depends on metadata, not renderer logic

Capability discovery is a query interface over module/composite/script metadata. It must stay separate from realtime
rendering.

The first useful surface is:

- module type list
- category
- ports
- parameter names/types/defaults/ranges/units/enums
- realtime notes
- short YAML examples where available

Do not implement discovery by scanning renderer internals during audio processing.

## Primitive Roadmap

### Implement in this change or immediate follow-up

- `noise`: required for synthetic percussion, hats, snares, and modulation sources.
- `impulse`: required for click/transient generation and sample-accurate trigger testing.
- `multiply`: required for VCA-like modulation, pitch/control scaling, tremolo, and ring-mod-style composites.
- `note_to_control`: required for normal instrument voices; should expose frequency, pitch ratio/CV, gate/trigger, and
  velocity control values.

### Clarify before implementing

- minimal oscillator waveform support: required if acceptance examples use sine/saw/pulse/triangle. Either implement the
  minimum waveform parameter now or restrict examples to current oscillator behaviour.

### Defer

- envelope follower
- general delay line
- FM operator
- resonator
- state-variable filter
- wavefolder
- sample-and-hold
- specialist 808/909 voice modules

## Risks / Trade-offs

- **Existing composite support may be accidentally bypassed** -> specs and tasks must reference `module_definitions` and
  expansion hardening, not a parallel composite syntax.
- **Primitive creep** -> start with the minimum primitives required by the first acceptance example; defer attractive
  but unproven modules.
- **Examples may assume unavailable oscillator behaviour** -> add minimal waveform support or simplify examples.
- **Diagnostics may become too large a first step** -> implement structured diagnostic foundations first, then source
  locations and suggested fixes incrementally.
- **Capability discovery may arrive before metadata exists** -> implement parameter metadata before broad discovery
  endpoints.
- **LLM concerns may distort the engine** -> keep LLM support as a future tooling consumer of stable schema,
  diagnostics, examples, and capability metadata.