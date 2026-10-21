## Context

The existing `dandrum_clap_adapter` is a JUCE-free, dynamically loaded CLAP binary with proven audio, MIDI, parameter, state and lifecycle behavior for two embedded fixture documents. It has no GUI and prepares its runtime directly from fixture text. The completed `dandrum_instrument_session` crate owns accepted sampler YAML, preparation, bounded native history, disposable attachments and atomic activation, while the production Trigger bundle currently obtains native functions and events through a JUCE-specific JavaScript object.

The selected proof platform is Linux x86_64 with X11 parenting and WebKitGTK 2.50.4. Clack already exposes the CLAP GUI extension at the pinned repository revision, and the investigation selected Wry 0.57.0 as the first child-WebView backend. The render callback must remain free of locks, allocation, file I/O, serialization, browser access and resource reclamation.

## Goals / Non-Goals

**Goals:**

- Add one real Trigger sampler identity to the JUCE-free CLAP binary, using the persistent session and existing Rust engine.
- Run the production Trigger bundle as an actual X11 child of a host-owned window and event loop.
- Keep the editor disposable while accepted session state, admitted work, native history and audio survive.
- Translate one real Trigger parameter gesture to CLAP automation and return host parameter feedback to React.
- Exercise a native mapping edit and undo through the same session implementation already used by the comparison host.
- Prove signed sample rendering, state restore, lifecycle failures and independent instances from a copied artifact/resource tree outside the checkout.
- Leave a format-neutral host core suitable for the following VST3 and standalone projection.

**Non-Goals:**

- VST3 wrapping, standalone audio/MIDI device ownership, named-DAW acceptance or installed release packaging.
- Wayland, Windows or macOS editor support in this increment.
- Full migration of every Trigger editing command, analysis service or telemetry stream.
- Retaining unreleased JUCE product/state/parameter identities.
- Changing DSP algorithms or using a WebView from the render callback.

## Decisions

### Separate the format-neutral Trigger host core from CLAP and Wry adapters

Add a focused Rust host crate that owns the instrument session, versioned editor protocol, public parameter model and runtime handoff. Its core modules contain no Clack, Wry, GTK, X11 or JUCE types. CLAP and Wry modules adapt their concrete lifecycle and handles at the edge. This gives the next VST3/standalone change one tested owner instead of copying CLAP policy.

Extending the fixture-only adapter directly would be initially smaller, but it would entangle session, GUI and CLAP types and force the format projections to repeat that work. Adopting Nice-plug now would discard the already proven Clack binary before the selected route has failed a gate.

### Keep session ownership off audio and transfer prepared runtimes through bounded slots

The main-thread host owns `InstrumentSession`. Activation constructs the next realtime processor and all scratch buffers off audio, then offers the owned runtime through a capacity-one nonblocking handoff. At a block boundary the audio processor swaps the pending runtime and returns the retired runtime through a preallocated reclamation slot; only the main thread destroys it. One in-flight replacement keeps capacity provable and maps naturally to the session's one admitted structural job.

Atomics carry scalar parameter values. Bounded single-producer/single-consumer queues carry parameter events, runtime replacements and acknowledgements. No mutex or reference-count destruction is permitted in `process`.

### Use a versioned local editor protocol and transport-neutral JavaScript backend

The Trigger native adapter selects a `window.__DANDRUM__` backend contract with an explicit protocol version. The existing JUCE bridge supplies the same contract for comparison builds. The Wry adapter serves the production build and packaged fonts through an application-local scheme and exchanges bounded request/reply/event JSON with monotonically increasing request IDs. Unknown methods, malformed payloads, oversized messages and stale attachment IDs receive explicit errors.

The initial JUCE-free method set is deliberately the subset required for session snapshot, public parameter gestures/feedback, note ownership, key-range mapping, job outcome recovery and native undo. Existing unsupported operations continue to report capability limits; the frontend never simulates native acceptance.

### Declare optional editor services without fabricating native success

The bootstrap descriptor may include `capabilities` with strictly boolean `metering`, `sourceWaveforms`, `windowTitle` and `editorResize` fields. An omitted profile or field preserves the existing full-contract behavior for every backend identity, including existing version-1 `juce-free` descriptors. The legacy JUCE selector and explicitly injected comparison backend retain their behavior. Present invalid metadata fails visibly before invoking native methods; backend names never imply service availability.

