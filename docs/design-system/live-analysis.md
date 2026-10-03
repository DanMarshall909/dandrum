# Live analysis implementation evidence

Task 6.3 remains in progress. Its capture foundation is
`InstrumentUiLiveCapture`: a processor-owned queue for one master-output tap,
at most two selected channels, 64 chunks and at most 256 frames per chunk.
The queue owns copied PCM; no frame retains a pointer to a host buffer, engine
or editor. The scheduled service described below now connects this queue to
an analysis worker. The processor capture connection and acknowledged handoff are verified below;
shared renderer subscriptions and actual live views remain pending.

## Capture contract

One audio producer writes the queue and one analysis worker consumes it.
One off-audio controller writes the requested channel mask. Zero disables
capture, while unsupported mask bits are rejected without changing selection.
The callback-facing method performs fixed-storage copies and lock-free atomic
publication, with no FFT, heap allocation, mutex, wait, I/O or worker wakeup.

Frames identify the master tap, requested channels, generation, actual engine
sample rate, stream, selection revision, sequence, sample position and count.
Signed samples are copied at full rate; no envelope or decimation stands in for
the eventual FFT input. Non-finite validation belongs to the off-audio worker.

When the queue is full, new chunks are rejected and counted. The producer does
not overwrite admitted history or move the consumer's index. The next admitted
chunk declares a gap. Time advances while disabled or overloaded; a resumed
stream therefore starts at its current sample position. Preparation resets the
stream's time origin. Channel selection changes reset continuity.

The atomic selection revision also changes when hide/show occurs entirely
between callbacks. Queued frames retain the revision under which they were
captured, allowing the worker to discard obsolete data without resetting
queue storage concurrently with audio.

## Behavioral evidence

`cxx-plugin-live-capture` uses a literal stereo fixture with distinct signed
values and transients on either side of a 256-frame boundary. It proves exact
513-frame chunking, copied ownership after host-buffer reuse, selected-channel
data, generation/rate/stream metadata, invalid-mask preservation, zero-length
callbacks, disabled/resumed timing and rapid hide/show revisions.

It fills the 64-entry queue and rejects another 1000 chunks under an allocation
guard. All admitted frames retain their original signed PCM and coordinates;
the next publication marks the exact missing interval. Compile-time checks
require lock-free 64-bit atomics and a capture object no larger than 140 KiB.

The initial successfully compiled contract stub failed the registered CTest
on missing contiguous PCM. The implemented test passes. Focused GCC 11 gcov
executes **51/51** new capture-header source records. Seven separately applied
copied-source faults are rejected by named assertions: unsigned PCM, lost time
origin, hidden overflow gap, omitted selection revision, excessive capacity,
callback allocation and skipped consumer history. This is focused calibration,
not exhaustive mutation or branch coverage.

The existing realtime architecture guard now covers this header as well as
meter capture. A copied valid mutex declaration initially passed the old guard;
the extended guard rejects it by the live header's name, while the real capture
passes through CTest. This source guard complements the allocation checks and
does not substitute for later callback instrumentation.

At the capture increment, both complete build configurations passed. CTest passed **27/27 native**
and **52/52 Web** without skips, including the new capture test and existing
sampler/303 renderer regressions. Strict OpenSpec validation and the unchanged
main-spec coverage gate pass; no delta scenario is synced or marked complete.

Raw commands, revisions, input hashes, RED/GREEN logs, gcov and copied faults
are retained in `/tmp/dandrum-live-analysis-evidence`. At that increment, scope/FFT processing,
gap-window resets, session delivery, processor integration, actual renderer
consumption, signed engine-output parity and callback timing remain pending.
This component test does not prove those system-level outcomes. Full task 6.3,
resource/stress work and automatic structural rebuilding remain unchecked.

## Contiguous numeric analysis

`InstrumentUiLiveAnalysis` now accumulates copied capture chunks off audio. Each
result owns a complete 1024-frame window, 128 signed min/max buckets of eight
frames per selected channel, and 513 spectral magnitudes. Windows advance by
256 frames. Frequency coordinates use the captured sample rate; scope bounds
and window coordinates use the original stream's frame positions.

