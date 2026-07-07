## Why

The Dandrum plugin (`add-juce-plugin-integration`) treats YAML instrument definitions as immutable while loaded, with structural authoring kept out of the DAW runtime surface. Authors still need a fast iteration loop, but building and maintaining an embedded YAML text editor with schema feedback and a graph preview is significant UI work with several open design questions (embedding model, graph renderer).

A much simpler v1: let the author edit the instrument's YAML file in whatever external editor they already use, and have the plugin watch the loaded file for changes and reload it automatically. This reuses the same off-audio-thread mute/stop/compile/start replacement transaction that any other instrument reload needs, without requiring any new in-plugin editing UI.

This was originally scoped as part of `add-juce-plugin-integration` (task section 12) as an embedded editor. It is being re-scoped here as a file-watch-and-reload capability instead, and kept as its own change since it depends on the immutable-instrument-replacement mechanism (section 3 of that change) landing first.

## What Changes

- Watch the currently loaded instrument's YAML file for external changes (e.g. modification time or content hash) off the audio thread.
- When a change is detected, automatically validate, compile, and prepare the edited instrument off the audio thread.
- On successful validation/compile: mute audio, stop/retire the old DSP safely, publish the new DSP, refresh the parameter surface if it changed, then unmute.
- On failed validation/compile: leave the previous DSP running unchanged and report the failure through editor/status reporting; never mute or go silent for a failed reload.
- Add a plugin editor control to enable/disable file watching (some users may prefer to reload explicitly rather than automatically).

## Capabilities

### New Capabilities
- `yaml-editor`: watches the loaded instrument's YAML file for external edits and reloads it through the existing safe instrument-replacement transaction, with no new in-plugin editing UI required for v1.

### Modified Capabilities
- (none — this is purely additive; it depends on, but does not change, `plugin-integration`'s existing immutable-instrument-loading, off-audio-thread-preparation, and realtime-safety requirements)

## Impact

- Depends on `add-juce-plugin-integration`'s immutable-instrument-loading and safe-replacement mechanisms (sections 2-3 of that change) being in place before file-watch-triggered reload can be implemented safely.
- Requires a lightweight file-watch mechanism (mtime/hash polling on a background/message-thread timer, not a native OS file-system-events dependency, to keep this portable across the plugin's supported platforms).
- Requires tests proving: a detected file change triggers a reload only through the explicit replacement path, and a reload that fails validation/compilation leaves the previous DSP running and reports the failure.
- No embedded YAML text editor, schema-feedback UI, or DSP graph preview is required for this v1 slice — those remain a possible future enhancement if file-watching alone proves insufficient.