The new Wry bootstrap explicitly declares those four services unavailable. This is local bootstrap metadata, not another protocol method. Session snapshot, parameter gestures/feedback, note ownership/heartbeat, supported mapping, job recovery and native undo remain real required services. No successful meter subscriptions, measured values, waveform jobs or resize acceptance are invented.

The frontend gates absent services during initialization, generation refresh, mapping/undo, rendering and disposal. Heartbeat and note cleanup remain independent of metering. It shows unavailable meters, unknown active voice count, an accessible unavailable-waveform state and host-controlled size; it supplies no generated preview peaks or fake silence measurements. Direct requests for absent waveform service return an explicit capability error without native mutation. Supported-service failures retain their existing errors and retry behavior.

### Verify shared owners before their format facade

Task 3.2 first verifies the concrete Wry library boundary: packaged assets, local loading, X11 child ownership, presentation and destruction. Its actual CLAP main-thread, API and call-order acceptance depends on the GUI facade in task 4.2. Keep 3.2 open until that proof exists. Task 3.3 may develop the shared snapshot and transport-neutral frontend independently after those library boundaries are verified; this does not claim a working CLAP editor or skip final integration. Use separate existing worktrees and focused commits, then verify each combined boundary.

The first snapshot projection reads the accepted session, prepared sources/maps, applied public parameter values and copied-resource provenance off audio. It preserves decimal session/attachment/revision/request identities, distinguishes structural request cursors from transport IDs, and does not create a new attachment on each read. Default source-free Rust tests remain independent of browser tooling; production parser and real Wry integration receive separate boundary checks.

### Initialize the local native transport before the application entry point

The Wry edge accepts a trusted host initialization script when constructing its child, installing it before loading the fixed packaged entry document. The default preview constructor keeps an empty initializer. A UI-thread-only evaluation method delivers trusted host reply/event scripts to that same browser; successful queueing is not evidence of JavaScript consumption. A real X11/WebKit test observes bootstrap data from the first application script and an IPC acknowledgement after native delivery, then verifies callback release on destruction. Wire serialization, attachment admission and bounded request/reply policy stay in the protocol and browser broker rather than this generic execution edge. This library proof does not complete the CLAP facade or task 3.3.

### Keep editor note intent bounded and arbitrate pitch ownership on audio

Retain the existing wait-free bounded SPSC event handoff and its explicit QueueFull result for ordinary queued note/parameter events. Editor keyboard notes additionally use a fixed 128-pitch native intent table, constructed off audio. One main-thread producer records the held attachment and last publisher; the controller will retain the document generation recorded for each press. Each atomic word publishes a checked onset sequence, retained onset velocity and desired velocity coherently. A conflicting editor owner, invalid note/velocity or exhausted sequence returns an explicit runtime error. Native attachment authenticity and document-generation validation belong to the controller, rather than this trusted-owner runtime API. Repeated same-owner presses may coalesce before audio; accepting intent does not promise voice allocation. Ordinary release preserves an unheard drum tap. Detach clears both desired and retained onset for its last-published rows, including an already released tap, without needing queue capacity or affecting a newer attachment.

The callback performs only finite scans and atomic loads; it never waits for a producer, locks, allocates, destroys resources or calls a browser. Audio-owned pitch state distinguishes accepted editor gates from host-held pitches. Host note-on takes over a same-pitch editor gate before its own onset; editor release/detach never releases that host gate. Host-held or host-touched pitches suppress editor onsets, and a still-desired editor intent may resume on a later untouched block after host release. A new onset or reconnect cannot precede successful release of the previous editor gate. Failed or budget-deferred releases retain their obligation. An unheard cancelled tap cannot start later. This is pitch arbitration over the existing engine, not independent source-tagged simultaneous voices.

