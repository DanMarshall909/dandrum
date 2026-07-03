## Why

Dandrum has the basic building blocks of a modular audio engine (YAML patches, built-in modules, graph routing, headless rendering) but lacks a coherent architectural framework to decide what belongs in the engine vs. what should be composed in YAML. Without clear decision rules, every new instrument request risks expanding the primitive registry with special-purpose modules, bloating the engine and defeating the declarative vision.

This change defines the declarative platform architecture: where primitives stop, composites begin, scripts glue, and presets demonstrate. It establishes the decision framework that keeps the engine small while enabling rich instruments through YAML composition.

## What Changes

- Define the **primitive/composite/script/preset decision framework** that guides all future module additions
- Specify **hard constraints for script modules** (determinism, no FS/network, bounded execution)
- Specify **structured validation and diagnostics** with stable error codes and YAML paths
- Specify **composite module authoring** — reusable instrument definitions from YAML
- Specify **capability discovery** surface for future tooling/LLM authoring
- Specify **acceptance examples** that prove the platform can express useful instruments without new primitives
- Update **built-in-modules** spec with a justified primitive roadmap (promote some, defer others, reject as composite)
- Update **yaml-patch-format** spec for presets, parameter bindings, and asset bindings
- Update **script-modules** spec with hard sandboxing constraints

## Capabilities

### New Capabilities

- `primitive-decision-framework`: Decision criteria and justified roadmap for what becomes a Rust primitive vs. YAML composite vs. script vs. preset
- `validation-diagnostics`: Structured diagnostics with stable error codes, severity, YAML paths, port references, expected/actual types, and suggested fixes
- `composite-authoring`: YAML composite module definition format, expansion rules, and parameter exposure
- `acceptance-examples`: Nine specified instrument examples (808 kick, 909 kick, snare, hi-hats, subtractive synth, sampler voice, drum machine mapper, effects rack, script mapping) that validate the platform
- `capability-discovery`: Future introspection API for module types, ports, signal types, parameter ranges, and realtime notes

### Modified Capabilities

- `built-in-modules`: Update module registry based on primitive roadmap; add noise, impulse/click, math/multiply, note-to-control, envelope follower, delay line
- `script-modules`: Add hard constraints — no filesystem, no network, no unbounded allocation, no blocking, pre-parse/pre-compile/pre-validate off audio thread
- `yaml-patch-format`: Add preset libraries, parameter bindings, asset bindings, validation metadata sections

## Impact

- **Rust engine crate** (`src/rust-engine/`): New primitive modules (noise, impulse/click, math/multiply, note-to-control, envelope follower, delay line); composite expansion logic; structured diagnostics types; capability discovery API
- **YAML schema**: Extended for composites, presets, parameter bindings, asset bindings, validation metadata
- **Script runtime**: Sandboxing constraints enforced at parse and execution time
- **Tests**: New unit tests for each new primitive, composite expansion, validation diagnostics, script sandboxing, and acceptance-level render tests
- **CLI/frontend**: No immediate changes; capability discovery is designed for future use
