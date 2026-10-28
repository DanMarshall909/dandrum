# Resource-free portable Empty state

This first task-6.1 criterion adds version-3 state for an accepted resource-free
Empty patch. It does not complete task 6.1 or 6.2. Selected sample payloads,
fully self-contained mixed/package state and the larger CLAP stream projection
remain subsequent criteria. The base is
`bbba79b605f9b4a550d1a46f57d8cfe9f6cbde04`.

The existing real-controller Empty test first proves packaged MIDI36 output
[-1,-1] at nondefault physical gain2, native Empty publication, callback safety,
exact silence and all23 retained descriptions/values. Its new fresh-state
assertion genuinely fails before implementation: save_state returns Invalid.
The exact test/source preimages and successful test build are retained; this is
behavioral RED, not missing-API or fixture failure. One initial attempt to retain
those preimages met a read-only metadata path; the authorized elevated retry
succeeded before production changes.

The new frame uses DTRGSTAT, explicit little-endian version3, bounded JSON length
and raw-payload length. This first subset requires no resources or payload.
The controller validates the supported profile and named values before external
replacement. Representable packaged saves retain their exact version2 encoding;
all legacy validation remains. The new path saves/restores resource-free Empty
without rewriting its document or needing an editor attachment. It refuses
nonempty selected/package candidates rather than emitting unrestorable state.

GREEN drops the original instance and its package, opens a fresh controller,
removes that controller's packaged WAV and loads only the saved bytes. It restores
the same logical Empty document, all23 public values including gain2, fresh
history and literal zero output. One-byte short reads work. The subsequent
characterization table proves rejection/rollback for28 corrupt frames: short
framing/body, unsupported versions/payloads, oversized metadata declarations,
trailing data, invalid UTF8/JSON/metadata/product/document/resource/parameter
content. Fifteen reader/writer fault positions include legacy continuation reads;
I/O errors preserve accepted document/history/values. These guard cases were
added after initial implementation and are not claimed as pre-implementation RED.

Covered production refactoring replaces an unnecessary future resource DTO/Vec
with a fixed empty array, rejecting unsupported entries without allocating them.
No module, dependency, sample owner or history is added. Covered test refactoring
reuses one actual Empty fixture and names each corrupt case. It checks parameter
and feedback identity per rejection and exact serialized equality after the
table, avoiding repeated YAML parsing:0.55s→0.28s before the final three legacy
I/O assertions. Direct codec coverage also verifies oversize metadata emits no
framing/body bytes. No mutation testing occurred.

Final full host193tests+2doctests, formatting and strict all-target hostClippy
pass. Strict all-target editor-adapterClippy and its23library tests pass; its
ignored FIFO child is run by the passing owned parent. Focused current LLVM
uses4exactexecutables/5rawprofiles including that child and actual adapter callers.
Every added executable annotation is hit:81state lines (including its inline
codec test),38controller lines. Broad state coverage is16/16functions,
174/181lines,293/307regions; controller104/114functions,1099/1265lines,
1560/1806regions. Partial generated/owner/instantiation coverage remains explicit;
no branch or full-product coverage is claimed. Active strictOpenSpec and the
unchanged631scenario/392mapped/239ratchet map pass.

[Binding](binding.json), [member index](raw-index.json) and the readback-verified
[raw archive](raw.tar.gz) bind RED/GREEN/refactor/ordinary checks, exact measured
source, LLVM exports/annotations/profiles and owned-run cleanup. Exact external
LLVM replay objects/source/profiles are separately archived at the bound hash.
No Cargo target or reusable dependency cache is included or deleted. One heavy
producer uses disk-backed caches; pre-run storage admission passes with about
83GiB free. The existing64GiB retention estimate remains exceeded, without cache
removal authority.

Main remains clean68dd655. CLAP-only scope and mutation exclusion remain.
Strict engineClippy remains FAIL129 unchanged baseline diagnostics. The earlier
published product's34success/10explicitSKIP/0FAIL official result remains prior
product evidence; this host component slice makes no new native release,
source-free product, installed/offline, physical/native-input, continuous selected
audio or named-DAW acceptance claim. Those product gates remain open. Normal
checkpoint, independent exact-candidate review and publication are separate gates
indexed by the external portable-empty-review record.
