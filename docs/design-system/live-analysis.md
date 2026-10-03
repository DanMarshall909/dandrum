# Live analysis implementation evidence

Task 6.3 remains in progress. Its capture foundation is
`InstrumentUiLiveCapture`: a processor-owned queue for one master-output tap,
at most two selected channels, 64 chunks and at most 256 frames per chunk.
The queue owns copied PCM; no frame retains a pointer to a host buffer, engine
or editor. The scheduled service described below now connects this queue to
an analysis worker. Processor output and renderer subscriptions remain pending.

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
test; that live identity integration is still pending.
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
Shutdown with pending structural work and concurrent live generation identity
are not covered by these held-reader cases. No structural acceptance case or
task is marked complete, and no delta spec is synced.
