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
Host automation stays outside this authoring owner.

The test consumer retains an actual graph processor and checks signed output,
publication ordering, rollback, reconnection and an independent second session.
It is an in-process protocol fixture; it proves no concrete host's reader safety
or continuing voices. Native adapter adoption, bounded 100-entry history and
128-outcome recovery, one pending worker, the typed browser transport and actual
WebView destruction proof remain pending in
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

The fast lane currently contains sixteen session tests (fifteen component cases and one revision-limit unit case); the slow
selection is empty. Combined selection is `-L '^session-(fast|slow)$'`. Full
`ctest --test-dir build/engine-only` also runs the source-absence build guard,
which executes these tests with JUCE/browser/UI source trees absent. The
retained JUCE comparison configuration keeps the same registered test target.

For package feedback directly:

```bash
cargo test --locked --manifest-path src/instrument-session/Cargo.toml
```

The tests prepare the existing Advanced Drum Kit and its real WAV. Literal
signed output at note 36 and silence at 60 are checked. A test-owned WAV is
removed after preparation to prove that reconnecting uses retained resources.
Headless connection tests do not establish actual native-editor destruction,
DAW hosting or the completed JUCE-free beta.
