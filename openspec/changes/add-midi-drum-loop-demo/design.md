## Context

oThe current JUCE wrapper builds one console app, `dandrum-drum-machine-demo`, that owns audio device setup, optional MIDI input, synthetic note/scale test modes, and a `RustEngineSource` connected to the Rust engine over FFI. The drum-kit example already has Rust-side coverage proving it loads, validates, and renders non-empty audio when driven by MIDI-like note events.

The missing piece is a broad integration path that loads the drum machine and repeatedly schedules a recognizable MIDI pattern through the same engine/audio callback path used for interactive playback. The purpose is not just convenience; it is to check that the currently separate pieces cooperate end-to-end.

## Goals / Non-Goals

**Goals:**

- Provide a command-line integration demo that plays a repeating drum loop using MIDI note on/off events.
- Load the drum-machine patch or preset before playback, with an explicit override path if useful.
- Exercise patch/preset loading, container expansion, event routing, voice triggering, realtime render preparation, MIDI submission, and JUCE audio playback where practical.
- Keep the loop deterministic and simple enough to verify in automated tests.

**Non-Goals:**

- Add VST, AU, CLAP, or plugin-host support.
- Build a polished sequencer, transport, tempo map, or UI.
- Improve drum sound quality beyond what the existing patch/preset provides.
- Add realtime editing of patterns while the loop is running.

## Decisions

### Extend the existing console app first

Add a user-invoked mode to `dandrum-drum-machine-demo` rather than introducing a second executable initially. The current app already initializes audio, loads patches, sends synthetic MIDI events, and waits for engine completion. Extending it with a drum-loop option keeps the integration path close to the executable users already run and reduces duplicate device setup code.

Alternative considered: add a dedicated `dandrum-drum-loop` executable. That may become useful later if demo modes grow, but it would duplicate most of the current wrapper structure for this small first step.

### Use the drum machine as the default asset

The demo should load a drum-machine patch or preset, not the lower-level drum-kit implementation directly. That keeps the user-facing demo aligned with the intended instrument-level abstraction while still relying on explicit YAML composition rather than a drum-specific Rust primitive.

Alternative considered: load `examples/patches/drum-kit.yaml` directly. That is useful as a low-level fixture, but it does not exercise the container-level path the demo is meant to showcase.

### Drive the loop through note events

The demo should call the same note-event path as MIDI input and synthetic-note modes. This keeps the demo honest: it verifies the drum patch responds to MIDI-like event input rather than bypassing routing with a special render path.

Alternative considered: render the pattern directly in Rust tests only. That is useful for verification but does not satisfy the user-facing goal of hearing the loop through the current audio app.

### Maximize integration coverage without making CI require audio hardware

Automated tests should cover the deterministic parts of the integration path: loading the selected drum-machine asset, deriving/scheduling loop events, preparing the engine, rendering audio, and confirming non-empty output. The manual demo run covers the final local audio-device playback step because CI may not have an audio device.

Alternative considered: require audio-device playback in CI. That would make the test brittle in headless environments and conflate audio hardware availability with engine correctness.

### Keep pattern and timing intentionally fixed

Use a fixed four-beat loop with kick, snare, and hat notes. A hardcoded first pattern is acceptable because the goal is a smoke/demo utility, not a drum sequencer. If the note map is not already conventional, implementation should choose notes that the existing drum-kit patch responds to and document them in the test/demo help.

Alternative considered: add pattern files or command-line pattern syntax. That adds authoring surface area before the basic playback path is proven.

## Risks / Trade-offs

- The loop may be musically crude → Accept for this change; the goal is audible proof, not a finished kit.
- Console timing based on sleeps may be less precise than audio-thread scheduling → Keep the pattern simple and treat this as a demo path, not a production sequencer.
- Tests may not be able to verify actual audio-device playback in CI → Verify the widest deterministic integration slice in tests, and keep device playback as manual/demo behavior.
- Loading presets may require API work if the C++ wrapper only loads patches → Prefer a drum-machine patch if preset loading would broaden scope, but do not fall back to the raw drum-kit patch as the default demo asset.
