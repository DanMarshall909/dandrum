## 1. Prepare

- [ ] 1.1 Confirm target names (`Composite*` → `Module*`) don't collide with existing types (e.g. `ModuleNode`, `ModuleId`, `BuiltInModule*`); pick disambiguated names where needed and record the mapping.
- [ ] 1.2 Capture a baseline: render the composite example patches and snapshot output (or rely on existing byte-identical render tests) so behaviour can be proven unchanged after the rename.

## 2. Rename source files

- [ ] 2.1 `git mv src/rust-engine/src/patch_composite.rs src/rust-engine/src/patch_module.rs` and update its `mod`/`use` references.
- [ ] 2.2 `git mv src/rust-engine/src/graph_composite.rs src/rust-engine/src/graph_module.rs` and update its `mod`/`use` references.

## 3. Rename identifiers

- [ ] 3.1 Rename type identifiers `Composite*` → `Module*` (`CompositeInputDeclaration`, `CompositeOutputDeclaration`, `CompositeBindingDeclaration`, `CompositeMappingDirection`, `CompositeParameterValueType`, …) across the crate, keeping `BuiltInModule*` for primitives.
- [ ] 3.2 Rename functions/methods `*composite*` → `*module*` (`validate_composite_*`, `apply_composite_parameter_bindings`, `collect_recursive_composite_paths`, …) and their call sites in `patch.rs`, `graph.rs`, `ffi.rs`.
- [ ] 3.3 Verify `ffi.rs` exported/`extern "C"`/`#[no_mangle]` symbol names are unchanged (rename internal names only); diff exported symbols before/after.
- [ ] 3.4 Update comments and any internal "composite" prose to "module" / "defined module".

## 4. Tests and docs

- [ ] 4.1 Rename `composite_*` test functions to `module_*`; keep assertions unchanged.
- [ ] 4.2 Update `spec-tests.map` `rust:<fn>` ids that reference any renamed test function so the spec-coverage gate stays green.
- [ ] 4.3 Update `docs/` references from "composite" to "module" / "defined module".

## 5. Verify

- [ ] 5.1 Build green and full test suite passes with identical behaviour (byte-identical render tests hold).
- [ ] 5.2 Run the rust-coverage and spec-coverage gates; confirm both pass.
