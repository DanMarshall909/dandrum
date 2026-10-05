# Automatic structural rebuild acceptance tests

Accepted behavior: an admitted structural edit immediately mutes this plugin,
hands engine ownership off safely, validates/rebuilds off audio, and automatically
resumes the edited or last working configuration. No draft, Apply, confirmation
or unapplied-changes workflow is involved. The DAW transport continues.

These are acceptance test specifications for the pending implementation in
[tasks.md, section 9](tasks.md). None of S1–S13 is complete. The ownership
and automatic reload prerequisites below have executable evidence; these reload
tests alone do not prove typed automatic structural editing. Implement each case
through RED → GREEN before claiming it complete, and map proving tests in
`spec-tests.map` when syncing these delta specs.

## Fixtures and observation

- Reuse the real processor, shared command service and renderer adapters. Do not
  substitute a mock rebuild state machine for the behavior under test.
- Use distinguishable prepared stereo constant fixtures already established in
  `PluginConstructionTest.cpp`: working output `+0.25`, edited output `-0.5`.
  Separately reuse `tests/fixtures/plugin-ui-knob.yaml` and its public
  `fixture.level` control for live values (`-0.5`/`+0.5` left, zero right at
  normalized values `0.25`/`0.75`). Assert signed samples, not just nonzero audio
  or equality between two paths.
- Add deterministic test barriers at worker validation/preparation and callback
  engine-reader ownership. Run the callback separately and prove it completes
  while the worker remains stalled. Timeouts are test failure guards, not the
  engine's ownership protocol; do not use sleeps to infer that readers exited.
- Prefill every output channel with a nonzero sentinel before muted callbacks;
  assert every sample is exactly zero. Instrument engine entry, allocation,
  blocking synchronization, preparation, joins and resource destruction with
  thread identities. A zero buffer alone does not prove callback safety.
- Record configuration/document generation, source/region/map metadata and host
  parameter object addresses, IDs, count/order and public-ID-to-slot mappings
  before the edit. Observe them again after success and every recovery path.
- Exercise both native and Web adapters for command/status/UI cases. Keep real
  DAW transport and multiple-instance evidence in the host integration lane.

## Cases

### S1 — Edit admission immediately mutes

Send one supported structural edit from each adapter and hold validation at a
barrier. Assert asynchronous job acceptance, rebuilding state and plugin mute
before releasing validation. Subsequent callbacks must complete with exact
silence. No second Apply, confirmation or reload command is sent. Repeat for a
stale generation and unsupported edit: rejection must leave the working state
and audio untouched.

Proves: **Edit starts a rebuild without an additional action** and the renderer
scenario **Supported structural interaction rebuilds automatically**.

### S2 — A stalled rebuild cannot block audio

Keep preparation stalled and process repeated small, normal and oversized
blocks, with nonzero prefilled outputs and incoming host/editor notes. Assert
all channels are zero and no callback enters an engine, allocates rebuild
resources, waits, joins, prepares or destroys anything. Release the worker only
after the callbacks have completed. Buffered notes must not be replayed as an
unbounded backlog after resume; held notes and tails need not survive.

Proves: **Preparation is stalled while callbacks continue**.

### S3 — An existing engine reader prevents unsafe retirement

Hold a callback after it has acquired the working engine, admit an edit, and
allow the off-audio coordinator to proceed. Assert that the old engine remains
valid and is neither mutated nor destroyed while its reader is held. The
callback may finish its already-started block. Release the reader and assert
acknowledged handoff and off-audio retirement. Vary the reader delay beyond the
old fixed-delay assumption; run with memory-safety instrumentation where supported.

Proves: **A callback already owns the previous engine**.

### S4 — No future callback is needed to rebuild

Admit an edit with no active engine reader and issue no callbacks while the job
finishes. Assert automatic activation and unmute, then process the first block
and observe the known edited signed fixture. The coordinator must not wait for
a callback that the host might never issue.

Proves: **Rebuild occurs without further host callbacks**.

### S5 — Success publishes the edited configuration and resumes

Release successful preparation. Assert the edited configuration and owned
prepared metadata, a new generation, re-enabled structural controls and automatic
unmute. Render `-0.5` from the edited fixture and a new mapped sampler note.
Retained old documents/analysis inputs must remain safe; stale results must not
replace the new document. No overlap, continuity or state migration is expected.

Proves: **Successful rebuild resumes the edited instrument**.

### S6 — Preparation failure restores signed audio and reports an error

Parameterize the real failure boundary: invalid configuration, missing sample
asset and rejected graph compilation. Observe the initial muted interval, then
automatic recovery to the original YAML/document/generation. Assert `+0.25`
from the working fixture and a fresh mapped sampler note, visible queryable
failure status, and enabled structural controls. Submit a valid edit immediately
afterward; it must be accepted without a draft reset or user confirmation.

Proves: **Edited configuration cannot be prepared**.

### S7 — Startup and activation failures cannot leave permanent mute

Use narrow fault-injection seams for worker startup and candidate activation.
Also change the host's prepared sample rate/block settings while a candidate
is pending so that it cannot activate under obsolete settings. Assert a
terminal failure, resumed working audio at the current host settings,
unchanged authoritative configuration, re-enabled controls, and off-audio cleanup
of partial candidates. Do not convert a setup failure into claimed recovery proof.

Proves: **Worker startup or activation fails**.

### S8 — One rebuild, no queued edits

Hold the first job pending and submit a second edit from another session through
both adapters. Assert a busy rejection, one worker, no deferred configuration
and no later second activation after releasing the first job. Both editors must
disable structural controls during rebuilding and restore them after either
success or recovery. Supported live parameter controls remain enabled.

