# VST3 native DPI repair — 23 November 2026

Source checkpoint: `9631af4c5511b05a65a81fbddd3e5c6df2974e26`.
Workflow authority remains [issue #45](https://github.com/DanMarshall909/dandrum/issues/45).
This repairs the reported Linux editor canvas mismatch; the full Linux proof,
independent completion review and publication remain unfinished.

## Observed failure and resulting behavior

On the 192-DPI desktop, REAPER 7.84 reports content scale 1 and a 1200x800
`ViewRect`, then realizes its X11 embedding parent at 2400x1600 **after** the
size callback. The previous child stayed 1200x800, leaving three quarters gray.
The initial native parent was only 543x351, so attachment-time measurement alone
was insufficient. The final editor follows the uniform realized parent transform
on the existing host UI tick and fills 2400x1600 while retaining its 1200x800
logical layout and document.

Standard `IPlugViewContentScaleSupport` requests also work at 100%, 125%, 133.3%
and 200%. The retained host frame negotiates the new size: `getSize` remains at
the old accepted size until `onSize`, while constraints use the requested scale.
Invalid factors, refused/unacknowledged resize, recursive scale calls and a live
view without a frame preserve the previous geometry. Foreign-thread calls reject
before affine UI state access. Nonuniform/provisional native parent dimensions
retain the requested content scale instead of stretching the editor.

Physical bounds are rounded independently; fitting both axes prevents a final-row
clip at 133.3%. Cached bounds/GTK DPI avoid repeated native layout work on unchanged
host ticks. The shared owner retains the same browser, page counter and focus
through repeated scaling, visibility changes and a physical click at 200%.

## Verified current results

| Boundary | Result and retained run |
| --- | --- |
| Ordinary VST3 owner | 88 tests pass; `dpi-affinity-owner`, including scale/frame/constraint/refusal/reentry/foreign-thread assertions |
| Native VST3 | All 3 explicit native lanes pass; `dpi-proof-native`; 704 complete callbacks with 0 allocations/0 frees and 351 independently signed renders during actual editing |
| Shared native owner |100%, 125%, 133.3%,200%, rounded parent conversion, repeated reports, unchanged page state and real pointer input; `dpi-repeated-owner-gtk1` and `dpi-repeated-owner-gtk2` pass under GTK scale 1/192 DPI and scale 2/96 DPI |
| CLAP regression | Fresh copied CLAP/current assets pass actual React startup, native lifecycle, literal signed audio, recreation and residency; `dpi-proof-clap-native` |
| Static checks | Strict all-target VST3/shared Clippy and formatting pass; OpenSpec strict validation passes; canonical spec coverage remains 657 criteria / 418 mapped / 239 existing backlog |
| Normal source checkpoint | Installed `.githooks/pre-commit` runs 1096 engine tests plus strict engine coverage; `dpi-source-checkpoint` passes |
| Installed release | Fresh current release/native journey passes outside the sandbox; `dpi-installed-release-boundary` |
| Qualified installed/offline | Copied release executable, 1680x1280 owned Xvfb, checkout masked, unrelated cwd, isolated network namespace/no routes, host devices retained: `dpi-installed-pinned-source-hidden` passes the full native journey |
| Final actual REAPER | Exact final DLL mapping verified; parent/owned child/GtkPlug all 2400x1600; full editor paint confirmed from its own XComposite pixmap; fresh process restores all 23 named values with 0 normalized error |

[Candidate identity](candidate.json), [commands/timings/log hashes](checks.json),
[actual host observation](reaper-observations.json), [X11 hierarchy](x11-tree.txt)
and the before/after named-value TSVs retain portable text proof. The complete
361-file inventory is private and hashed by the candidate record. Its only file
changes from v5 are the native DLL and source identity; editor/instrument/notices
are identical. All dependencies resolve, with no JUCE requirement.

## Coverage and refactoring review

Fresh ordinary/native VST3 profiles use the same object hash and matching product
source; their combined export covers every counted region of the scale request
and size-constraint methods. Separate owning shared-editor profiles cover both
GTK DPI
representations, parent conversion, rounded fit and unchanged-presentation path.
Unexecuted native I/O-error conversions and defensive ownership/thread guards are
retained as explicit limits. This is not 100% whole-owner, branch/MC/DC or mutation
evidence. Mutation remains excluded.

No module was extracted. The existing shared native owner supplies the scaling
policy/presentation; VST3 owns frame negotiation and its UI tick. An initially
added direct-scale convenience API had no production consumer and was removed;
the tests now exercise the actual host synchronization API. Existing test tables
and native lifecycle cases absorb the new scenarios. Small copyable sizing state
uses Rust `Cell` on the UI thread; it is not shared with the audio callback.
The parallel Trigger UI redesign and vendored JUCE were untouched.

## Retained failures and scope limits

The first extended source-hidden variant used a 3200x2200 display, read-bound
executable/sentinels and allocator preloading. It failed while waiting for the
physically typed level 2. The same release passes outside that sandbox and in the
previously qualified copied-executable/non-audited 1680x1280 offline harness.
The cause of this variant difference is unestablished; no passing claim is made
for the extended variant. Both raw logs/identities are retained privately.

The full unfiltered pinned SDK validator still reports 535 pass / 2 fail (exit 255):
the all-parameter/every-frame inputs exceed the accepted 1024-total-point/note
bound. It is not a clean validator PASS, and capacity was not enlarged.

A task-local resource sampler raced a deleted temporary fixture and interrupted
one ordinary run. Its producer/children were checked stopped and its cache lease
proved exclusive before removing only the interrupted-producer marker. A bounded
three-attempt sampler retry was calibrated; the complete current owner rerun
passes. Disposable crashed-test copies and dead deterministic engine fixtures
were removed only after ownership/liveness checks; warm caches, candidates and
host replay cases remain preserved.

The new delta-spec scaling scenario is covered by the tests above, but canonical
spec synchronization remains part of the unfinished complete-product gate.
Earlier v5 two-instance REAPER render/state evidence remains tied to v5; this
single-instance DPI reload does not requalify that whole case for the final DLL.
Live 44.1k, admitted editor-absent work/feedback, the configured/256 MiB audit,
independent whole-candidate Astra review, publication and native Windows/macOS
milestones remain outstanding. No source/product binaries were pushed or merged.
