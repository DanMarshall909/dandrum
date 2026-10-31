# Installed Trigger native callback overlap

Scope: OpenSpec `add-juce-free-trigger-beta-products` task 7.2 only. Work began
31 October and verification continued 1 November 2026 (Australia/Brisbane).
This adds a maintained acceptance host/test and CTest registration, with no
production code, dependency, public behavior or canonical-spec change. The
starting source/upstream was `f988aa0f4f6bc8d4eeac7b5cef91c685ec8f7176` on
`feat/juce-free-host-adapters`. The final candidate and review/publication record
are in shared repository metadata at `.git/codex/trigger-native-overlap-20261101/review.md`.

## Boundary and executable evidence

The registered `trigger-clap-installed-native-overlap` test dynamically loads an
external installed CLAP into two independent instances. Its two JACK clients use
the existing PipeWire server's process callbacks and connected native MIDI ports.
MIDI notes 36 and 48 arrive at offset 3 every half second; the host continuously
renders both instances, including blocks without events. It neither starts nor
reconfigures the audio server and connects no speaker/device ports.

The real X11 session receives XTest mouse/keyboard input through `xdotool`:
React Add source, GTK file chooser path/Open, Voice level drag, and native-history
Undo/Redo. JavaScript and GTK access only observe controls, snapshots and widget
geometry; they do not synthesize clicks, select files or mutate product state.
The host keeps GTK dispatch active while sending input. Native hover is checked
before clicking, and mouse release is checked on success with an unwind guard.

| Task/acceptance clause | Executable proof |
| --- | --- |
| Installed native selection, actual callback overlap, accepted public edit | `installed_native::installed_native_edits_overlap_server_audio_and_midi` and its isolated allocator child: external artifact, real server MIDI, native chooser import, level 1 to 0.5, balanced ordered parameter-3 Begin/Value/End through final shutdown. |
| Signed selected playback, history and editor independence | The same test removes the original WAV; checks literal kit -0.5, selected mono mean -0.125 plus two independently resampled frames in both channels; destroys the entire child/WebView, proves it gone, renders with no editor, recreates with a new attachment and unchanged YAML, resets both real audio endpoints, and proves Undo silence/Redo selected audio at retained level 0.5. Every observed note onset, including allowed old/new transitions, is checked. |
| Independent instance | Every note onset in instance B retains kit -0.5 and unmapped selected-note silence; its serialized state remains byte-identical after A's edits and view replacement. |
| Callback allocation/ownership safety | Calibrated per-thread allocation/free interposition brackets every complete CLAP process call and both resets on the actual server thread. Callback allocations transfer to explicit raw ownership before registration; successful JACK deactivation precedes reconstructed Box ownership and reclamation. Continuous progress and a 500 ms operational watchdog detect stalls. |
| Held view activity, failed creation and stale-view cleanup | Complementary registered native cases `parameter-note-destroy`, `parameter-creation-failure`, `parameter-timer-rollback`, `parameter-continuous-audio-audit`, `parameter-two-instances` and selected recovery cases run on the same maintained product. The new overlap case finishes its drag before destruction; it does not replace their held-note/held-gesture assertions. |

These map the active delta scenarios “Native editing survives complete editor
replacement” and “Native callbacks retain bounded ownership during replacement”,
with existing component proof for failed creation and selected-content ownership.
No canonical scenarios were edited or synchronized; 7.3 and final beta closure
remain open. Existing foundation proof is indexed in
[the selected-foundation audit](../trigger-selected-foundation-completion-2026-10-29/README.md).
The earlier physical-selection result in task notes remains valid; it was not
repeated merely to obtain another human approval.

## Reproduction and artifact binding

The existing moved build cache was backed up and its old worktree path replaced;
normal CMake configuration regenerated build files without deleting caches.
Configuration used `DANDRUM_ENGINE_ONLY=ON`, `DANDRUM_CLAP_PROOF=ON`,
`DANDRUM_TRIGGER_CLAP_EDITOR=ON`, `DANDRUM_TRIGGER_EDITOR_NATIVE_X11=ON`,
`CMAKE_BUILD_TYPE=Release`, the current `web/trigger/dist-native` bundle, and
`DANDRUM_TRIGGER_NATIVE_INSTALLED_CLAP=/tmp/dandrum-7-2-native-installed/lib/clap/DandrumTrigger.clap`.
From the feature worktree:

```sh
/home/dan/.local/bin/cmake --build build/engine-only --target dandrum-trigger-clap-product --parallel 1
/home/dan/.local/bin/cmake --install build/engine-only --prefix /tmp/dandrum-7-2-native-installed --component trigger-clap
DISPLAY=:1 XAUTHORITY=/run/user/1000/gdm/Xauthority GDK_SCALE=1 GDK_DPI_SCALE=1 CARGO_NET_OFFLINE=true /home/dan/.local/bin/ctest --test-dir build/engine-only --output-on-failure -V -R '^trigger-clap-(native-|installed-native-overlap$)'
```

