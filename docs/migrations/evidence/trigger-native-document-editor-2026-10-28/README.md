# Production native document edits and history

This advances tasks5.2/5.3/5.5 of `add-juce-free-trigger-beta-products`;
no beta task or scenario closes. Together with the preceding local initialization
checkpointdcc1ae01, it is intended for independent review against published
fa2230a. Selected-file consumer/mapping/Empty/recovery, portable state and
installed/native-input/device/named-DAW acceptance remain separate work. VST3
and standalone remain deferred.

The production JUCE-free NativeEngine now compiles supported document edits and
admits them through `prepareInstrumentSessionPatch` with exact instance,
attachment, decimal native request and revision identities. It polls the existing
session worker, validates correlated completion/snapshot metadata before publishing
and adopts the native physical values/geometry. The persistent session owns one
history entry; Store does not also record a browser closure. Existing typed
key-range work stays typed, preserving editor rule identity. Document Undo/Redo
adopts complete accepted snapshots. Native Redo is optional only on the CLAP
consumer; comparison backends retain their existing path and behavior.

The original fast RED called the legacy `prepareSampler` API. Further genuine
REDs catch a consumed ID not refreshed after failure, snapshot reads after closing
document/Undo/Redo operations, lost code-only errors when recovery fails, and a
rule identity lost after document-plus-typed-mapping work. Minimal repairs refresh
accepted metadata without publishing failed drafts, stop closed continuations
before new snapshot access, share code-aware error formatting and distinguish
full-document from typed mapping adoption. Rejected versus admitted-failed work,
fresh-editor Redo, both native/local parameter history orders and empty Redo have
separate public-outcome assertions. No independent browser instrument history is
introduced. Production/test refactor decisions are retained in the phase record.

The actual copied Wry/React scenario first qualifies a nondefault physical gain2
with literal[-1.0,-1.0] audio. A real Voice limit keyboard action must then serialize
a whole document through the native API. The preceding verifieddcc1ae browser
bundle supplies the historical negative control: it calls `prepareSampler`.
The current bundle changes8to9, native Undo restores8 and Redo restores9;
public identities and gain remain intact, signed output stays[-1.0,-1.0],
unmapped MIDI60 stays silent and instanceB retains its exact state/audio. This
runs after the large retained-document/editor-recreation scenario. Core/worker/
runtime production code is unchanged from the prior Rust boundary.

Three native calibration failures precede that genuine API RED. The first two
used an impossible disabled-Undo selector: the disabled label is `Nothing to undo`,
not an Undo-prefixed label. The third reset playback before an admitted UI gain
had reached audio; reset correctly cancels queued activity. The test now consumes
one real block before its reset/note oracle. These failures and the corrected
interpretation remain explicit; they are neither missing-document-API RED nor
product reset/notification defects. The new control interaction starts in a fresh
attachment to accepted native state. Native GUI locators use action identities;
actual method/argument, state, public value and signed audio assertions remain.

All167 frontend tests,12Chromium native-entrypoint cases and30native CLAP checks
pass. Type/component checks, formatting and strict all-target adapter Clippy pass.
Final Node22/V8 scope measures NativeEngine100%lines/86.86%branches/90.82%functions
and Store100%lines/86.45%branches/88.89%functions. Raw profiles and source overlays
are retained; execution is not a substitute for the behavioral assertions above.
No full branch/instantiation/GUI/physical-input or new whole-Rust LLVM claim is
made. Strict engine Clippy remains FAIL129 baseline diagnostics. Mutation stays
excluded; normal hooks and independent exact-candidate review remain distinct gates.

[Binding](binding.json) identifies source and exact external replay archives.
[Index](raw-index.json) hashes each [raw packet](raw.tar.gz) member, verified on
readback. Reconstruct current source from Gitdcc1ae01 plus its seven overlays;
RED source archives preserve the successive preimages. The native negative control
intentionally used the preceding built browser despite the worktree's fast-test
fixes; phase records bind both. Native replay includes exact product/test and all
local resource hashes. The actual tests use authenticated private X11/WebKitGTK
2.50.4, not a user's authenticated desktop or a named DAW.

All named runs retain exclusive command/output/cleanup receipts, with stopped
owned workers and removed scratch even on failure. Required evidence and reusable
disk caches remain retained. The earlier measured retention-budget excess and
alias-bearing scan gaps remain explicit; physical disk headroom is about103GiB.
Main stays clean at68dd655. Overall CLAP beta completion is not claimed.
