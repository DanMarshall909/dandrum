## 1. Patch Preset Surface

- [x] 1.1 Add failing Rust tests for parsing patch instrument ID, preset schema version, and public preset-surface
  declarations.
- [x] 1.2 Implement patch YAML parsing for instrument preset identity and public preset targets.
- [x] 1.3 Add failing Rust validation tests for duplicate preset targets and unresolved target destinations.
- [x] 1.4 Implement patch preset-surface validation diagnostics.

## 2. Preset Document Loading

- [x] 2.1 Add failing Rust tests for parsing valid YAML preset documents and rejecting unsupported preset formats.
- [x] 2.2 Implement preset document data structures and YAML parsing.
- [x] 2.3 Add failing Rust tests for instrument ID and preset schema version compatibility.
- [x] 2.4 Implement preset compatibility validation diagnostics.

## 3. Preset Target Validation

- [x] 3.1 Add failing Rust tests for accepted declared targets, rejected unknown targets, and rejected incompatible
  values.
- [x] 3.2 Implement preset value validation against patch-declared types, defaults, constraints, and asset binding
  kinds.
- [x] 3.3 Add failing Rust tests that graph, routing, render, event, script, and scheduling fields are rejected in
  preset documents.
- [x] 3.4 Implement structural-field rejection for preset documents.

## 4. Preset Application

- [x] 4.1 Add failing Rust tests that preset values override defaults before graph construction and omitted values use
  patch defaults.
- [x] 4.2 Implement the patch-plus-preset application step before graph construction or composite expansion.
- [x] 4.3 Add deterministic render tests for rendering the same patch, preset, assets, render settings, and input events
  twice.
- [x] 4.4 Ensure preset application does not bypass routing, port compatibility, many-to-one, or feedback-boundary
  validation.

## 5. Examples And Documentation

- [x] 5.1 Create `examples/presets/` directory with at least one example preset YAML file demonstrating valid instrument
  ID, schema version, preset targets, and metadata.
- [x] 5.2 Add an engine loading test that loads the example preset against its matching patch and asserts values are
  applied.
- [x] 5.3 Document the patch preset-surface YAML schema and external preset YAML schema with the example files.

## 6. Verification

- [x] 6.1 Add a CLI or engine entry-point test for loading a patch with an external preset file.
- [x] 6.2 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [x] 6.3 Run `ctest --test-dir build` after CMake configure/build is available.
- [x] 6.4 Run `openspec validate add-instrument-presets --strict`.
