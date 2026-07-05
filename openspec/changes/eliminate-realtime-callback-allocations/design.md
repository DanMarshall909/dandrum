## Context

The realtime graph processor prepares some scratch capacity, but the render path still depends on allocation-shaped abstractions:

- `ModuleOutputs` owns `HashMap<String, Vec<f32>>` audio/control output buffers and dynamic event collections.
- `CompiledInputProvider` gathers each input by allocating a new `Vec<f32>`.
- The polyphonic path creates per-block voice event vectors and per-block output maps.
- `RealtimeGraphProcessor::render_chunk` drains pending events into a newly collected `Vec<BlockEvent>`.

That is fine for a headless prototype, but it is not suitable for a JUCE AU/VST3 audio callback. This change makes realtime rendering structurally allocation-free for blocks no larger than the prepared maximum block size.

## Goals / Non-Goals

**Goals:**

- Make prepared-size realtime rendering allocation-free after runtime preparation.
- Move runtime routing from names/maps to compiled buffer IDs, queue IDs, and edge lists.
- Make module processors write into provided buffers instead of returning owned output maps.
- Make event routing bounded with explicit overflow behaviour.
- Preserve patch semantics, module behaviour, facade APIs, and FFI render behaviour.
- Keep DSP algorithms independent from graph, YAML, FFI, and frontend concerns.
- Migrate in stages: mono/global rendering first, then event modules, then polyphony.

**Non-Goals:**

- Do not change the YAML patch format.
- Do not add new module types.
- Do not redesign sampler asset preparation or script language semantics.
- Do not redesign JUCE host IO or the plugin parameter model.
- Do not require offline-only render paths to be allocation-free unless they share the realtime runtime.

## Decisions

### Compile a render plan, not just graph metadata

`CompiledPatch` will remain the validated patch contract, but realtime preparation will derive a render plan containing execution steps, pre-resolved audio/control input edges, pre-resolved event input edges, buffer IDs, event queue IDs, output bindings, and default control metadata.

```rust
pub struct RenderPlan {
    pub voice_steps: Box<[RenderStep]>,
    pub global_steps: Box<[RenderStep]>,
    pub audio_buffers: AudioBufferPlan,
    pub event_queues: EventQueuePlan,
    pub midi_input: Option<EventQueueId>,
    pub audio_output: Option<AudioOutputBinding>,
}

pub struct RenderStep {
    pub module_index: usize,
    pub module_kind: ModuleKind,
    pub input_buffers: Box<[BufferId]>,
    pub output_buffers: Box<[BufferId]>,
    pub incoming_edges: Box<[CompiledEdge]>,
    pub incoming_event_edges: Box<[CompiledEventEdge]>,
    pub event_inputs: Box<[EventQueueId]>,
    pub event_outputs: Box<[EventQueueId]>,
}

pub struct CompiledEdge {
    pub source: BufferId,
    pub destination: BufferId,
    pub signal_type: SignalType,
    pub gain: f32,
}

pub struct CompiledEventEdge {
    pub source: EventQueueId,
    pub destination: EventQueueId,
}
```

Rationale: the callback should execute a prepared schedule. It should not rediscover source port names, scan input port names, or look up output maps during rendering. Event routes use a separate edge type because event queue routing has different semantics from audio/control buffer routing: it copies bounded event payloads, can overflow, and never applies sample-rate gain or summing.

### Use audio/control arenas addressed by buffer IDs

Realtime audio and sample-accurate control signals will live in prepared arena storage sized by maximum block size, voice allocation, and compiled buffer count.

```rust
pub struct AudioArena {
    buffers: Box<[f32]>,
    frames: usize,
    buffer_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferId(pub usize);
```

The arena will expose short-lived slices during processing. The render path will clear accumulation buffers before summing inputs. Module output buffers should normally be fully overwritten by their processors.

Rationale: buffer IDs allow routing and modules to use index arithmetic rather than `HashMap` and string keys.

