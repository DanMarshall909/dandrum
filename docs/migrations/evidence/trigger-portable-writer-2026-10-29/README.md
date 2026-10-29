# Every new Trigger save owns its samples

This task 6.1 component builds on `11a7f708e01a23d33a656e78989c00175ee8b192`.
Every new save now uses the existing version-3 binary container, including
factory-only patches and compatible changed defaults. Version-2 packaged JSON
remains readable with its original validation and exact 65,536-byte input cap.
Full payload qualification and fresh selected-content CLAP acceptance remain
open; no task, scenario or beta completion is claimed.

The genuine RED is the existing all-23-values/signed-frames fresh-instance test,
strengthened before implementation to require the portable signature, manifest,
exact factory WAV body and restoration without either original or fresh package
files. It failed in 0.04 seconds because output began with `{"versio` instead of
`DTRGSTAT`; one test, exit 101. The source preimage and complete command/output
are retained. GREEN removes the original controller and its package, opens a
fresh controller against a different +0.25 WAV, proves that decoy audibly plays,
then removes it. Loading saved bytes restores the logical remapping, all 23
independently named nondefault physical values, fresh history and literal MIDI60
offset-5 frames -0.14536858/-0.1444931 on both channels; MIDI36 remains silent.
The original WAV hash and four source PCM words are independently fixed.

The writer no longer conditionally falls back to version-2 output. Factory
acquisition retains the already reviewed catalog reservation/hash/decode/live
budget policy; all bytes must match the accepted prepared content. Missing or
changed source files refuse before emitting state. Restored owned bytes can be
saved with no file. The obsolete private legacy writer was removed; legacy
input checks use the documented fixed v2 fixture instead of current output.
Metadata beyond the old 65 KiB writer cap now saves under the existing 1 MiB
metadata bound; its actual oversize refusal remains covered in the codec.

Wider regression checks exposed an existing supported packaged gated graph
outside the newer interactive profile. Version 3 therefore reuses the existing
packaged resource validator as well as the counted YAML budget for such graphs;
selected content still requires interactive admission. This preserves supported
held-note/restore behavior without another resource authority or broadening
interactive authoring. The small extracted packaged-document checker executes
through the retained real gated-note case. Compatible-default state is also
restored into a fresh controller with literal -0.25 stereo output.

Current-output test consumers now inspect strict binary framing and metadata.
Their existing history, metadata, note cleanup, parameter coherence and signed
PCM assertions remain. Explicit malformed legacy inputs retain version/product/
resource/value/duplicate/path and exact input-limit checks. Metadata editing is
test-only and retains the original raw WAV payload. Semantic JSON object order
is compared as metadata; raw WAV content is compared exactly. The calibrated
actual-DSO allocator child also uses the current format.

Complete 200 host tests plus 2 doctests, 41 session tests and 23 editor-adapter
library tests pass. The adapter's FIFO child is explicitly launched by its
parent. All 19 actual headless dynamic CLAP cases pass, including the parent
that launches its ignored allocation/deallocation audit child. Four-owner
formatting, strict all-target host/session/editor-adapter Clippy, strict active
OpenSpec, unchanged spec coverage (631 scenarios/392 mapped/239 ratchet) and
whitespace checks pass. Initial compiler/module-declaration failures and stale
JSON/refusal assertions are retained as setup/regression findings, not additional
preimplementation RED claims.

Two affected actual native editor cases pass on a freshly built debug GUI DSO
with private authenticated Xvfb: host feedback/rollback and complete structural
mapping/history/recreation. They use the unchanged production React bundle
`d157a01b…`, not a newly built release. DSO `992fa8d3…` and the exact test/resources
are sealed for replay. No physical desktop, installed offline or named-DAW claim
follows from this automated X11 component proof.

Focused LLVM evidence binds 10 objects and 15 fresh profiles: eight owner test
executables plus the actual instrumented headless DSO and dynamic host. All 31
added/moved controller annotations and 38 codec annotations execute. Both new
test helpers have 12/12 executed annotations. Broad owner/function/instantiation
coverage remains partial; branches were not measured. Execution alone is not an
assertion. The exact object/profile/source replay is sealed before hook reuse.

Production refactoring removes the superseded v2 output branch and reuses the
established packaged resource policy. Test refactoring shares a strict framed
metadata inspector/editor and fixed legacy input, retaining independent oracles.
No new decoder, catalog, dependency, framework or runtime owner is introduced.

[Binding](binding.json), [raw index](raw-index.json) and [raw evidence](raw.tar.gz)
retain the source overlays, RED/GREEN, failed intermediate checks, final native/
owner/coverage evidence and owned-run cleanup. Large exact LLVM/native/headless
replays stay outside Git, identified by hash. Normal-hook checkpoint, independent
review and normal publication are separate gates in the external
`portable-writer-review.md` record. Reusable disk caches are retained; the 64 GiB
retention estimate remains exceeded without eviction authority. No RAMdisk or
mutation run is used. Only verified archived duplicate plaintext reports may be
removed after review/publication; compact receipts and exact evidence remain.

Main remains clean at `68dd655`; the beta is CLAP-only with 10/17 tasks complete.
Engine strict Clippy's 129 baseline failures remain explicit and unwaived.
Prior GUI release `2fe7909d…`/bundle `d157a01b…` official validator 34 successes,
10 explicit skips and zero failures remains prior-release evidence. Four high
GitHub default-branch advisory findings remain uninvestigated under audit 8.1.
Current release/source-free/full-payload/installed/native-input/continuous
selected-audio/named-DAW/final-audit gates remain open; VST3 and standalone deferred.