The live and prepared services share `InstrumentUiSpectrumAnalysis`: a
1024-point periodic Hann window, one-sided peak dBFS normalization and a -120
dBFS floor. DC and Nyquist are not doubled. Prepared tails retain their existing
zero-padding contract; live analysis emits only complete contiguous windows.
Finite overrange PCM is scaled before the float FFT and restored in double,
preventing overflow without changing the measurement convention.

An explicit gap, unexpected sequence or position, changed generation, rate,
stream or selection discards the partial window. Invalid frames or non-finite
selected PCM do the same; unselected PCM is ignored. The first recovered result
requires 1024 contiguous frames and declares a gap. Later results resume the
256-frame hop. Reset affects only worker-owned accumulation, not queue storage.

`cxx-plugin-live-analysis` proves independent stereo sine peaks at -6.0206 and
-12.0412 dBFS, literal signed extrema, silence, endpoint scaling, padded prepared
tails, unequal capture chunks, retained result ownership and selected-channel
behavior. Nine discontinuity cases and ten malformed-frame cases exercise
reset and recovery; post-gap spectra reject the discarded pre-gap tone.
Float-maximum DC, Nyquist and interior sine inputs produce finite known dBFS.

Before extraction, focused gcov executed all **42/42** records at the prepared
service's numeric boundary. After implementation and refactoring it executes
**78/78** new numeric records: 52 live implementation, three identity comparison
and 23 shared spectrum records. Both headers are compiler dependencies; the
declarative spectrum header emits no executable record. Fourteen separate
copied-source faults compile and fail named assertions; the healthy copy passes.
This is execution coverage and focused fault calibration, not exhaustive branch
or mutation coverage, and not callback timing evidence.

The full Web suite exposed a prepared-spectrum paint race: ready labels could
commit before a passive React effect painted their Canvas. A read-only observer
in the original packaged Web runtime failed a literal marker-pixel assertion
before the draw hook moved to `useLayoutEffect`, then passed after the canonical
Vite build and asset embedding. Existing full/compact pixels and lifecycle
checks remain. The generated sampler bundle accompanies its source change.

Both complete build configurations pass with this increment. Final CTest passes
**28/28 native** and **53/53 Web**, without skips. TypeScript, Vite packaging,
strict OpenSpec validation and the unchanged main-spec coverage gate pass.
Commands, input hashes, failed and successful runs, gcov and copied faults are
retained in `/tmp/dandrum-live-worker-evidence`.

At the numeric increment, the analyzer was an unwired prerequisite: the sole
analysis worker, bounded session delivery, backlog policy, subscriptions,
processor capture and actual live renderer consumers remain pending. Neither
component tests nor the prepared-spectrum runtime prove signed engine-output
parity, live callback safety, stress timing or automatic structural rebuilding.

## Scheduled worker and bounded delivery

`InstrumentUiLiveService` owns one analysis worker and four fixed session slots.
Only the audio producer calls `capture` and `beginStream`; the latter requires
a safe preparation boundary. All subscription, status and delivery operations
are off audio. Visible sessions contribute their selected-channel union;
hidden or closed sessions stop contributing demand. Generation changes retire
sessions and their pending results. Result packets own their numeric arrays.

Each session permits one unacknowledged packet and one replaceable latest
packet. Acknowledgements must match session, generation and sequence. Slow
consumers therefore retain at most eight pending payloads across four sessions,
and resume at current coordinates instead of replaying every earlier window.
This service bound does not yet prove bounded browser transport queues.

The worker polls every 10 ms while subscribed and waits when there is no demand.
Each batch drains at most 64 chunks and keeps at most 2048 recent input frames,
with at most eight FFT measurements including previous overlap. Trimming resets
partial accumulation; its gap survives coalescing within a batch and across
unsent packets. Detected producer overflow discards queued obsolete history.
Fresh data must then fill a complete contiguous window. Generation and selection
are checked after a potentially stalled batch admission, and the demand revision
is checked again before publication. Queue storage is never reset by the worker.

