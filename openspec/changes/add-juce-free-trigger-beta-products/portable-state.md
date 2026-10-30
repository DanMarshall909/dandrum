# Portable state framing

This refines task 6.1 and its later CLAP projection in 6.2. It introduces no new
instrument/history owner, decoder or external dependency. Version-2 packaged JSON
remains readable with its existing exact 65,536-byte limit and validation.

The version-3 binary frame has an eight-byte ASCII signature `DTRGSTAT`, a
little-endian u32 version (3), a little-endian u32 JSON metadata byte length and a
little-endian u64 total raw sample-payload length. Metadata follows immediately;
raw WAV payloads follow in manifest order, with no base64 or alignment padding.
The reader rejects unsupported versions, invalid lengths and trailing bytes.
A short successful Read must be retried; premature EOF is invalid state, while
an actual reader/writer failure remains an I/O error. No accepted publication
occurs until the entire candidate passes validation/preparation.

Metadata is strict JSON with product identity, accepted document, named physical
parameters and a resource manifest. Manifest entries identify canonical
resource path, SHA-256 content identity and encoded byte length. The signature
and explicit version select this representation; metadata is not legacy JSON.
The metadata limit is 1 MiB, matching the qualified full-session projection bound;
accepted document/profile/parameter limits remain independently enforced.

The initial resource-free Empty subset remains readable: resources is an empty
array and payload length is zero. It restores the same document and all23 public
physical values into a fresh controller, with fresh external-replacement history
and exact silence. The selected-only extension below adds owned sample payloads.
Existing representable packaged saves retain their version-2 encoding at this
boundary; the mixed criterion below adds packaged content to selected patches.

Sample payload bounds, content/manifest validation and fully self-contained
packaged/selected saving are subsequent task-6 criteria. They must reuse the
qualified per-asset/catalog limits, enforce lengths before allocation and prove
fresh-instance signed PCM after source removal. Do not advertise a candidate
maximum payload or close task 6.1/6.2 before those criteria pass.

## Selected-only payload criterion

This criterion restores an accepted patch whose resources are all canonical
selected-content references. The manifest contains each distinct required
resource once, with path `selected/<lowercase SHA-256>.wav`, matching `sha256`
and positive `encoded_bytes`. No ambient paths, unused entries or duplicate
identities are accepted. Payload bytes follow in manifest order; their actual
hashes and complete WAV decode must agree before accepted publication.

Reuse the qualified catalog limits: at most256 resources,64MiB per encoded WAV,
256MiB aggregate retained encoded content and512MiB decoded capacity per instance.
Check manifest counts, per-entry lengths, checked sums and framing agreement
before reading payload buffers. Decode one bounded source at a time through the
same catalog reservation/hash/dedup/decoder/completion path used by native files;
old accepted/history/pending/runtime owners remain charged. EOF/trailing data,
content mismatch, decode/capacity/preparation/activation failure must preserve
accepted resources, history, values and signed output. No base64 or second audio
resource authority is added.

A supplied-snapshot native session replacement prepares independently of the
fresh instance's prior selected context. It uses the existing external-state
history reset and atomic runtime restoration. The owning test first publishes
real selected PCM, then saves, drops the original instance/root, removes the
fresh packaged WAV and plants a decoy at the logical selected path. Only saved
bytes may restore MIDI60 signed[-1,-1]/unmapped MIDI36 silence, current values,
fresh history, repeatable save and an unchanged independent instance.

The mixed criterion below builds on this selected-only tracer. Full payload-limit
qualification and CLAP stream integration remain subsequent criteria. Neither
task6.1 nor6.2 closes on these component criteria.

A controller with an admitted native structural job returns Busy after validated
metadata and before reading sample payloads. A live catalog acquisition also
refuses a competing payload reservation before its first read. Content reuse
still reads/verifies serialized bytes; a live hash never excuses missing/corrupt
payload. Actual reader/writer errors and short progress remain explicit.

## Mixed packaged/selected criterion

A mixed manifest may additionally name exactly the known document-relative
`assets/advanced-drums.wav` path. Its SHA/length/body are verified exactly like
selected data. The logical YAML is unchanged. The trusted native preparation
context can associate that explicit document path with its decoded owned
snapshot; canonical selected paths must still match their snapshot identity.
No missing selected authority can fall back to a file.

