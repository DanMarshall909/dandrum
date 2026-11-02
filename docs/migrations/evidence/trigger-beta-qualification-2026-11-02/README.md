# Trigger beta qualification evidence — 2 November 2026

This packet supports task **8.1** of `add-juce-free-trigger-beta-products`.
Independent completion review and its exact publication envelope remain the
completion gate. Task **8.2**, the final audit of every retained beta requirement,
remains open. This packet makes no overall beta-complete claim.

Dan selected **“Keep the beta private/local for now.”** The installed package,
test executables and LLVM replay binaries remain local. The repository packet
contains source-related text, JSON profiles, reports and hashes. No first-party
distribution licence has been selected and this is not redistribution clearance.

## Qualified source and artifacts

The tested source checkpoint is `7ea453e2510fdd3c40f40248699356b2440bcf16`, tree
`27931fd3c9ba889b35302318268ffdb006470f14`, based on
`c3e636cd345bba4162369e41b69fb8c9dd824170`. Later evidence/task receipt changes
must preserve those production, test and configuration bytes.

The maintained CMake product was rebuilt with the current native editor bundle
and installed outside the checkout. Its CLAP SHA-256 is
`da9fc60fa566ce77e882bf584cd5c9a00a905fa0929438374badc5ef04478186`.
[installed-manifest.json](installed-manifest.json) binds all **20** installed
files. The source/cache-hidden offline checks rehashed them after execution;
every file remained unchanged. The package requires the recorded GNU Linux
x86_64/X11/GTK3/WebKitGTK environment. It is not a self-contained Linux binary.

## Changes and behavioral evidence

- FFI output getters and waveform buckets initialize caller storage with raw
  `ptr::write`, avoiding references to output-only uninitialized storage. Tests
  exercise `MaybeUninit`, null outputs, signed samples and atomic preservation
  on invalid requests. All 74 unsafe exports now state caller obligations.
  Focused Miri passes before and after: **characterization, not RED evidence**
  that the former documented-contract mismatch was detected. The final-source
  Miri test also passes.
- Rejected gesture rollback previously omitted the host gesture end. A genuine
  failing regression now passes: the end is attempted once, the last accepted
  value remains, Undo remains retryable and the UI reports the error. Unused
  alternate velocity-drag handling was removed after characterization.
- Unix seed replacement previously changed permissions through an external hard
  link. The new failing regression passes with permission clearing restricted
  to Windows. ZIP 9 filename handling is adapted, and explicit header-consistency
  validation rejects mismatched local/central headers before replacing a version.
  A non-UTF-8 rejection hypothesis was discarded: CP437 names are valid.
- Covered Rust refactors simplify declaration/voice preparation and keep large
  transient Reverb/Script state behind owned boxes during preparation. Binding
  moves their contents into existing executors; no new callback allocation is
  introduced. Signed echo/nonfinite-control and filter-constructor impulse
  characterizations protect the numerical behavior.

`ModulePackageError::Kernel` now carries `Box<Diagnostic>` instead of an inline
`Diagnostic`. Repository callers compile; an external Rust caller constructing
that public variant would need to adapt. No external source-compatibility proof
is claimed. Thirteen reasoned function-level Clippy argument-count expectations
preserve split borrows/reference oracles; no correctness lint is waived. Existing
coverage exceptions moved only with their unchanged statements/reasons.

## Verification