`cxx-plugin-live-service` runs the real capture, FFT and scheduled worker. It
asserts signed scope extrema, independent stereo dBFS/frequency values, owned
retained packets, current frame bounds after stalls, bounded subscription and
work admission, overflow recovery, hide/show between callbacks, generation
changes before analysis and after completed FFT, and retained gaps. One-frame
chunks separately prove complete-window accumulation without padding; that test
does not establish sustained realtime throughput for one-frame host callbacks.
Stop-aware barriers hold the actual worker. Off-audio destruction stops and
joins it before reclaiming its queue and sessions.

The successfully compiled initial stub failed subscription admission. The
unbounded implementation later failed the backlog-gap assertion before the
bounded behavior was implemented. Final focused GCC 11 gcov executes **139/139**
service implementation source records; the declarative header emits no source
record. Ten copied faults compile and fail named behavioral assertions. An
earlier channel-union fault survived because a later stereo subscription hid
it; the independent two-subscriber assertion now rejects that fault. Failed
runs and original fixtures remain alongside the repaired evidence. These are
execution coverage and focused fault calibration, not exhaustive branch or
mutation coverage.

At the worker increment, the service was registered in CTest for both
configurations and linked only to its component test. Actual processor capture, shared commands/adapters,
native/Web live views, signed engine-output parity, callback instrumentation and
timing remain pending. Task 6.3 and the broader resource/stress and structural
runtime tasks stay unchecked. Raw commands, input hashes, coverage and faults
are retained in `/tmp/dandrum-live-service-evidence`.

Both complete builds pass. Final CTest passes **29/29 native** and **54/54 Web**,
without skips, including existing sampler and 303 runtime regressions. Strict
OpenSpec validation and the unchanged main-spec coverage gate pass. These wider
regressions do not turn this standalone service into processor or live-view
integration proof.

## Processor capture connection

The processor now owns the live service and links it into both plugin builds.
Its off-audio session API returns owned numeric packets; the audio callback only
copies the rendered master stereo output into the fixed queue. Disabled demand
still advances sample coordinates. Muted callbacks capture their actual zero
output. Existing host parameter objects and ordinary automation remain live.

`cxx-plugin-live-processor` uses the real processor and Rust engine with the
maintained `plugin-ui-knob.yaml` fixture. At 44.1, 48 and 96 kHz it asserts
bit-exact signed output, cleared extra lanes, actual window coordinates/rate,
known signed scope and DC dBFS values, hide/show freshness, muted zero output,
and stable host control identity after live parameter changes. A stop-aware
barrier holds the actual analysis worker while 72 audio blocks complete, causing
exactly eight rejected chunks in the fixed 64-chunk queue. Recovery discards
obsolete history and requires a fresh complete window at current coordinates.

Thread-local Linux linker instrumentation observes C++ `new`, directly linked
mutex acquisition, `std::thread::join`, JUCE FFT and engine preparation/destruction.
Real off-audio joins and FFT calls calibrate the observation boundaries. The
healthy schedule reports zero observed callback operations; separate copied
processor faults deliberately invoke each forbidden operation and fail with
positive counters. Channel swaps, an invented rate and a subscription-time
stream reset also fail named behavioral assertions. This is eight focused
faults, not exhaustive mutation, Rust allocator coverage, every OS call, callback
timing or DAW loading evidence.

The compiled admission stub failed before implementation. Focused GCC 11 gcov
executes **24/24 changed processor source records**; the unchanged processor is
not fully covered. A signed-zero expectation and test interception alignment
were repaired without changing the engine. Initial missing-include, browser
build and FFT-link setup failures are retained and not credited as behavioral
REDs or fault kills. Raw commands, inputs, coverage and faults are retained in
`/tmp/dandrum-live-processor-evidence`.

Both complete builds pass; CTest passes **30/30 native** and **55/55 Web** without
skips. This proves normal processor capture and held-worker independence. The
fixture constructs the real processor directly; original native/Web live-view
factory parity, shared adapters and bounded browser delivery remain pending.
At that increment, concurrent reload/reprepare identity and safe reader
retirement remained pending: fixed-delay reclamation was not an ownership proof.
The handoff increment below supplies a prerequisite for the concurrent identity
test; the live identity integration section below now records that regression.
Tasks 6.3/6.4, structural runtime and the full sampler/303 goal remain incomplete.

