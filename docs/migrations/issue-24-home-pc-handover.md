# Issue #24: home-PC handover — 14 October 2026

This is the historical handover snapshot. See [current implementation evidence](juce-replacement-progress.md#final-headless-clap-proof--15-october-2026) for the resumed Linux proof and remaining publication/migration work.

Continue [issue #24](https://github.com/DanMarshall909/dandrum/issues/24), the full JUCE replacement around the existing Rust engine and canonical React UI. **The migration is incomplete.** The independent engine build and owned runtime transfer are published and verified. The first headless CLAP rendering proof works, but its final release-profile verification and acceptance publication are unfinished.

This checkpoint deliberately preserves the unfinished implementation alongside this document so the home PC can resume from the current code. It is not a completed migration, release, merged implementation or approved production framework selection.

## Branch and preservation

- Fetch **`feat/juce-free-host-adapters`** from [GitHub](https://github.com/DanMarshall909/dandrum/tree/feat/juce-free-host-adapters). The handover commit contains this document, the CLAP source/tests/lockfile, opt-in CMake changes, pending OpenSpec plan and compact evidence.
- Investigation and scope correction are integrated into `main` at `68dd6553a2f07e71e2fe09ee1d7094d518559937`. Main remains the JUCE comparison product. The implementation branch has not been integrated into main.
- Last fully published prerequisite head before this checkpoint: `27a2dff2eeed7aebbcc7ca6e346f82d2803c3706` (acceptance checker recognizes external Rust adapter tests). Earlier material changes: independent build `fcd8eaf`, compiler-checked runtime transfer `677dfd0`.
- Source machine: WSL/Linux. Primary `/home/dan/code/personal/dandrum` is clean on main; task `/home/dan/code/personal/dandrum-worktrees/juce-free-host-adapters`; investigation checkout `/home/dan/code/personal/dandrum-worktrees/juce-replacement-investigation` is clean at the main revision. Preserve all existing worktrees/builds. No stashes or open PRs were found during the handover gate.
- Shared ledger on source machine: `/home/dan/code/personal/dandrum/.git/codex/deferred-work.md`, item `DD-20261014-JUCE-FREE-HOST-ADAPTERS` remains OPEN for this incomplete work. Git does not transfer that ledger: this document carries its continuation state to the home PC. Reconcile/create the same item in the home repository's actual Git common directory after applying its active-work gate; do not replace an existing ledger.
- The root agent took over the CLAP files after runner capacity errors. No compiler/test process remained at handover. The completed final retained-mode guard is 4/4 passing; do not restart a supposedly pending old process.

## Decisions already made

Dan instructed: “Integrate the investigation into main, then continue #24”; later “free some space then carry on”; now “push up a handover doc to continue on my home pc.” Continue this same work, preserving the checkpoint. Do not infer permission to merge this unfinished implementation into main or remove comparison products.

Keep **J0–J6 and Gates A–C** from the [investigation](../investigations/juce-replacement-investigation.md) and issue. Do not narrow #24 to its investigation or CLAP prototype. No legacy JUCE state/projects/IDs compatibility is required because no product has shipped. Replacement identities, versioned state and new-state round trips remain required. Mutation testing is excluded for now by explicit user instruction; normal tests, signed oracles and acceptance mapping still apply.

[Issue #23](https://github.com/DanMarshall909/dandrum/issues/23) owns the single persistent instance session/journal, typed editor SDK and EditorHost contracts. #24 consumes them: do not introduce another musical session or undo history. Release platforms/formats, project/distribution licence and ASIO requirements still need Dan's decision. Retain current Windows/macOS/Linux and AU/VST3/Standalone obligations until explicitly revised.

## Evidence at this checkpoint

| Work | Verified result | Limit |
| --- | --- | --- |
| Independent build | Fresh source fixture without JUCE/web/UI built all independent native targets; signed stereo ABI/MIDI checks; all 18 CTests passed in 106.56 s, including 1,088 Rust tests. Retained JUCE configuration inventory passed. | Build prerequisite, not replacement devices/editors/formats. |
| Owned runtime transfer | Genuine E0277 RED, then safe `Send` bounds on actual erased executor/filter owners. Three public signed filter/gain ownership/reset/instance tests pass. All 18 CTests passed in 67.42 s, including 1,091 Rust tests. | Exclusive ownership transfer, not concurrent `Sync`; no unsafe Send, lock or DSP change. |
| CLAP behavior | Real exported binary dynamically loaded: 12 host tests and 2 public signed baselines pass. Includes offset/order, root controls, literal signed TB-303, reset/reactivation/two instances, 384/768 kHz and fractional rates, bounded malformed lists, queue admission and held-note continuation. | Latest small manual formatting edits still need final source rebuild; no complete callback allocation/I/O proof. |
| CLAP packaging | Actual absent-JUCE/web/UI source build guard 2/2 passed in 252.646 s; packaged release binary and its dynamic CTest passed. | This first artifact predates restoration of matching release LTO/codegen settings and the last build-boundary test registration. Recheck final source/configuration. |
| Retained build selections | Latest default/explicit-disabled/independent/comparison guard 4/4 passed in 16.987 s. | Configuration/inventory regression, not native DAW or device testing. |
| Official CLAP validator | 0.4.1, both descriptors, no exclusions: exit 0, 52 successes, 28 skipped, 1 warning, 0 failures. | Initial release artifact only. Gain denormal processing measured 5.08× slowdown; state/GUI and other absent extensions are skipped, not proved. |
| Specs | Last published canonical validation 48/48; map 600 scenarios / 361 mapped / 239 unchanged existing todo entries. | Four new CLAP scenarios remain delta-only, unsynced/unmapped. Do not mark their acceptance complete yet. |

Portable raw results are in [evidence/issue-24-handover-2026-10-14](evidence/issue-24-handover-2026-10-14/). The validator JSON records every success/skip/warning. Its tested initial release artifact was 15,782,608 bytes, SHA-256 `45ee428519b4f9e4d23ad81dcabf159fa1cbc0762532266de4bec3a96f382780`; the binary itself is not committed. Rebuild on the home PC. Source-machine `/tmp` and `build/` evidence paths are historical and do not transfer through Git.

The [dependency receipt](juce-free-clap-dependencies.json) pins Clack to `27ca28391b57a61bad4bb5099288a7bf9247350d`, records lockfile hashes and upstream licence provenance. Linux normal production graph: engine 112 packages, adapter 117 (one local adapter plus four external format packages). Test host alone uses libloading with Clack host default features disabled. No framework dependency was added to the engine crate. This is provenance, not complete distribution clearance.

## Resume on the home PC

First read the home checkout's AGENTS.md and its resolved Git common directory ledger. Inventory every registered worktree, full staged/unstaged/untracked status, tracking, stashes and live PRs; fetch and repeat the complete snapshot before mutating. Resolve any **additional** active work explicitly. The user's handover request authorizes preserving/resuming this same #24 task, not discarding home-PC work.

Use a native filesystem checkout for the active tools: WSL/Linux source for Linux builds, Windows checkout for native Windows tools. Do not copy `.git`, source-machine worktree metadata or symlinked target directories across hosts. If there is already a clean worktree on this task branch, reuse it. Otherwise, after the gate, leave the primary checkout on main and create an isolated worktree from the fetched task branch. Example from the primary repository:

```bash
git fetch origin
git worktree add --track -b feat/juce-free-host-adapters \
  ../dandrum-worktrees/juce-free-host-adapters \
  origin/feat/juce-free-host-adapters
cd ../dandrum-worktrees/juce-free-host-adapters
```

This example assumes the local branch/path do not already exist. Inspect and reuse existing task state instead of forcing creation. A fresh clone must fetch the task branch; checking out only main misses the implementation.

Read exact OpenSpec workflow context, then resume **`prove-headless-clap-rendering` tasks 3.1–3.4**:

```bash
openspec list --json
openspec status --change prove-headless-clap-rendering --json
openspec instructions apply --change prove-headless-clap-rendering --json
```

Read all returned contextFiles. Tasks 1.1–2.4 are checked with the scoped evidence above; 3.1–3.4 remain unchecked. First perform the final release build and actual dynamic tests on Linux/WSL:

```bash
cmake -S . -B build/engine-only \
  -DDANDRUM_ENGINE_ONLY=ON -DDANDRUM_CLAP_PROOF=ON \
  -DCMAKE_BUILD_TYPE=Release
cmake --build build/engine-only --target dandrum-clap-proof --parallel 2
ctest --test-dir build/engine-only --show-only
ctest --test-dir build/engine-only --output-on-failure \
  -R '^clap-(dynamic-host|build-boundary)$'
```

The current CLAP CMake packaging is explicitly Linux-only. A Windows host using WSL proves Linux behavior only. Native Windows/macOS packaging remains work. Ensure Cargo is at the repository's expected location; source machine uses `$HOME/.cargo/bin/cargo` and `$HOME/.local/bin/cmake`.

1. Verify matching `[profile.release] lto = true`, `codegen-units = 1` in adapter and engine; these are now restored in source. The initial release build accidentally used a different profile; do not reuse its validator receipt as proof of the final source. Reuse one task-owned Cargo target and bound jobs; clean neither installed dependencies nor reusable builds.
2. Run the pinned official validator on the **newly built** `build/engine-only/clap/DandrumHeadlessProof.clap`, without test exclusions or hidden in-process shortcuts: `clap-validator validate --jobs 1 --json <artifact>`. Download the receipt's exact asset if needed and verify its recorded SHA-256. Record full counts/skips/warnings, artifact hash, profile, command and timing. Resolve the denormal issue through proper TDD/architecture review before claiming complete realtime hardening; do not change signed DSP behavior just to hide a warning.
3. Sync the four proven scenarios from the delta into `openspec/specs/juce-free-clap-proof/spec.md`; map them to genuinely proving functions in `src/clap-adapter/tests/clap_dynamic.rs`. Review each fingerprint before updating. Expected additive totals are 49 canonical specs and 604 scenarios / 365 mapped / 239 existing todo entries, if no other work has changed the base. Run `scripts/check-spec-coverage`, strict scoped and canonical OpenSpec validation, and review the final diff.
4. Commit verified changes with a specific subject and Why/What/Verification/Constraints body; stage only this task. Push without force, verify exact live head, then update tasks/progress/ledger truthfully. The handover checkpoint does not complete task 3.4's verified-slice publication requirement.

A formatter component installation was rejected by automatic approval review on the source machine because of the storage constraint; partial formatting was done manually. Do not assume `cargo fmt` passed. If rustfmt already exists at home, use it for the scoped new crate and review the diff; do not reformat unrelated engine files.

## Required work after the rendering proof

| Stage | Still required |
| --- | --- |
| J0 | Release/licence/driver decisions and remaining signed-render, resource and performance baselines. |
| J1 | #23 minimum owned session/SDK/EditorHost contracts, protocol/IDs/event channels and comparison-consumer adaptation. |
| J2 / Gate A | State/stopped flush and callback hardening; actual parented production React gesture/feedback, event-loop/lifecycle/failure/input/two-editor evidence; early VST3 projection and second **native** OS. Headless audio alone does not close Gate A. |
| J3 | Deliberate stable replacement IDs and bounded versioned state; invalid-state recovery, fresh-instance state/automation/assets and new-project round trips. No legacy JUCE importer. |
| J4 / Gate B | Covered JUCE services/FFT replacement; single #23 session/journal, destructive editor/undo, lost acknowledgements, reader reclamation and job recovery for Trigger and a small instrument. |
| J5 | Required VST3/AU/Standalone validators and real DAWs; drivers, physical-device settings/hotplug/recovery/chrome/MIDI. |
| J6 / Gate C | Realtime/event/reconfigure matrix, browser/input/security/resources, installed offline assets, licence/link/build audit with JUCE absent, maintained demos/CI/specs, fallback and verified cutover. |

Keep the engine independent and render callbacks free of locks, allocation, I/O, serialization, renderer access and reclamation. The current CLAP fixtures expose float32 stereo and MIDI 1 key semantics only; no CLAP note-ID/expression, MIDI 2, per-note modulation, state or editor is advertised. Parameters target public root controls, not child compiled slots. Integer preparation metadata is rounded for these resource-free fixtures; actual fractional rate reaches the floating-point runtime. Asset resampling is not proved.

## Storage and local state

Approved cleanups removed only Windows npm `_cacache`, uv/pip download caches (about 7.99 GiB observed relief), then exact native NuGet HTTP cache `C:/Users/DanMarshall/AppData/Local/NuGet/v3-cache` (about 2.03 GiB). Installed NuGet global packages, tools, sources, logs, branches and all reusable build outputs were preserved. No WSL shutdown/compaction was authorized or performed.

Physical Windows C: was about **1.91 GiB free** at the last recorded post-build reading, while WSL reported hundreds of virtual GiB. This is a historical reading, not home-PC capacity. Check the actual host volume before compiling; pause new source-machine compilation below 1 GiB. Fresh home builds need substantial additional capacity. The source task's Cargo debug cache was about 5.6 GiB, with release artifacts also retained.

Source cache links `build/engine-only/rust-target` and `build/engine-only-regression/cargo-target` point into the task-local `build/engine-only-regression/engine-clean/rust-target`. These are local optimization details, not portable paths to copy. Build a home-local cache. Do not delete reusable outputs/dependencies without exact target approval and a rebuild-cost explanation.

All Gates A–C remain OPEN. Resume from the checked-in source and this evidence; do not mark #24 complete from the handover or initial validator exit code.