Saving first reuses bytes retained in the catalog for the accepted prepared
source's content revision. For an ordinary packaged source without encoded
retention, read only the explicit packaged WAV within the existing per-asset
bound through the existing catalog reservation/hash/decoder/completion path.
Require its raw-content identity to equal the accepted prepared sample before
including it. Acquiring new content beyond the live allowance returns TooLarge
without emitting state or changing accepted playback. The catalog may use its
bounded encoded read/hash workspace before refusing new content, preserving
verified deduplication at full capacity. A competing acquisition reservation
returns Busy before payload allocation/read. Save preparation borrows the main
controller mutably for this catalog bookkeeping; the audio endpoint remains
independent and no controller borrow crosses a foreign host stream call. The
temporary snapshot remains charged until writing finishes, then releases its
quota. A changed/missing source is an explicit save failure, not
permission to serialize different audio. After portable restore, the accepted
snapshot/catalog retains those bytes and resaving needs no package file.

The generic filesystem preparation default stays unchanged. Disposable native
edits retain required trusted owned associations. Explicit Trigger package
resource reload instead discards such noncanonical associations and rereads the
package, preserving its missing/invalid-file refusal and rollback. No new decoder,
resource cache, memory-budget exception or product format is introduced. Existing
aggregate/decoded/live-union limits still apply; full boundary qualification and
CLAP streaming remain subsequent work.

## Bounded foreign stream projection

The CLAP consumer must accept the already-supported portable frames beyond the
legacy65KiB JSON limit. Parse bounded framing/metadata outside the main controller
borrow, then preflight the same native metadata/admission checks. An already busy
structural job, gesture/automation or nonempty-payload acquisition refuses before
payload I/O. Read exactly the validated payload extent and check EOF with no
controller borrow live. This transient byte buffer owns no instrument history,
resource catalog or decoder.

Foreign reads may reenter the plugin. After buffering, revalidate current native
admission, values/revision/generation and resource reservation through the same
atomic controller load path. A job acquired during reentrant reads may therefore
cause a final Busy after input consumption; it must not cause partial publication.
Retain legacy JSON's exact65,536-byte bound and existingv2 semantics. No new state
format/limit or default-writer switch is introduced by this consumer criterion.


## All new saves own their payload

Every new save uses version 3, including ordinary packaged-only patches and
compatible changed control defaults. Retain version-2 reading and its original
validation/65,536-byte input bound; no production version-2 writer is needed.
The existing mixed acquisition, live catalog budget and verified-content rules
apply equally to factory-only saves. A fresh controller must restore all named
physical values and independently known signed frames after the source instance
and package disappear, even when its initial package contained different PCM.
Current-output consumers inspect the binary framing; intentional legacy-input
checks use independently fixed version-2 data. Full payload qualification and
selected-content fresh CLAP acceptance still precede task-6 completion.

The established packaged resource profile remains supported inside v3: a
packaged graph accepted by legacy resource validation need not become an
interactive-editor graph to save/restore. Reuse that exact path/definition
resource validation and the counted YAML budget. Selected-content documents
still require interactive admission; no new ambient path authority is added.
Version 3 permits compatible mutable defaults while preserving parameter
identity/ranges and all existing version-2 input checks.


## Production-limit qualification

Qualify the declared inequalities with real valid content, not only synthetic
oversize headers or reduced test budgets. Retain 256-resource acceptance and
257-resource refusal, an exactly 1 MiB metadata save/restore and one-byte-larger
refusal, 64 MiB per-asset acceptance and max-plus-one refusal, and an actual
256 MiB encoded aggregate round trip. The framing ceiling is 24 header bytes,
1 MiB metadata and 256 MiB raw bodies; document/profile/live-union limits remain
independent and may refuse a candidate below that ceiling.

For the supported mono/stereo PCM16 decoder, decoded capacity accounts for two
bytes per encoded PCM data byte. At most 256 MiB encoded content with at least
four 44-byte WAV headers reaches 536,870,560 decoded-capacity bytes, 352 bytes
below the conservative 512 MiB ceiling. Four minimum-header 64 MiB WAVs attain
that value; exact 512 MiB is not an attainable accepted fixture under these
encoded/format constraints. Do not widen formats or limits to manufacture it.

The maintained large fixture uses distinct independently hashed WAV recipes and
near-end regions with literal signed two-frame oracles. Save, drop the original
controller/audio/package, restore through the bounded header/preflight/buffered
input path without a package WAV, and resave identical content. Prove live-union
rejection, reuse at full capacity, a missing final serialized byte with unchanged
accepted document/physical values/PCM, and release after the last runtime owner.
Temporary generated WAV/state files belong to the owned run; never retain them
as a new cache or checked-in corpus. Measure actual process time/RSS and keep
this ordinary case in the canonical serial slow lane, separate from fast tests
and all mutation execution. Its measurement is not a universal RSS guarantee.
