# Automatic muted reload foundation

This is partial implementation evidence for tasks 9.1–9.4 in the
[renderer-independent UI plan](../../openspec/changes/add-renderer-independent-plugin-ui/tasks.md).
It extends the existing instrument reload command, not the pending typed
structural editing commands. Section 9 remains unchecked.

## Processor-owned job

An admitted `requestInstrumentReloadJob` closes audio reader admission and mutes
this instance before launching work. One job runs at a time; another request is
rejected as busy, without a queue. The worker acknowledges existing callback
readers, validates/prepares the replacement, activates it or resumes the working
engine, and cleans discarded resources before reopening audio. Job completion
does not require an editor, status query, message loop or another audio callback.
Terminal records remain in the processor's bounded 16-job history.

Callbacks after gate closure clear every output and return. The worker may wait
for an already entered reader; the audio callback never waits for rebuilding or
joins a worker. Nested host reprepare and synchronous replacement still wait for
readers even when the asynchronous job already owns the closed gate. Reopening
after thread startup failure clears only the closed bit, preserving outstanding
reader acknowledgements.

Validation, preparation and activation failures resume the previous working
engine and configuration with a queryable error. During activation the candidate
remains owned by the worker until installation commits. Bindings and DSP are
prepared on the candidate while readers retain working metadata. If a host
notification throws, recovery restores working values without repeating the
throwing notifications. Watcher setup follows successful activation; an ancillary watcher
exception is reported on the completed job rather than claiming engine rollback.
Shutdown closes admission, joins the processor-owned worker off audio and keeps
the gate closed through cleanup. Standard and non-standard activation exceptions
both finalize recovery; shutdown waits for completion without retrieving an
exception result from the worker future.
Recovered activation failures also terminate the maintained replacement-phase
query as `failed`; the next successful installation reports `running`.

Off-audio configuration accessors return owned strings/files under the reload
lock. Synchronous notification readers see the committed kernel, metadata and
bindings, with captured working values or the latest ordinary host writes.
Recursive reload, state restore, preparation and preset calls are rejected during
activation. Returning a C++ value
keeps the caller's snapshot valid after the configuration is replaced; returning
a reference would only borrow the processor's mutable storage.

The fixed normalized host parameter keeps float bits and an ordinary-write
revision in one lock-free atomic word. A rebuilding-thread token is consumed
before notification, so recursive host writes—including identical numeric
values—remain distinguishable from candidate writes. Failed activation repairs
the real APVTS raw mirror; saved state copies authoritative values even when a
throw prevented APVTS notification or a reentrant save already flushed its tree.
Parameter objects, IDs, count/order, normalized range, default, resolution and
text conversion retain their established contracts.

## Executable evidence

The registered Linux component lanes `cxx-plugin-automatic-reload` and
`cxx-plugin-reload-job-contracts` use the actual processor and Rust kernel.
Preparation/render barriers hold real FFI operations rather than substituting
mock engines. Assertions cover:

- admission mute before validation, busy/stale admission, bounded history and
  persistent failure status;
- exact cleared zero in sentinel-filled 1/64/512/2048-frame callbacks while
  preparation is held, and independent signed audio from another instance;
- autonomous activation and invalid-YAML recovery with literal signed samples
  `-1` and `+1`, including host values written during preparation;
- an actual callback reader held beyond admission: validation and retirement
  wait for its explicit release; later callbacks access no engine;
- thread startup failure while a reader is held, retaining signed `+0.25` audio;
- pending worker shutdown witnessed at its actual off-audio join and cleanup;
- newer instrument generation and changed host rate/block settings rejecting
  obsolete candidates;
- throwing activation notification recovery, signed `+0.25`, and a later valid
  job producing `-0.5`, with coherent `failed`/`running` replacement phases;
- a held, failing activation notification with concurrent configuration
  observation returning the restored working identity/YAML;
- five two-control range-change failures, preserving recursive, same-value,
  concurrent and early automation, raw/saved values and literal signed samples
  `+0.25`, `+0.5`, `-0.75` and `+0.0625`, followed by valid reloads;
- synchronous retained YAML/file/document/parameter/state reads, recursive
  ownership rejection, a real valid preset, and subsequent committed metadata;
- integer/custom activation exceptions with terminal errors, recovery,
  readmission and safe processor teardown.

