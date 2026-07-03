## 1. Parameter Declaration Model

- [ ] 1.1 Add failing Rust tests for parameter declaration metadata: name, type, default, min, max, enum values, unit,
  description, required flag, and preparation timing.
- [ ] 1.2 Implement core parameter declaration and scalar value types for number, string, boolean, and enum values.
- [ ] 1.3 Add failing Rust tests proving built-in module parameter declarations are registered beside existing port
  metadata.
- [ ] 1.4 Extend built-in module definitions to expose static parameter declarations without changing port contracts.
- [ ] 1.5 Add failing Rust tests proving declaration metadata is reusable for future capability discovery and
  LLM-assisted authoring without reading DSP implementation code.
- [ ] 1.6 Add failing Rust tests proving declaration metadata can answer authoring questions about valid parameter
  names, types, defaults, ranges, enum values, units, descriptions, and static timing.

## 2. Parameter Validation Diagnostics

- [ ] 2.1 Add failing Rust tests for structured diagnostics containing stable code, severity, YAML path, module ID,
  parameter name, expected values, actual values, message, and suggested fix where safe.
- [ ] 2.2 Implement structured parameter diagnostic types and stable diagnostic codes.
- [ ] 2.3 Add failing Rust validation tests for unknown parameters, missing required parameters, wrong scalar types,
  invalid enum values, numbers below minimum, and numbers above maximum.
- [ ] 2.4 Implement declaration-driven parameter validation shared by module YAML values, composite instance values,
  preset-applied values, and CLI override values.
- [ ] 2.5 Add failing tests proving diagnostics contain enough machine-readable fields for future LLM repair loops to
  propose safe YAML edits for unknown, mistyped, out-of-range, and invalid enum values.

## 3. Default Resolution And Resolved Parameters

- [x] 3.1 Add failing Rust tests proving omitted optional parameters resolve to declared defaults and required
  parameters without defaults fail validation.
- [x] 3.2 Implement deterministic default resolution that produces complete resolved parameter maps for prepared module
  instances.
- [x] 3.3 Add failing Rust tests proving equivalent patches resolve to identical parameter maps across repeated loads.
- [x] 3.4 Integrate resolved parameter maps into graph preparation so DSP state construction consumes validated values
  instead of raw YAML values.

## 4. YAML Module Parameters

- [ ] 4.1 Add failing YAML parsing tests for module instance `parameters` values on built-in module instances.
- [ ] 4.2 Extend patch YAML parsing to preserve module instance parameter values with source paths for diagnostics.
- [ ] 4.3 Add failing validation tests proving unsupported fields and values that cannot be parsed deterministically are
  rejected before graph preparation.
- [ ] 4.4 Connect parsed module instance parameters to declaration-driven validation and default resolution.

## 5. Composite Public Parameters

- [ ] 5.1 Add failing YAML parsing tests for composite public parameter declarations with scalar types, defaults,
  constraints, units, descriptions, and required flags.
- [ ] 5.2 Extend composite definition parsing to preserve public parameter declarations and instance-provided composite
  parameter values.
- [ ] 5.3 Add failing validation tests for invalid composite parameter declarations, duplicate parameter names, invalid
  defaults, and inconsistent constraints.
- [ ] 5.4 Implement composite parameter declaration validation using the shared parameter declaration model.
- [ ] 5.5 Add failing tests proving composite parameters expose musical controls without automatically exposing every
  internal module parameter.

## 6. Composite Binding Resolution

- [ ] 6.1 Add failing Rust tests for resolving direct `${parameter}` references from composite instance values to
  internal module parameters.
- [ ] 6.2 Add failing Rust tests for resolving literal number, string, and boolean internal module parameter bindings.
- [ ] 6.3 Implement minimal binding parsing for literals and direct `${parameter}` references only.
- [ ] 6.4 Add failing validation tests for unknown composite references, destination parameter type mismatches, and
  unsupported expression syntax.