## Acknowledged engine handoff

Existing engine installation, host reprepare and state restoration now close a
lock-free callback reader gate. A callback uses one bounded atomic acquisition;
after closure, it clears every output and returns before reading the engine or
slot storage. A reader acquired before closure retains access through its whole
callback. The replacement thread waits for the acknowledged count, changes the
engine/slots and destroys the retired engine off audio, then reopens admission.
The processor destructor also closes admission before engine cleanup. Hosts
must still stop issuing callbacks before deleting the processor object.

Linux registered baseline and held-reader tests use the actual processor and
Rust engine. The maintained gain fixture produces literal `+0.25` before
replacement and `-0.5` afterward, with the same public host object and count.
The held test stalls return from an actual completed render. Reload, reprepare
and state restoration remain pending while that reader is held; later
1/64/512/2048-frame callbacks clear four sentinel-filled lanes to bit-exact zero
without entering the renderer. Explicit reader release allows exactly one
retirement and resumed signed audio. Quiescent replacement finishes without
further callbacks. The original held-reader regression failed before production
changes because the old engine was retired while its reader was held.

The test's deliberate held-render sleep only controls the observation schedule;
it does not measure real-time performance. Thread-local instrumentation covers
C++ `new`, directly linked mutex acquisition and FFI preparation/destruction,
with zero observed callback operations on the healthy paths. It does not cover
every Rust allocation, OS operation or DSP branch. The callback count is an
ownership mechanism, not permission for concurrent DSP callbacks on one
processor. Normal host callback serialization remains required.

Final focused gcov executes **36/36 changed executable processor records**;
whole-processor and exhaustive branch coverage are not claimed. A copied healthy
processor passes, while nine compiled/linked faults fail named assertions:
fixed-delay retirement, missing handoff in each of the three replacement paths,
failure to reopen admission, failure to restore mute, and actual callback
allocation, direct mutex acquisition and FFI destruction calls. The three
callback faults produce positive corresponding counters. Unsafe retirement
faults fail at the real destruction request before freeing borrowed storage.
An initial null-pointer observer error was corrected and the entire matrix
rerun; that failed calibration is retained and not credited as a fault kill.
Commands, input hashes, RED/GREEN/refactor runs, raw gcov, copied sources and
fault evidence are retained in `/tmp/dandrum-engine-handoff-evidence`.

Both complete builds pass, with **32/32 native** and **57/57 Web** CTest cases
passing without skips. The Web final build uses serial execution after a
parallel repeat hit duplicate JUCE Linux subprocess-helper generation; the
failure and successful full retry are retained. Strict OpenSpec, unchanged
main-spec coverage mapping and local document links pass. Existing sampler/303
runtime regressions remain separate from the pending structural UI and live
view integration proof.

This is partial task 9.2 evidence. Reload still prepares before installation;
the processor-owned structural admission, one-job coordinator, immediate mute
before validation, editor-independent completion, failure recovery and host
surface compatibility checks remain pending. Existing off-audio preset writes
and reentrant engine writers also need the later serialization/live-binding
work; this increment does not make every engine operation concurrency-safe.
Shutdown with pending structural work is not covered by these held-reader cases.
Concurrent live identity is covered by the additional integration below. No structural acceptance case or
task is marked complete, and no delta spec is synced.

## Live identity during engine replacement

`cxx-plugin-live-handoff` extends the same real processor/engine fixture with an
active live worker. Reload, host reprepare and state restore hold an old render
while replacement waits at the acknowledged gate. Later sentinel-filled
1/64/512/2048-frame callbacks still return exact zero without engine access.
Retained old packets keep literal `+0.25`, their 48 kHz rate, generation and
captured output-stream frame coordinates after replacement. Host parameter
object/count stay stable.

