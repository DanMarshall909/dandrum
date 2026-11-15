# Stopped VST3 feedback checkpoint — 15 November 2026

This is the feedback/projection portion of task 3.2. The task remains open:
balanced host gestures and once-only gesture cancellation are still pending.
The Linux native VST3 editor, installed REAPER qualification, official validator,
combined review and publication remain pending too. Mutation is excluded.

Zero-sample host changes now reach the controller through processor-owned
main-thread service, including while playback and the editor are absent. The
existing signed flush/cursor, history and complete callback allocator/reentry
tests remain in the full suite. Delivery uses the standard-interface proxy and
bounded queues: eight transport packets, the existing 1024-entry native ring,
and at most64 feedback/recovery iterations per service pass. Coherent scalar
reads retry off audio; this is not a hard service-latency bound.

The shared owner tags feedback with the completed scalar-write interval. A
coherent snapshot reads values, settlement, last-write origins and the global
cutoff together. Actual parameter writes and scalar restoration update origins;
empty flushes, reset cancellation and reconfiguration preserve them. These
runtime-local stamps are neither musical generations nor portable state.
Reset/settlement regressions first failed, then passed after putting reset in
the write interval and moving settlement inside the coherent reader.

Both adapters now call the same canonical parameter-feedback frame encoder.
CLAP's wire shape and discrete runtime behavior are retained. VST3 puts ordering
metadata outside that frame. It validates complete batches before publishing
any cache value, buffers feedback arriving before an initial snapshot, orders
different parameters independently, and retains exact refused packets for retry.
Malformed namespaces, values, counters, offsets, duplicate parameters and native
stamp/origin metadata preserve the cache.

A successful controller-only import preserves imported values even if its
processor differs. It immediately issues a correlated processor barrier;
obsolete issued snapshots are validated and acknowledged without applying them,
so delayed initial replies and old-request retries cannot block delivery. The
barrier's processor observation defines feedback freshness. If a host proxy
accepts but defers the request, the imported projection remains authoritative
until that barrier is processed. Ordering is not inferred from notify success or
from the return time of setComponentState. Subsequent host setParamNormalized
calls still update the projection immediately under the existing contract.
Dedicated tests distinguish delayed replies from delayed requests.

Independent review found a native-ring overflow gap beyond transport saturation.
Its regression ended at normalized0.375 instead of the accepted final0.875.
An atomic loss signal now requests one coherent recovery snapshot after the
retained ring prefix drains, without borrowing the live audio endpoint from
main merely to inspect the signal. Failed snapshot staging restores the signal.
The test exceeds both capacities, then stops writing and proves convergence in
both stopped and active processing. A second RED exposed unrelated imported
pan being replaced by the processor default:0.5 instead of0.375. Per-parameter
origins fix that distinction, including actual writes that return to an earlier
value. An adopted snapshot supersedes queued intermediate feedback through its
global cutoff, including the same interval; ordinary equal-interval points
remain ordered by event sequence. Snapshot identity/cutoff conflicts reject.

Current-source verification passes61 VST3 tests,229 shared-host fast tests,
two doctests and the separate real256MiB portable-state slow test (62.04s test
execution;63.49s complete producer). Ignored allocator/reentry children are
executed by their calibrated parents; the host's ignored slow test is run
separately. CLAP passes20 dynamic tests and the actual copied-release X11 test
`copied_native_host_feedback_updates_rendered_control_without_polling_or_undo_pollution`.
Its product/test/resources are hash-bound; owned cleanup reports exit0, scratch
removed and no remaining workers. Strict all-target Clippy for the shared host,
VST3 and editor-enabled CLAP, formatting, scoped OpenSpec and canonical spec
coverage pass. No acceptance scenario is marked newly complete here.

Fresh LLVM exports use only profiles from these owner runs and their exact
recorded objects. The consolidated named-value and canonical-counter parsers,
shared dispatcher and new shared projection/encoding APIs execute all their
regions. The receipts also disclose unexecuted lifecycle/invariant returns,
counter exhaustion and snapshot staging recovery, plus uncalled unit copies
and a coherent-reader retry edge. Whole-owner100%, branch/MC-DC and arbitrary
foreign-memory safety are not claimed. Parser duplication was consolidated
after behavior coverage; no new module or second session authority was added.

`source.json` binds all11 changed source/test files and their base patch.
`verification.json` contains compact source-bound coverage, object/profile
identities, commands/timings and native cleanup/binding evidence. Compressed
members contain text diagnostics only. Failed fixture/lint trials are retained;
one unused-variable lint log was overwritten during its final rerun, so its
preserved diagnostic explicitly identifies that limitation. Full private logs,
LLVM profiles/JSON, products and native artifacts remain under
`/tmp/dandrum-vst3-remaining-20261115/`, with checkpoint retention under
`.git/codex/`. No plugin, sample, profile or browser binary is published here.
