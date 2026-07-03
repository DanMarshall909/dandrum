## Context

Dandrum currently has a working Rust DSP engine with YAML patch loading, graph routing, built-in modules (osc, gain, mixer, ADSR, filter, sampler, saturation, dynamics, delay/reverb), a headless renderer, and a JUCE wrapper for realtime audio. The existing architecture was built feature-by-feature without an overarching decision framework for what belongs in the Rust engine vs. what should be expressed in YAML.

The project needs a coherent platform architecture that:
- Keeps the Rust engine small and justified
- Enables rich instruments through YAML composition
- Provides clear rules for future contributors
- Supports LLM-assisted authoring eventually without being driven by it now

## Goals / Non-Goals

**Goals:**
- Define the four-layer architecture: Rust primitives → YAML composites → Script glue → Presets
- Specify the decision framework that gates all future primitive additions
- Add the minimum set of new primitives justified by the decision framework
- Define sandboxed script execution with hard realtime constraints
- Specify structured validation/diagnostics with stable error codes
- Define composite expansion as a deterministic compile-time step
- Specify acceptance examples that validate the platform without new primitives
- Design a capability discovery API for future tooling

**Non-Goals:**
- Implementing a GUI editor
- Building an LLM generation layer
- Adding every possible synth primitive
- Hardcoding 808/909 as special Rust modules
- Making drum machine a sampler/mixer/sequencer
- Unrestricted scripting in the audio thread
- Hiding signal chains inside opaque containers

## Decisions

### D1: Four-layer architecture

Primitives → Composites → Scripts → Presets, with clear gates between layers.

- **Rust primitives**: own audio-rate DSP, realtime-safe state, performance-critical paths. Each must pass the five-question gate (performance-critical, reusable, realtime-safe state, awkward as YAML, testable DSP).
- **YAML composites**: own reusable instrument/voice/effect definitions. Composed entirely from existing primitives and other composites. Expanded at load time into flat primitive graphs.
- **Scripts**: own event/control transformation. Bounded, sandboxed, pre-validated. No audio-rate DSP. No FS/network. No unbounded allocation during render.
- **Presets**: own named usable configurations of modules/composites with parameter values. Examples demonstrate platform capability, not engine features.

**Rationale**: Clear layer boundaries prevent scope creep. Each layer has different performance, safety, and authoring requirements. An LLM can later target the YAML layer without touching Rust.

### D2: Composite expansion is a compile-time graph transformation

Composites SHALL be expanded into their constituent primitive graphs during patch loading, before rendering begins. The expanded graph is flat (primitives only), and all validation (type checking, cycle detection, port resolution) runs on the expanded graph.

**Rationale**: Simplifies the renderer — it only needs to know about primitives. Simplifies validation — all tools work on the same flat representation. Keeps the realtime path simple and deterministic.

**Alternatives considered**: Dynamic composite dispatch (runtime module call stack). Rejected because it complicates validation, cycle detection, and realtime scheduling.

### D3: Script sandboxing through pre-validation and constrained runtime

Scripts SHALL be parsed, validated, and compiled off the audio thread. The runtime SHALL enforce:
- No filesystem or network APIs available in script scope
- No heap allocation during execution (pre-allocated scratch buffers)
- Bounded instruction count per block
- No recursive graph calls
- Deterministic execution (same input → same output)
- Stable error reporting through the diagnostics system

**Rationale**: Pre-validation catches errors before rendering starts. Constrained runtime prevents realtime violations. Determinism enables offline rendering to match realtime.

### D4: Drum machine is an event mapper, not an audio engine

The drum machine module SHALL be a stateless event transformer: it maps incoming MIDI events to named pad event outputs. It SHALL NOT contain samples, synthesis chains, sequencers, or mixers. Pad events flow to explicitly connected downstream voice modules.

**Rationale**: Keeps the drum machine small and reusable. Voices can be any module or composite (synthetic, sampler, effect). Explicit connections keep signal flow visible in the YAML.

### D5: Structured diagnostics with stable error codes

All validation and runtime errors SHALL produce structured diagnostic records containing:
- Stable error code (string, namespaced by subsystem)
- Severity (error, warning, info)
- YAML file path and line/column range
- Module ID and port name where applicable
- Expected type/value and actual type/value
- Human-readable message
- Suggested fix where safe to compute

**Rationale**: Structured diagnostics support both human debugging and future LLM repair loops. Stable error codes let tools reference known issues without string matching.

### D6: Capability discovery as a separate query interface

Capability discovery SHALL be a query API (not a file scan) that returns module type metadata: ports, signal types, parameter names/ranges/defaults, realtime notes, and category (primitive/composite/script). It SHALL be implemented as a separate concern from the renderer.

**Rationale**: Keeps the renderer lean. The discovery API can be used by CLI tools, future GUIs, and LLM tool-calling without affecting realtime paths.

## Risks / Trade-offs

- **Composite expansion hides intermediate state**: Expanded graphs can be large. Debugging a composite means understanding its expansion. **Mitigation**: Diagnostics map errors back to original composite module IDs and YAML paths. Provide a `--show-expanded-graph` CLI flag for debugging.
- **Script sandboxing limits expressiveness**: Users may want general scripting. **Mitigation**: Start constrained; relax only with specific justified extensions. General-purpose scripting belongs in a separate tooling layer, not the audio engine.
- **Primitive gate may be too strict**: Legitimate primitives could be blocked. **Mitigation**: The gate is a review tool, not a hard reject. If a candidate passes 3/5 criteria with strong rationale, it can still be added. Document the exception.
- **Capability discovery scope creep**: Could grow into a full introspection system. **Mitigation**: Define the MVP surface in the spec; defer advanced queries (e.g., dynamic port enumeration) to future changes.
- **Offline/realtime determinism gap**: Realtime scheduling may introduce subtle differences from offline rendering. **Mitigation**: Define determinism requirements in the spec; test with byte-exact comparisons for fixed-block renders.
