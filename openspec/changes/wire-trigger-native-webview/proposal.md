## Why

The complete React sampler shell runs only against a silent adapter. Connect that editor to the existing JUCE/Rust sampler so users can edit and play actual samples in an embedded WebView.

## What Changes

- Embed the Trigger React shell and its local assets in a maintained sampler standalone.
- Add a native adapter for prepared sample edits, undo/redo, host parameters, MIDI notes, waveform data and meters.
- Select the native adapter inside JUCE while retaining the explicit silent browser preview.
- Reject unsupported DSP capabilities without publishing simulated successful edits.

## Capabilities

### New Capabilities

- `trigger-native-webview`: Native-backed full React sampler shell and transactional editor operations.

### Modified Capabilities

None.

## Impact

React adapter and bundle, JUCE WebView resources and native functions, immutable YAML replacement jobs, launcher inventory and regression tests. Work on the primary Linux/WSL checkout as explicitly requested. Reuse the real Rust sample-map engine; adding unsupported effects or synthesis engines is outside this integration.
