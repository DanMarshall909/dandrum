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
bound and require its raw-content identity to equal the accepted prepared sample
before including it. A changed/missing source is an explicit save failure, not
permission to serialize different audio. After portable restore, the accepted
snapshot/catalog retains those bytes and resaving needs no package file.

The generic filesystem preparation default stays unchanged. Disposable native
edits retain required trusted owned associations. Explicit Trigger package
resource reload instead discards such noncanonical associations and rereads the
package, preserving its missing/invalid-file refusal and rollback. No new decoder,
resource cache, memory-budget exception or product format is introduced. Existing
aggregate/decoded/live-union limits still apply; full boundary qualification and
CLAP streaming remain subsequent work.
