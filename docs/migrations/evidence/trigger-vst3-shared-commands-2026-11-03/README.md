# Shared Trigger command routing before VST3 consumption

Task 2.1 of `prove-trigger-vst3-linux`, 3 November 2026. CLAP now calls the
format-neutral `dandrum_trigger_host::editor_commands` implementation for editor
commands, parameter replies and shared error policy. The existing controller,
session/history, public values, portable state and runtime remain the owners.
Native chooser admission and format callbacks remain at the adapter edge.
This is a shared-code checkpoint, not a completed VST3 instrument.

The old dispatcher was first measured with 22 passing tests. An added
characterization covering selection/capability/unknown-job replies passed with
unchanged production behavior (23 tests), preserving state and signed playback.
Thirteen generic behavior tests moved beside the shared owner, with their
fixture reused by the remaining CLAP tests. Moving compile-time fixture paths
and an accidentally removed test import caused two setup failures; both logs
remain separate from behavior evidence. This refactor does not claim an
artificial RED for new product behavior.

The initially unchanged extraction passed all 13 tests and measured 66/69 lines
and 103/108 regions. Refactoring consolidated existing input-error policy and
fallible-result handling. A `Reply` variant retains an already encoded
snapshot/job response; the other carries a value for bounded encoding. The
final source-bound measurement executes **81/81 lines, 114/114 regions and 8/8
functions**, with the behavioral assertions still green. Branch and MC/DC
instrumentation are unavailable in this run; execution coverage alone is not a
claim that every possible defect is rejected.

Current owning Trigger host tests pass **217** cases, with the existing slow
maximum-payload test intentionally excluded from the fast run. Current CLAP
editor unit tests pass **11** cases, with the intentional child-only FIFO case
ignored; the parent exercises that boundary. Both packages pass strict Clippy
for the selected owning targets with `-D warnings`. Formatting, whitespace,
strict selected OpenSpec validation and the unchanged canonical scenario map
(657 total / 418 mapped / 239 backlog) pass. Existing scenario test identities
remain valid after relocation; no VST3 scenario is synced or closed here.

The exact rebuilt CLAP product then passes **all 36 selected maintained boundary
CTests**, including actual production React/Wry creation, destruction/recreation,
public gestures/feedback, structural/history and selected-source operations,
independent instances, complete callback audits and installed/native overlap.
The native test group takes 178.14 seconds including preparation. The two
fixture binding files and `verification.json` identify the copied product,
reported test executables, resource hashes and source checkpoint. Private ELFs,
raw LLVM profiles, installed resources and screenshots remain local. The public
archive contains source/text evidence only.

The native supervisor runs one heavy producer at a time, uses an owned X11
session, checks available RAM/free disk and cumulative retained output, and
refuses cache admission that would retire preserved generations. All owned
children exit zero. Sampled memory stays above 18 GiB and free disk above
109 GiB. The plain build adds about 0.45 GiB and the editor product about
0.75 GiB; the native test/preparation phase adds about 1.75 GiB. These measured
increases show why the remaining initial 16 GiB allowance could not admit this
checkpoint. Dan approved 32 GiB output growth after reassessment; other limits
and all acceptance criteria are unchanged. Reusable caches/dependencies remain.

`verification.json` indexes the results and production/test refactoring decisions;
`text-evidence.tar.gz` retains raw failures, commands, logs, coverage and native
receipts. No mutation testing, new REAPER/VST3 acceptance, Windows/macOS support,
independent completion PASS or publication is claimed by this checkpoint.
