# Interactive patch contract

This refines tasks 5.2 and 5.3 and their production frontend consumers in 5.4
and 5.5. It does not close a task or claim that the following endpoints already
exist. Keep the selected-content authority and limits in `selected-samples.md`.
No VST3, standalone, portable-state or broader plugin framework work starts here.

## Explicit bounded wire profile

Keep the existing native bootstrap/backend contract at version 1. Existing
version 1 requests and events retain their 16,384-byte ingress and 65,536-byte
egress limits, identities, replay policy and JUCE comparison behavior.

A live attachment opts in with a version 1 `negotiateEditorProtocol` request
whose positional arguments are `[2]`. Its accepted result is exactly:

```json
{
  "wireVersion": 2,
  "methods": ["prepareInstrumentSessionPatch", "getSamplerSession"],
  "limits": {
    "documentBytes": 65536,
    "requestBytes": 524288,
    "replyBytes": 1048576,
    "replayDocumentBytes": 262144
  }
}
```

These are fixed implementation budgets to qualify, not caller-selected limits.
The existing kit document is 6,699 bytes. A 64 KiB decoded YAML document can
expand to 393,216 bytes under worst-case JSON escaping, below the 512 KiB full
request cap. Check decoded UTF-8 document bytes and complete UTF-8 wire bytes
independently. A 1 MiB reply cap is a hard encoding limit, not a guarantee that
every possible document's expanded projection fits.

Negotiation changes only the live attachment's framing permission. It changes
no patch, value, history or session request cursor. Repeating `[2]` with a fresh
transport ID returns the same profile. Other choices fail explicitly. An exact
transport retry retains the existing `duplicateRequest` policy; a lost
negotiation acknowledgement can be recovered with a fresh transport request.
Another attachment or instance does not inherit negotiation. Detach revokes it.

Version 2 is accepted only after successful native negotiation and only for the
two listed operations. Other methods and all unsolicited events stay version 1.
The browser enables the profile only after validating the correlated result,
records each pending request's expected reply version and retains one transport
ID cursor across both versions. A wrong-version response consumes no promise.
Outstanding version 1 replies and events continue to work after negotiation.

Do not advertise patch capability before its real worker, publication and
history endpoint exists. A negotiation-only RED/GREEN step is internal TDD, not
a publishable full-patch completion boundary.

## Admission, recovery and replay

`prepareInstrumentSessionPatch` takes
`[instanceId, attachmentId, sessionRequestId, expectedRevision, yaml]`.
The first four are canonical decimal strings with the existing native identity
and revision semantics. The envelope transport request ID remains independent
of the session request ID. No browser sample bytes, paths, metadata, inverse or
resource manifest grants resource authority.

Admission returns the existing small accepted `job_id` string result. Work uses
the existing capacity-one preparation worker shared with mapping and selection.
Preparation and native publication are one admitted session operation; do not
introduce a separate browser-owned prepared-candidate commit protocol. Existing
version 1 `getInstrumentSessionJobStatus` recovers its running/completed/failed
outcome through a new live attachment, including when the admission reply was
lost. Its compact result does not embed the document. Rejected busy, invalid,
stale or conflicting input must distinguish unadmitted work from an owned job.

Full-patch retries compare the identity tuple and exact decoded YAML bytes.
Include wire version in transport replay identity. Retain immutable shared exact
document bytes, not whole encoded messages or large reply caches. Keep the
existing 128-entry replay bound and add 262,144 charged document bytes per
attachment (at most 4 MiB across 16 attachments). Conservatively charge each
document-bearing replay record. Byte pressure evicts oldest document-bearing
records without evicting unrelated version 1 records to recover zero bytes;
the high-water mark keeps evicted IDs retired. Count pressure retains its
existing oldest-entry eviction. Share buffers with admitted work, while charging
their session/history lifetimes separately from transport replay.

After native admission, work, resources and retained outcomes belong to the
session and survive editor destruction. Before admission, disposal releases only
view-owned work. Status recovery never re-executes an edit or loosens the native
admission-owner check. Existing mapping retry behavior is not changed implicitly.

## Bounded supported patch validation

Validate the entire candidate before preparation/publication. Resolve only the
packaged `assets/advanced-drums.wav` and canonical selected references whose
immutable content is already admitted in this instance. Reject external paths,
other resource kinds/origins and missing selected authority without filesystem
fallback. Retain only the selected owners actually required by the candidate.

The interactive Trigger profile supports its current sample-player graph and
the builtin silence/control routing needed for New Empty, not arbitrary user
module libraries or scripts. Its initial finite validation limits are:

| Domain | Maximum |
| --- | ---: |
| Decoded YAML document | 65,536 UTF-8 bytes |
| Expanded YAML string content, including keys and tags | 1,048,576 UTF-8 bytes |
| Expanded YAML values, including keys and tagged content | 131,072 |
| YAML value nesting | 64 levels |
| Sample sources | 256 |
| Sample regions, total | 1,024 |
| Sample maps | 64 |
| Sample zones, total | 1,024 |
| Root modules | 64 |
| Root cables | 128 |
| Root ports | 64 |
| Sample-player voices | 128 |
| Display names and identifiers | 256 UTF-8 bytes each |

Before constructing an owned YAML tree, count expanded values and string bytes
with the same parser, without retaining a second tree. Charge aliases for their
expanded content. Stop at the first exhausted budget, before schema validation
or resource acquisition. Keep ordinary aliases, quoted `&`/`*` characters and
supported scalar/tag parsing; do not use a textual character ban. These limits
bound interactive admission only and do not change the engine's general YAML
language. Qualify them with the packaged kit, a real larger supported document,
and modest repeated-content rejection fixtures. A count limit is not a claim
of constant-memory parsing or an exact whole-process memory ceiling.

