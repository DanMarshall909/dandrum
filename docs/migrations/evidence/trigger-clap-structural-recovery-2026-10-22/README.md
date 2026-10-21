# Native structural recovery checkpoint

This developer-only `trigger-editor` boundary extends published base
`072c1af49f3a4cc81117887541c70c5f3c3f5633`. It joins the existing persistent
preparation worker, mapping admission, retained status and native Undo to the
actual copied CLAP binary and unmodified production React bundle. The maintained
CMake product stays headless. No OpenSpec checkbox or canonical scenario is closed.

## Observable acceptance

| Criterion | Executed oracle |
|---|---|
| Admitted mapping outlives lost reply and complete view loss | Actual audio first has a pending restore occupying its capacity-one slot. Native mapping7001 is acknowledged but delivery to the browser broker is withheld. Snapshot before destruction is revision1/history0. After actual child destruction, host GLib service and real CLAP blocks publish once. Recreated view is revision2/undo1. MIDI36 is silent and MIDI60 is literal stereo -0.5. |
| Fresh attachment recovers retained outcome | Two fresh status requests return completed7001/revision2/history1; unknown7002 rejects. A calibrated getter watcher observes only these exercised paths and a deliberately stale mapping rejection. Independent history and signed audio remain unchanged. |
| Recreated production header invokes structural Undo | Real header `Undo Change Key Range (Ctrl+Z)` dispatches one authenticated native Undo7002 at revision2. It becomes `Nothing to undo`; retained outcome is revision3/undo0/redo1. MIDI36 returns literal stereo -0.5 and MIDI60 is silent. Final X11 child destruction is asserted. |
| Failed admission preserves state | Owner test rejects stale revision as `staleRequest` and stopped worker as `commandRejected`, preserving revision0/history0. Existing borrow/backpressure tests cover bounded deferral. |
| Maintained driver includes the scenario | Registration test first fails because `trigger-clap-native-parameter-structural-session` is absent; minimal CMake registration passes, then actual CTest executes it. |

The exact native Rust case is
`structural_native::copied_native_mapping_survives_lost_reply_and_view_loss`.
It reuses the maintained X11, public WebKit JS and real CLAP/audio helpers. Browser
observers are test-side only; no private product extension or altered app asset.

## Behavior-first sequence

The author completed separate mapping, status-recovery and header-Undo cycles.
Each genuine native RED reports `capabilityUnavailable` at the intended missing
command after its preceding setup and audio assertions passed. Each GREEN uses
the same criterion, followed by owner checks, formatting, strict Clippy, focused
LLVM and refactoring review before the next RED. The final covered refactor
renames the shared error translator to `command_error` and extracts the scenario
body from its spawn closure. Exact phase logs, commands, environments and source
snapshots are retained in [author-raw.tar.gz](author-raw.tar.gz); the index records
50 source states and35 unique source objects, with content deduplication.

Initial DISPLAY-unset failure is setup only. An initial missing `--release` built
debug dependencies and briefly overlapped the intended release build; both ended
normally before a guarded stop found no live target, so no signal was sent. The
first actual native RED receipt omitted DISPLAY/GDK/XAUTH keys: later notes give
command attribution, not a claim they were originally captured. Subsequent
receipts capture them. A mistaken test expectation for stale revision was
corrected to the existing protocol's `staleRequest`; that correction is not a
production RED. Cargo-reported unchanged host ELF reuse and changed DSO provenance
are bound separately, without claiming one compiler invocation rebuilt both.

## Final verification

| Gate | Result |
|---|---|
| Author native regression | 12 outer cases, 12.218s; nested allocator child is additional process, not another outer test |
| Author exact owner tests | 13 pass; final fmt and warnings-denied Clippy for editor/default/plain Trigger pass |
| Root maintained native lane | 20 CTests pass, 21.109s: one Python driver, two Cargo preparations, 17 native outer cases (G1 five, G2 eleven, G3 one) |
| Root full host | 154 tests/doctests pass, 4.521s |
| Root copied-source CLAP boundary | All8 maintained cases pass, 135.177s; source fixture excludes JUCE/web/ui; only its build cache is redirected to the returned warm target |
| Dependency and linkage | Native/default/plain development graphs audited; actual copied native DSO, G1 host and copied-source default DSO contain no JUCE dependency; default ELF remains native-free |
| OpenSpec/spec coverage | Active change strict validation, all51 canonical specs, 618 scenarios/379 mapped/239 ratchet pass; no task, fingerprint or map edit |

Actual Cargo JSON identifies each copied DSO and test executable in
[root-actual-cargo-bindings.json](root-actual-cargo-bindings.json). Final normal
DSO SHA256 is `a32e20c3684b24e951d68621424a97e03cafce49ebe7620ca83bfa99f6b7a127`;
structural host ELF is
`d207969f8cddc38f8b4c02bf38a81fe504a95784c55af35e1768621c8ec3be9e`.
Production app SHA256 is
`4b9ce5a356fea5f22703f9046c77a80ac616c00242348a12fe81f58fe4f760f2`.
Full editor tree and instrument YAML/WAV hashes are independently matched.

The calibrated actual-plugin callback audit observes positive allocation and
free controls, then0 allocations/0 frees during33,342 callbacks, including343
whole-callback/service overlaps and32,972 post-DOM/context callbacks. This is
callback-safety evidence, not uninterrupted destroy/recreate audio acceptance.

## Coverage, ownership and limitations

[Root reconstruction](root-final-coverage-assessment.json) rehashes exact final
ELFs, profiles and raw LLVM exports. Across native and owner runs, distinct source
regions execute: structural service24/24, worker wake5/5, error translation5/5,
status6/6, mapping/Undo admission6/6 and worker shutdown1/1. Startup executes6/7;
the OS thread-start error closure is unforced. Existing Core-drop attachment
fallback is not exercised by these final selections. Raw duplicate generic and
uncalled copies remain visible. The owner run's27 profile files include listing
processes; they are not27 tests. These are source-region execution claims, not
branch, every-instantiation, whole-GUI coverage or assertion proof.

A worker shares only `Arc<AtomicBool>` wake state. Main-thread `RefCell` borrows
remain local and defer when busy. Active audio alone owns its render endpoint;
only an endpoint returned by CLAP deactivation is serviced quiescently. The weak
GLib source survives view destruction; final Core destruction cancels it and joins
the worker before dropping controller state. No new production module or package
is introduced.

Tasks3.2/3.3/4.2/4.3 and the full beta remain open. Native heartbeat/expiry,
held-note/gesture cleanup, injected creation failure, continuous lifecycle audio,
two independently edited native instances and physical pointer qualification
remain unproved here. The fractional-rate validator failure is unwaived. VST3,
standalone audio/MIDI, named DAW and installed offline products remain later gates.
Mutation is explicitly excluded. All reusable caches and the known untracked
Runtime cache are preserved; main stays clean at68dd655.

## Portable raw material

[author-archive.json](author-archive.json) and [root-archive.json](root-archive.json)
record archive hashes. Both archives were read back member-by-member and compared
byte-for-byte. [root-raw.tar.gz](root-raw.tar.gz) includes root registration RED/GREEN,
source integration, actual CTest/Cargo preparation, copied-source logs, linkage,
strict spec checks and setup qualifiers. Neither archive includes compiler caches,
ELFs or raw profiles; their immutable external copies and hashes remain retained.
Independent review and normal-hook publication are separate live-tracker receipts.
