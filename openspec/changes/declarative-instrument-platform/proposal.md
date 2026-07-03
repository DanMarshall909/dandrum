## Why

Dandrum already has the foundation of a declarative modular instrument engine: YAML patch loading, inline composite
`module_definitions`, typed graph routing, deterministic composite expansion, a Rust DSP engine, headless rendering,
sampler support, effects, and a JUCE wrapper.

The missing piece is not a second architecture. The missing piece is a platform alignment spec that explains how future
work should extend the existing model without bloating the primitive registry or hiding instrument policy inside opaque
runtime modules.

This change defines the platform rules for deciding whether behaviour belongs in a Rust primitive, YAML composite,
script module, preset, or future tooling layer. It also tightens the near-term primitive roadmap so Dandrum can build
useful synthesizers, drum voices, samplers, and effects from YAML while preserving realtime safety and deterministic
rendering.

LLM-assisted patch authoring is an important future use case, but it is not implemented by this change. The engine
should first become a stable declarative platform that future LLM tooling can target.

## What Changes

- Define the **primitive/composite/script/preset/tooling decision framework** that guides future module additions.
- Preserve and harden the existing inline YAML composite model based on `module_definitions` instead of replacing it
  with a parallel `type: composite` model.
- Specify a minimal justified primitive set for the next platform milestone: `noise`, `impulse`, `multiply`, and
  `note_to_control`.
- Defer primitives that are useful but not yet proven necessary, including `envelope_follower`, `delay_line`,
  `FM operator`, `resonator`, `wavefolder`, and specialist drum voice modules.
- Specify minimal oscillator waveform support where acceptance examples require it, rather than assuming unavailable
  oscillator behaviour.
- Specify hard constraints for script modules: deterministic, bounded, pre-validated, no filesystem/network access, no
  blocking calls, and no audio-rate DSP in the first implementation.
- Specify structured validation and diagnostics with stable error codes, source locations, port references,
  expected/actual values, and safe suggested fixes.
- Specify acceptance examples that prove the platform can express useful instruments through primitives and composites
  without special-purpose Rust instrument modules.
- Specify capability discovery as a future query surface built on module and parameter metadata, separate from render
  paths.

## Capabilities

### New Capabilities

- `primitive-decision-framework`: Classify proposed behaviour as Rust primitive, YAML composite, script, preset, future
  tooling, or out-of-scope before implementation.
- `validation-diagnostics`: Structured diagnostics with stable error codes, severity, YAML paths, port references,
  expected/actual values, and suggested fixes.
- `composite-authoring`: Hardening of the existing inline `module_definitions` model, including deterministic expansion,
  exposed parameters, source mapping, and optional future external composite library loading.
- `acceptance-examples`: A staged set of example instruments proving platform capability, starting with a synthetic
  808-style kick and expanding to snare, hat, subtractive synth, sampler voice, effects rack, script mapping, and
  drum-machine routing.
- `capability-discovery`: Future introspection API for module types, ports, parameters, categories, realtime notes, and
  example snippets.

### Modified Capabilities

- `built-in-modules`: Add only the minimum new primitives needed for the next milestone: `noise`, `impulse`, `multiply`,
  and `note_to_control`; add minimal oscillator waveform support if required by acceptance examples.
- `script-modules`: Add hard sandboxing and realtime constraints while keeping script scope to event/control
  transformation.
- `yaml-patch-format`: Extend existing schema sections instead of duplicating them; add patch-level parameters/presets
  only after existing metadata/assets/composites remain compatible.

## Impact

- **Rust engine crate** (`src/rust-engine/`): New minimal primitives; parameter metadata; structured diagnostics;
  optional capability discovery API; possible oscillator waveform support.
- **YAML schema**: Compatible extension of the existing patch schema, especially `module_definitions`, parameters,
  presets, and asset validation metadata.
- **Composite expansion**: Harden existing expansion and diagnostics instead of replacing the current model.
- **Script runtime**: Pre-validation and execution constraints before script behaviour is trusted in render paths.
- **Tests**: Unit tests for new primitives, validation diagnostics, existing composite expansion hardening, and
  deterministic acceptance renders.
- **CLI/frontend**: Optional debug and discovery commands may be added outside realtime render paths.