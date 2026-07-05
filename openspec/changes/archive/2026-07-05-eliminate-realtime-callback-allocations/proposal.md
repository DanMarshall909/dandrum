## Why

The Rust engine currently renders modular synth patches inside a path that is intended to be called from a JUCE audio callback. That path still uses allocation-shaped runtime abstractions: `ModuleOutputs` owns `HashMap<String, Vec<f32>>` buffers and event collections, input gathering returns fresh `Vec<f32>` buffers, the polyphonic path creates per-voice `Vec`/`HashMap` structures, and pending events are drained into a newly collected `Vec<BlockEvent>` per render chunk.

This is acceptable for a headless prototype, but it is not acceptable for a DAW plugin audio callback. The AU/VST3 host-facing render path must have a bounded realtime contract: after preparation, processing a block no larger than the prepared maximum block size must not allocate, grow collections, perform name-based lookup on the hot path, or depend on unbounded event emission.

Existing architecture specs already require prepared realtime resources, but the requirement is broad. This change makes the contract concrete and refactors the runtime around a compiled render plan, preallocated audio/control arenas, bounded event queues, and module processing that writes into provided buffers instead of returning owned `ModuleOutputs`.

## What Changes

- Introduce an explicit render plan derived from `CompiledPatch` that pre-resolves module execution steps, input edges, output buffers, event queues, port metadata, and default control values.
- Replace callback-time `HashMap<String, Vec<f32>>` module outputs with prepared audio/control buffer arenas addressed by compiled buffer IDs.
- Replace input-gathering functions that allocate `Vec<f32>` with routing that clears and sums into prepared input buffers.
- Replace module processors that return owned `ModuleOutputs` with processors that write into a realtime `ProcessContext`.
- Replace callback-time event `Vec` growth with bounded prepared event queues and explicit overflow behaviour.
- Split the polyphonic render path into voice event routing, active voice processing, voice accumulation, global processing, and voice retirement so each part can use prepared storage.
- Add allocation-safety tests/checks that fail if prepared-size realtime rendering allocates or grows scratch capacity.

## Impact

The external patch format and observable audio behaviour should remain unchanged. This is an internal runtime architecture change. It will touch the graph processor, compiled patch metadata, dispatch layer, process function signatures, realtime processor scratch state, and event routing.

The work is intentionally staged so a simple mono compiled patch can become allocation-free first, before migrating the full module set, event modules, and polyphonic rendering.

## Non-Goals

- Do not change the YAML patch format.
- Do not add new DSP modules.
- Do not redesign the JUCE wrapper, plugin parameter model, or host/device IO.
- Do not change voice stealing semantics except where needed to route events through bounded prepared queues.
- Do not redesign the scripting language; only constrain realtime event emission and callback-time allocation.
- Do not optimize DSP algorithms beyond removing allocation and ownership churn from the render path.

## Success Criteria

- A prepared realtime runtime can process repeated blocks up to the prepared maximum block size without allocation or scratch capacity growth.
- Runtime routing no longer requires port-name lookup, module-output `HashMap` lookup, or callback-time construction of audio/control buffers.
- Event delivery is bounded, deterministic, and reports overflow without allocating in the callback.
- Offline/realtime parity tests continue to pass for representative mono, event-driven, sampler, voice-to-global, and polyphonic patches.
- Existing FFI and facade render calls keep their public contract while using the new allocation-free runtime internally.
