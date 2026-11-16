# Trigger VST3: binding and host-service qualification

Task 1.2 of `prove-trigger-vst3-linux`, 3 November 2026. This is prerequisite
ABI/service evidence, not a completed Trigger product or REAPER acceptance.
Dan approved the complete plan and 12 active execution-hour reassessment cap,
one heavy producer, four compiler jobs, 16 GiB incremental outputs plus 2 GiB
scratch, 8 GiB available-RAM/free-disk floor and mutation exclusion. Reusable
builds/dependencies remain retained. The existing local workflow tracker owns
execution accounting and current status.

`qualification.json` pins the inspected `vst3 = "=0.3.0"` and
`com-scrape-types 0.1.1` archives/revisions, official SDK/validator and actual
REAPER executable. `src/vst3-adapter/Cargo.lock` preserves both registry
checksums. The adapter is only a dependency scaffold: no product factory or
musical implementation is advertised yet. No SDK generator runs at product
build time. The helper declares MIT/Apache licensing but omits root license
texts; that packaging gap remains explicit. Private binaries remain local.

## ABI and ownership

The independently compiled official C++ headers and pinned Rust bindings agree
on the measured sizes/alignment of 13 boundary structures, four critical field
positions, Linux result constants and eight interface IDs (see `abi-*.txt`).
This is Linux x86_64 ABI evidence; it does not establish other platforms or
validate arbitrary foreign memory. `probe-sources.tar.gz` retains the one-off
measurement/probe source and upstream MIT/Apache notices, without compiled
binaries. Published log copies strip trailing whitespace; unmodified raw logs
remain in the private proof directory.

The adapter must borrow input interface arguments with `ComRef`, retain them
with `to_com_ptr` only when needed beyond the call, and adopt returned references
with `ComPtr::from_raw`. `into_raw` transfers an owning reference to the host.
Rust's `Drop` releases a `ComPtr`; unlike a C# reference, its scope determines
when that release occurs. `ComWrapper` uses atomic reference counts, which do
not make GUI/session fields thread-safe or authorize their destruction on an
audio thread. The processor must explicitly clear main-thread resources on
termination; process/reset receive only the exclusively owned audio endpoint.

Official headers mark controller, connection notification and state methods as
UI-thread operations; process is on the processing thread and `setProcessing`
may run on either. Never hold a mutable owner borrow across a foreign call:
`IConnectionPoint` can be proxied and `notify` can return inline/reentrantly.
Use standard bounded messages, not private peer pointers or a global lookup.
The pinned SDK's actual `ConnectionProxy` exposes only standard interfaces and
forwards notifications on its checked UI thread. Required runtime/proxy/failure
and callback-safety tests remain implementation obligations.

## Actual native REAPER service

REAPER 7.84 Linux x86_64 revision `4b6323`, qualified X11/GTK configuration
`linux_ime=1`, isolated display/config/cache/plugin path and unrelated working
directory. The gain-derived contract probe uses separate processor/controller
classes and never creates a view. The successful
`reaper-service-cleanup-attempt-1` receives **212 timer callbacks**, all on the
same OS main thread as initialization (`tid == pid`). Factory host context
supplies `IRunLoop`; processor initialization context does **not**. Registration
and unregistration return zero; timer is released; REAPER and both owned X11
workers exit zero. The product must retain the factory-context service in each
processor independently of view lifetime.

The processor-only variant reports no controller class; REAPER releases that
processor immediately rather than retaining a sounding instance. This is a
host-policy limit, not evidence that a controller owns service. Real Trigger
processor-only initialization/state/render/service still requires a synthetic
COM-host proof. Normal REAPER acceptance uses the agreed separate controller.

Failed attempts remain separate: `reaper-attempt-1` used Windows-style
`vstpath64`; Linux uses `vstpath`, so it discovered no probe. The next attempt
proved editor-absent service but timed out at a disposable-project save dialog
after processor/timer teardown; its owned processes were stopped and receipts
retained. Diagnosing the dialog and handling native No/Still Evaluating input
produced the clean final probe. No private event loop was added to the plugin.

## Official validator and resources

Official SDK **v3.8.0 build 66**, commit and four submodule pins in the receipt.
The unmodified validator target builds with plugin/GUI/hosting examples disabled
and passes its **43 internal self-tests**. No validator tests are filtered out.
The first configuration enabled unrelated editorhost and failed on missing
`gtkmm-3.0`; the corrected official options retain the validator and avoid that
unused example. Logs preserve both outcomes. Actual Trigger validation remains
required once a candidate exists.

Initial admission observed roughly 130 GiB free disk and 18.5 GiB available RAM.
`initial-inventory.json` is a bounded logical footprint, with linked-entry and
worktree-discovery limitations explicitly retained; it is not a deletion receipt
or a physical allocation measurement. Shared cache and dependency observations
are separate from this task's incremental output allowance. The new named
`.ai-rules.json` workload is estimated and must be checked before producers.

References: [vst3-rs](https://github.com/coupler-rs/vst3-rs),
[official SDK pin](https://github.com/steinbergmedia/vst3sdk/tree/9fad9770f2ae8542ab1a548a68c1ad1ac690abe0),
[Linux host-context run-loop contract](https://steinbergmedia.github.io/vst3_doc/base/classSteinberg_1_1Linux_1_1IRunLoop.html),
[connection proxy contract](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IConnectionPoint.html).