Use only builtin `midi_input`, `sample_map_player`, `audio_mixer` and
`control_mixer` modules, with at most one MIDI input and one sample player;
disallow inline/external defined modules, poly expansion and scripts in this
interactive profile. Keep channels within the supported mono-control/stereo-audio
surface. Existing decoder/catalog limits remain independent. Qualify normal
single-file and nontrivial larger real documents against these limits, including
the complete projected snapshot. Reject a candidate atomically if its required
version 2 snapshot cannot be encoded under 1 MiB; never publish an accepted state
that its new editor cannot retrieve.

Qualify the complete reply against the bounded later sizes of generation,
revision, history counts, correlation/native request IDs, selection summaries
and physical/normalised parameter number spellings. Changing supported host
values or widening counters must not strand an otherwise unchanged accepted
document. A size-only conservative projection may overestimate these fields;
its representation never crosses the wire. Retain the hard read-side encoder
and its explicit `oversized` result for pre-existing externally loaded states
that were not admitted through this interactive profile.

Version 1 `getSamplerSession` still returns the complete snapshot when it fits.
For an unrepresentable larger snapshot, return the existing small `oversized`
error envelope rather than leaving the promise pending or truncating data.
New clients negotiate before requesting a potentially larger version 2 snapshot.
Events remain small notifications/values; do not broadcast whole documents.

## Native publication and history

The worker prepares owned YAML and selected-resource leases off audio. Publish
only through the persistent session and existing block-boundary runtime handoff,
with the accepted document, required resources and one native history entry
changing atomically. Preparation, compatibility, capacity or projection failure
preserves accepted state, physical values, history, feedback and signed audio.

Preserve public alias identity/order, target root port identity/type/channel
layout and physical ranges. Mutable authored defaults alone do not change
compatibility. Current host-owned physical values must be installed before the
new runtime's first consuming block, including nondefault values and concurrent
automation. True incompatible interfaces still reject explicitly.

Native history owns before/after document and selected-resource states.

The version-1 `redoInstrumentSession` operation takes the same four canonical
decimal-string identity arguments as Undo: instance, attachment, native request
ID and expected revision. It shares the existing worker and history cursor.
Exact pending/completed retries recover the retained action before consulting
the current cursor; repeating a completed Redo does not apply a second edit.
For structural commands, transport rejects future revisions but lets the native
session distinguish retained past-revision retries from genuinely new stale
work. A new stale request remains unadmitted and leaves its native ID usable.

Existing Undo and the corresponding native Redo restore those states with one revision
transition, preserving current host-owned values. Bound history by its existing
100-entry limit and the shared catalog's live-content budget. Render-clock
reconfiguration uses the current precise clock when preparing a historical
document; old history must not restore an obsolete render configuration.
External host-state replacement retains its separate fresh-history semantics.

New Empty preserves the stable public surface and produces exact silent audio;
subsequent normal single-file mapping is a valid supported edit. A normal import
must choose/present a valid mapping instead of silently stacking across occupied
kit keys. Full production frontend import and physical input remain separately
required by tasks 5.4, 5.5 and 7.2.

## Verification boundary

First add an executable existing-dispatch test proving the version 1 packaged
session/signed-audio control, then requiring the agreed negotiation result.
Today's actual `capabilityUnavailable` response is the expected RED. Compilation
or test-fixture setup failure is not RED. Preserve that exact evidence.

Continue through real frontend serialization of a meaningful document above
16 KiB, native authority/admission, worker publication and signed waveform/native
history assertions. Exercise version/UTF-8/wire/document/replay boundaries,
cross-attachment and cross-instance permission, invalid resources/layouts,
backpressure, reply loss, editor recreation, Undo/Redo and current physical
values. Parser or handshake success alone cannot close tasks 5.2 or 5.3.
Retain copied native CLAP/Core ingress evidence separately from fast dispatch
tests. Mutation remains excluded; use physical caches and owned-run cleanup.

## Interim legacy state boundary

Until task 6 introduces the portable state codec, a legacy version-2 save must
validate that the current accepted document and physical values can be restored
by that codec, and bound the complete serialized container before writing any
bytes. Refuse unrepresentable selected-resource, empty or changed-default states
explicitly. Preserve valid packaged saves and all existing restore validation;
do not emit an unreadable state, rewrite authored defaults or treat selected
filesystem paths as portable authority. This temporary refusal is not completion
of the required beta fresh-instance restoration.

## Production consumer representability boundary

The JUCE-free consumer publishes the canonical native snapshot after a document
operation. Browser drafts are compiler inputs, never an additional accepted
instrument. Only the fixed projected sample player, root instrument name, voice
limit/steal, existing mapping geometry/root/removal, sample-region DSP geometry
and representable candidate controls/order/removal are currently editable.
Selector policy is restricted to the projected round-robin/weighted forms.
Allowed field sets reject partial edits that mix a DSP change with unsupported
metadata. Existing-rule bulk changes cannot create new browser rule identities.

Child names, empty nodes, hierarchy, output/bus/chain/template metadata, selector
reset/names/mute/solo, explicit processing-history/slice operations and arbitrary
browser history deltas are rejected before native admission. Region geometry
edits publish native operation history without a browser-only processing stack.
Rejecting these edits preserves patch, request cursor and native Undo/Redo,
including a recreated editor. Comparison-backend behavior is unchanged.

Native selection/import/new mapping and New Empty remain pending in their owning
tasks. Their production consumer entrypoints must not report successful imports
before the admitted-resource and canonical-result flow is implemented. This
restriction is not completion of those tasks, and does not restrict the existing
native protocol's independently verified full-document endpoints. Broader editor
metadata needs a deliberate bounded native representation before being enabled;
retaining browser closures does not satisfy native history or recreation.