Reserve graph-event capacity for all validated timed host events and their possible editor takeover releases. Service retained cleanup before draining ordinary queued events or starting editor notes; defer bounded queued work when editor cleanup consumes its graph-event budget. Each accepted graph note operation uses the checked engine enqueue result. Shared takeover/cleanup release credit is counted once, with a bounded fair pitch scan. Normal callbacks without editor work retain the established queue behavior. Prove saturation using the actual graph capacity and a representative gated sample child with literal stereo prefix and sustained signed tails; a naturally ending one-shot is not a release oracle. One-shot release must keep its natural sample tail.

New editor onsets require the current document generation; a matching held release uses its recorded press generation. The existing wire carries a document generation, not a unique physical press nonce; the controller must not claim stronger same-generation release discrimination. An ordinary mapping swap may resume a still-desired physical pitch on the new graph, while an already released/cancelled onset is not replayed. Panic reset discards pre-reset onset/voice activity and suppresses its resumption until a new press. Successful external state restore clears prior editor intent along with existing queued activity; failed restore preserves it. Explicit detach before callback observation cancels unheard work; a racing already observed onset is cleaned on a subsequent eligible callback, without synchronous waiting. Heartbeat scheduling and semantic editor disposal must later join the concrete format facade; this shared-owner component alone does not complete task 3.3.

### Preserve admitted structural work across editor loss

The format-neutral controller first exposes a bounded take-preparation, submit-completion and service boundary over the existing session's one active request and 128 retained outcomes. Preparation/completion wrappers retain private controller/native-request authority and own their work; they move to a real worker without borrowing an editor. A foreign or duplicate completion is rejected while returning ownership and preserving the genuine pending work. Live sealed protocol context supplies the real native attachment for snapshot, parameter, structural and status operations; mutable public correlation fields are not authority. Native undo admission reuses the session's existing implementation. Known request-status recovery, rather than replaying undo against changed history, retrieves its terminal outcome.

Main/control service performs at most one retirement reclamation and one ready completion per call. After off-audio reclamation, the exclusively borrowed runtime's activation readiness decides whether publication can proceed. A busy slot keeps the prepared job running and owned; never call session completion speculatively because it retains activation failures as terminal outcomes. Preflight JavaScript-safe next generation before publication. A successful structural publication balances all retiring parameter gestures using their already reserved End capacity, advances document generation and preserves scalar values and applied/admitted parameter sequences. Deferred or failed work preserves the current gestures, generation and accepted document/history. Unknown or evicted job status is an explicit error, never invented running/success state.

The take/submit component is followed by an actual bounded worker owned by the persistent instrument host, with startup/shutdown/panic and editor-loss proof. It is not owned by Wry or a format-specific editor. Concrete host servicing must wake both for completed preparation and later runtime capacity after deferral, independently of a destroyed editor's GTK timer. These worker/wake and actual frontend recovery gates remain open until their tests pass; explicit test-driven servicing at the component boundary does not claim automatic product progress or complete task 3.3.

### Start persistent preparation before acknowledging structural work

The controller may explicitly start one private preparation worker with a format-neutral, nonblocking and nonpanicking service-wake handle. Startup must succeed before admitting new native work. In this mode structural admission automatically transfers its owned preparation before returning its acknowledgement; external manual take cannot steal it. Capacity-one synchronous channels carry owned preparations and completions only off audio. Main uses nonblocking operations; the idle worker may wait for work, then requests service after inserting its completion. One main service call drains at most one completion and performs the existing bounded publication step. A zero-sized thread-affinity marker keeps the controller on its constructing control thread; the native facade must construct it on the actual host main thread.

Editor detach never stops this worker. Instrument teardown closes admission, disconnects the request sender, joins after current preparation finishes and destroys results off audio, before session/runtime destruction. Unexpected worker termination is a fatal service-unavailable condition that preserves accepted state/audio; it must not fabricate a native outcome or report a permanently running job as healthy. Internal panic recovery and arbitrary I/O cancellation are not promised. Busy completion remains owned without a wake loop; the facade later routes the real runtime swap report to another main service opportunity. Core scheduler tests are distinct from actual CLAP callback proof.

### Install the browser broker from trusted native bootstrap data