Host reprepare requests 96 kHz after the held callback took its block metadata.
An off-audio retirement observer reads the actual completed old window while
admission remains closed: it must still report `+0.25` at 48 kHz. Reprepare keeps
the public generation; reload/state restore advance it and reject obsolete
subscriptions/acknowledgements. Each replacement starts a new live stream at
frame zero. A first 512-frame half-window produces no packet; another 512 frames
produce one gapped `0..1024` window of literal `-0.5` with the current generation,
rate and frequency coordinates. Old partial/overlap data cannot join that window.

The initial fixture failed because baseline FFT overlap completed valid old
windows; that is a setup/oracle correction, not evidence of splicing. The final
fixture resets demand after acknowledging baseline, while the reprepare case
separately observes and acknowledges a complete old window before reopening.
Actual callback guards observe zero C++ allocation, direct mutex acquisition
and FFI preparation/destruction. These are Linux observation boundaries, not
all Rust allocations, OS calls, DAW scheduling or callback timing.

The production handoff/capture code is unchanged from `54f8b4f`. GCC 11 gcov
executes the same 36/36 H1 guard records with the additional live schedule;
whole-processor/branch coverage is unclaimed. Five compiled/linked copied
processor faults fail named assertions: lost stream reset, fixed rate, late
old-block rate relabelling, swapped capture channels and fixed-delay retirement.
The healthy copy passes. Sources, failed setup, commands, gcov and faults are
retained in `/tmp/dandrum-live-identity-evidence`.

This proves the controlled three replacement schedules, not arbitrary racing
host setters, failed reprepare labels, pointer reuse across several replacements
without callbacks, or every engine writer. Shared commands/adapters, actual live
views, browser delivery and the automatic structural coordinator/recovery remain
pending. Tasks 6.3/6.4 and section 9 remain unchecked; no delta spec is synced.

Final complete builds pass (native 8.619 s, Web serial 668.216 s), including
Standalone and VST3 targets. Final CTest passes **33/33 native** and **58/58 Web**
without skips; the sampler/303 original runtime regressions retain their own
scope. Strict OpenSpec, unchanged main-spec map and local document links pass.
All warm builds, dependencies and raw failed/successful evidence are retained.


## Live master displays and browser commands

The sampler now offers **Scope** and **Live FFT** alongside prepared Wave and
Spectral. Both native and React views consume actual master-output PCM through
the processor-owned live service. L/R selects an independent captured channel;
it does not mix channels or change the instrument. A signed scope uses 128
buckets over one complete 1024-frame output window. The spectrum uses the
shared periodic Hann, 1024-point FFT, 256-frame hop and one-sided peak-dBFS
measurement with a -120 dBFS floor. Its frequency axis is logarithmic and omits
DC. Labels distinguish output-stream frames/rate from prepared sample frames.
These are bounded current-window displays; no live spectrogram history is added.

Five registered Web commands subscribe, set visibility, take, acknowledge and
unsubscribe. Every 64-bit identity/position crosses as a decimal string. The
bridge emits only selected channels and fixed-size numeric arrays, with no
engine/host-buffer pointers. Bridge destruction and generation changes release
subscriptions. One browser controller per bridge session owns both live display
modes, permits one request through acknowledgement, coalesces visibility and
rejects obsolete replies. It closes admitted demand without waiting for delayed
Promise replies. Native components retain an owned typed packet copy; React
validates coherent copied measurements before deriving plot coordinates.

`cxx-plugin-live-bridge` exercises the real registered closures, Rust engine and
worker with literal +0.5/-0.5 left, silent right, 96 kHz, 6000 Hz bin coordinates
and exact stream windows. It proves acknowledgement backpressure, malformed and
stale rejection, hidden resumption and eight bridge lifetimes without leaking
the four subscription slots. This is an in-process command lane, not a browser
substitute. `live-analysis-web-transport` separately controls Promise delivery,
including stalled requests/acknowledgements, hide/show, closure, stale packets
and renderer errors. `live-analysis-web-view` asserts exact large frame IDs,
signed channel geometry, host-rate frequency, malformed measurement rejection
and paint-adapter output.

