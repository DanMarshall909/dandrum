## Why

The published JUCE-free Trigger CLAP editor foundation is not yet the first beta: real native selected-sample loading, portable restoration, named-DAW acceptance and installed release proof remain. The user revised the first beta to CLAP-only on 22 October 2026. VST3 and standalone are deferred to later increments and are not prerequisites for completing this beta.

## What Changes

- Complete the first CLAP beta with the production React/Wry editor, existing Rust engine and persistent format-neutral TriggerHost/session.
- Let native input select supported WAV files, retain immutable owned content, publish mapping/full-patch edits atomically and play independently known signed PCM.
- Preserve accepted edits and native undo/redo across complete editor destruction/recreation while host audio continues.
- Add bounded self-contained CLAP patch/sample/parameter state, with fresh-instance restoration after original selected files disappear and existing packaged-kit state remaining readable.
- Resolve the explicit official fractional-rate validator failure through precise host/preparation/playback behavior, without rounding, truncation or exclusions.
- Prove named-DAW CLAP automation and independent instances, installed offline CLAP artifacts outside the checkout and complete JUCE-free build/runtime/linkage.
- Preserve normal verification, independent material-boundary review/publication, mutation exclusion, disk-backed reusable caches and owned disposable-run cleanup.

## Capabilities

### New Capabilities

- `juce-free-trigger-products`: the CLAP beta's precise host clocks, native/callback/DAW acceptance and installed offline artifacts. The existing capability identifier is retained for continuity; its active requirements are CLAP-only.
- `trigger-sample-resources`: native sample selection, immutable resource identity, atomic patch publication/history and portable patch/sample/parameter restoration.

### Modified Capabilities

None. Existing CLAP, editor and session requirements remain in force. This increment adds the missing CLAP beta contracts and supersedes the earlier integral-only host-rate limitation through implementation work.

## Impact

Affected owners include `src/trigger-host`, `src/instrument-session`, engine render/preparation configuration, the existing CLAP/Wry edges, production Trigger import/document handling and build/install/verification fixtures. TriggerHost remains free of Clack, Wry, GTK, WebKit, VST3 and JUCE types. The current CLAP adapter and shared engine/session/runtime/parameter/state/editor machinery remain the architecture.

The proof platform remains Linux x86_64 X11 with WebKitGTK 2.50.4. VST3, standalone/device ownership, other native platforms and broader plugin frameworks are outside this increment. The later VST3 preference and bounded feasibility gates are recorded in `future-products.md`; no VST3 or standalone implementation starts here. Main remains clean at `68dd655`. The CLAP beta closes only after every retained beta requirement is proven.