The trusted Wry initializer supplies `window.__DANDRUM_NATIVE__` with `protocolVersion: 1`, decimal `instanceId`, `attachmentId` and optional transport `nextRequestId`. Before selecting an engine, the production entry point synchronously installs the existing bounded browser broker using Wry's `window.ipc.postMessage`. It publishes the normal `window.__DANDRUM__` descriptor plus a `receiveNativeMessage` function for trusted native reply/event delivery. The returned local lifetime handle disposes outstanding browser requests/listeners during entry-point failure or disposal. No bootstrap data means the existing silent preview or comparison backend remains selected. Invalid present data, missing IPC or conflicting existing native ownership fails visibly; it never silently falls back to preview. No new protocol methods or controller authority arise from these globals. Browser/Node fixtures prove this installation contract; actual Wry/controller/CLAP calibration remains a separate joined gate.

### Let CLAP own the parent window, event loop, automation and state boundary

Implement the pinned Clack GUI extension for the Trigger identity only. Creation records a supported X11 configuration; `set_parent` validates and stores the host window; the Wry child is created and destroyed on the CLAP main thread. `show`, `hide`, accepted fixed sizes and failure results follow the host calls without starting a private GTK loop.

React gestures enter a bounded main-thread queue as begin/value/end events. CLAP parameter flush emits the corresponding host events, and audio/stopped host changes return through a bounded feedback queue plus `request_callback` to the main thread, which publishes an editor event. Structural history stays in the instrument session and scalar host automation stays outside that journal.

CLAP state version 2 stores the replacement product identity, accepted session YAML, resource identity and public values with strict size/version validation. Restore prepares a complete candidate before publishing any field. Packaged sample resources resolve relative to the copied plugin resource tree, never the source checkout or an absolute developer path.

### Test the binary through an actual host parent and copied resource tree

Extend the dynamic Clack host fixture with the GUI host extension and a real X11 parent. The test loads the exported release binary, copies the plugin plus Trigger resources to a temporary directory outside the checkout, creates the WebView, drives real DOM controls and inspects host automation/events, state and signed audio. It covers invalid parents, create failure, hide/show, resize rejection, complete destruction/recreation, lost acknowledgement, held-note/gesture cleanup and two instances.

Unit tests remain close to the format-neutral protocol and handoff. Browser tests prove transport selection without needing native audio. The existing JUCE packaged-WebView suite remains a comparison, not proof of this adapter.

## Risks / Trade-offs

- **Wry/GTK main-thread constraints conflict with CLAP host behavior** → store the WebView only in the plugin main-thread object, use the host loop, assert callback thread identity and reject creation when the contract cannot be met.
- **A runtime replacement could be dropped on audio** → use capacity-proved bidirectional slots, prohibit a second activation until retirement is reclaimed and add an allocation/destructor sentinel test.
- **IPC scope grows toward a second backend implementation** → expose only methods required by mapped scenarios and keep unsupported Trigger operations explicit.
- **CLAP host callbacks differ between the synthetic host and a DAW** → exercise the official validator where applicable and retain named-DAW acceptance as a separate required gate.
- **Linux child WebView behavior differs under Wayland** → claim X11 only and fail unsupported APIs explicitly.
- **Resource lookup accidentally depends on the checkout** → run the acceptance binary and resources from a temporary external tree after hiding source asset paths from the process.

## Migration Plan

1. Characterize the current Trigger transport, CLAP fixture behavior and session/runtime ownership with failing tests.
2. Add the format-neutral host crate and bounded runtime/parameter/session protocol with complete focused coverage.
3. Add the transport-neutral Trigger backend while keeping the JUCE comparison tests green.
4. Add Wry resource serving and X11 child lifecycle tests.
5. Add the Trigger CLAP identity, GUI/automation/state extensions and dynamic host acceptance.
6. Run strict build, dependency/link, validator, callback-safety, browser and source-bound evidence gates; independently review before syncing and publishing.

Rollback is removal of the opt-in Trigger identity and its new host/editor crate. The existing two headless CLAP fixtures and JUCE comparison product remain intact until the later format cutover.

## Open Questions

- The exact installed resource-directory convention belongs to the packaging increment; this proof uses a deterministic sibling resource tree that can be copied independently.
- Physical device and named-DAW availability on the selected native Linux machine must be established before the following VST3/standalone acceptance plan is finalized.
