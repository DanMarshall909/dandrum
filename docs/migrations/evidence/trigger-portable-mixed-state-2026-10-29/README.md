# Mixed packaged and selected portable state

The initial candidate e854f081 failed independent review because its packaged
save decoded outside catalog accounting. The repair and renewed evidence are in
[the budget repair packet](../trigger-portable-mixed-budget-2026-10-29/README.md).
This packet retains the initial source-bound evidence; its source hashes identify
that historical candidate, rather than the later repair.

This task6.1 component extends the selected-only state at
`853c25736333d8857b5e25d6e629871409a94f37` to patches that retain the packaged kit
and add an imported sample. Tasks6.1/6.2 remain open: packaged-only version3,
full payload qualification and CLAP stream integration are subsequent criteria.
No beta scenario is closed here.

The genuine RED publishes a mixed patch through native admission, verifies the
packaged MIDI36[-1,-1] and distinct selected MIDI60[-.25,-.25] at physical gain2,
then fails save_state with Invalid. Its preimplementation sources and command
output are retained. An initial GREEN attempt's moved-Arc compiler error is a
setup failure, not behavior RED; the subsequent criterion passes.

Version3 may now bind exactly `assets/advanced-drums.wav` to verified embedded
bytes while preserving the original logical YAML. Its body has the same
length/hash/WAV/catalog validation as selected resources. Trusted native
preparation associates the document path with an immutable Arc snapshot;
canonical selected paths must still match their content identity. Ordinary
filesystem contexts and package-origin resources retain their prior behavior.
The catalog and compiled sample ownership retain the encoded bytes off audio.

An ordinary packaged source has no encoded snapshot. Saving reads only its
explicit regular package file within the existing per-asset bound, requires its
raw-content hash to equal the accepted prepared source, and uses the existing
decoder. Changed or missing bytes refuse the save before output instead of
serializing different audio. A restored source reuses its catalog snapshot and
resaves without a package file. Existing limits and decoder/resource authority
remain; no memory-budget exception, dependency or fallback path is introduced.

GREEN destroys the original instance/root, removes the fresh instance's packaged
WAV, detaches its editor and starts no worker. A decoy at the selected path is
left unchanged. Saved bytes restore identical YAML, all23 current physical
values, fresh history, MIDI60[-.25,-.25], MIDI36[-1,-1] and unmapped MIDI61 silence;
resaving is byte-identical and independent B retains[-.5,-.5]. The60-byte selected
WAV is literal and pinned to SHA256
`2e484972f38876c961b3c23fb4952487dce3e18501c7fb0efd1b8cb7011734d5`.
Actual scoped audio threads borrow only RuntimeAudio; every measured callback
has zero allocation/deallocation.

Engine characterization covers trusted associations after file/root loss,
ordinary-file reread failures, canonical selected identity, sample-rate/kind/
origin/path rejection and association removal. Explicit package reload discards
noncanonical aliases: overwritten or missing files preserve accepted playback;
a later valid replacement publishes independently known +.25 output. Native
full-document preparation retains only the associations the document needs.
The now-uncalled selected-only replacement constructor is removed in favor of
the complete supplied-resource API; no unused compatibility wrapper is added.

Complete host tests,41session tests,5engine snapshot tests,23editor-adapter
library tests (including its separately launched FIFO child), formatting, strict
all-target host/session/editor-adapter Clippy, active strictOpenSpec and unchanged
spec map631/392/239 pass. Focused final LLVM binds6executables/7fresh profiles.
Every annotated added production line executes:39engine,43session,1host,
62controller and16codec lines. Broad owner, branch and instantiation coverage
remain partial; execution is not itself an assertion. Exact replay executables,
profiles, merged data and source are archived separately before hook cache reuse.

[Binding](binding.json), [member index](raw-index.json) and [raw evidence](raw.tar.gz)
retain source overlays, RED/GREEN, ordinary checks, final-source LLVM and owned
cleanup. All owned workers/scratch are cleaned; reusable disk-backed caches stay.
The64GiB retained-target estimate remains exceeded without eviction authority.
Normal engine hooks and independent exact-candidate review/publication are
separate gates recorded in the external portable-mixed-review record.

Strict engine Clippy remains FAIL129: exact code/message/primary-source identities
match the baseline (128 distinct identities), with no new/removed diagnostic.
The prior GUI2fe/d157 official34success/10explicitSKIP/0FAIL result and the prior
source-free product build remain prior-source evidence. No current rebuilt
GUI/official/source-hidden/physical-input/continuous-selected-audio/DAW claim is
made. CMake/dependency inputs are unchanged; the next isolated product rebuild
is reserved for the completed CLAP state-stream path to avoid another several-GiB
intermediate fixture build. Main stays clean68dd655, mutation excluded,
CLAP-only beta active and VST3/standalone deferred.
