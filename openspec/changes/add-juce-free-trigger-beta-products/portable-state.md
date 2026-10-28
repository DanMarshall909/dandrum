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
parameters and a resource manifest. Manifest entries will identify canonical
resource path, SHA-256 content identity and encoded byte length. The signature
and explicit version select this representation; metadata is not legacy JSON.
The metadata limit is 1 MiB, matching the qualified full-session projection bound;
accepted document/profile/parameter limits remain independently enforced.

The first bounded implementation criterion is a resource-free Empty frame:
resources is exactly an empty array and payload length is zero. It restores the
same document and all 23 public physical values into a fresh controller, with
fresh external-replacement history and exact silent output. Nonempty manifests
remain explicitly unsupported at this intermediate boundary; no selected-sample
portable-state or whole-task completion is claimed. Existing representable
packaged saves retain their version-2 encoding during this first criterion.

Sample payload bounds, content/manifest validation and fully self-contained
packaged/selected saving are subsequent task-6 criteria. They must reuse the
qualified per-asset/catalog limits, enforce lengths before allocation and prove
fresh-instance signed PCM after source removal. Do not advertise a candidate
maximum payload or close task 6.1/6.2 before those criteria pass.
