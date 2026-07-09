## 1. Kernel types and validation (additive, lands beside old model)

- [x] 1.1 Add kernel types in `graph.rs`/new module: `GraphDefinition` (name, static params, ports, nodes, connections), `Node` (id, definition ref, static args, default overrides), extended `Port` (signal type, channel count, control default/min/max/unit), `Connection` — with behaviour tests for construction and equality
- [x] 1.2 Implement static parameter declarations and resolution: typed params (int, enum, resource), name pass-through only, structured diagnostics for missing/unknown/mismatched/expression arguments (tests per `static-parameters` scenarios)
- [x] 1.3 Implement channel-count resolution (literal or static-param reference) and connection validation for signal type + channel count match, with diagnostics reporting both counts
- [x] 1.4 Implement control→audio promotion validation (legal, records promotion step), audio→control rejection, event conversion rejection
- [x] 1.5 Implement unconnected-input default resolution: declared default → instance override → incoming cable precedence; reject overrides of unknown ports

## 2. Recursive flattening and compilation pipeline

- [x] 2.1 Implement recursive flattening of composite nodes to atomic nodes with deterministic namespaced identities and recursion/depth diagnostics (replaces current single-level `module_definitions` expansion path)
- [x] 2.2 Implement expansion caching keyed by (definition identity, resolved static args) with per-instance disjoint runtime state
- [x] 2.3 Add `feedback_delay` primitive; rewrite cycle validation so cycles are legal only through `feedback_delay` (audio and control), remove `feedback_boundaries` attributes and per-module cycle-breaker metadata; scheduler cuts at feedback nodes
- [ ] 2.4 Add per-node latency metadata to the atomic node contract and declare true latencies in the registry: spectral processor (`fft_size - 1`), overlap-add convolution (one partition block); audit remaining builtins and declare zero explicitly
- [x] 2.4b Implement the standalone latency-balancing compile pass over the flattened graph: accumulate declared latency along paths, produce a compensation-delay preallocation plan where unequal paths converge, compute total root latency from the root output sources; reject cycles without `feedback_delay` and feedback cycles containing nonzero-latency nodes with structured diagnostics. Inserted compensations and root latency are reported via the returned `LatencyPlan` (the compilation/preparation surface), not as compiler diagnostics
- [ ] 2.4b-wire Consume `LatencyPlan` in the compilation/preparation path: preallocate the compensation-delay buffers and expose total latency to the host; lands with the 2.6 pipeline rewire (until then 2.4b is a standalone, independently tested pass)
- [ ] 2.4c Behaviour test: impulse through a dry path mixed with a unit-impulse-IR convolution path arrives time-aligned at the mix; spectral dry/wet topology aligns per resolved FFT size
- [ ] 2.5 Insert compiler-generated control→audio promotion steps into the flattened graph, visible in diagnostics/discovery output
- [ ] 2.6 Rewire `preparation.rs`/`compiled_patch.rs`/`graph_processor/render_plan.rs` to consume the flattened kernel graph (statically dispatched flat node array over existing arenas), keeping offline/realtime parity tests green

## 3. Builtins on the kernel model

- [ ] 3.1 Re-declare all builtin ports with channel counts and control defaults/ranges; convert every builtin tunable param to a control input port (delete `ModuleNode::params` consumption per builtin as converted)
- [ ] 3.2 Declare builtin static parameters (channel counts, max delay length, FFT size, resource refs) distinct from ports; make declarations authoritative for `static`/`defaults` validation
- [ ] 3.3 Convert stereo builtins to channel-polymorphic via `channels` static param (echo, reverb, mixer, gain, frequency splitter, etc.), collapsing `_l`/`_r` port pairs to multichannel ports
- [ ] 3.4 Delete `audio_output` builtin; add rejection diagnostic directing to root ports
- [ ] 3.5 Convert assets to resource-typed static parameters on sampler/convolution builtins; remove `asset_bindings` mechanism

## 4. Poly combinator

- [ ] 4.1 Implement `poly` structural node: wraps a definition, static `max_voices` + allocation-policy enum, preallocates flattened voice instances/state/queues at preparation
- [ ] 4.2 Implement note-event routing with oldest-steal policy and per-voice intrinsic ports (note, velocity control; gate event) injected into the wrapped definition scope
- [ ] 4.3 Implement voice completion: `done` output convention plus gate-release + silence-threshold/timeout fallback (documented constants); retired voices contribute no stale output
- [ ] 4.4 Implement per-output summing across active voices into prepared accumulation buffers; allocation-free full-polyphony render test (reuse `realtime_allocation_tests.rs` harness)
- [ ] 4.5 Support nested `poly` expansion; remove `ExecutionScope`, `VoiceToGlobalDirectRouting`, `voice_allocator.rs`, and `voice_allocation` handling once all callers migrate

## 5. Host buses and FFI

- [ ] 5.1 Implement root-port ↔ named-bus binding at preparation: name matching, channel-count validation, unbound-output failure, unbound-input silence
- [ ] 5.2 Extend FFI with root-port enumeration (name, direction, signal type, channels), per-bus buffer binding, and total-latency query for plugin latency reporting; keep invalid-pointer containment tests
- [ ] 5.3 Move render settings (sample rate, block size, duration) out of patch documents to host/render invocation; add rejection diagnostic for `render:` in patches; CLI gains render flags (sample rate default 48000, block size default 128, required `--duration-frames`) with per-example settings encoded in the Rust test harness
- [ ] 5.4 Update JUCE demo and CLI/offline renderer to declare and bind buses (stereo host binds one 2-channel `master` bus); verify demo produces sound

## 6. YAML schema and document shape

- [ ] 6.1 Implement the kernel patch document shape: `metadata`, `static_params`, `ports` (root inputs/outputs), `module_definitions`, `modules` (with `static` and `defaults` mappings), `connections`; reject legacy `render`, `voice_allocation`, instance `parameters`, `${name}` bindings, `asset_bindings`
- [ ] 6.2 Make composite `module_definitions` full graph definitions: static params, public control ports with defaults/range replacing composite `parameters`; patch document loadable as a composite definition (patch/module symmetry test)
- [ ] 6.3 Re-point preset surface to aliases on root ports and resource static params; update preset validation/application (`instrument-presets` scenarios) and preserve determinism tests
- [ ] 6.4 Rewrite `schema/patch.schema.yaml` for the kernel document shape
- [ ] 6.5 Update capability discovery to the unified schema: port metadata with channels/defaults/range for all definition kinds, static-parameter metadata, delete separate parameter metadata

## 7. Migration, examples, and cleanup

- [ ] 7.1 Migrate all `examples/patches/*.yaml` and `examples/presets/*.yaml`: params→`defaults`, `_l`/`_r`→2-channel ports, `audio_output`→root ports, `voice_allocation`→`poly`, render settings→CLI flags in the test harness; compare renders against reference output where reference WAVs exist
- [ ] 7.2 Migrate drum-kit example to per-pad `poly` nodes and multiple named 2-channel root output ports
- [ ] 7.3 Delete remaining legacy code paths (old expansion, params plumbing, preset-surface mapping, stereo output binding) and their tests; confirm no unreferenced code remains
- [ ] 7.4 Update `docs/nomenclature.md` and other docs for kernel vocabulary (graph definition, static parameter, poly, feedback_delay, root ports, buses)
- [ ] 7.5 Full verification: `cargo test`, ctest, coverage against target per repo policy, JUCE demo end-to-end, `openspec validate` clean
