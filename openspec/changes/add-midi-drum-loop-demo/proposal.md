## Why

The declarative drum kit now loads and renders in focused tests, but there is no broad integration check that exercises the drum container, preset or patch loading, MIDI event scheduling, engine routing, realtime rendering, and JUCE playback path together. A small MIDI-driven drum loop integration demo will check as much of the stack as practical without introducing plugin-host complexity.

## What Changes

- Add a user-invoked integration demo mode or small executable that loads the drum container patch or preset and plays a simple repeating MIDI drum loop.
- Use the existing Rust engine, JUCE audio output, MIDI-to-engine path, patch/preset loading, graph routing, and realtime render path rather than adding VST hosting or plugin support.
- Keep the musical pattern intentionally basic; sound-design quality is out of scope for this change.
- Add automated and manual verification that the demo target can load the selected drum container asset, schedule MIDI loop events, render non-empty audio, and start through the existing executable path.

## Capabilities

### New Capabilities

- `midi-drum-loop-demo`: Defines the integration behavior for playing a built-in/simple MIDI drum loop through a drum container asset and the existing engine/audio output path.

### Modified Capabilities

- None.

## Impact

- C++ JUCE wrapper command-line/demo entry points.
- Rust engine FFI and patch/preset loading only if existing APIs are insufficient for the demo path.
- Drum container patch or preset selection.
- Rust and/or CTest coverage for loading and rendering the scheduled MIDI loop.
