# Native Trigger note leases — 22 October 2026

This partial developer-editor checkpoint routes authenticated note heartbeat to
the existing note authority and services lease expiry from the existing GLib
source. It does not complete OpenSpec tasks3.2/3.3/4.2/4.3 or beta acceptance.

The three copied-DSO tests use an actual composed gated sample voice with480000
frames, signed stereo onset-0.5 and tail-0.125. Every test renders fewer than half
those frames. Qualification proves sustained playback and authenticated release.
Heartbeat renewal survives the original deadline and an unused-pitch NoteOff
sweep. Periodic expiry uses GLib/audio progress without another note command;
editor+host output-0.25 becomes host-only-0.125, then zero after host release.
A subsequent authenticated snapshot proves the attachment remains operational.
The unchanged production React bundle is exercised through authenticated methods;
physical keyboard/pointer input and continuous lifecycle audio are not claimed.

`author-raw.tar.gz` retains exact RED/GREEN output, commands, exits, timing,
calibration notes, compiler capture, raw LLVM exports, immutable artifact/profile
bindings and deduplicated source snapshots. `author-handoff.md` identifies the
individual receipts. `root-raw.tar.gz` retains independent registration RED/GREEN,
actual maintained fixture preparation and native runs, full host and focused
configuration tests, linkage, spec and source-input reuse evidence. Each archive
has a byte-verified member index and external SHA256 receipt. ELF binaries and raw
profiles remain in the named external evidence tree; their hashes are retained,
not represented as portable archive contents. No caches are archived.

Verified scope:

- Author:15 native parameter cases,13 adapter owner tests,14 core note tests,
  formatting, strict Clippy and three restored ordinary-DSO note cases pass.
- Root:23 maintained native CTests pass in83.725s;154 host tests/doctests and
  three affected build/configuration/development-dependency guards pass.
- Root reconstruction of final native+owner LLVM executes5/5 new service source
  regions and6/6 shared dispatch call-chain regions. Native alone executes4/5
  service regions; owner assertions exercise busy-borrow deferral. This is source
  execution, not whole-GUI, branch or every-instantiation coverage.
- Strict active-change/all51 canonical-spec validation and spec coverage pass
  (618 scenarios,379 mapped,239 backlog), with no mapping/checkbox closure.
- Current ordinary and maintained native artifacts have no JUCE/unresolved
  linkage. The headless artifact is explicitly a prior verified base artifact.

The prior eight-case copied-source headless build run is reused, not rerun.
`root-build-boundary-reuse.json` compares76 existing compiler-dependency inputs
byte-for-byte with current source; GUI/note-test modules are excluded by the
unchanged headless feature gate. One copied TB-303 asset was deliberately removed
by that earlier installed-runtime test; its current bytes match the published
base, but the deleted copy cannot be rehashed. Three affected guards were freshly
executed. This qualified reuse does not close task5.1 or prove a source-free
native-editor build, installed beta package or fresh full build matrix.

Setup X11 denial, observer syntax and initial PCM16 expectation calibration are
separate from genuine heartbeat capability-rejection and leaked-editor-voice
REDs. Normal product and Cargo-test DSOs differ with observed feature/compiler
invocation differences; no byte-reuse claim is made. Historical N2 compiler
commands are process strings, not recovered NUL argv/full environment. Portable
coverage bindings identify actual instrumented ELFs despite an older staging
receipt's normal-origin field. See the handoff and raw checkpoints for details.

Held-note/gesture destruction, late callbacks, creation failure, independently
edited native instances and physical lifecycle acceptance remain open. The
fractional-rate validator failure is unwaived. The maintained product remains
headless. VST3, standalone devices, named-DAW acceptance and installed offline
artifacts remain required. Mutation is excluded; reusable caches are preserved.
Independent review, commit and normal publication receipts belong to the live
tracker and are not preclaimed here.
