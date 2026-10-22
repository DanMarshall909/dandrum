# Selected-sample foundation contract

This refines the existing CLAP beta tasks; it adds no product or decoder format.
The first implementation boundary is task 5.1 plus selected-resource admission
and status from 5.2, with the trusted native chooser needed to acquire those
resources from 5.4. It does not complete full-patch publication, native import
mapping/history, production import UI, portable state or installed/DAW acceptance.

## Trusted acquisition and ownership

`chooseSamplerFiles` opens one asynchronous local GTK file chooser, parented to
the current editor on its existing X11/main thread. It accepts no browser paths,
metadata, bytes or resource identifiers. The chooser response acquires regular
local files through native code. On Linux, opening must not block on FIFOs or
follow a final-component symlink; reject nonregular files. Preserve exact OS
path bytes while acquiring the file; only a bounded display basename crosses
the wire. File reading and decoding run off audio, with no nested GTK loop.

A choosing dialog is view-owned and unadmitted. Editor destruction invalidates
its callback authority and cancels it. Once the instance accepts the opened
files into its bounded preparation worker, that work and its result become
session-owned and survive editor loss. A new live attachment can query the
result. No callback retains a strong WebView or changes another instance.

Selection alone never changes the accepted patch, parameters, history or audio.
One bad file rejects the entire selection; no partial catalog result escapes.
Cancellation yields an explicit cancelled outcome. Busy, invalid, stale and
unknown-selection results are explicit. A later valid selection remains possible.

## Existing bounded wire version

Use optional methods in the existing version 1 envelope for this small metadata
flow. Keep its 16 KiB ingress, 64 KiB egress, identity validation, replay policy,
attachment limits and unknown-version rejection unchanged. Existing version 1
consumers require no negotiation or framing change for existing methods. Larger
whole-patch work will require its own explicitly negotiated framing; do not
silently enlarge version 1 here.

| Method | Arguments | Result |
| --- | --- | --- |
| `chooseSamplerFiles` | `[]` | `{selectionId, phase:"choosing"}` after creating the chooser |
| `getSamplerSelectionStatus` | `[selectionId]` | Bounded current or terminal selection result |
| `releaseSamplerSelection` | `[selectionId]` | Releases a terminal outcome's leases; never an accepted/pending/historical/runtime owner's leases |

Selection IDs are positive canonical decimal strings, scoped to the instance
and never reused. A status has `selectionId` and a phase: `choosing`, `preparing`,
`ready`, `cancelled`, `failed` or `released`. Failed status contains a stable bounded error
code. Ready status contains at most 16 files with `resourceId`, `name`,
`contentSha256`, `encodedBytes`, `sampleRateHz`, `channelCount` and `frameCount`.
Every numeric field comes from the exact retained bytes, never the file label.
`getSamplerSession` exposes bounded selection summaries so a new editor can
recover session-owned work even if the original admission reply was lost.

Only a live attachment can create a chooser or query/release its own instance's
selection. Transport retries never reopen a chooser or execute acquisition
twice. Stale callbacks cannot admit files after their view is retired. Once
worker admission succeeds, view loss cannot revoke that work. Query/release is
idempotent for retained outcomes; unknown/retired IDs fail explicitly.

## Content and preparation authority

Retain immutable encoded WAV bytes and an immutable decoded sample together.
Reuse the existing mono/stereo PCM16 decoder; reject empty playable data. Derive
the SHA-256 identity from the complete owned encoded file and verify metadata
against that same decode. Canonical selected references are
`selected/<64 lowercase hex SHA-256 digits>.wav`. They are logical references,
never absolute files, and resolve only against this instance's admitted owners.
Missing authority for a canonical selected ID must fail without trying the
filesystem. Ordinary non-content filenames under a directory named `selected`
keep the existing engine filesystem contract; interactive patch admission still
requires its explicit packaged/admitted resource whitelist. Packaged
and ordinary document resources retain their current reread/rollback semantics.

Preparation, candidates and render reconfiguration carry the selected authority
explicitly. Compiled sample handles retain the same snapshot through active,
pending and retired runtimes. Replacing/removing the original file cannot alter
the admitted bytes, identity, metadata or sample values. Reclamation happens
only after every relevant owner releases its lease, on control or worker
threads, including the existing retired-runtime reclamation path.

