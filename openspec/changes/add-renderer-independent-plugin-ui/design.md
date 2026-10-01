## Context

See [proposal.md](proposal.md) for scope and pinned evidence. The fetched sampler revision owns prepared sample metadata independently of source inputs and already supports shared/per-pad host controls. Its plugin UI is still embedded HTML with a browser-specific bridge and editor lifetime. No renderer-independent UI change exists at that revision, so this proposal introduces a separate follow-on change rather than altering the archived engine contract.

The revised export broadens the visual vocabulary to key maps, layered synth/sample/patch sources, module chains, sends and outputs. Those displays do not establish new engine capabilities. In particular, a sample map's round-robin alternatives are not simultaneous source layers, and arbitrary patch layering is not implied by drawing overlapping rectangles.

## Goals / Non-Goals

**Goals:** implement a browser-independent C++ presentation boundary, two proven renderer paths, asynchronous analysis and bounded telemetry, with evidence that visualization cannot make audio wait on consumers. Preserve current sampler automation identities and external authoring.

**Non-Goals:** introduce graph authoring inside the DAW editor, add in-plugin modulation assignment, implement missing synth/patch layering or host routing features implicitly, replace the sampling primitives, or guarantee scheduling against arbitrary operating-system contention. Hosts own plugin callback scheduling; this design controls dependency, resource and callback-work bounds.

## Decisions

### Shared typed state precedes either renderer

```mermaid
flowchart BT
    R["Rust DSP / JUCE audio callback"] -->|"bounded capture"| Q["Preallocated SPSC buffers"]
    Q -->|"one consumer per buffer"| A["Meter aggregation / analysis services"]
    P["Prepared metadata with retained lifetime"] --> A
    A --> S["Typed C++ snapshots / capabilities"]
    S --> N["Native JUCE Components and Graphics"]
    S --> B["Web adapter: serialization and assets"]
    B --> W["React / Canvas"]
    N -->|"commands on message thread"| C["Shared host command service"]
    W -->|"bounded asynchronous requests"| B
    B --> C
    C --> H["Stable host parameters / bounded MIDI admission"]
    H --> R
```

The shared presentation library contains C++ value types, command validation, operation state, analysis coordination and format-neutral results. It has no browser, JSON, DOM, `juce::Graphics` or image-type dependency. A JUCE host adapter owns APVTS integration, file selection and message-thread dispatch. Rust retains the target-neutral engine and typed preparation model behind C FFI.

The native renderer consumes typed snapshots; the Web adapter serializes them only after leaving the realtime boundary. Do not route native controls through JavaScript, an embedded browser, or JSON. Duplicating a controller in each renderer would make gesture, generation and lifetime semantics drift; sharing drawing code across C++ and React would overconstrain both.

Processor-owned realtime storage remains valid independently of any editor. Each queue has exactly one producer and one consumer; fan-out to renderer subscribers happens after consumption. A view session owns subscriptions and command state, not the active engine. Analysis uses retained immutable sample ownership or an off-audio-thread copy; neither renderer nor worker retains an unprotected pointer into a replaceable instrument. Allocation release and the final destruction of retained assets occur off the audio callback.

### Separate documents, commands and visual frames

Prepared documents describe stable source/region/zone IDs, actual control groups, parameter descriptors, supported source/layer/routing metadata and actual host bus/channel layout. Documents and replies carry instrument generation; a renderer session ID distinguishes a reopened editor within that generation. Live frames additionally carry stream ID, sequence, audio sample position and validity/gap flags.

Example command names are `beginGesture`, `setParameter`, `endGesture`, `noteOn`, `noteOff`, `reloadInstrument`, `requestWaveform`, and `requestSpectrogram`. Native calls enter the same service directly on the message thread; browser requests add transport identifiers. Validate finite normalized values and capabilities. An accepted parameter write means the host value was admitted, not that an audio block has processed it. Reload/analysis return job IDs and publish durable terminal states queryable after reconnect.

Coalesce continuous pointer values while retaining their final value and gesture boundaries. Parameter notifications update authoritative state without emitting another command. Track admitted command sequence and generation to prevent delayed echoes from undoing a newer local edit. Rejection reconciles the optimistic display to authoritative state. Timeouts/disconnects clear bounded browser promises; do not automatically retry non-idempotent note or job requests.

