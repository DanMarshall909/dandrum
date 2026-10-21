# Two independently edited native Trigger instances — 22 October 2026

This partial task 4.3 checkpoint adds one copied-release characterization case
and its maintained registration. Production is unchanged; no task or beta closes.
One shared Entry factory creates two instruments, each with its own actual X11
parent, unmodified React view, host ledger, controller and exclusive audio worker.

A uses physical level 0.5, mappings 36→60→62 and two structural undo entries.
B uses physical level 2, mapping 36→61 and one entry. Actual rendered scalar edits
produce each instance's balanced Begin/Value/End and normalized feedback. Parameter
edits leave structural history untouched. Native mapping setup is followed by
view reopening to initialize React from accepted history, before pinning B's
active baseline. Composite instance/attachment authority is checked; attachment
strings need not be globally distinct.

The oracle parses the copied PCM16 mono 48 kHz WAV independently of the engine,
pins original samples -16384 and -358, and checks all 12,000 kick frames on both
channels plus region-end silence. A's literal anchors are [-0.25,-179/32768] at
offset 5; B's are [-1,-716/32768] at offset 3. Outputs start with sentinel values.
Mapped and foreign/old-key probes check signed audio and silence independently.

B's original worker then emits note 61 every 64 blocks of 256 frames, without
reset, reactivation, GUI replacement or waiting on A's phase. Every callback
checks the advancing waveform. A invokes actual header Undo, restores its earlier
accepted 60 YAML/saved bytes and undo1/redo1, recreates its child, resets its live
voice on its worker, and stops/joins/deactivates/reactivates at maximum 512 frames.
The replacement worker actually processes 512-frame silence and a fresh mapped
waveform. After every intervention, B preserves its accepted document, history,
physical/normalized values, saved bytes, composite attachment and live child.
Both endpoint returns match their own plugins; children disappear before parents
and the shared Entry. Recoverable failure cleanup retains both stop/join paths.

Bounds are setup 15 s, shared active 20 s, outer 60 s, at most 2,000 callbacks per
instance across A's reactivation, 32 total commands, and at least 10 ms pacing.
The 500 ms gap/duration watchdog is a synthetic stall detector, not a physical
audio-device deadline. Real callback/API intervals are retained; no universal
synchronous-call overlap, hardware or global framework quiescence is claimed.
Queue operations and test receipts are outside process; this adds no allocator
claim beyond the separately calibrated callback checkpoint.

Verification and identities:

- Author: 20 native cases, five GUI cases, 13 fast tests, formatting and strict
  all-target Clippy pass. Final ordinary exact case passes in 9.016 s. Earlier
  characterization-only, compile-borrow, missing-display/socket and stale React
  setup failures retain distinct source/log bindings; none is product RED.
- Root: all 28 maintained CTests pass in 43.978 s, with exact Cargo artifact and
  complete copied resource bindings. This is one fast driver, two preparations,
  25 outer native cases and two nested allocator children. Registration has genuine
  missing-label RED/GREEN; three affected guards and five-artifact linkage pass.
- Root's actual case records A 549×256 then 38×512 callbacks, with host frame start
  carried from 0 to 140544. B remains at 590×256. All receipts have no mismatch or
  failure, with maximum measured gap below 10.75 ms. Forty operation intervals
  are retained separately from callback intervals.
- Root independently hashes two LLVM ELFs, two profiles and tools, merges/exports
  without warnings and reproduces equal parsed data. New test coverage is 729/762
  lines, 1294/1346 regions and 46/49 functions. Malformed-fixture, worker failure,
  timeout/mismatch/cleanup and timing-dependent pending paths remain explicit
  gaps. No full-owner or branch-coverage claim follows.
- Final test sources are aa586418/66eff974; ordinary test ELF 70d43594, ordinary
  DSO 17251daa, Cargo-test DSO e9200fbb and unchanged React app 4b9ce5a3 are distinct
  identities. Instrumented test 9084632a is newly built; production DSO 37381100
  instrumentation is reused only with unchanged source/compiler/extern bindings.

Indexed byte-verified archives retain source states, commands, logs, compiler and
LLVM bindings. The maintained resource-policy PNG is a source fixture; compiled
binaries, raw profiles and reusable caches stay external at their exact hashes.
Root hashes all 965 author files and verifies 1,766 preserved Runtime cache entries.
Prior eight-case headless source-free evidence remains qualified reuse of 76
matching inputs plus one deliberately removed copied asset, not a fresh native
product build. Strict active-change/all 51/spec-coverage gates pass with the
618/379/239 mapping baseline unchanged.

Lost-ack recovery with an unchanged second instance, live-editor invalid-state
feedback, the maintained GUI-enabled product/source-free seam and validator
compatibility assessment remain separate gates. Later VST3, standalone, native
input/sample import, named-DAW and installed offline beta work remains open.
Review/publication receipts belong to the sole live tracker. Main remains clean
at 68dd655; all caches are preserved and mutation remains excluded.