Rust `Arc` shares an immutable owner rather than copying its audio buffers.
Cloning a handle must not clone encoded or decoded arrays. The resource budget
must remain charged while any runtime or outcome still pins the snapshot;
removing a catalog lookup entry alone does not release that charge.

## Resource limits and qualification

The published qualification packet is
`docs/migrations/evidence/trigger-selected-decoder-qualification-2026-10-23`.
Four real 64 MiB mono/stereo process runs peaked at 264,180–264,252 KiB RSS
(about 258 MiB). This measured the current decoder, not the future catalog.
Retained encoded plus decoded arrays can reach three times encoded size; the
temporary data-chunk copy can raise one decode to four times encoded size.

The first catalog uses these finite limits, enforced by the catalog and qualified below:

| Limit | Value |
| --- | --- |
| Files per selection | 16 |
| Encoded bytes per file | 64 MiB |
| Union of live encoded bytes per instance | 256 MiB |
| Union of live decoded array capacities per instance | 512 MiB |
| Frames per selected file | 33,554,432 |
| Unique live selected resources per instance | 256 |
| Simultaneous chooser | 1 |
| Simultaneous preparation worker job | 1, shared with structural preparation |
| Retained terminal selection outcomes | 32 |
| Display basename | 256 UTF-8 bytes |

Live union includes staged/admitted work, retained results, accepted/pending
patches, native history and active/retired runtimes. Deduplicate identical bytes
within an instance without weakening this lifetime accounting. Reserve capacity
before acquisition/decoding. Reserve one read workspace of at most 64 MiB plus
one byte and bound bytes actually read, rather than trusting metadata or using
unbounded `read_to_end`. Identify existing content before decoding, allowing
duplicates to reuse their live owner even at a full catalog without a second
decoded copy. Reject new content beyond its retained allowance before decoding.
Decode serially, retaining at most one 64 MiB temporary payload copy. Release
failed/cancelled reservations and evicted/released outcome leases.
Outcome eviction cannot invalidate already pinned patch/runtime owners.

The array estimate at the aggregate limits is 768 MiB retained plus up to 64 MiB
decoder scratch, before bounded object/allocator overhead. This is not an RSS
guarantee. Measure real success, rejection and reclamation paths and report their
peaks.

The production catalog probe subsequently ran actual worker-thread preparation
at the limits with 64 MiB mono and stereo WAVs. Single-file cases peaked at
263,896 and 264,056 KiB RSS; four-file cases peaked at 854,180 and 853,904 KiB
(about 834 MiB). Four files retained exactly 268,435,456 encoded bytes and
536,870,560 decoded-capacity bytes. At that limit a fifth distinct file was
rejected, duplicate content reused its retained owner, and source removal left
metadata and independently known signed samples intact. Releasing the outcomes
returned charged usage to zero. Debug process times were 5.08/5.77 seconds for
one file and 23.84/24.86 seconds for four. These are headless debug-process
measurements, not native editor, DAW or universal RSS guarantees. Raw commands,
source/tool hashes and process measurements belong to the source-bound evidence
packet. All 640 MiB of generated probe WAVs were removed after workers joined;
reusable disk-backed targets remain.

Region/document limits belong to the subsequent full-patch contract;
this metadata-only slice must not claim those limits or task 5.2 complete.

## First executable proof

Before production changes, add a copied GUI-enabled CLAP test using the existing
string backend. Prove the packaged signed-audio control first, then call
`chooseSamplerFiles` and require the choosing result. Today's actual
`capabilityUnavailable` response is the expected behavioral RED; compilation or
invalid native setup is not RED.

Keep the same test through GREEN: select a real, distinct tiny PCM16 WAV using
the actual GTK chooser, query ready metadata/content identity, replace/remove
the original and query the unchanged outcome from a recreated editor. Verify
selection alone preserves the accepted session/state and packaged signed audio.
Use real engine preparation/render and runtime owner tests to consume the
selected snapshot and assert independently known signed PCM, source removal,
isolation, bounded failure and off-audio reclamation. Do not substitute a fake
browser path/token or a parser-only result for this proof.

GTK API-driven chooser selection is native acquisition/contract evidence, not
actual physical input acceptance. Full production import/mapping, Undo/Redo,
continuous selected audio, fresh portable restore, installed artifacts and
named-DAW gates remain pending in their owning tasks.