The existing original Web editor test holds the first admitted worker before
validation. It proves busy rejection deterministically, running-state recovery
after close/reopen, failed-job status and subsequent kick-to-303 reload. The
construction test retains its normal success/failure reload cases; the old
timing-based stale-candidate arrangement is replaced by the registered component
test with a held real validator and a completed newer replacement.

Focused gcov executes **214/214 changed executable processor records**, selected
from the diff against `421367f`. It does not establish whole-processor or
exhaustive branch coverage. Before the final status repair, AddressSanitizer
instrumented the owning processor and component fixture; reused Rust/JUCE
archives were not ASan instrumented. That exact round2 candidate's retained
twenty-three copied-source faults target admission mute, reader count, automatic resume,
shutdown gate, generation/settings rejection, validation before acknowledgement,
activation rollback, host-write revision/token handling, raw/saved values,
staged metadata, all five recursive mutation guards and non-standard exceptions. These are component checks,
with end-to-end browser/DAW tests excluded from mutation runs.
Two current-source copied faults additionally omit the failure and success
terminal phases; each fails its named recovery assertion. The historical ASan
and 23-fault runs are not relabelled as reruns of the final status repair.

Complete native and Web builds include both products' Standalone and VST3
targets. CTest passes **36/36 native** and **66/66 Web**, with no skips. Strict
OpenSpec validation and the unchanged implemented-baseline scenario map pass.
The Web lane includes the actual packaged TB-303/sampler runtime, bridge,
construction, host surface and existing live-consumer stress regressions.

Raw commands, source copies/hashes, compiler/link maps, outputs, terminal statuses,
gcov data, fault diffs and retained failures live in the task evidence directory
`/tmp/dandrum-automatic-reload-evidence` (initial foundation) and
`/tmp/dandrum-reload-repair-evidence` (first review repairs), plus
`/tmp/dandrum-reload-status-repair-evidence` (replacement-phase repair).
Production and test refactoring removed
redundant prepared-result fields and separated fast job behavior from the larger
construction scenario, shared the covered surface publication steps, and reused
the activation-failure fixture across exception kinds. No production module was extracted.

## Corrections and evidence limits

Three product REDs exposed admission without immediate mute, erased outstanding
reader count after startup failure, and failure reporting after candidate engine
installation. A further RED exposed uncommitted configuration metadata. Each
was observed at the real owning boundary before its production fix. Review
repairs add product REDs for partial host-value rollback, synchronous candidate
metadata publication, non-standard exceptions stranding mute and a stale
`muted` replacement phase after audio recovery. The phase assertions reuse the
existing standard/integer/custom failure fixture and its next-valid activation.

Retained setup/harness failures are separate: an early fixture declaration-order
compile failure; an exact-zero assertion corrected after explaining the literal
IEEE negative zero produced by a negative gain on a silent channel; and a GNU
thread-join wrapper alignment error identified by ASan. The latter needed even
alignment because member-function pointers use their low bit for virtual
dispatch. These are not product RED or successful fault detections. Muted
callbacks still require positive cleared zero in every lane. Repair evidence
also retains a const-cast compile error, fixture line-ending normalization and
the positive zero produced by summing a negative zero into a second gain's
zeroed input. Fault-harness corrections distinguish an earlier preparation
failure from retirement and remove an accidental unconditional return; all 23
intended faults are independently confirmed by their named assertions.

A full parallel Web build exposed both plugin targets generating the same
JUCE subprocess helper files concurrently. The repository's Linux browser
build now orders the sampler target after the first plugin target. The same
two-job full build passes; vendored JUCE and native-only dependencies are unchanged.

This foundation preserves the existing reload command's ability to replace an
instrument with a different public surface. Compatible typed structural edits,
strict exposed-surface rejection, renderer rebuilding/disabled/error states,
gated-note reset/backlog tests and complete coordination of other preparation
paths remain pending. Legacy synchronous reload, file watching and state restore
still prepare through their existing paths; ownership mutations serialize with
the job, but this is not one global preparation coordinator. Preset application
now serializes with worker replacement; its existing direct kernel updates are
not a new lock-free preset delivery system.

These controlled component/browser checks do not prove DAW transport continuity,
callback timing, full structural acceptance S1–S13, complete sampler layouts or
tasks 6.3/6.4. No baseline specifications, fingerprints or scenario-map todos are
changed by this increment.
