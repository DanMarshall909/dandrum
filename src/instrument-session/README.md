# Instrument session

The framework-free Rust session owns an accepted YAML document and prepared
sample resources for an instrument instance. It supplies bounded disposable
connections and copied snapshots. Render callbacks consume the prepared Rust
engine directly; they do not call into the session.

This is the foundation of [issue #23](https://github.com/DanMarshall909/dandrum/issues/23),
consumed by the JUCE replacement in #24. Current executable behavior covers
initial preparation, 16 live attachments, retired/foreign attachment rejection,
dropped-connection reclamation and reattachment without resource reload. A typed
`SetZoneKeyRange` prepares an uncommitted candidate from native map/zone IDs,
derives its inverse from accepted YAML, and prepares actual samples using the
retained native resource context. It preserves all unrelated YAML values and
asset references; YAML formatting/comments are normalized when serializing the
candidate. Invalid ranges, unknown/ambiguous targets, invalid connections and
failed sample preparation leave the accepted document and sound unchanged.

Candidate preparation does not publish an edit or change revision/history.
`commit_mapping` consumes a candidate belonging to the same instrument/revision.
The off-audio session stages its next document, revision, journal and returned
snapshot before asking an `ActivationHost` to install prepared resources. The
returned lease keeps audio and host snapshot admission closed while the session
swaps accepted state. Its infallible `publish` receives borrowed, already accepted
metadata, then releases admission. Unpublished leases must restore the old host
runtime. Publication must not allocate, notify or unwind; concrete adapters must
prove this contract at their own reader/handoff boundary.

Native undo/redo prepare the range recorded by the native version-1 journal;
failed preparation or activation leaves accepted state/cursor unchanged. Prepared
work can publish after its original connection detaches. Foreign/stale candidates,
retired/foreign history connections, empty history and exhausted revisions reject
before activation. Snapshots copy the next undo/redo entry and its native label.
The journal retains the latest 100 accepted edits. Once full, it discards the
oldest entry; undo reaches the state immediately after that discarded edit. A
failed branch preserves redo, while an accepted branch truncates it.

Structural mapping requests add instrument, request and expected-revision identity
to the current attachment used for admission. A newly admitted `MappingJob` owns
its accepted YAML, preparation context and render settings, so it is `Send` and can
finish on a worker after the attachment disappears. Each session admits one such
job at a time; overlapping requests, direct preparation, commit, undo and redo
receive `Busy`. A request ID reused with different content is rejected, and stale
or cross-instance requests cannot activate. IDs increase monotonically within a
live session; one retained high-water mark rejects an admitted identity after its
terminal outcome leaves the bounded recovery window.

The session retains the latest 128 terminal outcomes, including preparation and
activation failures. A fresh attachment can query an outcome or retry the exact
request and receive the same `Arc<RequestOutcome>` without preparing, activating
or appending history again. The successful path allocates the outcome and both
references before host installation; retaining it after installation uses the
preallocated bounded deque. A completion sent to the wrong session returns an
owned routing error, allowing that same prepared result to finish at its source.

`prepare_document_replacement` validates and prepares an externally supplied
canonical YAML document without changing accepted state. Its commit uses the same
activation lease, advances the revision, resets incompatible history and swaps in
a fresh preallocated outcome window while admission remains closed. Failed,
foreign, stale and busy replacements preserve the working session. The monotonic
request high-water mark survives replacement, so retired requests cannot reapply;
new requests based on the former revision are stale. Host parameter automation
remains authoritative runtime state outside the authoring journal and is reapplied
by the host when it installs a replacement runtime.

The protocol test consumer retains an actual graph processor and checks signed
output, publication ordering, rollback, reconnection and an independent second
session. The native comparison processor now also constructs this owner for
sample-map instruments through a C ABI static library. Its processor-owned
worker prepares mapping jobs without an editor, then publishes the exact prepared
engine through the existing audio-reader gate. Host render reconfiguration waits
for admitted structural work and rebuilds the newly accepted state, retaining its
revision, YAML and native history.

The committed native swap updates only engine/session state while the reader
gate is closed; parameter-surface notifications run after the activation lease
returns. The host proof instruments that exact window and rejects any publisher
mutex acquisition. Plugin-state restore waits for admitted mapping work and
then retires the session. The restored embedded YAML remains authoritative for
later sample-rate or block-size preparation even when its mapping differs from
the original file; ordinary file-loaded instruments retain file-based
preparation. The C facade also turns an incomplete callback table into a
retained terminal activation failure instead of calling missing function
pointers.

The concrete Linux proof holds an entered `processBlock`, verifies that the old
engine is retired once after reader release, maps `kit/kick` from MIDI 36 to 60,
checks literal signed `-0.5` output, rejects one activation without changing
runtime/metadata, and races a block-size change against held preparation. The real
sampler Standalone and VST3 and the proof executable each link one combined Rust
session archive, no separate engine archive, and one engine symbol set. Legacy
whole-document replacement now waits for admitted work and retires the native
session; adopting that path into the same journal, the typed browser transport
and actual WebView destruction proof remain pending in
`openspec/changes/add-persistent-instrument-session/tasks.md`.

Internal instance/attachment identities are opaque Rust lifetime anchors; their
pointers are never a wire representation. The eventual native transport owns
lossless external identities. `Arc` retains an anchor while a token exists;
`Weak` lets the session recognize live connections without keeping editors alive.
Snapshots own their copied YAML; modifying a snapshot cannot change accepted state.

Run component tests with the maintained build configuration:

```bash
ctest --test-dir build/engine-only --output-on-failure -L '^session-fast$'
ctest --test-dir build/engine-only --output-on-failure -L '^session-slow$'
```

The fast lane currently contains twenty-eight session tests (twenty-five public
component cases, two revision-limit unit cases and one native-facade lifecycle
case); the slow selection is empty.
Combined selection is `-L '^session-(fast|slow)$'`. Full
`ctest --test-dir build/engine-only` also runs the source-absence build guard,
which executes these tests with JUCE/browser/UI source trees absent. The
retained JUCE comparison configuration keeps the same registered test target.

For package feedback directly:

```bash
cargo test --locked --manifest-path src/instrument-session/Cargo.toml
```

The concrete native boundary and its library graph are registered in native
comparison builds:

```bash
ctest --test-dir build/native-session --output-on-failure \
  -R 'cxx-plugin-instrument-session|native-session-link-graph'
```

The tests prepare the existing Advanced Drum Kit and its real WAV. Literal
signed output at note 36 and silence at 60 are checked. A test-owned WAV is
removed after preparation to prove that reconnecting uses retained resources.
These tests do not establish actual native-editor destruction, React transport,
named-DAW hosting or the completed JUCE-free beta.
