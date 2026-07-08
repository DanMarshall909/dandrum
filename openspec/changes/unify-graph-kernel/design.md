## Context

Dandrum compiles declarative YAML patches into realtime-safe execution graphs. Today the model has several parallel concepts that this change unifies:

- `Graph`/`ModuleNode`/`Port`/`Cable`/`SignalType` in `graph.rs`, with `ExecutionScope::{Voice, Global}` annotated per node and a `VoiceToGlobalDirectRouting` diagnostic enforcing voice isolation.
- Composites via top-level `module_definitions` with `maps_to`/`maps_from` port forwarding — already recursive in spirit, but a patch is a distinct document shape (metadata, render settings, `modules`, `connections`) rather than a graph definition itself.
- Module `params` as a `BTreeMap<String, String>` distinct from control input ports; presets target a separate `preset_surface`.
- A hardcoded stereo `audio_output` primitive (`left`/`right`) and stereo render assumptions; examples hand-unroll stereo into `_l`/`_r` mono port pairs (see `examples/patches/module-echo.yaml`).
- Feedback handled by `feedback_boundaries` attributes plus `CycleDetected` validation.

The architecture review (2026-07-08) benchmarked the proposal against Cmajor/SOUL (closest relative: processors + graphs, stream/value/event endpoints, compile-time flattening), Faust (structural composition + compilation works; polyphony bolted on outside the core is its weakness), Max/Pd (`poly~` validates subgraph-as-node; keep signal/event runtime split), VCV Rack (warns against unifying rates at runtime), SuperCollider (named arbitrary buses; SynthDef/Synth mirrors Definition/Node), and CLAP (bus model to align the host boundary with).

Constraints: realtime callback stays lock-free, allocation-free, IO-free (README contract). Runtime stays statically typed — no runtime variant switching. Existing flat-arena work (`graph_processor/audio_arena.rs`, `render_plan.rs`) is the execution substrate to build on, not replace.

## Goals / Non-Goals

**Goals:**

- One recursive model: `GraphDefinition` → `Node` → `Port` → `Connection`, with primitives (Rust) and composites (YAML) exposing identical interfaces; patch = root graph definition.
- Parameters as ports with defaults; unconnected inputs read defaults; presets target root ports.
- Polyphony as a composable `poly` node instead of an engine-wide execution scope.
- Named host buses with arbitrary channel counts; no stereo assumptions anywhere in the kernel.
- Static parameters on graph definitions (channel counts, voice counts, replication, max delay).
- Explicit feedback-delay primitive as the only legal cycle boundary.
- Latency metadata in the node contract from day one, with compensation implemented in this change: the zero-latency assumption is already false today (`spectral_processor` documents `fft_size - 1` samples; overlap-add convolution carries one block of latency), so parallel dry/wet paths around those modules are currently misaligned.
- Compilation = recursive flatten → typecheck → schedule → buffer-plan, keeping incremental recompiles fast for live editing.

**Non-Goals:**

- Native code generation (Cranelift/LLVM). The compiled artifact is designed so codegen can slot in later as an optimization tier; this change ships the statically-dispatched flat interpreter.
- A `Metadata` signal type. Runtime-flowing metadata is deferred until a concrete feature demands it (beat detection outputs are control/event; tempo/transport are host ports).
- Oversampling/multi-rate regions beyond the audio/control rate distinction.
- Backward compatibility with the current patch YAML shape (shipped examples are migrated in-repo; there are no external users yet).
- GUI/editor work beyond keeping `schema/patch.schema.yaml` accurate.

## Decisions

### D1: Kernel types

`GraphDefinition { name, static_params, ports, nodes, connections }`, `Node { id, definition_ref, static_args, port_defaults_overrides }`, `Port { name, direction, signal_type, rate, channels, default }`, `Connection { source: PortRef, destination: PortRef }`. A primitive is a `GraphDefinition` whose body is a Rust implementation; a composite's body is nodes + connections. The patch file *is* the root `GraphDefinition`. Alternative considered: keep patch as a wrapper document holding a graph — rejected because the asymmetry is exactly what forces duplicate validation/discovery paths today and blocks module reuse of whole patches.

### D2: Signal type × rate, not fused