- [ ] 6.5 Implement deterministic composite binding resolution before graph preparation and ensure nested composite
  expansion uses namespaced resolved values without collisions.

## 7. Preset-Applied Parameter Values

- [ ] 7.1 Add failing tests proving preset-applied or patch-instance parameter values override composite defaults before
  graph preparation.
- [ ] 7.2 Implement preset-applied parameter value handling as an input layer over declared module or composite
  parameters.
- [ ] 7.3 Add failing tests proving preset-applied values cannot target undeclared parameters or bypass module/composite
  parameter validation.
- [ ] 7.4 Verify compatibility with the `add-instrument-presets` public preset-surface model without making internal
  module parameters automatically presettable.

## 8. Capability Discovery And Authoring Metadata

- [ ] 8.1 Add failing tests for querying built-in module parameter declarations as machine-readable capability metadata.
- [ ] 8.2 Add failing tests for querying composite public parameter declarations as machine-readable capability
  metadata.
- [ ] 8.3 Implement a minimal internal capability view over parameter declarations suitable for documentation, tools,
  and future LLM authoring workflows.
- [ ] 8.4 Add tests proving the capability view exposes valid YAML parameter names, types, defaults, constraints, enum
  values, units, descriptions, and static timing metadata.
- [ ] 8.5 Add tests proving capability metadata can be produced without instantiating DSP state or rendering audio.

## 9. CLI Override Developer Nicety

- [ ] 9.1 Add failing parser tests for `--set module_id.parameter=value` CLI override syntax.
- [ ] 9.2 Implement CLI override parsing as temporary developer experiment values that do not mutate source YAML files.
- [ ] 9.3 Add failing tests proving CLI overrides apply after YAML parsing and before validation/resolved graph
  preparation.
- [ ] 9.4 Add failing tests proving CLI overrides are type-checked, range-checked, enum-checked, and rejected for
  unknown module IDs or parameter names.
- [ ] 9.5 Add failing tests proving repeated overrides for the same module ID and parameter use the last value in
  command-line order deterministically.

## 10. Resolved Graph Preparation And Realtime Safety

- [ ] 10.1 Add failing tests proving graph preparation receives concrete resolved parameter values for every prepared
  module instance.
- [ ] 10.2 Ensure offline render, compiled render, and realtime render preparation share the same parameter resolution
  path.
- [ ] 10.3 Add tests or assertions proving audio callbacks do not parse YAML, resolve bindings, validate parameters,
  format diagnostics, or allocate due to parameter lookup.
- [ ] 10.4 Add deterministic render tests proving the same resolved patch, assets, render settings, and input events
  produce identical output.

## 11. Synthetic 808 Kick Acceptance Example

- [ ] 11.1 Add a `synthetic_808_kick` composite example with public parameters `tune_hz`, `decay_ms`, `punch`, and
  `click`.
- [ ] 11.2 Bind the kick composite parameters to internal oscillator, envelope, gain, click/noise, or equivalent module
  parameters using only literals and direct `${parameter}` references.
- [ ] 11.3 Add an example YAML patch or preset that tunes the kick through declarative parameter values.
- [ ] 11.4 Add an end-to-end render test proving valid YAML tuning renders without Rust DSP code changes.
- [ ] 11.5 Add validation tests proving invalid kick values produce structured diagnostics before rendering.
- [ ] 11.6 Add a deterministic render test proving valid kick parameter changes affect output deterministically.
- [ ] 11.7 Add a capability metadata test proving the kick composite's public controls are discoverable for future
  LLM-assisted authoring.

## 12. Verification

- [ ] 12.1 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [ ] 12.2 Run CMake/CTest verification if build configuration is available: `$HOME/.local/bin/cmake -S . -B build`,
  `$HOME/.local/bin/cmake --build build`, and `ctest --test-dir build`.
- [ ] 12.3 Run `openspec validate add-declarative-parameter-system --strict`.
- [ ] 12.4 Confirm every declarative-parameter requirement has implementation and test evidence before marking the
  change complete.
