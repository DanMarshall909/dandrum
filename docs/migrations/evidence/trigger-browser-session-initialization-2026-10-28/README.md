# Browser initialization restores large retained sessions

This bounded checkpoint advances task5.2 and the lifecycle part of5.5. The
production JUCE-free adapter now negotiates the existing wire profile before
requesting its first session snapshot. Comparison backends retain their earlier
initialization. No Rust production, callback, dependency or history owner changes.
Full-patch/import/Empty/native-Redo consumer wiring and portable restoration
remain subsequent work; no beta task or scenario closes here.

The fast behavioral RED asks for a retained >64KiB snapshot using version1;
its finite transport stand-in rejects that call rather than leaving an unresolved
promise. The second RED proves negotiation failure must precede any snapshot
query or editor publication. GREEN validates the actual transport's correlated
handshake, large document, nondefault value, native history and exact identities.
Existing bootstrap/presentation fixtures now require that handshake explicitly.

The actual copied Wry/Core scenario publishes a meaningful512-region document,
asserts the snapshot exceeds65536bytes, destroys the editor completely, recreates
it through production initialization, and checks retained document, instance,
new attachment and history. Native Undo then restores the prior document and
known signed audio while the second instance remains unchanged. A hash-verified
published fa2230a browser bundle is the historical negative control: its initial
publication succeeds, but editor recreation visibly fails with `Native sampler
failed: oversized`. The first256-region fixture did not exceed64KiB and failed
the explicit precondition; it is retained as fixture calibration, not behavioral
RED. The subsequent identical512-region scenario passes with the rebuilt current
browser. This is headless X11/WebKitGTK2.50.4 component evidence.

All156 frontend tests,12Chromium native-entrypoint cases and30actual native CLAP
checks pass. The changed NativeEngine's complete measured V8 scope remains100%
lines,86.22%branches and90.53%functions; the transport remains100% in all three
metrics. Coverage records execution; message/identity/value/history/error and
actual native assertions supply the oracles. No full-branch, whole-GUI LLVM,
physical-input/device, installed or named-DAW claim follows. Formatting, type/
component checks and strict all-target adapter Clippy pass. Strict engine Clippy
remains FAIL129 baseline diagnostics. Mutation stays excluded.

[Binding](binding.json) identifies source and external exact replay archives.
[Index](raw-index.json) hashes all [raw packet](raw.tar.gz) members, verified on
readback. Source overlays start from immutable Gitfa2230a. The negative native
control intentionally uses that published browser bundle even though the source
worktree already has the fast-test-driven fix; phases distinguish those inputs.
Complete native bindings include product/test/all local resource hashes. Rust
production remains the previously verified35a79 source, with its owning LLVM
measurements unchanged. Browser code is measured with Node22/V8.

All named runs verify stopped owned workers and scratch removal, including
failure paths. Chromium uses a short owned disk path to avoid Unix socket length
limits. Reusable disk caches are preserved. Bounded retention inventory measures
three complete cache roots at about63.3GiB and about1.7GiB of additional fixture/
evidence output; alias-bearing parent scans are explicitly incomplete lower
bounds. This exceeds the estimated64GiB retention budget and was reviewed:
indexed replay archives and required evidence remain retained, no cache eviction
or deletion authority is inferred, and physical free space remains about104GiB.
Further owned fixture compaction remains a cleanup opportunity distinct from
reusable-cache deletion. Main stays clean at68dd655; VST3/standalone remain deferred.
Normal-hook checkpoint, independent approval and publication remain separate
recorded gates, not implied by this evidence packet.