Proves: **Another structural request arrives during rebuild** and the renderer
scenario **Rebuilding disables structural controls only**.

### S9 — Host identities survive; incompatible surfaces recover

Compare all recorded host parameter objects, IDs, count/order, slots and
automation bindings after successful, failed and repeated compatible edits.
Drive a previously bound slot and assert its known signed output. Parameterize
incompatible edits adding/removing/renaming/reordering exposed parameters or
changing their ranges; each must report incompatibility, recover and leave
the original host surface intact.

Proves: **Compatible structural edit retains automation identities** and
**Edit would change the exposed host surface**.

### S10 — Ordinary parameters stay live, including during rebuilding

Change ordinary cutoff/volume or `fixture.level` through host automation and
both adapters. Assert the expected signed render, no mute, no structural job
and no generation change. Then change a value more than once while structural
preparation is stalled. On both success and recovery assert the latest host
value reaches the resumed engine, with the same host gesture/slot binding and
without a second structural rebuild.

Proves: **Ordinary parameter changes while playing** and
**Automation changes while a rebuild is pending**.

### S11 — Editor lifetime does not own recovery

Close the requesting editor while preparation is stalled and reopen an editor.
Query the same processor-owned job and rebuilding state, release the worker
and observe automatic success/recovery. Separately destroy a processor during
pending work from a non-audio thread; assert safe worker shutdown, no dangling
engine/sample access and no joins/destruction on an audio callback.

Proves: **Editor or processor closes during rebuild**.

### S12 — DAW transport and another instrument keep running

In a real VST3 host, run transport and two plugin instances with distinguishable
audio. Trigger a structural edit and hold its worker long enough to observe the
mute interval. Record advancing host transport, uninterrupted callbacks and
audible signed output from the other instance. Observe automatic resume and
repeat with a failing edit. Processor-only checks supplement this host run;
they cannot prove DAW transport continuity by themselves.

Proves: **DAW and another instance continue playing**.

### S13 — Structural UI has no local unapplied configuration

In both actual renderer runtimes, submit a supported structural edit through its
normal control. Assert immediate rebuilding state, disabled structural controls
and no draft mode, Apply, confirmation or unapplied-changes indicator. On failure,
show the error and working prepared values; on success, show the new prepared
values. Unsupported reference callbacks must leave the configuration untouched.

Proves the renderer scenarios **Supported structural interaction rebuilds
automatically**, **Unsupported prepared component interactions are restricted**
and **Rebuilding disables structural controls only**.

## Owning executable suites

Extend `tests/cpp/PluginConstructionTest.cpp` for processor/host-slot and signed
render contracts, `tests/cpp/PluginEditorBridgeTest.cpp` for shared command
admission/rejection and Web transport, and
`tests/cpp/NativeEditorSmokeTest.cpp` plus the real WebView runtime for UI state.
Extract a focused structural-rebuild suite if concurrency setup would obscure
the existing suites; register it with CTest before claiming executable coverage.

### Ownership prerequisite implemented separately

Linux CTest cases `cxx-plugin-engine-handoff-baseline` and
`cxx-plugin-engine-handoff` use the real processor and Rust engine. They cover
normal reload, host reprepare and state restore, before structural commands are
connected. The held-reader test calls the actual renderer, then stalls its
return beyond the former five-millisecond delay. Replacement must wait off
audio and must not request destruction of that engine. Later 1/64/512/2048-frame
callbacks clear four sentinel-filled lanes to exact zero, without engine render
entry or observed C++ allocation, directly linked mutex acquisition, or FFI
preparation/destruction. Explicit reader release allows exactly one retirement;
resumed output changes from literal `+0.25` to `-0.5` and retains the fixture's
host parameter object/count. Quiescent replacement needs no further callback.

These tests supply part of S3/S4's ownership evidence. They do not submit a
structural edit, hold structural validation, prove autonomous jobs or recovery,
cover every allocator/OS operation, load a DAW, or complete S3/S4. The test's
held-render sleep is a deterministic observation seam, not production handoff
or audio timing evidence. Focused coverage and deliberate retirement and
callback faults are recorded in [live-analysis.md](../../../docs/design-system/live-analysis.md).

### Automatic reload prerequisite implemented separately

The registered `cxx-plugin-automatic-reload` and
`cxx-plugin-reload-job-contracts` lanes extend the real processor fixture with
admission mute before actual validation, autonomous completion without further
callbacks/editor polling, exact-zero stalled callbacks, held-reader startup
failure, obsolete generation/host settings, throwing activation recovery and
pending-worker shutdown. Signed audio and stable host object identity are
asserted; a held activation notification proves coherent working metadata after
failure. The original Web bridge lane holds the admitted worker and proves
running/failed status across close/reopen. The
[reload record](../../../docs/design-system/automatic-reload.md) retains focused
current coverage, historical ASan/23 calibrated component faults, two current
replacement-phase faults and full build/test results.

These regressions advance prerequisites of S1–S4/S6–S8/S10–S11 through the
existing reload command. They do not submit a supported typed structural edit,
complete renderer disabled/error reconciliation, reject incompatible structural
surfaces, prove gated-note backlog reset or establish DAW transport continuity.
None of S1–S13 or section 9 is marked complete.

Review repair regressions additionally prove partial host/raw/saved-value recovery
with recursive/concurrent/same-value automation, synchronous retained working
metadata and state, rejection of recursive ownership mutation, non-standard
exception recovery, terminal failed/running replacement status, and next-valid
activation. These remain prerequisites through
the reload command; they do not complete typed structural acceptance cases.