The installed product SHA-256 is
`8cdd23f5023f7873729d837dd011d90012380ebf320900b09cd9088b98bb591e`, identical to
the accepted desktop embedding correction. `installed-manifest.json` binds all
20 installed files; the editor `app.js` is
`d157a01be9cc121d3e310b2b4ca099cddb36d355328d45170d02e406d6fa918e`.
The production frontend is unchanged since its accepted foundation snapshot.
`parameters-binding.json` separately binds the maintained product, resources and
Cargo-selected acceptance executable. `raw.tar.gz` retains exact compiler
invocations, CTest commands/output, setup attempts and checks; `raw-sha256.txt`
identifies the archive.

## Independent review correction

The first independent review rejected checkpoint `73cc221ff16185bd0ed188925c48d864109c0bf7`:
although heap addresses were stable, deriving C callback pointers and subsequently
moving their owning `Box` values did not satisfy Rust's documented aliasing contract.
The repaired fixture uses `Box::into_raw` before registration, retains a private
raw-allocation guard, permits main-thread access only before activation or after
successful quiescence, and reconstructs each Box only for reclamation. Failed
initialization retains RAII cleanup; failed deactivation still aborts before any
callback owner can be freed. This is a test-host ownership correction, not a plugin
change. The original review and replacement candidate are retained in the existing
review record. Final evidence below is superseded by the repair qualification when
noted; the earlier raw archive is preserved.

## Initial qualification before the ownership repair

Pre-review registered native run: **34/34 CTests passed in 123.91 seconds**; the installed
overlap case took 9.34 seconds. PipeWire/JACK supplied 48000 Hz, 1024-frame blocks.
The two-instance host completed 385 callbacks and received 17 MIDI events, with
zero reported callback failures, two audited endpoint resets and one ordered,
balanced public gesture. Fully contained callback/MIDI counts were 36/1 during
chooser/import, 48/2 during public editing, and 69/2 during destruction/absence/
recreation. Each accepted phase proved both the kit and selected-note outcomes.

Packaging/native-driver/static-callback checks passed 3/3 in 3.01 seconds. Changed
test-target strict release Clippy, adapter formatting, strict active OpenSpec
validation, spec coverage (631 scenarios / 392 mapped / 239 existing ratchet), and
diff whitespace checks pass. Normal commit coverage and push checks are retained
in the review/publication record; they are not inferred from the native results.

An earlier all-native run exposed the diagnostic-output/parser regression above;
the next passed 34/34 in 126.50 seconds. The final test-host main-loop correction
was then requalified by the complete 34-case run. One GTK
`WIDGET_REALIZED_FOR_EVENT` warning remains during the chooser transition in the
final log (earlier versions emitted two). Its cause is not conclusively diagnosed;
all chooser state, sound, destruction and cleanup assertions passed. No warning-free
GTK or general GTK defect-freedom claim is made.

## Repair verification status

The raw-ownership repair passes strict Clippy. Its latest complete CTest attempt
passes all 33 other native checks; installed overlap fails before the first click
because the graphical desktop is locked. `loginctl show-session 34` reports
`Type=x11`, `Active=yes`, `State=active`, `LockedHint=yes`, and GNOME's
`org.gnome.ScreenSaver.GetActive` returns true. Browser queries and real server
callbacks continued during these failures. Window-activation synchronization and
fresh motion did not overcome the lock; these failures are not product RED evidence.
The user has been asked to unlock the desktop for the remaining exact acceptance
run. No unlock/authentication bypass is attempted. Task 7.2 is reopened pending
that qualification and renewed independent review.

Two earlier full-suite attempts each timed out in a different unchanged component
case (invalid stopped flush, then parameter flush). The former passed unchanged
on an isolated rerun in 1.17 seconds; both passed in the latest 33-case component
result. Their causes remain undiagnosed; all attempts are retained in `repair-raw.tar.gz`, with the ready executable bound
by `parameters-binding-repair.json`. They are not claimed fixed by the raw-ownership change.

## Interpretation and limits

This is real native-server and X11 input evidence, not named-DAW automation,
physical MIDI hardware, DAC/listening, hard-realtime scheduling or public release
qualification. The PCM oracle covers onsets and three selected waveform frames,
not every sample. The allocation sentinel does not detect every lock, syscall or
browser operation: unchanged production ownership remains separately supported
by bounded adapter translation, fixed handoff capacities, prepared graph rendering
and off-audio retirement in `trigger.rs` and `trigger-host/src/runtime.rs`, plus
existing ownership tests. The static callback guard has its own narrower scope.

No production behavior changed, so there is no new product RED claim. Early
failures were test-host setup, input routing or oracle evidence (including rejection
of unchanged gain when 0.5 was required), not missing production behavior. An
intermediate shared logging change made an existing audit's output parser reject a
passing child; restoring its original mode fixed that regression. Review refinements
check release success and validate the complete gesture stream after teardown.
Strict Clippy found one collapsible conditional in the new fixture; it was repaired.
No new production module was extracted; the stable test-only JACK ABI wrapper is
local to this boundary, and existing sample/allocator helpers are reused.

Mutation remains excluded by recorded user direction. This does not close the
existing strict-engine-Clippy baseline or full-owner coverage/dependency/licence
audits reserved for task 8.1. Reusable build, Cargo and untracked adapter targets
are preserved. Storage admission passed with about 27 GB free against an 18.25 GB
requirement; the existing retained-cache estimate is over budget and supplies no
deletion authority. Temporary samples and allocator probes are RAII-owned; clients
are stopped before their callback owners are released.
