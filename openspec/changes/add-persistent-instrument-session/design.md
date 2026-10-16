## Context

Issue #23 is the sole owner of accepted authoring state and history; issue #24 consumes it. The existing native engine prepares real samples and coordinates runtime replacement, but `Store.dispatch` records page-local closure commands only after the engine promise resolves. Closing the page can therefore separate accepted audio from history. The current `getSamplerSession` response is a snapshot, not a persistent journal.

## Goals / Non-Goals

**Goals:** one instance-lifetime native owner, the first real mapping edit/undo transaction, bounded admission and reconnect semantics, and an actual disposable-editor proof. Keep canonical YAML and the existing Rust sampler/preparation behavior.

**Non-Goals:** rewriting all domain algorithms or editors, a generic command executor, arbitrary client YAML patches or client inverse history, new DSP, disk-persistent undo, legacy JUCE envelopes, selecting final VST3/device/WebView frameworks, or claiming GUI execution from headless tests.

## Decisions

1. Add a framework-free Rust owner under `src/instrument-session`, consuming the existing engine. Canonical accepted YAML plus its native resource root remain authoritative. Use the existing loader and `prepare_kernel_patch_with_context`, which includes authored sample assets. The graph-only preparation overload omits those assets and is not a sampler replacement. Keep concrete format/window structs out of public session types. Internal attachment/instance identities use opaque reference-counted lifetime anchors; they are not OS handles or wire pointers. The transport maps these to lossless wire identities when connected. Bound live attachments to 16 and reclaim dropped/retired connections.
2. The first typed musical intent is `SetZoneKeyRange` addressed by explicit native map/zone IDs. Validate MIDI bounds and locate exactly one accepted zone. Derive the before range natively. Mutate only that YAML field, reload it through the normal kernel schema and prepare actual resources. A parallel Trigger patch serializer or whole-graph client inverse would broaden authority unnecessarily. Preserve unrelated fields and asset references.
3. Separate stable instrument-session identity, disposable attachment identity, request identity and expected revision. A currently attached connection is required for new commands; retired/cross-instance attachments fail. Retain a bounded outcome for an admitted request. A new attachment can query an old outcome or retry the same request/content without reapplying it; a conflicting reuse fails. Revisions invalidate requests whose outcome was evicted. Wire identities/revisions are lossless decimal strings; Rust owns the definitions, with generated/validated language bindings when transport is connected.
4. Accepted state, journal/cursor and the active runtime share a success boundary. Stage all fallible parsing, preparation, history/outcome allocation and revision checks before activation. Concrete hosts close audio admission and wait for readers using their existing handoff. An activation lease keeps audio/snapshot publication closed until the session swaps its staged accepted state and journal; release then exposes the new state. Failure restores the previous runtime/document/history. The session never renders, locks, serializes, allocates history or retires an engine from the audio callback.
5. Prepare owned work off the editor thread. Admit at most one structural job per session; later structural requests receive busy rather than silently replacing/queueing an edit. A worker completes an admitted job without editor polling. Attachment loss cancels view resources, notes and gestures, but cannot cancel accepted instrument work. Existing JUCE coordination supplies the first integration; direct Rust consumption by CLAP uses the same session owner. Prove the native library link boundary rather than bundling competing engine copies.
6. Keep 100 journal entries and 128 terminal request outcomes, plus one active structural job. Entries contain version, label, native map/zone identity and before/after range, without executable closures. Undo/redo prepare and activate the corresponding canonical document and move the cursor only on success. A new accepted edit removes redo; failed/rejected work preserves it. External accepted document replacement retires old requests and resets history; ordinary host automation does not append undo commands.
7. Preserve explicit native map/zone identities in the React projection. Session-owned mapping commands/undo bypass `Store`'s local closure journal. Retain its preview behavior and still-unmigrated commands during staged adoption; no native success may silently fall back to MockEngine. Cosmetic title/resize failures remain separate from accepted instrument edits.

## Test Boundaries

- First characterize the Advanced Drum Kit through the real public Rust preparation/runtime APIs: note 36 produces known signed PCM; 60 is silent. Reuse its real WAV and sampler child, not intrinsic wiring or fake audio.
- Test session ownership, revision/admission, bounded history/outcomes, deduplication, failed preparation/activation and native undo with that same actual runtime. Fast domain tests assert named outcomes, literal output and independent instances.
- Extend the existing `PluginTriggerNativeBridgeTest` and native handoff tests for real consumer integration, retained jobs after editor loss and atomic publication. A callback test stand-in does not prove the concrete host's rollback or reader safety.
- The final proof destroys the actual editor and WebView after admission/with a lost acknowledgement, reattaches, observes mapping 60 and one history entry, and undoes to 36 with literal signed output. A second instrument remains unchanged. Headless attachment tests alone leave this gate pending.

## Risks / Trade-offs

- Native zone IDs are currently collapsed/transformed by React compilation → carry explicit original map/zone IDs and use the narrow native operation, rather than round-tripping reconstructed whole graphs.
- Runtime installation can succeed before history publication → preallocate/stage the journal and use the concrete handoff lease; test failed activation and held readers at the real host boundary.
- Resource paths are native service policy → editors send identities; preserve original assets and use the existing native preparation resolver. No unrestricted filesystem endpoint.
- Source/host tests are currently available on native Linux but no usable display/DAW is configured → keep real editor/DAW gates explicitly pending until the environment supports them.
- The completed fixture-state format stores only controls → integrating authoring state requires an explicit replacement-state version; do not silently reinterpret fixture schema version 1.

## Migration Plan

Deliver covered increments in the existing migration worktree: real sampler baseline; native session owner/admission; transactional mapping/history; worker/host handoff; React adoption; destructive-editor/native-host proof. Commit verified material increments independently. Do not mark the whole change complete or synchronize unproven scenarios. The full JUCE-free beta remains active throughout.
