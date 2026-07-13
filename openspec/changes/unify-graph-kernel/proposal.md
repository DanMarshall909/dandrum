## Why

The engine has accreted parallel concepts — patches vs modules, params vs control inputs, voice vs global execution scopes, a hardcoded stereo `audio_output` — that each need their own validation, discovery, and documentation, and that block planned features (arbitrary buses, surround, nested polyphony, LLM-generated instruments). Unifying them into one recursive graph kernel removes special cases before more features are built on top of them: every future feature should emerge from composition rather than a new engine concept.

## What Changes

- **BREAKING** Introduce a unified recursive graph kernel: `GraphDefinition` (static parameters, ports, nodes, connections), `Node` (instance of a definition), `Port` (signal type, rate, channel count, multiplicity, default value), `Connection`. A patch becomes simply the root graph definition; primitives are graph definitions implemented in Rust; composites are graph definitions implemented in YAML. Both expose identical port interfaces.
- **BREAKING** Parameters dissolve into ports: a module parameter is a control input port with a default value and optional range metadata. Unconnected inputs read their defaults. Preset surfaces become named references to root-graph ports.
- **BREAKING** Voice scope (`ExecutionScope::Voice`) is replaced by a `poly` structural node that wraps a graph definition, instantiates up to N copies, routes note events per an allocation/steal policy, exposes per-voice intrinsics (note, velocity, gate), detects voice completion, and mixes instance outputs. Voice-to-host isolation becomes structurally impossible rather than a validation rule.
- **BREAKING** Remove stereo assumptions: the `audio_output` primitive with hardcoded `left`/`right` is replaced by root graph output ports; the host binds named buses with arbitrary channel counts to root ports. Ports carry a channel count so one connection can be an N-channel bundle.
- Add static (compile-time) parameters on graph definitions — channel counts, voice counts, replication counts, max delay lengths — resolved during compilation; the compiler caches expansions keyed by (definition, static-args).
- Feedback cycles become legal only through an explicit feedback-delay primitive with a declared delay amount; implicit `feedback_boundaries` are removed and the scheduler remains a plain topological sort.
- Add per-node latency reporting with compiler-inserted compensation delays at converging paths and total-latency reporting to hosts. This fixes an existing defect: spectral (`fft_size - 1` samples) and overlap-add convolution (one block) already carry real latency, so parallel dry/wet paths around them are currently misaligned. Feedback cycles containing latency-bearing nodes are rejected.
- Signal rate becomes explicit alongside signal type; control→audio promotion uses inspectable sample-and-hold upsampling; audio↔event conversion stays illegal. Runtime stays statically typed — no runtime variant switching.
- Compilation recursively flattens composite definitions until only atomic Rust nodes remain, then schedules and plans buffers as today.
- The compiled runtime becomes channel-aware through contiguous buffer spans and represents each `poly` as an explicit nested runtime region; the existing graph-wide voice scope and fixed stereo output are removed only after hosts, packages, and examples migrate.

## Capabilities

### New Capabilities

- `graph-kernel`: the recursive GraphDefinition/Node/Port/Connection model — patch as root definition, port semantics (type, rate, channel count, default), unconnected-input defaults, recursive flattening, static runtime typing.
- `static-parameters`: compile-time parameters on graph definitions, resolution rules, expansion caching, and validation of static-argument mismatches.
- `poly-combinator`: polyphony as a structural node — instantiation limits, event routing, allocation/steal policy, per-voice intrinsics, voice-done detection, output mixing, preallocation of max voices.
- `host-buses`: named host buses with arbitrary channel counts bound to root graph ports for plugin, device, and offline-render boundaries.
- `latency-compensation`: per-node latency metadata and compiler-inserted delay compensation across parallel paths.
- `module-packages`: package entries load as graph definitions and resolve resource static arguments relative to the package root.

### Modified Capabilities

- `modular-routing-graph`: ports gain rate, channel count, and default value; multi-channel connections; cycles legal only through the feedback-delay primitive; `ExecutionScope` and `VoiceToGlobalDirectRouting` diagnostics removed.
- `composite-authoring`: composite definitions become full graph definitions (same interface as primitives), gain static parameters, and lose patch/module asymmetry.
- `yaml-patch-format`: patch document becomes a root graph definition; module `params` replaced by port defaults; `audio_output` and stereo render outputs replaced by root output ports; render settings become host-boundary concerns.
- `feedback-routing`: implicit feedback boundaries replaced by the explicit feedback-delay primitive requirement.
- `instrument-presets`: preset targets resolve to named root-graph ports instead of a separate parameter surface.
- `capability-discovery`: discovery reflects the unified port model (one schema describes primitives, composites, and patches alike).
- `rust-engine-architecture`: compiled-patch pipeline specifies recursive flattening, expansion caching, and flat arena execution of the kernel model.
- `built-in-modules`: primitives declare ports with rate, channel count, and default value; module parameters become port defaults; `audio_output` removed; `poly` and `feedback_delay` join the registry.
- `drum-kit-patches`: drum-kit example expresses polyphony via `poly` nodes and routes to named buses instead of `voice_allocation` and shared `audio_output`.
- `script-modules`: script-backed graph definitions declare their event/control interface once; instances use the ordinary node shape.

## Impact

- **Rust engine**: `graph.rs` (ModuleNode/Port/ExecutionScope), `patch.rs` (YAML schema, params), `patch_module.rs`/`graph_module.rs`/`module_library.rs` (composite handling), `graph_processor/*` (render plan, polyphony, arenas), `builtins/*` (port declarations gain rate/channels/defaults; `audio_output` removed), `voice_allocator.rs` (subsumed by poly node), `preparation.rs`/`compiled_patch.rs` (flattening pipeline), `ffi.rs` (bus binding API).
- **Schema**: `schema/patch.schema.yaml` rewritten for the kernel document shape.
- **Examples**: all `examples/patches/*.yaml` migrate (stereo L/R port pairs collapse to 2-channel ports; `audio_output` → root ports; voice scope → `poly`).
- **JUCE host / CLI / offline render**: consume named buses instead of fixed stereo output.
- **Module library** ($LIB/$USER_LIB packaging): module packages describe graph definitions with static parameters and package-relative resource resolution; discovery metadata format changes. Package migration is part of this change because final legacy cleanup depends on it.
- Existing patches and presets are not backward compatible; a migration pass over shipped examples is part of the change.