`signal_type ∈ {audio, control, event}` stays the user-facing vocabulary (matches `docs/nomenclature.md`), but internally audio = per-sample float stream, control = per-block float, event = timestamped queue. Compatibility rules: same-type connects; `control → audio` is legal via implicit promotion (compiler inserts an upsampler node, linear-interpolated); `audio → control` requires an explicit downsampler/follower node; event never implicitly converts. Rationale: VCV's all-audio-rate unification wastes CPU; Csound/SC rate systems are where the savings live. Alternative: require explicit converters everywhere — rejected as boilerplate that hurts LLM and human authoring for the overwhelmingly common knob→audio-param case.

### D3: Parameters are ports with defaults

`ModuleNode::params` (stringly-typed map) is deleted. Every tunable becomes a control input port with `default`, optional `min`/`max`/`unit` metadata. Node instances may override port defaults (`port_defaults_overrides`), which is what YAML `params:` blocks compile to during migration. `preset_surface` becomes named aliases onto root-graph ports; preset application writes port defaults. Static shape-affecting values (e.g. delay max length, channel count) are *not* ports — they are static params (D5). Rationale: one concept, one validation path, and modulating anything becomes "connect a cable" with no param/port distinction to discover.

### D4: `poly` combinator replaces voice scope

A structural primitive `poly { definition_ref, static max_voices, allocation policy }`:

- Preallocates `max_voices` flattened instances of the wrapped definition (satisfying the no-allocation callback contract).
- Routes incoming note events to instances: policy `oldest-steal` initially, declared as an enum so alternatives can be added.
- Injects per-voice intrinsic input ports into the wrapped definition's scope: `voice.note` (control), `voice.velocity` (control), `voice.gate` (event or control — see Open Questions).
- Detects voice completion via a convention: the wrapped definition may expose a `done` event/control output; absent that, gate-off plus output-silence tracking frees the voice.
- Sums instance audio outputs per output port (the "voice mixer" from the original proposal is the node's mix stage, not a separate concept).

`ExecutionScope`, `VoiceToGlobalDirectRouting`, and `voice_allocator.rs` as a standalone concept are subsumed. Rationale: Faust's core weakness is polyphony outside the model; Max's `poly~` proves subgraph instancing as a node; nesting (per-pad voices, unison) falls out for free. Alternative: keep scope annotations and add nesting rules — rejected; every nesting rule is a special case the combinator makes structural.

### D5: Static parameters and expansion caching

Graph definitions declare `static_params` (typed: int, enum, resource-ref) usable in port channel counts, `poly` voice counts, replication counts, and primitive configuration. Nodes supply `static_args`. The compiler resolves all static args before flattening and caches expanded definitions keyed by `(definition_id, static_args)`. Channel-count polymorphism rides on this: a port's `channels` may reference a static param, so one `echo` definition serves mono/stereo/N-channel (collapsing today's `_l`/`_r` duplication). No general expression language — static args flow through by name and simple arithmetic is deferred. Rationale: this is the minimum genericity that makes "no stereo assumptions" real; a full metaprogramming layer (Faust's algebra) is explicitly deferred.

### D6: Feedback-delay primitive

Cycles are illegal except through the `feedback_delay` primitive, which declares its delay (≥1 block initially; per-sample feedback deferred). The scheduler stays a plain topological sort with `feedback_delay` nodes as cut points; `feedback_boundaries` node attributes are removed. Rationale: matches Faust `~`/gen~ `history`; makes the cycle rule teachable in one sentence and the scheduler trivially correct.

### D7: Host buses

The host boundary is: root graph input/output ports ↔ named buses declared by the host (`main: 2ch`, `sidechain: 1ch`, `cue: 2ch`, …). Binding is by name with channel-count validation at prepare time. The FFI (`ffi.rs`) exposes bus enumeration and buffer binding; JUCE/CLI/offline render map buses to devices/plugin ports/files. Bus vocabulary aligns with CLAP's audio-ports/note-ports model so plugin mapping is near 1:1. `render:` settings (sample rate, block size, duration) leave the patch document and become host/render-invocation concerns; examples carry them in a sidecar render config for the CLI. Alternative: keep render settings in patches for convenience — rejected; a patch that hardcodes its sample rate can't be a reusable module.

### D8: Compilation pipeline

`parse → resolve definitions (library + inline) → resolve static args → recursively flatten to atomic nodes → typecheck ports/rates/channels → cycle-check (feedback_delay cuts) → latency balance → topological schedule → buffer-reuse coloring into arenas → CompiledPatch`. Latency balancing accumulates per-node declared latency along paths, inserts compensation delays where paths of unequal latency converge, reports total root latency to the host, and rejects any feedback cycle containing a nonzero-latency node (compensation inside a loop is impossible). Composite and `poly` latency is the accumulated latency of the flattened contents; voices are identical instances, so poly latency is uniform. Flattening reuses the expansion cache; the whole pipeline stays linear-ish in flattened node count so live edits recompile fast. Execution remains the existing flat statically-dispatched node array over `audio_arena`. Per-voice instances share read-only data (wavetables, samples, coefficients) and get disjoint state slices.

### D9: Migration is one atomic in-repo cut, staged by layer

Trunk-based, no compatibility shims: (1) kernel types + validation land alongside old types; (2) compiler pipeline switched to kernel model; (3) builtins re-declare ports with rate/channels/defaults, `audio_output` deleted, `poly` + `feedback_delay` added; (4) YAML schema + all examples migrated; (5) old types (`ExecutionScope`, `params`, `preset_surface` plumbing, `feedback_boundaries`) deleted. Tests migrate with each stage; coverage target holds per repo policy (behaviour-driven, real code paths).

## Risks / Trade-offs

- [Flattening blow-up: composites × voices × channels multiply node count and compile time] → expansion caching keyed by (definition, static-args); shared read-only data; measure compile time on the largest example and keep the pipeline allocation-light; incremental recompile is a fast-follow if needed.
- [`poly` voice-done detection is heuristic when no `done` port exists] → silence-tracking threshold + release timeout are explicit, documented constants; authors of sustained instruments must expose `done`; diagnostics flag wrapped definitions with no gate/done path.
- [Implicit control→audio promotion hides cost] → promotion inserts a visible node in diagnostics/discovery output so the cost is inspectable; per-connection opt-out is possible later.
- [Big-bang YAML break with many examples/specs touching the old shape] → migration is mechanical (params→defaults, `_l`/`_r`→2ch ports, `audio_output`→root ports); each example re-renders and is compared against reference output where reference WAVs exist.
- [Compensation delays cost memory proportional to latency × channel count at every unbalanced convergence] → compensation buffers are preallocated at preparation like all other state; the compiler reports inserted compensation in diagnostics so authors can see and restructure expensive topologies; worst offenders (spectral, convolution) already impose this cost implicitly today as misalignment instead of memory.
- [Undeclared latency in a primitive silently misaligns output, defeating the balancer] → declared latency becomes part of each primitive's registry declaration next to its ports, and behaviour tests assert impulse alignment through dry/wet topologies for every latency-bearing builtin.
- [Static params without an expression language may prove too weak (e.g. "channels = parent channels − 1")] → accepted; name-passing covers the known cases (echo, mixer, poly); revisit with evidence before adding expressions.
- [FFI/host churn across JUCE, CLI, offline render] → bus API is the single new boundary; stereo hosts bind one 2ch `main` bus, so host-side diffs are small.

## Migration Plan

Stages match D9 and land as ordinary commits to main (trunk-based). Rollback = revert the stage's commits; stages 1–2 are additive and independently revertable, stage 3+ are the cut. Verification per stage: full `cargo test`, example render comparisons, and the JUCE demo binary producing sound through the `main` bus.

## Resolved Questions

- `voice.gate` is an **event port** (note-on/off edges). Matches `adsr`'s existing event consumption; a `gate_to_control` primitive can be added later if a level-style consumer needs it. (Decided 2026-07-08.)
- `poly` has **no built-in velocity scaling** — per-voice level shaping is composed inside the voice definition (e.g. a `gain` fed by the velocity intrinsic). (Decided 2026-07-08.)
- Module library packaging (`module_package.rs`, $LIB/$USER_LIB) migrates in a **follow-up change** after the kernel lands; this change keeps the package manifest untouched except where compilation requires it. (Decided 2026-07-08.)
- CLI/offline render settings are **flags only**: defaults for sample rate (48000) and block size (128), explicit `--duration-frames` required for offline render. Per-example settings live in the Rust test harness; no sidecar render-config schema (can be added later with zero migration cost if LLM authoring workflows need shippable render specs). (Decided 2026-07-08.)