`native-live-renderer` and `web-live-renderer` use the shipped sampler processor
and editor factories. The original native component or packaged React/WebKit
Canvas renders actual engine output at 1200x800 and 820x560. Independent pixel
oracles assert positive/negative half-scale PCM, silent right-channel scope,
and the known Hann bin-1 magnitude of constant half-scale audio at 96 kHz.
Actual rate/settings labels and output-frame bounds are checked. Reload requires
a current-generation view; switching to Wave hides live analysis, then returning
to Scope requires newly captured frame bounds. Occupying all four real service
slots shows explicit unavailable state, and releasing them permits recovery.
No peak injection, replacement browser or manually published host state is used.

Focused GCC 11 gcov executes 99/99 changed bridge records and 124/124 changed
native editor records; whole-module/exhaustive branch coverage is unclaimed.
Node 18.20.8 V8 executes all lines, functions and branches of both new shared
JavaScript modules; this does not measure the complete React component. Seven
compiled bridge faults, six transport faults, six geometry faults and three
compiled native drawing faults fail named assertions after healthy baselines.
These calibrated faults are not exhaustive mutation analysis. The first copied
bridge build had an include-path setup failure. A transport fault initially
cancelled pending tests, and a rounded-boundary geometry fault initially
survived. Both test gaps were repaired and rerun; a later geometry driver also
needed the correct rejecting test name. Failed and successful runs, copied
sources, hashes, coverage and actual plot captures remain in
`/tmp/dandrum-live-ui-evidence`.

This increment changes the UI/command adapter, not Rust or processor DSP and
ownership handoff. It proves the listed controlled application schedules, not
DAW/VST3 loading, callback timing, all browser lifecycle races, long-duration
React stalls or complete resource/stress profiling. Tasks 6.3/6.4 remain
unchecked pending their consolidated resource and runtime evidence. Structural
admission, automatic rebuild/recovery, renderer structural controls and section
9 remain pending. No delta spec is synced or archived, and the sampler/303 goal
remains active. Full builds pass, including Standalone and VST3 (native 88.685 s, serial
Web 680.847 s). An admission-test timing flaw caused the first Web suite to fail
61/62: startup frame counts could expire before browser reply delivery. The test
now passively records the original native subscribe Promise reply without
changing its value, and waits for the resulting React state. A repaired owning
Web test build passes; the refreshed full native build also passes. Final CTest
passes **34/34 native** (24.086 s) and **62/62 Web** (51.731 s), without skips.
Strict OpenSpec, unchanged main-spec map and document links pass. Warm builds,
dependencies and failed/successful evidence are retained.

## Sustained consumer and original-browser reply stalls

The C13/C14 verification increment advances tasks 6.3/6.4 without changing
production C++, Rust, React, assets or engine behaviour. It extends the real
service component test and registers `web-live-stalled-replies` in CTest.

`cxx-plugin-live-service` now takes four real packets, keeps all four consumers
unacknowledged, and processes sixteen subsequent 2048-frame batches. Every
batch checks the eight retained delivery positions, rejected additional takes,
capture loss and the eight-window work limit. An ordinary C++ `new` interceptor
observes zero allocations during warmed capture/analysis/publication, and the
worker observer sees one persistent off-caller thread. Recovery asserts the
current `32768..33792` window, literal -0.5/+0.25 channels and unchanged +0.5
older copies. Hiding all consumers clears demand and delivery; resumption after
16384 inactive frames requires the gapped `50176..51200` window.

| Measured or declared boundary | Value and scope |
| --- | --- |
| Live subscribers | Four fixed slots |
| Retained delivery positions | Four outstanding consumer copies plus four replaceable latest values |
| Service object | 205864 bytes with GCC 11/Linux; excludes worker stack and separately allocated FFT state |
| Typed packet | 14472 bytes on that build |
| Warmed ordinary C++ allocation | Zero observed `new`/`new[]` calls; direct malloc, aligned allocation and process RSS are not measured |
| Component work | 130 observed windows including warmup and resumed capture; at most eight per checked batch |
| Browser live request | One occupied request through acknowledgement, including delayed native Promise replies |
| Browser packet size | Largest observed serialized fixture packet was 27778 bytes; this is a measurement, not a universal byte quota |

