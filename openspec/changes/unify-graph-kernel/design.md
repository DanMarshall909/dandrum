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

`GraphDefinition { name, static_params, ports, nodes, connections }`, `Node { id, definition_ref, static_args, port_defaults_overrides }`, `Port { name, direction, signal_type, rate, channels, multiplicity, default }`, `Connection { source: PortRef, destination: PortRef }`. Multiplicity is part of the kernel contract rather than recovered from the legacy registry: ordinary inputs accept one source and explicit summing inputs accept many. A primitive is a `GraphDefinition` whose body is a Rust implementation; a composite's body is nodes + connections. The patch file *is* the root `GraphDefinition`. Alternative considered: keep patch as a wrapper document holding a graph — rejected because the asymmetry is exactly what forces duplicate validation/discovery paths today and blocks module reuse of whole patches.

### D2: Signal type × rate, not fused

`signal_type ∈ {audio, control, event}` stays the user-facing vocabulary (matches `docs/nomenclature.md`), but internally audio = per-sample float stream, control = one value held for a processing block, event = timestamped queue. Compatibility rules: same-type connects; `control → audio` is legal via implicit promotion (compiler inserts an inspectable sample-and-hold node that fills the audio block with the control value); `audio → control` requires an explicit downsampler/follower node; event never implicitly converts. Interpolation is deferred until control values carry timestamps or previous/next values with a defined cross-block contract. Rationale: VCV's all-audio-rate unification wastes CPU; Csound/SC rate systems are where the savings live. Alternative: require explicit converters everywhere — rejected as boilerplate that hurts LLM and human authoring for the overwhelmingly common knob→audio-param case.

### D3: Parameters are ports with defaults

`ModuleNode::params` (stringly-typed map) is deleted. Every tunable becomes a control input port with `default`, optional `min`/`max`/`unit` metadata. Node instances may override port defaults (`port_defaults_overrides`), which is what YAML `params:` blocks compile to during migration. `preset_surface` becomes named aliases onto root-graph ports; preset application writes port defaults. Static shape-affecting values (e.g. delay max length, channel count) are *not* ports — they are static params (D5). Rationale: one concept, one validation path, and modulating anything becomes "connect a cable" with no param/port distinction to discover.

### D4: `poly` combinator replaces voice scope

A structural primitive `poly { definition_ref, static max_voices, allocation policy }`:

