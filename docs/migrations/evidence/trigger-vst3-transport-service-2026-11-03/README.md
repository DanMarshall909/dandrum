# Processor-owned VST3 coordination through standard messages

This task2.3 checkpoint uses the actual dynamic components and standard
`IConnectionPoint`/`IMessage` host proxies. There is no private owner interface,
serialized pointer or global instance lookup. The processor owns the single
shared ParameterController/session/history and preparation worker. The edit
controller reads metadata and receives a bounded projection; it requests current
values from that owner instead of constructing another instrument.

The transport has eight pending packets and eight retained once-only replies,
fresh128-bit routing scopes, a bounded payload and finite send-attempt budget.
Inline callbacks can disconnect/reconnect without acknowledging a new queue
head. Failed delivery retains the reply; full storage rejects new work before
musical admission. Old/foreign/conflicting identities cannot replay an edit.
The shared versioned command dispatcher and journal remain authoritative.

All27 current tests pass:7 discovery,11 connection,4 processor-owner and5 service.
They include malformed frames/replies with preserved values/correlation,
standard-interface-only proxies, both connection orders, independent histories,
retained mapping Undo after projection loss, lost replies, capacity preservation,
processor-only real-worker completion without any edit controller/native editor,
main-thread affinity and termination/reference cleanup. An orderly terminal
message revokes a surviving endpoint. One processor-owned10ms IRunLoop timer
also drives retries for an unsent controller request. Service starts without a
view and ends when the processor terminates.

Registration can call back inline. Partial initialization stays invisible;
failed registration can retry, refused unregister keeps a disabled weak token,
and termination during registration rolls back once. `Weak` timer ownership
lets a retained late callback become harmless after the processor disappears.
`Registration::unregister` returns its owner on failure rather than losing the
only token needed for cleanup.

Strict Clippy passes. verification.json records current source identities and
partial owner coverage; factory and explicit parameter mapping are fully
executed in the actual cdylib report. The combined Cargo report also contains an
unused test-harness copy of C exports, so it under-reports their shared source
lines; the artifact-specific export resolves that attribution. Profiles are TDD
diagnostics, not the final fresh-profile completion gate. Mandatory unsupported
message accessors, future audio/state/view stubs and defensive platform failures
remain visible. Branch/MC-DC instrumentation is unavailable.

Two initial owner tests had incorrect fixture assumptions: the existing journal
normalizes YAML presentation, and a refused FIFO head blocks later replies.
The corrected assertions check literal restored key ranges/history and actual
FIFO delivery. Failed trials remain in the text archive, separate from real RED
for missing messaging, backpressure recovery, termination admission, service
registration, partial initialization, cleanup refusal and service-driven delivery.

Debug artifacts are about197MiB each. Repeated NODELETE fixtures could retain
more than2GiB of backing copies until process exit; that earlier transient peak
was not sampled. Disposable debug copies now omit debug sections (about59MiB),
with every allocated section and LLVM coverage section verified byte-identical.
The cached originals remain intact; ordinary small release candidates are copied
byte-for-byte. An initial section-inspection command accidentally requested an
in-place objcopy output and was rejected by the read-only sandbox; the source
artifact was unchanged and verification was replaced with read-only ELF parsing.
The32GiB cumulative-output cap and other resource limits remain unchanged.

Task2.3 stays unchecked because processor-only restoration depends on task3.3.
Audio, curves/stopped controls, portable state, actual React/Wry lifecycle,
installed/source-hidden operation, official validation, native REAPER acceptance,
independent completion review and publication remain required. This checkpoint
claims none of those later outcomes. All compiled binaries remain private.
