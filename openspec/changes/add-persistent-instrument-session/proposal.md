## Why

Trigger currently records undo only after a page-owned engine promise resolves, so accepted native edits can outlive their history when the editor disappears. The JUCE-free beta needs one framework-independent instrument session shared by concrete hosts and disposable React connections.

## What Changes

- Introduce one native-owned accepted YAML document, revision and bounded serializable journal for an instrument instance.
- Start with a typed sample-zone key-range operation and native undo/redo; derive inverse values from accepted state and prepare real sample assets before publication.
- Separate disposable attachment/request identities from the instrument lifetime; retain admitted job outcomes and deduplicate retries after lost replies.
- Connect this owner to the actual native preparation/handoff and React mapping workflow, bypassing the page-local journal for session-owned commands.
- Prove mapping 36→60, editor destruction/reconnection, native undo and literal signed sample output, including failure and independent instances.

## Capabilities

### New Capabilities

- `instrument-session`: persistent instance editing state, prepared mapping transactions, native bounded history and disposable editor connections.

### Modified Capabilities

None. Reuse existing structural preparation/handoff and sampler semantics; do not duplicate their general feature specifications.

## Impact

Issue #23 owns this session/SDK boundary; #24 consumes it. Begin with a framework-free Rust session crate over the existing engine, then connect concrete native adapters and the canonical React editor. Preserve YAML as document authority, original assets, current working editors and the JUCE comparison build. Full VST3/standalone packaging and the denormal-hardening finding remain separate beta requirements. No new DSP primitive, alternate undo owner, visual redesign, legacy JUCE compatibility or disk-persistent undo is introduced.