- Preallocates `max_voices` flattened instances of the wrapped definition (satisfying the no-allocation callback contract).
- Routes incoming note events to instances: policies `oldest-steal` and `reject-new` preserve both existing stealing and no-steal behavior.
- Injects per-voice intrinsic input ports into the wrapped definition's scope: `voice.note` (control), `voice.velocity` (control), `voice.gate` (event or control — see Open Questions).
- Detects voice completion via a convention: the wrapped definition may expose a `done` event/control output; absent that, gate-off plus output-silence tracking frees the voice.
- Sums instance audio outputs per output port (the "voice mixer" from the original proposal is the node's mix stage, not a separate concept).

Each compiled `poly` owns an explicit runtime region: allocator, flattened voice-instance state ranges, event queues, output accumulators, and child schedule. Sibling and nested regions therefore have independent allocation domains. `ExecutionScope`, `VoiceToGlobalDirectRouting`, and `voice_allocator.rs` as a graph-wide concept are removed only after every caller has migrated. Rationale: Faust's core weakness is polyphony outside the model; Max's `poly~` proves subgraph instancing as a node; nesting (per-pad voices, unison) falls out for free. Alternative: lower `poly` back into the existing graph-wide voice scope — rejected because one allocator cannot represent sibling or nested voice pools.

### D5: Static parameters and expansion caching

Graph definitions declare `static_params` (typed: int, enum, string, resource-ref) usable in port channel counts, `poly` voice counts, replication counts, and primitive configuration. String static parameters preserve construction-time inline text such as Rhai script source; they remain compile-time values and are not ports. Nodes supply `static_args`. The compiler resolves all static args before flattening and caches expanded definitions keyed by `(definition_id, static_args)`. Channel-count polymorphism rides on this: a port's `channels` may reference a static param, so one `echo` definition serves mono/stereo/N-channel (collapsing today's `_l`/`_r` duplication). No general expression language — static args flow through by name and simple arithmetic is deferred. Rationale: this is the minimum genericity that makes "no stereo assumptions" real; a full metaprogramming layer (Faust's algebra) is explicitly deferred.

### D6: Feedback-delay primitive

Cycles are illegal except through the `feedback_delay` primitive, which declares its delay (≥1 block initially; per-sample feedback deferred). The scheduler stays a plain topological sort with `feedback_delay` nodes as cut points; `feedback_boundaries` node attributes are removed. Rationale: matches Faust `~`/gen~ `history`; makes the cycle rule teachable in one sentence and the scheduler trivially correct.

### D7: Host buses

The host boundary is: root graph input/output ports ↔ named buses declared by the host (`main: 2ch`, `sidechain: 1ch`, `cue: 2ch`, …). Binding is by name with channel-count validation at prepare time. A root input with no same-named host bus reads silence; an extra host input bus is ignored. The FFI (`ffi.rs`) enumerates prepared root ports and accepts validated planar channel-buffer views on each render call rather than retaining caller-owned pointers. JUCE/CLI/offline render map buses to devices/plugin ports/files. Bus vocabulary aligns with CLAP's audio-ports/note-ports model so plugin mapping is near 1:1. Preparation settings contain sample rate and maximum block size; offline invocation separately supplies duration and output destinations. Alternative: keep render settings in patches for convenience — rejected; a patch that hardcodes its sample rate can't be a reusable module.

### D8: Compilation pipeline

`parse → resolve definitions (library + inline) → resolve static args/resources → recursively flatten composites to atomic nodes and explicit poly regions → typecheck ports/rates/channels/multiplicity → cycle-check (feedback_delay cuts) → latency balance → topological schedule → channel-span buffer-reuse coloring into arenas → CompiledPatch`. Each resolved logical port owns a contiguous physical buffer span `(first_buffer, channel_count)` and each logical connection expands to channel-wise compiled edges. Latency balancing accumulates per-node declared latency along paths, inserts channel-matched compensation delays where paths of unequal latency converge, reports total root latency to the host, and rejects any feedback cycle containing a nonzero-latency node (compensation inside a loop is impossible). Composite and `poly` latency is the accumulated latency of the flattened contents; voices are identical instances, so poly latency is uniform. Flattening reuses the expansion cache; the whole pipeline stays linear-ish in flattened node count so live edits recompile fast. Execution remains statically dispatched over preallocated arenas. Per-voice instances share read-only resources (wavetables, samples, coefficients) and get disjoint state slices.

Resolved static construction data and effective control defaults remain distinct in the compiled representation. Atomic state constructors consume typed static arguments and resolved resource handles; render planning consumes typed control defaults and runtime control slots. The transitional legacy adapter may populate those structures from `ModuleNode::params`, but kernel execution never asks individual builtins to parse the undifferentiated legacy map. Numeric static arguments never become mutable runtime control slots.

Latency compensation applies only to audio connections. Control values are block-rate state and events retain explicit timestamps, so neither is routed through an audio compensation primitive. Every root audio output is delayed to the maximum accumulated root latency before host binding, giving the host one truthful total-latency value and keeping separately named output buses sample-aligned.

### D9: Migration is staged by executable vertical slices

Trunk-based with one explicit transitional adapter: (1) kernel types + validation land alongside old types; (2) typed compiled construction/default data and channel-span routing replace the mono assumptions under the kernel path; (3) root buses and explicit poly regions become executable before any legacy output or voice mechanism is removed; (4) resources, presets, discovery, schema, FFI, hosts, packages, and examples migrate in capability cohorts; (5) `audio_output`, `ExecutionScope`, `params`, legacy preset plumbing, and old expansion are deleted only after production callers reach zero. Every intermediate slice keeps tests and the demo path green. Compatibility exists only inside the in-repo adapter during migration and is not a supported public patch format.

### D10: Resources, presets, and configurable primitive definitions

Resource static arguments resolve against an explicit preparation context carrying the document/package root and host sample rate. Resolution produces typed immutable resource handles, validates resource kind, deduplicates shared data, and preserves package-relative paths. Presets apply to a resolved root instance before flattening: value aliases replace root control defaults and asset aliases replace resource static arguments, with the precedence declared in the preset specifications.

Primitives whose interface is author-defined, notably scripts, are represented as named graph definitions rather than nodes with ad-hoc ports. A script-backed definition declares its public ports plus script language/source static arguments, then instances use the ordinary `Node` shape. This preserves the rule that every node gets its interface from its referenced definition. Port multiplicity is declared in that same interface.

Generic channel-independent builtins (gain, mixers, filters, delays, promotion, and similar processors) support arbitrary resolved channel counts by allocating state per channel where needed. Intrinsically stereo algorithms may constrain `channels` to supported values: echo and reverb support mono and stereo in this change and reject larger counts with a structured static-argument diagnostic. Arbitrary host bus widths remain valid because graphs can compose or split processors explicitly; not every primitive must accept every width.

### D11: Allocate model capability by reasoning risk

Implementation tasks carry a model-tier tag that describes the minimum reasoning capability expected for primary ownership. Tiers describe capabilities rather than vendor or model names so the plan remains useful as models change:

- **`[frontier]`** — use a frontier reasoning/coding model for architecture, ownership boundaries, realtime state machines, unsafe FFI, cross-cutting migrations, and destructive cleanup. These tasks require reconciling multiple invariants and are expensive to repair after a locally plausible mistake.
- **`[standard]`** — use a capable smaller coding model for bounded implementation whose architecture and acceptance behavior are already explicit. The model must still follow TDD, inspect adjacent code, and run the focused and full relevant test suites.
- **`[mechanical]`** — use a fast smaller model for repetitive schema/example/document transforms or command execution after the transformation and expected output are fully specified. Work must be performed in small batches with deterministic validation.

Model tier is a floor, not a prohibition on using a stronger model. A task escalates to `[frontier]` immediately if implementation exposes an unspecified ownership boundary, changes the compiled representation or public ABI, requires unsafe-code reasoning, affects realtime allocation/lifetime guarantees, produces conflicting acceptance criteria, or cannot preserve behavior with the planned mechanical transform. Smaller models SHALL stop and report the ambiguity rather than inventing architecture or compatibility behavior.

Frontier work should leave behind typed boundaries, tests, diagnostics, and migration helpers that deliberately make downstream work suitable for standard or mechanical models. Mechanical migrations must not begin until their prerequisite capability tasks pass and one representative fixture has been migrated and reviewed. Task 7.10 is mechanical only while all checks pass; diagnosis and remediation of non-obvious failures escalate according to the subsystem involved.

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

Stages match D9 and land as ordinary commits to main (trunk-based). Rollback = revert the stage's commits. Verification per executable slice includes focused behavior tests and the full Rust suite; host-boundary slices also run CTest and the JUCE demo through the named `master` bus. Destructive deletion is delayed until migrated callers and forbidden-symbol checks prove the old path is unused.

### Minimal-touch execution

The migration inventory shows cost concentrated in a few production files plus a large but **mechanically uniform** test/example surface. Rewrite as little as possible; bridge and codemod the rest.

- **Preserve DSP implementations, replace insufficient compiled structure.** The 2.6 bridge remains useful for mono regression coverage, but multichannel ports, named buses, and structural polyphony require channel spans, explicit root bindings, and poly regions in the compiled representation. Keep processing algorithms and static dispatch; do not preserve mono/stereo graph structure that cannot express the requirements.
- **Build the kernel document front end before switching preparation.** Task 3.2 supplies authoritative construction-time declarations; the graph-producing portions of 6.1/6.2 then parse patches and composites directly into `GraphDefinition`. Task 2.6 consumes that model and MUST NOT adapt through `Graph::from_patch_declarations`, because doing so would retain the single-level expansion path the kernel replaces.
- **Keep the L/R DSP internals.** `echo.rs`/`reverb.rs` `process(in_l, in_r)` remain intact. Dispatch adapts mono or stereo channel spans and rejects unsupported wider instances during static resolution; generic processors handle arbitrary channel spans.
- **Use a transitional output sink only behind the bridge.** Until channel-span root buses land, the bridge may synthesize the legacy stereo `audio_output` runtime node from mono root ports named `left` and `right`. It is not authorable kernel syntax. Remove it only after Rust, FFI, JUCE, CLI, offline rendering, packages, and examples bind first-class root buses.
- **Migrate examples in capability cohorts.** Simple mono patches can be transformed mechanically. Script, resource, polyphonic, multibus, and package-backed examples migrate only after their corresponding kernel capability exists and retain behavior-specific render comparisons. The drum kit is its own cohort rather than part of the generic bulk transform.

### Refactoring tooling

Pick the tool by surface — the biggest surfaces are *strings*, not symbols.

RustRover (IDE):
- **Structural Search & Replace (SSR)** for repetitive Rust patterns, e.g. `ModuleNode::new(ModuleId::new($id$), "audio_output")` and `audio(AUDIO_IN_L)`/`audio(AUDIO_IN_R)` dispatch pairs. Save as reusable templates.
- **Introduce Constant then Rename** in that order: SSR-replace bare `"audio_output"` literals with the existing `AUDIO_OUTPUT` const, then Rename the single symbol with full Find-Usages safety (also serves the no-hardcoded-strings rule).
- **Change Signature** for the L/R dispatch adapters; **Rename** for the `AUDIO_IN_L`/`_R` port constants in `graph/builtin_ports.rs`.

Command-line codemod tools:
- **`rust-analyzer ssr`** — the CLI equivalent of SSR: `rust-analyzer ssr 'ModuleNode::new(ModuleId::new($id), "audio_output") ==>> …'`. Scriptable/CI-able, uses the same engine as the IDE.
- **`ast-grep` (`sg`)** — AST-structural search/replace with rules for **both Rust and YAML**; single tool for cross-language codemods, good for the uniform patch transforms.
- **`comby`** — language-aware, multiline structural rewrite; strong for the repetitive YAML blocks SSR can't reach.
- **`yq`** (mikefarah) — structure-aware YAML edits (delete `render:`, rename `parameters`→`defaults`) across the 31 patches — correct choice over regex for structured keys.
- **`sd`** — simple textual replace for the inline-YAML-in-Rust-string-literals (`type: audio_output`) that SSR/`sg` won't match because they're inside string literals.
- **`cargo clippy --fix` / `cargo fix`** — mop-up pass after bulk edits.

Discipline: run every bulk edit in small, committed batches gated by `cargo test`; with `#![deny(dead_code)]` and 500+ tests a bad rewrite surfaces immediately, but only bisectably if batched.

## Resolved Questions

- `voice.gate` is an **event port** (note-on/off edges). Matches `adsr`'s existing event consumption; a `gate_to_control` primitive can be added later if a level-style consumer needs it. (Decided 2026-07-08.)
- `poly` has **no built-in velocity scaling** — per-voice level shaping is composed inside the voice definition (e.g. a `gain` fed by the velocity intrinsic). (Decided 2026-07-08.)
- Module library packaging (`module_package.rs`, $LIB/$USER_LIB) migrates before legacy expansion cleanup in this change because package entries and package-relative resources otherwise keep the old graph and parameter model alive. Package distribution/versioning changes remain out of scope. (Revised 2026-07-13.)
- CLI/offline render settings are **flags only**: defaults for sample rate (48000) and block size (128), explicit `--duration-frames` required for offline render. Per-example settings live in the Rust test harness; no sidecar render-config schema (can be added later with zero migration cost if LLM authoring workflows need shippable render specs). (Decided 2026-07-08.)
- Latency compensation is **audio-only**; control and event edges are never passed through the audio compensation primitive. All root audio outputs are aligned to the maximum root latency before host binding. (Decided 2026-07-13.)
- Preparation switches to the kernel only after 3.2 and the graph-producing portions of 6.1/6.2. No temporary `PatchDocument → legacy Graph → kernel` adapter is permitted. (Decided 2026-07-13.)
- Inline Rhai source remains supported as a typed string static parameter, analogous to a string constructor argument; it is resolved before flattening and never becomes a connectable runtime port. (Decided 2026-07-13.)
- Control-to-audio promotion is sample-and-hold for the current block-rate control model; linear interpolation is deferred until control timestamps or boundary values are specified. (Revised 2026-07-13.)
- The poly allocation-policy enum includes `oldest-steal` and `reject-new`, preserving existing no-steal examples. (Revised 2026-07-13.)
- FFI audio buses use planar channel buffers supplied per render call; the engine does not retain host-owned audio pointers. (Decided 2026-07-13.)
