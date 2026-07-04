## Why

Dandrum needs a safe scripting layer for behaviour that is too policy-like for Rust primitives and too awkward to express as YAML wiring alone. The near-term use cases are event routing, MIDI note mapping, velocity/accent logic, probability, step-sequencer decisions, and scalar control generation.

Most patches are expected to be LLM-authored or LLM-assisted, so the scripting language should optimise for deterministic generation, strict host constraints, and easy validation rather than broad human familiarity. Rhai is a strong first runtime because it is Rust-native, embeddable, AST-compilable during preparation, and easier to constrain than a general Lua or JavaScript runtime.

This change makes scripting useful without compromising Dandrum's realtime contract. Scripts are event/control-rate only. Rust primitives and YAML composites remain responsible for audio-rate DSP.

## What Changes

- Define Rhai as the first concrete script runtime behind the existing `ScriptRuntime` abstraction.
- Restrict scripts to event/control processing once per render block, never per sample.
- Compile and validate Rhai source during patch preparation, outside the audio callback.
- Execute only precompiled scripts during rendering.
- Expose a tiny host API for reading input events/controls, emitting bounded events, writing bounded scalar controls, and reading/writing bounded numeric state.
- Enforce operation, call-depth, state-size, output-event, output-control, and data-size limits.
- Reject scripts that declare audio-rate outputs or attempt unsupported behaviour.
- Treat script failures as deterministic no-op output plus structured diagnostics, never as render panics.

## Non-Goals

- No audio-rate DSP in Rhai.
- No user-defined oscillators, filters, FFT, convolution, granular processing, or sample-by-sample processors in Rhai.
- No filesystem, network, environment, process, threading, dynamic module loading, or arbitrary host callbacks from scripts.
- No Lua, JavaScript, WASM, or native extension runtime in this change.
- No UI editor or LLM authoring workflow in this change.

## Impact

- **Rust engine crate**: Add a Rhai-backed `ScriptRuntime` implementation and integrate it into render preparation and graph processing.
- **Patch validation**: Validate script ports, source, limits, and unsupported audio outputs before render.
- **Diagnostics**: Add structured script diagnostics for parse, validation, execution-budget, unsupported API, and bounded-output failures.
- **Realtime rendering**: Keep script execution bounded, block-rate, and pre-prepared.
- **Examples**: Add deterministic event-router and control-mapper examples.
