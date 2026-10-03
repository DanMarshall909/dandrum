# Live analysis implementation evidence

Task 6.3 remains in progress. Its capture foundation is
`InstrumentUiLiveCapture`: a processor-owned queue for one master-output tap,
at most two selected channels, 64 chunks and at most 256 frames per chunk.
The queue owns copied PCM; no frame retains a pointer to a host buffer, engine
or editor. This foundation is not yet connected to `processBlock`, an analysis
worker or either renderer's subscriptions.

## Capture contract

One audio producer writes the queue and one future analysis worker consumes it.
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
captured, allowing the future worker to discard obsolete data without resetting
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

Both complete build configurations pass. Final CTest passes **27/27 native**
and **52/52 Web** without skips, including the new capture test and existing
sampler/303 renderer regressions. Strict OpenSpec validation and the unchanged
main-spec coverage gate pass; no delta scenario is synced or marked complete.

Raw commands, revisions, input hashes, RED/GREEN logs, gcov and copied faults
are retained in `/tmp/dandrum-live-analysis-evidence`. Scope/FFT processing,
gap-window resets, session delivery, processor integration, actual renderer
consumption, signed engine-output parity and callback timing remain pending.
This component test does not prove those system-level outcomes. Full task 6.3,
resource/stress work and automatic structural rebuilding remain unchecked.
