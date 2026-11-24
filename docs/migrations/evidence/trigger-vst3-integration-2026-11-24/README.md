# Experimental VST3 native editor integration — 24 November 2026

Dan requested integration of the existing `work/complete-vst3-linux` commits
into `main`. This increment preserves the opt-in experimental VST3 product.
[Issue #45](https://github.com/DanMarshall909/dandrum/issues/45) and OpenSpec
`prove-trigger-vst3-linux` remain open at 13/17 tasks. No complete Linux,
Windows/macOS, new installed-binary or unrestricted validator acceptance is
claimed, and canonical specs are neither synchronized nor archived here.

## Review repairs

Independent review of `051bd9ae` found two host-reentrancy defects. Both have
behavior-first regression evidence and local checkpoint commits:

- `a2a62436`: a host terminating the processor inside `registerTimer` and then
  returning `kNotImplemented` previously made initialization falsely succeed.
  The existing service test now covers successful and unsupported registration,
  including refused unregister followed by cleanup retry. The common lifetime
  check rejects initialization and preserves required registration cleanup.
- `444c3aed`: copying the host frame's COM pointer during DPI negotiation
  previously called foreign `AddRef` while holding a Rust `RefCell` borrow.
  Reentrant `setFrame(null)` aborted the process. The view now snapshots a local
  `Rc` reference before calling the host. A raw standard-ABI frame fixture
  reproduces the old abort and proves reentrant frame removal, accepted 2400x1600
  geometry and final release. Existing scale/refusal/lifetime cases still pass.

The service RED returned success where failure was required. The frame RED
reported `RefCell already borrowed` and SIGABRT. Their GREEN owner suites pass
7 service tests and 6 view tests. No module was extracted. The frame repair uses
the existing controller ownership convention; the service test extends its
existing table rather than adding a duplicate journey.

## Native fixture diagnosis

The first fresh production-editor run failed physical level entry. Passive
trusted-event tracing reproduced the value `12`: the earlier mouse click was
still being dispatched when Ctrl+A was sent, and the caret changed from the
preselected range to its end before the trusted `2` arrived. The old predicate
could accept text selected earlier by JavaScript without proving Ctrl+A delivery.

The fixture now waits for a trusted completed click on the intended active input,
then trusted Ctrl+A keydown/keyup plus a nonempty full selection before physically
typing. The signed audio, balanced gesture and exact entered-value assertions
remain intact. Bounded input traces are included on future DOM failures.
`document.hasFocus()` was false even while trusted keys reached the embedded
input; it is diagnostic context rather than the readiness oracle.

A separate diagnostic run observed valid asynchronous selection status `choosing`
immediately after the GTK response. The bounded wait now permits `choosing` and
`preparing`, still requiring terminal `ready` and every existing decoded-content,
resource identity, deletion, Undo/Redo and signed-output assertion.

## Verification and retained limits

Product source is `444c3aed7edac103b5af2f9021bd722613726a60`; the accompanying
commit adds only fixture/evidence changes. [Verification receipts](verification.json)
retain commands, exit codes, timings, log hashes and the coverage object identity.

| Check | Result |
| --- | --- |
| Current VST3 ordinary owner | 89 passed; the 3 native journeys run separately; parent-only deadline/audit children remain parent-owned |
| Current native X11/browser/production journeys | All 3 pass; production passes twice after the fixture repair |
| Final audited native overlap | 682 complete callbacks, each 0 allocations/0 frees; 340 independent signed renders during actual native editing |
| Strict VST3 all-target Clippy | Pass with `trigger-editor` and `-D warnings` |
| Normal source checkpoint hooks | Both repair commits pass all 1096 engine tests and strict engine coverage |
| OpenSpec and canonical coverage | Strict change validation passes; 657 criteria / 418 mapped / 239 existing ratchet backlog |
| Source-bound coverage | 36 current ordinary/native profiles exported against the identified current DLL; all counted regions of the changed lifetime guard execute; scale method 63/63 counted regions execute |

The source-bound export does not establish 100% whole-owner, branch/MC/DC or
mutation coverage. Processor initialization retains unrelated uncovered regions.
Mutation remains excluded. Formatting and whitespace checks are also required
before the final checkpoint.

The earlier shared-editor and CLAP native evidence remains linked in the
[DPI record](../trigger-vst3-dpi-2026-11-23/README.md). All 12 recorded product
hashes and 12 raw log hashes matched the original candidate at this session's
start. Subsequent production changes touch only VST3 initialization/frame
ownership; shared editor, engine, CLAP and canonical React source are unchanged.
Regenerated native editor assets match all 15 recorded installed editor files.

The full pinned validator's retained result is still 535 pass / 2 capacity-limit
failures, not a clean PASS. Earlier installed/offline and actual REAPER evidence
is tied to its original binaries. The repaired source has not been repackaged or
requalified as a new installed REAPER binary. Full configured/256 MiB audit,
remaining REAPER acceptance, whole-product review and native Windows/macOS
milestones stay open. This integration review cannot close tasks 5.2 or 6.1–6.3.

Raw RED/GREEN/native/coverage logs are retained locally under
`.git/codex/vst3-integration-20261124/`. Prior evidence and reusable caches are
preserved; unrelated untracked `mockups/` is untouched. The exact integration
review verdict and publication result belong to issue #45 and the local handoff.
