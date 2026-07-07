## 0. Module terminology rename

- [x] 0.1 Confirm target names (`Composite*` → `Module*`) do not collide with existing types (`ModuleNode`, `ModuleId`, `BuiltInModule*`); record any disambiguated names. Disambiguated: local `composite_types` → `defined_module_types` (avoids clash with the `module_types` module) and `composite_type_name` → `module_parameter_type_name` (avoids confusion with the `module_type` field).
- [x] 0.2 `git mv src/rust-engine/src/patch_composite.rs src/rust-engine/src/patch_module.rs` and update `mod`/`use` references.
- [x] 0.3 `git mv src/rust-engine/src/graph_composite.rs src/rust-engine/src/graph_module.rs` and update `mod`/`use` references.
- [x] 0.4 Rename Rust identifiers and functions from composite terminology to module terminology (`Composite*` → `Module*`, `validate_composite_*` → `validate_module_*`, `apply_composite_parameter_bindings` → `apply_module_parameter_bindings`, etc.), keeping `BuiltInModule*` for primitives.
- [x] 0.5 Verify exported FFI/ABI symbols remain unchanged; rename Rust-internal names only. Confirmed: all `#[no_mangle]` exports remain `dandrum_engine_*`/`dandrum_patch_*`.
- [x] 0.6 Rename `composite_*` test functions and update matching `spec-tests.map` ids; keep behavior assertions unchanged. `spec-tests.map` references no Rust test-fn names (entries are keyed on spec scenario titles + fingerprints), so no map ids required changes.
- [x] 0.7 Update comments and docs to say module / defined module rather than composite, except when referring to legacy filenames or migration history. Also renamed the `composite-*.yaml` example patches to `module-*.yaml` (no public consumers yet).

## 1. Macro roots and reference resolution

- [x] 1.1 Introduce configurable macro roots (`$LIB`, `$USER_LIB`) with engine/host defaults, exposed as constants (no hardcoded literals), and a resolver that expands a `$MACRO/...` path to an absolute path. Implemented in `module_reference.rs` (`MacroRoots`, `resolve`, `LIB_MACRO`/`USER_LIB_MACRO` constants).
- [x] 1.2 Make an unknown macro a hard error during preparation, with a clear diagnostic; add path-escape (`..`) rejection so references stay within their macro root and package root. `ModuleReferenceError::{UnknownMacro,PathEscape}` with `library.unknown_macro`/`library.path_escape` diagnostics.
- [x] 1.3 Detect a `type` beginning with `$` as an external module reference (vs built-in type name / inline defined-module type) during patch parsing/preparation. `module_reference::is_external_reference`.

> Note: Section 1 delivers the pure resolution/detection layer and its diagnostics, unit-tested. Wiring these calls into the preparation pipeline lands with §2 (package loader) and §3 (plumbed macro roots), since a resolved reference is only actionable once loading and configured roots exist.

## 2. Module package format and loading

- [x] 2.1 Define the module package format (folder with entry YAML mirroring the folder name; co-located resources resolved relative to the package root) and a loader that reads a package's entry YAML. Implemented in `module_package.rs` (`ModulePackageDocument`, `load_package` enforcing the folder/entry name-mirror invariant via `ModulePackageError::NameMismatch`).
- [x] 2.2 Resolve module-internal resource paths relative to the package root so packages are portable. Package-declared `assets` paths are rebased onto the package root on injection; `..`/absolute resource paths are rejected as `ResourcePathEscape` (`library.path_escape`).
- [x] 2.3 Inject a loaded external package as an inline `module_definitions`-equivalent entry and expand it through the same defined-module expansion path (no second expansion implementation). `expand_external_references` appends each distinct reference as a `module_definitions` entry keyed by the reference string; proven graph-identical to the inline definition by `external_reference_expands_identically_to_inline_definition`.

> Note: Section 2 delivers the loader and the pure `expand_external_references(patch, roots)` injection, unit-tested end-to-end (load real package → inject → validate → build graph, asserting graph-identity with the inline definition). Calling it from the preparation pipeline is deferred to §3, which plumbs the concrete `$LIB`/`$USER_LIB` roots that the injection needs.

## 3. Version-first layout and `$LIB` seeding

- [x] 3.1 Implement version-first resolution `<root>/<version>/<module>/<module>.yaml` with coexisting versions and a `latest` alias. Implemented by resolving `$LIB/latest/...` to the highest numeric version directory while leaving pinned `$LIB/<version>/...` references stable.
- [x] 3.2 Implement the seeded standard library: canonical storage location, CRC recording, and CRC-compare-then-extract into `$LIB/<version>/` off the render path (preparation/load only), with atomic per-version extraction. Implemented in `module_library.rs`; hosts seed via `seed_standard_library`, CRC is recorded in each version directory, and extraction publishes only after a staging directory has been fully written.
- [x] 3.3 Make re-seeding additive (add version dirs, repoint `latest`, keep old versions resolvable); skip extraction when the CRC is unchanged. Implemented by seeding one version directory at a time, preserving other version directories, and using resolver-driven `latest` selection over the highest seeded numeric version.

## 4. Bundled content

- [ ] 4.1 Author the `drum_voice` module package (extracted from the existing inline definition) with any resources co-located.
- [ ] 4.2 Author a reusable multi-stereo-output `drum_machine` module package that satisfies the `drum-machine` spec's multiple-stereo-output requirement, carrying its own resources.
- [ ] 4.3 Build the seeded library archive containing the versioned default module set and wire it into the engine's canonical storage location.

## 5. Tests and docs

- [ ] 5.1 Add tests: an external `$LIB` module reference loads and expands identically to the same definition inline (byte-identical rendered output); `$USER_LIB` reference works; unknown macro and path-escape are rejected.
- [x] 5.2 Add tests: CRC-unchanged skips extraction, CRC-change re-seeds additively, pinned older versions still resolve, and `latest` follows the newest version. Added module-library seeding tests for unchanged CRC skip, changed CRC replacement, additive newer-version seeding, pinned older version presence, and resolver-driven `latest` selection.
- [ ] 5.3 Add tests proving the bundled `drum_machine` module exposes a main plus at least one additional stereo output pair and routes voices to distinct outs (closing the drum-machine multi-output acceptance criteria).
- [ ] 5.4 Update example patches to reference the shared `$LIB` modules instead of duplicating them inline (keeping at least one inline example to prove inline still works); document module packaging, versioning, macro roots, and defined-module terminology.

## 6. Verify

- [ ] 6.1 Full Rust test suite passes with identical behavior for existing inline defined-module patches.
- [ ] 6.2 Run the rust-coverage and spec-coverage gates; confirm both pass.
