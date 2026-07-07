## Context

`add-juce-plugin-integration` establishes the Dandrum plugin as a runtime surface: it loads an immutable YAML instrument definition, exposes generic JUCE controls for `preset_surface.parameters`, and keeps `processBlock` strictly realtime-safe. That change explicitly excludes any graph/YAML authoring from the plugin's normal runtime UI.

This change adds a companion capability: authors edit the instrument's YAML file externally (any text editor), and the plugin watches the loaded file for changes and reloads it automatically through the same off-audio-thread mute/stop/compile/start transaction that any other instrument reload uses. No in-plugin text editor, schema-feedback panel, or graph-preview UI is required for this v1 slice.

## Goals / Non-Goals

**Goals:**
- Let an author iterate on an instrument's YAML using their own editor, and see changes picked up automatically by the running plugin.
- Make the reload a safe, explicit, off-audio-thread mute/stop/compile/start transaction — indistinguishable from any other instrument reload.
- Guarantee a failed reload never leaves the plugin silent or in a corrupted state; the previous DSP keeps running.
- Let the author disable auto-reload if they don't want it (e.g. while making a series of unrelated edits).

**Non-Goals:**
- No embedded YAML text editor, schema/validation panel, or DSP graph preview in the plugin UI for this v1 slice.
- No live mutation of the running graph while the file is mid-edit (a reload only happens once the file settles and validates).
- No preset reconciliation logic here — that is owned by the instrument-replacement mechanism in `add-juce-plugin-integration` (section 3/6), which this change depends on and reuses.
- No native OS file-system-event APIs required — simple polling (mtime or content hash) on a low-frequency background timer is sufficient and keeps this portable.

## Decisions

### The plugin watches the loaded instrument file, not a text buffer

Rather than owning an editable copy of the YAML, the plugin tracks the file path and a cheap change signal (modification time, falling back to a content hash if mtime is unreliable on some filesystems) for the currently loaded instrument. A low-frequency timer (e.g. every 500ms-1s) on a non-audio thread checks this signal.

Rationale: this avoids building and maintaining editor UI entirely for v1, while still giving authors a fast edit/reload loop using whatever editor they already prefer.

### A detected change triggers the same replacement transaction as any other reload

When the watcher detects a change:

1. read and parse the file off the audio thread;
2. validate and compile the candidate instrument off the audio thread;
3. prepare assets and realtime buffers off the audio thread;
4. reconcile existing preset/current parameter values against the new `preset_surface.parameters`;
5. mute audio, publish the replacement DSP, refresh the parameter surface if the layout changed, then unmute;
6. if validation/compile/prepare fails at any step, leave the previous DSP running unchanged and report the failure through editor/status reporting — do not mute or go silent for a failed reload.

This is the same flow `add-juce-plugin-integration` section 3 defines for any explicit reload; file-watching is just an additional trigger for it, not a separate code path.

### Auto-reload can be disabled

The plugin editor exposes a toggle to enable/disable file watching. Some authors will want to batch several unrelated edits before reloading, or may be temporarily editing an invalid intermediate state; disabling the watcher avoids repeated failed-reload diagnostics during that time.

## Risks / Trade-offs

- **Depends on unfinished work**: the mute/stop/compile/start transaction described here reuses the immutable-instrument-replacement mechanism from `add-juce-plugin-integration` (tasks 3.1/3.3-3.5), which is not yet implemented. This change cannot be completed until that lands.
- **Polling vs. native file-system events**: polling is simpler and more portable but introduces up to one polling interval of latency and periodic disk stats; acceptable for an authoring convenience feature, not a hard realtime requirement.
- **Partial-write races**: an external editor may write a file in a way the watcher observes mid-write (e.g. a truncate-then-write save). Debounce the change signal (e.g. require it to be stable for one extra polling interval) before triggering validation/compile, to reduce spurious failed-reload diagnostics from transient partial writes.

## Open Questions

- Should the watcher also cover referenced sample assets, or only the top-level instrument YAML?
- Should this remain polling-based indefinitely, or is a native file-system-event backend worth it once the plugin ships on all target platforms?