### Replace owned `ModuleOutputs` returns with write-into-context processing

Module dispatch will move from this shape:

```rust
fn process_module(...) -> ModuleOutputs
```

to this shape:

```rust
fn process_module(
    step: &RenderStep,
    ctx: &mut ProcessContext<'_>,
    state: &mut PerModuleState,
);
```

`ProcessContext` gives modules read-only input slices, mutable output slices, input events, and bounded event writers. DSP algorithms remain below this layer and do not need graph IDs, YAML names, or FFI details.

Rationale: returning owned output maps encourages allocation and ownership churn. Writing into prepared destinations makes realtime safety explicit.

### Bound event storage and report overflow

Realtime events will use prepared fixed-capacity queues. Event writers will be fallible and will set overflow flags instead of allocating. Event routing between modules will use prepared `CompiledEventEdge` entries that copy from source `EventQueueId`s into destination `EventQueueId`s, rather than looking up event port names in `HashMap<String, Vec<BlockEvent>>` containers during the callback.

```rust
pub struct EventQueue {
    events: Box<[BlockEvent]>,
    len: usize,
    overflowed: bool,
}

pub struct EventWriter<'a> {
    queue: &'a mut EventQueue,
}
```

Overflow behaviour must be explicit. Critical events such as note-off and panic/reset events should be preserved where possible. Lower-priority generated/script diagnostic events may be dropped or coalesced.

Rationale: unbounded script/event output is incompatible with realtime plugin rendering.

### Split polyphonic rendering into allocation-free phases

The current polyphonic block function combines event routing, voice processing, accumulation, global processing, output collection, and voice retirement. This change will split those phases so each can reuse prepared storage:

1. Route incoming events to prepared per-voice event queues.
2. Process active voices using per-voice arena views.
3. Accumulate voice outputs into prepared global accumulation buffers.
4. Process global nodes.
5. Bind or copy the final audio output to the host buffers.
6. Retire finished voices.

Rationale: polyphony is the highest-risk allocation source. Separating phases makes bounded storage visible and testable.

### Prove realtime safety with tests/checks

The implementation will add tests that render repeated prepared-size blocks and assert that capacities do not grow. Where feasible, add a test allocator or allocation counter around `RealtimeGraphProcessor::render` so regressions fail immediately.

Rationale: realtime allocation discipline is easy to accidentally break. Capacity tests are useful but weaker than allocation-count tests, so both should be used where practical.

## Migration Plan

1. Add characterization tests for existing realtime capacity and allocation behaviour.
2. Introduce render-plan, buffer ID, and arena types behind the current public API.
3. Make a simple mono compiled patch render through the arena path without allocation.
4. Migrate non-event module processors to write into `ProcessContext`.
5. Migrate event modules and bounded event queues.
6. Migrate sampler, script, and other variable-output modules with explicit quotas.
7. Split and migrate polyphonic rendering.
8. Remove obsolete `ModuleOutputs`/`HashMap` render-path storage.
9. Run Rust tests, CMake/CTest, and OpenSpec validation.

## Risks / Trade-offs

- Broad signature churn can hide audio regressions. Mitigation: keep parity tests passing and migrate module families in small steps.
- Event overflow semantics can change musical behaviour under overload. Mitigation: document priority and overflow rules and test them explicitly.
- The arena can introduce aliasing/borrow complexity. Mitigation: use buffer IDs internally and expose short-lived views to module processors.
- Clearing too much arena memory may waste CPU. Mitigation: clear only accumulation destinations and required scratch buffers.
- Clearing too little risks stale samples. Mitigation: add tests for disconnected/missing inputs and inactive voices.

## Open Questions

- Should event overflow be `DropNewest`, priority-based, or coalescing per event type?
- Should the first implementation use a custom allocation counter in tests or only capacity-regression tests?
- Should offline rendering eventually share the realtime arena path for parity, or keep a simpler allocation-friendly path?
