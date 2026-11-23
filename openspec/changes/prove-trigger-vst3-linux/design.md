## Context

**Accepted plan, 3 November 2026.** Dan explicitly approved the proposed plan
and resource budget in the implementation goal. This includes separate
components (D1), pinned bindings (D2), full-curve/failure policy (D4), shared-code
reuse (D6), and the finite D8 allowance. Native Windows/macOS remain mandatory
subsequent milestones with exact OS/CPU matrices refined later. Discovery in
task 1.2 must still qualify the actual host/service and validator contracts.
Main/source baseline: `79efd72078260dc67c068456a10a7c0b830347ed`.
The previous private Linux CLAP beta is complete. Its evidence remains bound to
its actual source/artifact identities; it cannot prove a new VST3 binary.

`ParameterController` owns the format-neutral Trigger session, portable state,
public descriptors and worker service. `RuntimeAudio` is transferred exclusively
to the processing endpoint. Session activation/reclamation and event queues are
bounded. `NativeEditor` already owns a disposable X11/Wry child. Much of the
editor orchestration still lives in the CLAP adapter and contains CLAP-specific
gesture delivery, timers and host callbacks.

Following the 17 November foundation split, [issue #45](https://github.com/DanMarshall909/dandrum/issues/45)
owns the remaining workflow/status. OpenSpec retains accepted design, acceptance
criteria and verified task completion; older private deferred-work entries are
historical evidence. The finite execution/storage limits and reassessments are
recorded in D8 below.

## Goals / Non-Goals

**Goals:** a real installed Linux/REAPER Trigger VST3 with discovered identity,
signed note/audio and curve timing, portable state, native editor recreation with
Undo, editor-absent/stopped service, independent instances, callback safety,
CLAP regression, independent review and source/evidence publication.

**Outside this milestone:** native Windows/macOS execution, Ableton/Bitwig,
Wayland, standalone devices, AU and public binary distribution. Native Windows
and macOS remain required subsequent milestones of the VST3 work, not optional
ideas or outcomes established by Linux/cross-compilation.

## Decisions

### D1 — Separate processor and controller; one processor-owned session

**Accepted by Dan, 3 November 2026: "separate".** Register distinct
processor/controller classes.
The processor's instance coordinator owns one `ParameterController` and its
session/history. VST3's edit controller is a projection, not another instance of
the shared type merely because their names both contain "controller".

Use standard `IConnectionPoint`/`IMessage` communication for bounded commands,
snapshots, replies and attachment lifecycle. The controller must work when a
host inserts a connection proxy exposing only standard interfaces. Querying
the peer for a private shared-owner interface is not a sufficient connection
mechanism: `ivstmessage.h` explicitly permits proxies, and the SDK's
`ConnectionProxy` forwards messages without forwarding arbitrary interfaces.
Keep `kDistributable` unset; proxy support does not claim remote execution.
Do not exchange pointer integers, use a global instance lookup or manufacture
a session on controller creation.

Reuse the existing versioned editor command/session identity and size limits
where possible, with explicit handshake/request identities and finite pending
reply storage. An accepted transport send is not an accepted musical edit;
replies/status must retain the shared owner's once-only admission semantics.
Do not assume notifications can only return asynchronously: the host may forward
them inline. Release mutable borrows before calling foreign interfaces and test
reentrant reply/disconnect, failed sends, stale/replayed messages, capacity and
disconnect/reconnect through a standard-interface-only proxy. Break connection
cycles on disconnect/termination. A surviving projection must fail cleanly after
processor termination. This is a new adapter transport, not a second journal or
resource authority.

The processor must initialize, restore state, render, service completions and
terminate without a controller ever being instantiated. Creation, disconnect,
reattachment and destruction orders are ABI-level test cases. Main-thread-only
owners stay on that thread; only the prebuilt audio endpoint and bounded shared
communication enter processing. COM reference counting alone does not establish
thread safety or permit dropping GUI/session resources on an audio thread.

Alternative: one non-distributable class implementing processor and controller.
That avoids the message/projection connection boundary and can reuse more of
the current direct-controller editor orchestration. Dan selected the standard
two-component structure for this proof, with the bounded transport and its
failure/lifetime tests retained in the plan.

### D2 — Exactly pinned interface bindings at the adapter boundary

Use `vst3 = "=0.3.0"`, with `com-scrape-types 0.1.1` pinned in the committed
lockfile. Inspect the actually resolved sources before implementation. A release
pin avoids a moving Git branch and needs no SDK generator at product build time.

The inspected upstream release archives have these identities:

| Package | SHA-256 | Recorded source revision |
| --- | --- | --- |
| vst3 0.3.0 | `e31e3fe7a6f028ded8c5a9984cf283319e6dc63572c9b71da1d4ee9d71d3b6bf` | `2d86c810be3e22367f92438800cd96a3e9cd3a37` |
| com-scrape-types 0.1.1 | `5ce183b975b8fc736a20919c4002717255fbd867df575c792aeefa9a56a73ad0` | `33f1859fbdcfa247c1fdd17d565cb5285860e4ce` |

The 61,475-byte and 9,463-byte archives were inspected directly from
`static.crates.io`; local receipts are in
`/tmp/dandrum-vst3-refinement-20261103/`. The bindings include MIT/Apache licence
texts; the helper archive declares the same licensing but lacks root licence
texts. Record notice provenance and that packaging gap rather than inventing
redistribution clearance. Independently pin the official SDK/validator revision
used for boundary validation and inspect ABI/interface identifiers against the
bindings; the upstream gain example is not an acceptance test.

Keep COM/unsafe pointer handling in `src/vst3-adapter`. Reuse safe Trigger
interfaces. Validate host counts, layouts, ranges, ordering and lifecycle before
publication; document host pointer-validity contracts and every unsafe operation.
Do not claim to validate arbitrary inaccessible foreign memory.

### D3 — Stable identities and one shared value policy

Allocate and commit new fixed processor/controller class IDs, distinct from the
JUCE comparison products. Declare a versioned explicit mapping from the existing
23 public aliases to VST3 integer parameter IDs. IDs must not depend on current
enumeration, a runtime hasher or a selected patch. Do not change CLAP identities.

Use each shared descriptor's physical minimum/maximum/default. Convert VST3
normalized values linearly into physical values, retaining f64 until the shared
DSP's documented precision boundary. Reject nonfinite/out-of-range host values;
test all endpoints/defaults and representative nondefault round trips. Preserve
the shared named-physical-value state representation and current public aliases.

### D4 — Honor VST3 automation curves with bounded processing

**Policy accepted by Dan, 3 November 2026: "yes"** to exact ramps/jumps,
controls before equal-frame notes, atomic malformed/excess rejection with
silenced output and preserved accepted state, and stopped controls without DSP
time advancement or Undo entries. Detailed capacities remain to be qualified.

VST3 points define piecewise-linear normalized curves, including an implicit
previous value at frame -1 when frame 0 is omitted. A discontinuity is represented
by adjacent old/new points. Preserve the previous block endpoint, interpolate
before physical conversion, hold the last endpoint and prove block-partition
invariance. Sampling only the last point or treating every point as a held step
does not meet this contract.

The current runtime accepts ordered discrete events. Qualify a safe,
format-neutral curve/segment entrypoint or covered equivalent using the real
runtime. Keep the existing CLAP discrete-event behavior unchanged. Do not expand
every parameter/sample into the bounded host-point queue, silently truncate a
ramp or increase capacities until tests happen to pass. Storage must be
preallocated and execution bounded by frames, public controls and admitted points.

Proposed initial bounds: existing maximum 16,384 frames, 1,024 total input curve
points plus note events per callback, at most 23 public parameter queues and no
new unbounded musical storage. Validate the complete batch before consuming
runtime work. A rejected batch preserves accepted values/pending transitions and
clears accessible output without partial notes/parameter application. Final
capacities require measured qualification within these limits.

Order controls before notes at an equal frame; preserve host note-list order
within that frame. Queue-local point offsets must be strictly increasing;
duplicate parameter queues, malformed points, invalid supported note values,
failed point retrieval and excess capacity reject atomically. Explicitly
unimplemented event types follow a documented ignore/unsupported policy rather
than becoming a spurious note. Reuse the current pitch/velocity semantics; new
per-note expression or voice-allocation behavior is outside this proof.

Zero-sample/no-audio process calls apply admitted scalar changes through the
existing flush path without advancing DSP time or playing notes. Main-thread
`setParamNormalized` updates the controller projection; processor values follow
the host's processing/flush delivery contract, avoiding double application.
Editor gestures call `beginEdit`/`performEdit`/`endEdit` on the host main thread,
with exactly-once closure on editor loss. Host automation stays outside native
Undo. Prove concurrent automation, feedback and editor recreation.

### D5 — Service belongs to the instrument, not the view

Use Linux `IRunLoop` service obtained from the host context where available, not
only from an attached `IPlugFrame`. Register bounded main-thread service for
worker completions, reclamation, editor notes/gestures and feedback independently
of view lifetime. Host service and GUI servicing are separate capabilities.
No GUI/preparation/reclamation, blocking lock, allocation or free in process/reset.

Qualify REAPER's actual run-loop/context behavior early. Headless rendering may
remain available without optional native GUI service; an unavailable required
service must produce an explicit capability/error and preserve usable sound.
It must not claim accepted asynchronous editing that can never finish. Register
and unregister ownership on initialization/termination, including failed partial
initialization and reentrant host callbacks. Do not add a nested or private host
event loop to make the probe pass.

### D6 — Share editor integration behind tested boundaries

**Common-code direction accepted by Dan, 3 November 2026:** "as much common
code should be retained as possible". This applies across the implementation,
not only to the editor. Keep session/history/sample ownership, runtime execution
and reclamation, parameter validation/conversion policy, portable-state framing,
editor command dispatch/reply encoding and reusable editor orchestration shared.
Curve support extends the common runtime rather than creating a VST3-only DSP
path. Do not fork the CLAP GUI module, state codec, command switch or parameter
policy to obtain a quick VST3 proof.

Extract currently CLAP-local format-neutral behavior when both formats need it,
starting with characterization tests. Stage each extraction at its first consumer;
do not postpone common command routing until after implementing a VST3 copy.
Require full coverage for any newly extracted module and exercise CLAP against
the resulting shared implementation before adding the new consumer.

Keep the canonical React bundle, editor SDK/protocol and Wry/X11 child. Shared
application code takes narrow callbacks/transport capabilities instead of Clack
or VST3 types. CLAP may call shared services directly while VST3's separated
controller uses bounded messages to those same services. Neither projection
owns a second journal or accepted document.

Only the incompatible edges differ: COM/CLAP interface plumbing and lifecycle,
host gesture delivery, host-loop registration, native parent handles and
format-specific bundle/module discovery. Share their format-neutral policy
where possible, while keeping the platform handles and ABI types at the edge.
Avoid speculative framework layers; extract concrete behavior with two real
consumers and retain existing headless dependency boundaries.

For Linux, parent using `IPlugView`'s X11 window contract; keep host parent/loop
ownership outside the plugin. The initial logical viewport is the existing 1200x800. Host content scale must consistently scale the physical X11 child, GTK/WebKit presentation and VST3 geometry; preserve working geometry when scaling is invalid or the host refuses a resize. REAPER7.84 on the observed192-DPI Linux desktop sends content scale1 and1200x800 ViewRects while realizing its X11 parent at2400x1600. Reconcile that uniform native-parent transform on attachment/onSize without guessing from desktop DPI or changing the host coordinate space; provisional/nonuniform parent rectangles retain the requested content scale.
Detach revokes native/IPC/dialog callbacks and ends owned gestures/notes once,
destroys the actual WebView/native child, and leaves the instrument/history
alive. Reattachment creates a new view/connection and recovers accepted outcomes.
Retain the existing verified code-residency requirement for GTK/WebKit callbacks
and qualify it for the VST3 module identity rather than assuming CLAP paths.

### D7 — Shared portable state, native bundle resource location

Processor `IBStream` state adapts the existing bounded versioned
patch/sample/named-parameter container, including its legacy packaged-kit reader.
Controller component-state loading derives a projection without loading another
musical session or emitting `IComponentHandler` automation/gesture callbacks.
Persistent musical authority stays with the processor. Support successful
partial stream reads/writes by continuing until complete; distinguish those
from truncated EOF, zero progress and failed I/O. Check bounds,
malformed/unsupported versions and invalid resources before commit; failed
loads preserve audio/history/feedback.
Fresh loads have the existing external-replacement history semantics; retained
Undo is required across view/controller loss, not across a new plugin instance.

Install a standard Linux bundle:
`DandrumTrigger.vst3/Contents/x86_64-linux/DandrumTrigger.so`, with shared
instrument/editor assets under `Contents/Resources/`. Resolve from the actual
loaded module/bundle, never cwd, checkout or a build-time absolute path. Pin all
installed file hashes and bind actual host execution to this artifact. Use
isolated test installation/search paths, not the user's production plugin set.

### D8 — Accepted finite resource envelope

**Approved by Dan, 3 November 2026:** "This goal approves the proposed plan and
resource budget." Initial implementation/verification allowance:

- 12 active executor-hours through the next adopt/repair/defer decision, excluding
  user wait time; checkpoint after discovery/audio, automation/state and native
  editor/REAPER. This is a cap for reassessment, not a delivery-time guarantee.
- One implementation owner in the primary checkout; independent reviewers are
  read-only. One heavy native/Cargo/coverage producer at a time, at most four
  compiler jobs. No paid services/hosted CI or newly purchased host licences.
- Keep at least 8 GiB MemAvailable and 8 GiB free disk before each heavy run.
  Approved incremental output allowance 32 GiB plus 2 GiB temporary growth on
  the observed shared filesystem. Reuse the managed Cargo 64 GiB retention
  policy; budget retained VST3-specific native outputs/evidence at 8 GiB and
  measure them separately. No approval to delete reusable caches/dependencies.
- Use existing owned-run supervision/cleanup and managed Cargo driver. Retain
  raw logs outside model context, source/artifact hashes, all failed host trials
  and compact portable receipts; remove only proven owned disposable scratch.
- Reproduce one final current candidate in the installed dynamic/native host and
  actual REAPER acceptance cases. Additional full trials require a changed input,
  specific failure or unresolved evidence gap. Bound each external-host probe
  to two attempts per fixed case, then diagnose/review before another experiment.
  A planned timeout is a failed attempt with cleanup evidence, not proof of a
  dead process; inspect its live handle/owned workers before recovery/restart.
- Preserve the recorded mutation exclusion; ordinary tests, coverage, CLAP
  regression, official validator and independent review remain required. No
  mutation job is started or implied by this proposal.

Resource reassessment, 3 November 2026: after 15.59 GiB of retained Cargo
output growth, Dan asked about current RAM/disk availability and replied
"ok go" to the proposed 32 GiB output allowance. This raises only incremental
output growth from 16 to 32 GiB. All other limits, cache/dependency preservation,
mutation exclusion and acceptance criteria remain unchanged. The initial
observations and foundation evidence retain their original 16 GiB envelope.

Resource reassessment, 16 November 2026: Dan replied "approved" to the
prepared proposal for **56 GiB cumulative incremental outputs** (previously
32 GiB) and a **112 GiB managed retention ceiling** (previously 80 GiB).
The 2 GiB scratch allowance, 8 GiB free-disk/MemAvailable floors, one heavy
producer, at most four compiler jobs, cache/dependency preservation, mutation
exclusion and original active-executor-hour reassessment cap remain unchanged.
Require `DANDRUM_CARGO_PRESERVE_GENERATIONS=1` and the non-mutating full cache
admission preflight before each producer: currently 17 GiB free for 8 GiB
growth + 1 GiB temporary growth + 8 GiB reserve, plus retention/growth/temp and
all-root cumulative output/scratch accounting. Refuse pressure rather than
evicting an earlier generation. These ceilings do not guarantee completion or
that the full allowance is usable on the current filesystem. No approval to
delete reusable caches/dependencies is inferred.

Warm-build reassessment, 16 November 2026: Dan replied "ok go ahead" to
**3 GiB output growth per producer for the existing warm configuration**.
Recent observed net managed growth was about 1.77 GiB for the first ordinary
host profile and 1.03 GiB for the full host LLVM run. The new warm admission is
3 GiB output + 1 GiB temporary growth + 8 GiB disk reserve = **12 GiB free**.
The 56 GiB cumulative output cap, 112 GiB retention cap, 2 GiB scratch allowance,
8 GiB disk/MemAvailable floors, preservation mode1, one heavy producer, max four
compiler jobs, mutation exclusion and original 12 active-executor-hour cap stay
in force. Qualify the unchanged generation/dependency graph and already prepared
profile before using the smaller reservation. Cold/new-generation/dependency/
profile work is refused while the 3 GiB driver policy is active. Restore the
original 8 GiB driver reservation (17 GiB free), or obtain separately reviewed
configuration-specific driver admission, before executing cold work. This approves continued warm verification, not cache deletion or
Linux acceptance. Continue measuring before every producer.

Foundation publication, 17 November 2026: Dan requested a separate ticket for
all remaining work and a merge of the completed foundation while remote REAPER
testing is unavailable. [Issue #45](https://github.com/DanMarshall909/dandrum/issues/45)
owns that follow-up. The original implementation allowance is exhausted; this
authorizes split-and-merge closeout, not restarting native implementation.
Restore the maintained driver's ordinary 8 GiB growth reservation before
publication: the smaller warm allowance depended on private local checks and
does not qualify cold builds on another checkout. Existing cumulative ceilings,
floors, preservation and mutation policy remain. Native editor, installed-host
acceptance and complete-product validation/review remain unproven requirements;
leave this change open and unsynced until those outcomes are genuinely proved.

Planning observations: Pop!_OS 22.04 x86_64, Ryzen 5900X, 32 GiB RAM;
MemAvailable about 19 GiB; shared checkout/tmp free about 132 GiB; root `build/`
logical size about 9 GiB, Cargo registry about 1.6 GiB. The existing workload's
9 GiB growth + 8 GiB reserve admission passes. These are observations, not reserved
capacity or a qualification of the newly proposed larger VST3 workload. Record
the agreed workload in `.ai-rules.json` and recheck before its first producer.

Resource reassessment, 23 November 2026: Dan requested completing the general
VST3 work separately from the parallel Trigger UI redesign and replied "ok go
for it" to another **4 active execution hours**, **8 GiB additional retained
outputs** and **1 GiB temporary space**, with the existing **112 GiB managed
retention ceiling**, **8 GiB free-disk/MemAvailable floors**, one heavy producer
and at most four compiler jobs. Preserve reusable caches/dependencies, keep
`DANDRUM_CARGO_PRESERVE_GENERATIONS=1`, no paid services and mutation excluded.
The new tranche starts from a recorded restart baseline; it does not renew the
old cumulative allowance or promise completion. Reassess at either cap. First
checkpoint is task4.2 native attachment/paint and safe destruction/recreation
using the existing UI. Issue45 remains workflow authority; the separate UI
redesign is not part of this change.

### D9 — Native platform milestones and initial host configuration

Linux target: qualify this Pop!_OS/Ubuntu 22.04-compatible glibc x86_64 system,
X11, GTK3/WebKitGTK, REAPER 7.84 revision `4b6323`, scale one, existing
`linux_ime=1` configuration. Use 48 kHz offline signed renders and the available
44.1 kHz/1024-frame PipeWire/JACK live configuration. Disable anticipative FX
only where live cache/timeline observations require it and record that limit.
Test fractional clocks and varying block sizes in the dynamic host/validator.
This does not establish an older Linux minimum or default REAPER configuration.

| Required following milestone | Proposed initial native target | Boundary and required qualification |
| --- | --- | --- |
| Windows | Windows 11 x86_64, native REAPER x64 | HWND child, WebView2 runtime/resource packaging, host UI-thread service, teardown, installed/offline state/audio/automation/Undo and independent instances |
| macOS | macOS 13+ Intel x86_64 and Apple Silicon arm64, native REAPER per architecture | NSView/WKWebView parent, app-main-thread service, bundle resources, packaging/loading and the same native installed/REAPER acceptance gates |

These OS/CPU targets are provisional planning candidates, not commitments or
compatibility claims. Exact minimum versions and CPU architectures will be
refined with Dan when the native machines/toolchains can be inspected. Linux
agreement retains both native milestones without prematurely fixing their matrix.
Keep platform parenting, scheduling, module location and packaging behind narrow
platform modules from the start. Later per-platform changes must determine actual
minimum versions, signing/notarization/distribution requirements and exact
REAPER versions on native machines. Cross-compilation alone cannot close them.
Ableton/Bitwig compatibility remains required closer-to-release qualification.

## Verification and stop conditions

Every spec scenario gets a genuine proving test and evidence description before
canonical sync. Use literal signed PCM/timing/physical-value oracles, including
negative controls that reject sign reversal, shifted automation, corrupted state
and lost history. Do not equate coverage execution with an asserted outcome.

Run fast owning tests after each RED/GREEN; retain RED evidence, then focused
coverage/refactor. At material checkpoints run affected shared CLAP/session/editor
tests. Final gate includes all relevant CTests/slow lanes, source-hidden installed
execution, the full pinned official VST3 validator without hidden exclusions,
actual native editor input, actual REAPER save/fresh-process reload and signed
renders, source-bound coverage, dependency/notices and spec-map checks. Document
unsupported SDK capabilities and concrete coverage/environment limits honestly.

Require independent GPT-6 Astra completion PASS for the exact candidate and its
publication envelope, normal hooks/local checks, unchanged reviewed head/base,
and satisfied remote restrictions before source/evidence PR publication/merge.
Do not publish private product, host or test/replay binaries. Sync accepted specs
and map scenarios only when the assertions genuinely prove them; archive after
completion under the repository workflow.

Stop dependent integration and report a concrete adopt/repair/defer decision if
the envelope is exhausted, ownership cannot meet the contract, curves lose timing,
service requires unsupported host behavior, callback safety fails or installed
REAPER cannot complete editor/state/history acceptance. Preserve working CLAP and
all valid evidence. Another wrapper/framework requires explicit ADR revision.
An inconclusive proof leaves the original goal unfinished; do not redefine it.

## Risks / Trade-offs

- Separate components add bounded messaging, ownership and connection ordering
  → qualify early through real COM interfaces, a standard-interface-only proxy,
  inline/reentrant failures, processor-only operation and disconnect tests.
- VST3 curves exceed the current discrete event model → prove literal ramps and
  cross-block continuity before GUI integration, preserving CLAP behavior.
- REAPER/GTK service may differ by format → qualify context run-loop, stopped
  operation and actual native view early; record exact host settings/failures.
- Shared editor extraction can regress CLAP → characterize before extraction,
  require full extracted-module coverage and retain real CLAP boundary checks.
- Native foreign pointers/teardown are unsafe boundaries → narrow audited
  adapter contracts, lifecycle/failed-creation tests and audio alloc/free sentinels.

## Migration Plan

Use a default-off VST3 build option and isolated install directory while proving
the candidate. Retain CLAP and its installed baseline. A failed candidate can be
removed from the proof search path without changing the CLAP product or accepted
instrument data. No existing user's plugin scan settings or saved project is
rewritten. Product integration follows the complete proof, not merely discovery.

## Open Questions

Task 1.2 resolved the Linux service prerequisite: factory-context `IRunLoop`
works on REAPER 7.84 without an editor; initialization context does not expose
it. Retain the factory-context service per processor. REAPER itself releases a
processor that advertises no controller class; processor-only operation remains
a dynamic-host obligation, while normal REAPER uses the separate controller.
Official validator is pinned to SDK v3.8.0 build 66.
See [qualification evidence](../../../docs/migrations/evidence/trigger-vst3-foundations-2026-11-03/README.md).

## Primary references

- [Accepted ADR](../../../docs/decisions/0001-direct-rust-vst3-adapter.md).
- [VST3 component/ownership/threading architecture](https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/API%2BDocumentation/Index.html).
- [Component connections may be proxied](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IConnectionPoint.html).
- [Automation curve and implicit-point contract](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IParamValueQueue.html).
- [Linux host run-loop contract](https://steinbergmedia.github.io/vst3_doc/base/classSteinberg_1_1Linux_1_1IRunLoop.html).
- [Binding source/scope](https://github.com/coupler-rs/vst3-rs).
- [Retained REAPER configuration/evidence](../../../docs/migrations/evidence/trigger-clap-daw-acceptance-2026-11-02/README.md).

DPI repair allowance, 23 November 2026: Dan reported the 200% desktop's
2400x1600 host parent containing only a1200x800 native child and approved
another one hour to implement and verify the DPI repair. The tranche starts
05:22:50UTC and ends06:22:50UTC; earlier storage caps, floors, preservation,
serial heavy-producer policy and mutation exclusion remain unchanged. This
repairs native host integration without changing the parallel UI redesign.