| Gate | Observed result |
| --- | --- |
| Maintained native CMake configuration | Engine-only, CLAP proof, Trigger editor and native X11; mutation OFF. Full build and local install pass. |
| Complete configured CTest set | **71/71 pass**, 694.05 seconds. Includes source-free build boundaries, strict all-target Clippy, notices, native lifecycle, callback audits and the real maximum-payload slow lane. [Per-test results](ctest-results.json). |
| Rust engine | **1,096** library tests, three runtime ownership and five selected-snapshot integration tests pass under final LLVM capture. Normal strict-coverage hooks and doctest checks pass. |
| Graphical adapter ordinary suite | 23 library, 19 proof dynamic, two public baselines and 20 Trigger dynamic tests pass with explicit current product paths. Ignored native/child/manual cases remain identified; available native cases run through CTest separately. |
| JavaScript | **314** Trigger unit tests; authored type/component checks; normal and native production builds; disabled-capture checks; maintained native-adapter and React-shell wrapper entry points pass. |
| Browser | **43/43** pass in the canonical wrapper. Every selected test has a run-bound receipt. Actual JSX/Unicode calibration proves called/uncalled callbacks, a missed alternative and an unimported owner. |
| Installed native overlap | Passes with actual XTest input, native chooser, audio/MIDI callbacks, independent instances, signed PCM and callback allocation audits. This is automated native input, not a new physical-device or named-DAW run. |
| Installed offline isolation | **Three** headless cases and **one** real React/Wry lifecycle case pass. Bubblewrap hides source/Cargo/cache/original temporary paths, unshares network/PID namespaces, uses synthetic `/dev`, and runs an owned isolated Xvfb. Namespace identities and exact commands are retained. |
| Official validator 0.4.1 | Unfiltered installed-library run: **92 successes / 25 skips / one warning / zero failures** across its three descriptors. Trigger plus library checks: **34 successes / 10 skips / zero warnings or failures**. The warning is a **2.41× denormal timing ratio on experimental signed-gain**; it remains unremediated and is not hidden by exclusions or a clean-performance claim. The headless proof run has 63 successes / 18 skips / no warning or failure. |
| Specs | Strict selected-change validation passes. Canonical mapping check: **639 scenarios, 400 mapped, 239 existing ratchet entries**. No scenario text or fingerprint was changed to silence the gate. |

The initial raw adapter invocation failed because its default product path did
not exist; the explicit-product full rerun passes. An unsupported LLVM flag
combination was rejected before its slow test; the corrected invocation retains
and merges ordinary plus slow profiles. Raw failures and corrections remain in
the packet. Mutation remains excluded by user direction; no mutation job ran.

## Coverage scope and limits

[Engine](engine-coverage-summary.json): **36,156/38,654 lines, 93.54%**, full
default test target capture with CLI source files excluded. This includes useful
test-only legacy/parity code and additional emitted library instantiations.
It is a different scope from the earlier library-only 94.57% baseline; those
percentages are not a before/after regression measurement. Unhit functions and
generic instantiations remain visible. No Rust branch or MC/DC instrumentation
was enabled.

[Host](host-coverage-summary.json): **4,347/4,442 lines, 97.86%**, ordinary public
tests plus the actual maximum portable-state slow test. The slow test adds a
real size/resource boundary assertion without increasing line coverage. Native
functional execution is separate from these LLVM percentages.

The browser provider measures **React/JSX only**: **1,525/1,555 mapped original
lines** across all **71** selected product owners, with none never loaded.
Thirty missed mapped lines remain in AssetBrowser, ConfirmDialog, ModulatorList,
ValueDialog, WaveformEditor and ZoneLane. The obsolete asset-heading branch has
no maintained producer; modal focus, mapping menus and waveform-handle paths
remain explicit test/refactoring candidates. In particular, ValueDialog's
focus-order selector omits the supplied numeric field's initial focusable
spinbutton element; its unsupported native slice-remapping boundary is not
qualified by this packet.

Vite's development plain-JavaScript import rewriting did not provide trustworthy
original-column maps. Those **69** inventoried owners are outside this browser
profile. Node V8 measures its separate `.mjs` unit-test scope and has remaining
gaps in menus/projections and mock-only operations. Do not combine those
numerators into whole-frontend coverage. Generated/HMR range starts and raw
Istanbul attribution remain separate; original-source function/branch coverage
and native WebKit coverage are not established. Empty/truncated V8 evidence,
foreign/missing receipts and invalid/stale maps have calibrated rejection tests.

## Dependency, licence and environment limits

