## Context

`CompiledPatch` now holds a resolved execution plan: compiled nodes, index-based input port references, execution order, voice/global node lists, and copied render settings. `graph_processor.rs` still renders from raw `Graph`, rebuilding topological order and string-keyed routing maps on each offline render call.

This change is the next migration step. It proves `CompiledPatch` can drive the existing audio behavior before replacing the primary render path. The existing raw `Graph` functions remain available so parity tests can compare both paths directly.

## Goals / Non-Goals

**Goals:**

- Add an offline render entry point that renders from `CompiledPatch` plus input events and sampler assets.
- Preserve current audio output exactly for covered patch paths.
- Reuse existing per-module processing behavior instead of rewriting DSP logic.
- Replace render-time order/routing derivation in the new path with `CompiledPatch::execution_order` and `CompiledNode::input_port_map`.
- Add focused parity tests comparing old raw `Graph` rendering and new compiled rendering.

**Non-Goals:**

- Do not remove or replace the existing raw `Graph` render path.
- Do not change JUCE/FFI APIs.
- Do not change YAML patch format or patch validation behavior.
- Do not add dependencies or multithreading.
- Do not broadly redesign polyphony or realtime execution.
- Do not optimise beyond avoiding route/order recomputation in the compiled path.

## Decisions

### Decision: Add a separate compiled render entry point

Add `render_offline_compiled` or an equivalent function in `graph_processor.rs` that accepts `&CompiledPatch`, input events, and sampler assets. Keep existing `render_offline` and `render_offline_with_sampler_assets` signatures unchanged.

Alternatives considered:

- Replace existing raw `Graph` rendering immediately: rejected because parity needs both paths available side by side and this is not the final realtime migration.
- Hide compiled rendering behind existing functions: rejected because callers and tests need an explicit migration surface.

### Decision: Share module processing, adapt routing access

The compiled path should reuse existing module processing behavior where practical. The raw path currently reads routing through string-keyed maps. The compiled path should provide equivalent input-gathering helpers over `CompiledNode::input_port_map`, resolving source buffers by source module index and output port index.

Alternatives considered:

- Convert `CompiledPatch` back into raw routing maps: rejected because it would fail the point of proving index-based routing.
- Rewrite the renderer around new runtime data structures: rejected as too broad for a parity step.

### Decision: Scope parity tests to currently supported paths

Parity tests should cover oscillator-to-output, supported MIDI/ADSR/gain voice behavior, and sampler rendering when simple asset setup is available. Polyphonic parity should be included only if it can reuse existing polyphonic render behavior without redesign.

Alternatives considered:

- Require broad polyphonic migration now: rejected because the goal is safe incremental proof, not final realtime architecture.
- Test only trivial oscillator output: rejected because compiled routing must prove event/control/audio behavior where currently supported.

## Risks / Trade-offs

- [Risk] Duplicating raw and compiled input helpers can temporarily increase maintenance cost -> Mitigation: keep the compiled helpers small and covered by parity tests, then remove raw helpers during the later migration.
- [Risk] Exact audio parity may reveal hidden ordering assumptions -> Mitigation: use identical patch documents, assets, render settings, and events in each test so differences isolate compiled execution behavior.
- [Risk] Polyphonic rendering may require broader state/template changes -> Mitigation: treat polyphonic parity as optional unless it fits the existing structure without redesign.
