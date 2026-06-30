## 1. Patch Preset Surface

- [ ] 1.1 Add failing Rust tests for parsing patch instrument ID, preset schema version, and public preset-surface declarations.
- [ ] 1.2 Implement patch YAML parsing for instrument preset identity and public preset targets.
- [ ] 1.3 Add failing Rust validation tests for duplicate preset targets and unresolved target destinations.
- [ ] 1.4 Implement patch preset-surface validation diagnostics.

## 2. Preset Document Loading

- [ ] 2.1 Add failing Rust tests for parsing valid YAML preset documents and rejecting unsupported preset formats.
- [ ] 2.2 Implement preset document data structures and YAML parsing.
- [ ] 2.3 Add failing Rust tests for instrument ID and preset schema version compatibility.
- [ ] 2.4 Implement preset compatibility validation diagnostics.

## 3. Preset Target Validation

- [ ] 3.1 Add failing Rust tests for accepted declared targets, rejected unknown targets, and rejected incompatible values.
- [ ] 3.2 Implement preset value validation against patch-declared types, defaults, constraints, and asset binding kinds.
- [ ] 3.3 Add failing Rust tests that graph, routing, render, event, script, and scheduling fields are rejected in preset documents.
- [ ] 3.4 Implement structural-field rejection for preset documents.

## 4. Preset Application

- [ ] 4.1 Add failing Rust tests that preset values override defaults before graph construction and omitted values use patch defaults.
- [ ] 4.2 Implement the patch-plus-preset application step before graph construction or composite expansion.
- [ ] 4.3 Add deterministic render tests for rendering the same patch, preset, assets, render settings, and input events twice.
- [ ] 4.4 Ensure preset application does not bypass routing, port compatibility, many-to-one, or feedback-boundary validation.

## 5. Frontend And Verification

- [ ] 5.1 Add a CLI or engine entry-point test for loading a patch with an external preset file.
- [ ] 5.2 Document the patch preset-surface YAML and external preset YAML examples.
- [ ] 5.3 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [ ] 5.4 Run `ctest --test-dir build` after CMake configure/build is available.
- [ ] 5.5 Run `openspec validate add-instrument-presets --strict`.
