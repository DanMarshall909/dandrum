## Why

The JUCE-free CLAP proof can render and persist fixture controls, and the persistent instrument session can survive editor loss, but the production Trigger React editor still reaches the engine only through JUCE. The next migration gate needs one real host-parented editor that joins those proven owners without introducing UI-thread or render-thread coupling before the same host core is projected to VST3 and standalone.

## What Changes

- Add a JUCE-free Trigger instrument identity to the experimental CLAP product, backed by the persistent session and the existing Rust engine rather than a fixture-only document.
- Embed the production Trigger bundle in a real Linux X11 child WebView owned through the CLAP GUI lifecycle, including parent validation, resize, show/hide, destruction and recreation.
- Make the Trigger native adapter consume a versioned transport contract so the existing JUCE comparison bridge and the new JUCE-free bridge share the same frontend behavior.
- Route one real parameter gesture through CLAP begin/value/end automation and return host feedback to React without polling or mutating the accepted structural journal.
- Route supported mapping and native undo through the persistent session, preserving admitted work and accepted audio when the WebView disappears.
- Prove literal signed sample output, note cleanup, state/parameter round trips, lifecycle failures and two independent instances through the dynamically loaded binary and an actual parent window.
- Keep VST3 projection, standalone audio/MIDI devices, named-DAW acceptance, installed packaging and non-Linux platforms explicit follow-up gates.

## Capabilities

### New Capabilities

- `juce-free-trigger-editor`: Defines the production Trigger bundle, transport, host-parented WebView, parameter gesture/feedback and destructive lifecycle behavior for the JUCE-free host.

### Modified Capabilities

- `juce-free-clap-proof`: Adds a real sampler/session-backed Trigger identity and CLAP GUI/automation/state behavior to the exported JUCE-free binary.

## Impact

- Extends `src/clap-adapter` and introduces a focused JUCE-free editor/transport owner that depends on the completed `src/instrument-session` boundary and existing engine.
- Adds pinned native WebView dependencies and Linux X11/WebKitGTK build inputs without adding JUCE or browser code to the engine-only core.
- Reuses the production `web/trigger` bundle through a transport-neutral native adapter and preserves the JUCE bridge as the comparison consumer.
- Adds dynamic-host and real-window tests, CMake resource/build wiring, dependency/link audits, OpenSpec mappings and source-bound evidence.
