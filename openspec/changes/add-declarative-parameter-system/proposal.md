## Why

Dandrum patches and composites can define DSP graphs, but tuning musical values still too often requires Rust code changes or hardcoded module behavior. A declarative parameter system lets authors and future LLM-assisted tools discover valid controls, tune instruments from YAML and presets, and repair invalid patches while keeping Rust as the source of truth for what each module supports.

## What Changes

- Add a static declarative parameter model covering module parameters, composite parameters, preset-applied parameter values, and CLI overrides as distinct input layers.
- Add built-in module parameter declarations in Rust with typed defaults, numeric constraints, enum constraints, units, descriptions, required flags, and static/realtime-preparation metadata.
- Add YAML module instance `parameters` values validated against the declared parameter model before graph preparation.
- Add composite-level public parameters and minimal binding from composite parameters to internal module parameters using only literals and direct `${parameter}` references.
- Add deterministic default resolution and parameter binding resolution before graph construction, composite expansion, compiled graph preparation, offline rendering, or realtime rendering.
- Add structured validation diagnostics for unknown parameters, missing required values, wrong types, range violations, enum violations, and invalid bindings so humans and future LLM repair loops can understand safe fixes.
- Make parameter declarations reusable as machine-readable capability metadata for future tools and LLM authoring workflows.
- Include CLI `--set module_id.parameter=value` overrides only as a secondary developer-nicety input source that applies after YAML parsing and before validation/resolved graph preparation.
- Preserve realtime safety by requiring all parameter parsing, validation, binding, diagnostics, and resolved graph preparation to happen outside the audio callback.

## Capabilities

### New Capabilities
- `declarative-parameters`: Defines static parameter declarations, typed parameter values, defaults, validation, composite public parameters, parameter binding, resolved parameter values, diagnostics, determinism, realtime-safety boundaries, and machine-readable capability metadata for future tool and LLM integration.

### Modified Capabilities
- `yaml-patch-format`: Adds module instance `parameters`, composite public parameter declarations, and minimal parameter binding syntax to the YAML patch schema.
- `built-in-modules`: Requires built-in module definitions to declare supported static parameters in addition to ports and delay-boundary metadata.

## Impact

- Rust engine module registry gains parameter declarations beside existing module and port definitions.
- YAML patch parsing and validation gain typed parameter values, default resolution, and binding resolution before graph preparation.
- Composite expansion uses resolved parameter values rather than hardcoded internal constants.
- Offline render and future realtime entry points share the same resolved patch/graph preparation path.
- CLI rendering accepts temporary developer experiment overrides without mutating source YAML files.
- Diagnostics and declarations become structured and stable for human users, future capability discovery, and LLM-assisted authoring/repair loops.
