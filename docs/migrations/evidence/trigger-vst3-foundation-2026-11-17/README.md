# Direct Rust VST3 foundation

Dan requested a foundation-only merge on 17 November 2026 and moved the remaining
Linux proof into [issue #45](https://github.com/DanMarshall909/dandrum/issues/45).
He is connected remotely and cannot test REAPER now. This is an experimental
source checkpoint; actual native editor and installed-host acceptance remain
required. The OpenSpec change stays open, with seven tasks unchecked and no
unproved scenarios synchronized into the baseline specs.

The completed boundaries are recorded in the earlier source-bound receipts:

- [Host/binding/service qualification](../trigger-vst3-foundations-2026-11-03/README.md),
  [shared command handling](../trigger-vst3-shared-commands-2026-11-03/README.md)
  and [dynamic discovery](../trigger-vst3-discovery-2026-11-03/README.md).
- [Signed audio](../trigger-vst3-audio-2026-11-03/README.md),
  [invalid extents](../trigger-vst3-extents-2026-11-03/README.md) and
  [callback ownership](../trigger-vst3-callback-ownership-2026-11-15/README.md).
- [Shared curves](../trigger-vst3-common-curves-2026-11-03/README.md),
  [VST3 timing](../trigger-vst3-curves-2026-11-03/README.md),
  [stopped feedback](../trigger-vst3-stopped-feedback-2026-11-15/README.md) and
  [host gestures](../trigger-vst3-host-gestures-2026-11-15/README.md).
- [Processor state](../trigger-vst3-processor-state-2026-11-03/README.md),
  [controller restoration](../trigger-vst3-controller-state-2026-11-15/README.md)
  and [shared native editor orchestration](../trigger-vst3-shared-editor-2026-11-16/README.md).

Subsequent local checkpoints add affine view metadata, disposable standard-message
view connections, an owner-preserving shared automation peek and processor-side
view/control automation routing. One shared ledger remains authoritative. Each
connection has its own delivery counter; acknowledgement matches the role and
complete `(owner, event)` head. Cancellation removes only the departing owner.
A transport-level guard spans the actual foreign automation call, including
queued retries and inline acknowledgement, before allowing teardown.

Runtime RED evidence caught missing view routing, reentrant termination and a
queued reply-pump retry bypass before their repairs. The owner-peek API's initial
RED was compiler/API absence, not an executed behavior failure. Final routing
source passed 80 native-feature adapter tests in both ordinary release and LLVM
lanes, 74 headless tests, strict both-feature Clippy and format/spec checks. Three
ignored subprocess entries are exercised by marker-checking parent tests. The
normal commit hook passed 1,096 engine tests and strict engine coverage.

These browser-lane tests use a fake receiver; they do not prove real child
IComponentHandler acceptance or balanced native host gestures. The default
adapter has no native view, and the opt-in metadata view's `attached()` returns
failure. There is no actual VST3 browser client, GTK/Wry child parenting/paint or
native lifetime qualification yet. The existing CMake/JUCE VST3 products are
separate. Full current-product validator, installed/offline REAPER, native Undo
and combined product review/publication/archive remain in issue #45.

[verification.json](verification.json) indexes checkpoint identities, exact final
producer receipts, coverage limits and the UTF-8-only diagnostic archive. Reused
evidence remains bound to its actual source and products; test totals are not a
whole-owner coverage claim. Compiled products and raw LLVM objects/profiles stay
private. Foundation publication has its own combined review gate; earlier local
checkpoint approvals do not authorize the complete Linux product claim.

Factory creation relies on the SDK's
[UI-thread lifecycle contract](https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/API%2BDocumentation/Index.html#vst-3-threading-model).
The sequential foreign-thread rejection tests prove their named entrypoints;
they do not promise arbitrary concurrent host calls are safe. The split's current
cheap checks pass strict OpenSpec validation, the existing scenario map and all
34 cache-driver contract tests. Test fixtures clean up their temporary caches.

The maintained build policy restores the ordinary 8 GiB growth reservation for
cold-build safety. The earlier 3 GiB warm-only allowance depended on private
checks and is historical. Preserve reusable caches/dependencies and retain the
mutation exclusion. Windows/macOS are mandatory subsequent native milestones.