The service's outstanding position records an acknowledgement identity; its
packet has already been copied to the consumer. The table does not count an
additional outstanding packet array inside the service. Fixed object sizes do
not measure total processor/browser memory or prepared cache retention.

The browser lane uses the original sampler processor/editor factories, compiled
engine and packaged React/WebKit page. A test observer forwards the original
registered requests unchanged and delays one actual reply. It never supplies
sample values, packets, host state or replacement drawing code. While the host
fixture continues 64-frame processing at its prepared 96 kHz rate, every left
sample equals literal +0.5 or -0.5 and the right remains silent. These are
controlled callbacks in a JUCE application, not a DAW scheduling benchmark.

| Schedule | Required original-consumer outcome |
| --- | --- |
| Packet reply held for five seconds | No additional get or acknowledgement; old plot stays fixed; worker continues and retains at most two delivery positions; fresh negative scope follows release |
| Acknowledgement reply held for five seconds | No additional get or acknowledgement; native acknowledgement has completed and only latest remains; fresh positive scope follows release |
| Hide while a packet reply is held | Wave removes the live panel, native demand/payloads become zero and settled analysis stops; release/show produces current coordinates |
| Reload while a packet reply is held | The existing bridge refreshes the document, discarding the old page and Promise; the new page displays current generation, complete windows, actual settings and signed pixels |
| Close while a packet reply is held | Original editor destruction clears native demand/payloads; reopening through the original factory displays current data after intervening callbacks |

Each recovery is checked against the original Canvas pixels, current generation,
a complete 1024-frame span and actual 96 kHz/Hann/FFT/hop/floor labels. The test
also runs the existing full/compact scope/spectrum and admission schedules.
CTest marks this distinct 24-second lane `runtime;stress`, runs it serially,
requires a display and disables AT-SPI with `NO_AT_BRIDGE=1`. Accessibility is
outside this lane. Five-second holds measure delayed operation replies, not a
frozen JavaScript event loop or arbitrary renderer stalls.

The current complete component suite executes all 139 live-service GCC gcov
records; exhaustive branch, JUCE and browser coverage is unclaimed. Three
compiled, linked copied component faults fail the new named assertions after
a healthy baseline: per-publication allocation, lost acknowledgement gating and
obsolete latest-packet retention. Mutation selection includes only the service
component case, with no end-to-end tests. Separately, a test-only calibration
submits an extra actual registered browser request: the original-consumer lane
fails its named request-count assertion in three seconds. Production sources
and assets are unchanged by that calibration; it is not a product mutation run.

Retained failures include a null-buffer fixture crash confirmed by ASan, a C++
raw-string compile error, the first calibration driver's unmatched source
fragment, and a browser test timeout. The timeout exposed a harness assumption
that reload retained its JavaScript page; the actual bridge refreshes it. The
harness now requires retirement and fresh-page recovery. A bounded diagnostic
also hit an unavailable AT-SPI registry. The repaired component fixture passes
ASan. The component loop was reduced from 64 to 16 batches to keep ordinary
feedback below a second while retaining repeated four-consumer coalescing.
No production refactor is needed for these characterized schedules. Raw commands,
sources, linkage maps, coverage, setup failures and healthy/calibrated runs are
retained in `/tmp/dandrum-live-stall-evidence`.

Tasks 6.3/6.4 and section 9 remain unchecked. Prepared cache byte accounting,
broader combined resource workloads, event-loop stalls, callback duration/CPU/RSS
and DAW hosting remain separate open evidence. This increment does not finish
the sampler/303 layouts, automatic structural rebuilding or the overall goal.
The live pull/acknowledgement bound also does not bound other host-state events;
the existing `publishParameterUpdates` push path needs its own event-loop-stall
check before claiming bounded browser publication as a whole.

Final complete builds pass, including Standalone/VST3 (native 7.616 s, serial
Web 670.151 s). CTest passes **34/34 native** (26.893 s) and **63/63 Web**
(71.8 s), with no skips. The normal browser stress run passes in 24.87 s after
the extra-request calibration is disabled; its earlier healthy owning run
also passes. Strict OpenSpec and the unchanged main-spec map remain finalization
gates. No baseline scenarios/fingerprints are synchronized by this increment.