ZIP is **9.0.5** in all four relevant Rust locks; host/adapter rtrb is **0.3.5**;
Trigger Vite is **6.4.5**. Unrelated sampler/TB303/OpenCode work and PR #25 are
untouched. A complete OSV lookup covers **363** unique crates.io versions plus
the exact shared Clack commit, with pagination and known-vulnerable controls;
five first-party/path identities are explicitly skipped. No matching ZIP/rtrb
record was returned. npm audit reports zero advisories for the final Trigger
lock. This is catalogue evidence, not absence-of-vulnerabilities proof.

Remaining Rust findings are affected GLib 0.18.5 and maintenance notices for
proc-macro-error, smartstring and unsafe-libyaml. Inspection of 204 normal/build
packages and 6,678 Rust source files found no supported production source caller
of GLib's vulnerable string iterator. Exact source hashes and caller reasoning
are retained; generated expansions and a complete machine call graph were not
proved. Installed native GLib, WebKit/JavaScriptCore and Soup have the named
advisory/version limitations in the dependency packet. No OS update, ESM change
or whole-system exploitability conclusion is claimed.

The current notices pin 204 Rust identities, four JavaScript runtime packages
and three font notices. Available Rust/JavaScript licence-text hashes are checked
against source, including smartstring's `LICENCE.md` spelling. A corrupted digest
previously passed; the new gate diagnoses it. Five Rust archives still contain
no top-level licence text; Rust/JavaScript text is not installed separately from
the JSON inventory, while font OFL text ships. These are recorded limitations,
not legal compatibility or redistribution clearance.

Primary references and complete package/version observations are retained in
`dependency-system-review/` and `osv-current-20261102/` inside the archive:
[ZIP advisory](https://github.com/zip-rs/zip2/security/advisories/GHSA-m665-v7pm-7grj),
[rtrb advisory](https://rustsec.org/advisories/RUSTSEC-2026-0274.html),
[Vite advisory](https://github.com/vitejs/vite/security/advisories/GHSA-8g5g-mxqh-grgj),
[OSV query contract](https://google.github.io/osv.dev/post-v1-querybatch/).

## Retention and replay

`raw-text-and-profiles.tar.gz` and [archive-manifest.json](archive-manifest.json)
retain checked text/report members, raw scoped Node counters, per-test browser
captures, source inventory, test/tool identities and current command receipts.
Every archive member is rehashed after creation. Earlier exploratory reports
are supplemental; no claim depends on an unavailable early raw attachment.

Private files under `.git/codex/trigger-beta-qualification-20261102/` retain the
exact LLVM objects, raw profiles, merged profdata, source snapshots and installed
native hosts. The [engine](engine-final-replay-receipt.json) and
[host](host-final-replay-receipt.json) replays verify every counter and source
location. Cargo adds metadata and reserializes derived percentages, so JSON byte
equality is explicitly false; each percentage is checked against its equal
integer ratio with absolute tolerance `1e-10`. The installed private archive's
[receipt](installed-replay-receipt.json) verifies every member. Replay requires
the pinned LLVM/native tools, not a publicly distributed beta binary.

The owned native supervisor stopped its desktop and proved no remaining workers;
scratch was removed. Disposable archive-verification extractions were removed.
The managed cache retired the inactive pre-update generation under its existing
64 GiB retention policy; the current warm generation and installed dependencies
remain. No manual shared build/cache deletion was performed. Reusable Miri tools
remain isolated in the task directory. The rustup manager auto-updated
1.29.0→1.29.2 during that isolated installation; stable Rust 1.96 remains default.

Historical physical-input and REAPER 7.84 evidence remains bound to its original
package/configuration in the preceding packets. It is not a fresh named-DAW or
physical-device claim for this hash. Task 8.2 must reconcile those boundaries,
unsupported capabilities and all retained requirements before declaring the
CLAP beta complete. VST3, AU, standalone devices and other host/OS configurations
remain outside this qualification.