Audio-originated parameter listeners may only publish atomic values/dirty flags for timer observation. The vendored JUCE `ParameterAttachment` calls `triggerAsyncUpdate()` for off-message-thread notifications; do not assume that is compatible with the strict no-posting callback requirement. Use a covered timer-observed binding for both renderers and perform host begin/update/end gesture calls on the message thread. JUCE [documents that AsyncUpdater can block when posting](https://docs.juce.com/master/classjuce_1_1AsyncUpdater.html).

Maintain release intent outside lossy telemetry: for example, use a preallocated per-note release mailbox and editor-session release generation drained by audio. Serialize note admission on the message thread and preserve ordering between queued note-ons and later disconnect/release intent, so saturation cannot leave an editor-owned gated note stuck. Specify/test the chosen ordering before implementation; it does not change timestamped host MIDI delivery.

### Capability-aware design-system views

Keep one maintained semantic token source and generate CSS and a C++ token header. Reuse the supplied icons and licensed fonts. Add native component specifications for KeyMap, LayerStack and OutputBusses: the new export supplies React sources/types/examples for these but no updated native handoff.

| View | Shared data | Runtime interaction |
| --- | --- | --- |
| KeyMap | Actual key/velocity ranges, source identity, selection/layering mode, root note | Select and audition; prepared bounds are read-only |
| LayerStack | Supported prepared source/chain descriptors and public control bindings | Inspect; edit only parameters with valid bindings |
| OutputBusses | Actual named host bindings, channel counts, feeds when available | Inspect routing and per-channel meters; no fabricated output pairs |
| Knob / Slider | Public ID, normalized/actual value, range, units, scope | One complete host gesture; keyboard and typed entry |
| Waveform / spectrum | Numeric analysis plus frame/rate/settings metadata | Inspect cached data, overlay observed cursor |

Use `editable=false` for prepared KeyMap interactions. Audit LayerStack/OutputBusses callbacks individually; a single exported prop is not proof that every structural action is disabled. Do not expose arbitrary module internals as controls. In-plugin topology changes remain prohibited by `plugin-integration`; a future external authoring product requires a separate change.

The maintained kit's snare split is 63/64 and its controls include per-pad scopes. Use those live descriptors in acceptance fixtures. Never replace them with the export's 95/96 demo split, artificial layering, hardcoded module chains, simulated activity or claimed host modulation sources.

### Telemetry has explicit work and memory bounds

Prepare fixed-capacity capture queues and maximum tap/channel counts before processing. First implement master/output metering; per-layer or per-voice taps require an explicit capability and preparation budget. Measure peak and sum-of-squares per block or during an existing output pass. Aggregate by sample count, including partial/variable blocks. Use double precision for aggregate energy where useful outside the callback and verify silence, signed values and non-finite-input handling.

Queue-full handling rejects the new visual frame and increments a bounded counter. The producer must not discard an old element by changing a consumer-owned index. The consumer may discard stale backlog. A single consumer then distributes a coherent snapshot to either renderer. Keep clipping in an independent lock-free latch/counter with generation-aware acknowledgement; define reset ownership so a reset cannot erase a newer clip occurrence. Verify the atomics used are lock-free on supported build targets.

Start presentation at 30 Hz, with configurable bounded capture/analysis rates. Native painting and browser animation interpolate visually using elapsed time and measurement timestamps. A Web adapter keeps at most one unacknowledged telemetry payload and one replacement snapshot per stream; while blocked it coalesces rather than queues more browser messages. Give commands and job status their own bounded admission/status path. Hidden/disconnected sessions release subscriptions; workers stop unnecessary analysis and audio sees an atomic capture-enabled flag without freeing storage.

Low-priority, bounded worker execution reduces contention; asynchronous execution alone is insufficient. Cap worker count and cache memory, chunk static work, cancel stale jobs and benchmark multiple plugin instances. Never spin indefinitely, issue a system wake/message from audio, or join a worker in a callback. Timers/polling outside audio consume queues. Worker-to-message-thread notification may use ordinary async mechanisms with cancellation-safe receiver ownership.

### Share numeric waveform and spectral results

Use prepared sample content and source-frame coordinates. Waveform reduction produces per-channel min/max buckets for a specified frame range/resolution. Cache by content/revision, region, channel and reduction settings; pathname alone is insufficient. Changing the playhead updates an overlay only. UI values in seconds are derived from the source rate, while live cursor timestamps include host frame timing; support differing source and host rates.

Static spectrogram jobs initially use a Hann window, a documented power-of-two FFT size and hop, explicit magnitude normalization and dBFS floor, and a bounded frequency grid. Final settings and tolerances belong to tested metadata, not unexplained constants in a renderer. Share magnitude arrays; native `Image` and Canvas/WebGL texture caches are downstream renderer details. Reuse compatible existing analysis routines where their measurement contract matches.

Live scope capture can publish signed min/max envelopes. FFT input instead requires contiguous sample windows; do not FFT min/max envelopes or simply retain every Nth sample. Initially capture the subscribed mono/selected channel at full analysis rate; any subsequent downsampling requires anti-alias filtering and a declared effective sample rate. Include sample positions and gap flags. On loss, discard a partial FFT window rather than concatenate nonadjacent samples. Workers limit backlog and publish bounded numeric column batches. Canvas is the initial web drawing backend; adopt WebGL only if measurements warrant it.

### Make editor choice a build boundary

Separate common UI services, native components, and web transport/resources into build targets. Use an editor factory/configuration value independent of instrument data; remove compulsory HTML from the neutral demo description. A native-only build disables browser features and requires neither WebView SDKs/WebKit nor Node. The web configuration compiles React/CSS/fonts/icons ahead of time and embeds local assets via the resource provider. No production CDN scripts, runtime Babel or development server.

Both configurations keep the existing instrument/plugin identity and state schema. Package and test them independently; simultaneous installation of variants with the same plugin identity is not a supported selection mechanism. The initial proof needs build-time renderer selection, not hot-switching an editor during playback.

The supplied TB-303 React panel is an additional web client of the same asset and command path. Its seven prepared public controls bind by ID; the playable keyboard sends editor note events. The source mock's tuning, waveform, tempo, sequencer, transport, programmed steps and moving lights are presentation examples, not prepared capabilities. Replace the demo knob defaults with authoritative values, show fixed saw-wave preparation as read-only if exposed, and leave unsupported editing unavailable. No browser timer may claim to schedule audio steps or report host-driven playback without an observed transport/event capability.

## Risks / Trade-offs

- Host/OS scheduling and shared CPU/GPU resources can still cause contention -> bound resource use and report callback timing/deadline evidence under explicit workloads; avoid absolute scheduling promises.
- Rich mockups imply engine features not yet available -> capability-gate views and retain prepared/external-authoring semantics.
- UI consumers can stall for arbitrary time -> bounded publication, independent clip/release state, reconnect snapshots and generation checks.
- Retained samples and spectral caches consume memory -> bound jobs/cache bytes and reclaim/cancel off audio; test reload and teardown while jobs are stalled.
- Pixel-identical renderers are expensive -> share semantic tokens and behaviour; verify readable, faithful full/compact layouts on each real runtime.
- Source-text checks alone miss indirect callback work -> combine static guards with runtime allocation/forbidden-operation instrumentation and stress measurements, stating platform limits.

## Migration Plan

1. Resolve repository publication/base state, then create one isolated task worktree based on the published sampling revision or its verified integrated successor. Do not reopen archived advanced-sampling tasks.
2. Characterize current host slots, gestures, MIDI admission, reload, sampler metadata and signed audio before extracting shared services. Keep the existing editor available until a replacement slice passes.
3. Implement typed shared state/commands and native-only build separation; prove one knob in native and web against identical host behaviour.
4. Add bounded metering and a real meter in both views, then prepared waveform data and both waveform renderers. Verify these three controls before expanding the sampler layout.
5. Add static spectral analysis, then subscribed live capture/analysis with gap handling and backpressure tests.
6. Compose the revised capability-aware KeyMap/LayerStack/OutputBusses views, preserving controls that the existing sampler exposes. Add native handoff details and offline packaging.
7. Run focused and full integration gates, record stress/visual evidence, map every new scenario to a proving test, then sync/archive through OpenSpec only after implementation is complete. Every implementation task remains unchecked in this proposal.

Rollback selects the previously verified editor build while preserving instrument identity and persisted state. Reverting presentation must not require reverting completed sampling-engine functionality.

## Verification Evidence Required

For deterministic functional tests, replay the same timed parameter/MIDI schedule with telemetry enabled, disabled and consumers stalled; compare to known signed samples as well as full-output equality. Test full queues, stale jobs, variable blocks, generation reuse/reconnect, editor closure during a note and gesture, source-content changes at the same path, and clip acknowledgement races.

For runtime evidence, test native and web separately at 1200x800 and 820x560 logical sizes, common display scaling, 44.1/48/96 kHz and small/normal/oversized blocks. Record baseline and enabled callback distributions, maximum duration, observable missed deadlines, capture losses, worker utilization and cache memory. Include multiple instances, analysis cancellation, resizing and repeated hide/open. Timing acceptance covers only the documented tested environment. Builds and deterministic PCM parity do not establish runtime visual or scheduling quality.
