## Why

The user-facing noun for a reusable graph building block is "module" (a patch author writes `type: …` and, for YAML-assembled ones, `module_definitions`). The engine internals and spec prose still call the YAML-assembled kind a "composite," creating a terminology split that will only worsen as the `add-module-library` work introduces a first-class "module library." Renaming "composite" → "module" internally makes the code match the vocabulary users and specs will use.

## What Changes

- Rename internal Rust identifiers from `Composite*` to `Module*` for the YAML-defined (composed) module concept: types (e.g. `CompositeInputDeclaration` → `ModuleInputDeclaration`), functions (e.g. `validate_composite_*` → `validate_module_*`, `apply_composite_parameter_bindings` → `apply_module_parameter_bindings`), and the source files `patch_composite.rs` → `patch_module.rs`, `graph_composite.rs` → `graph_module.rs`.
- Keep the primitive-vs-composed distinction as an **adjective**, not the noun "composite": primitives stay `BuiltInModule*`; the YAML-assembled kind is a **defined module** (defined via the existing `module_definitions` field).
- Update code comments, `docs/`, and internal "composite" prose to "module" / "defined module".
- Update any `spec-tests.map` test ids that reference renamed test functions.
- **No behavior changes.** Public YAML (`module_definitions`, `type`) is unaffected; rendered output is identical. This is a pure refactor.

## Capabilities

### New Capabilities
- `module-authoring`: pins the naming/terminology contract — YAML-assembled graph building blocks are "defined modules," distinguished from primitive/built-in modules by category rather than by the term "composite" — and requires the rename to be behavior-preserving.

### Modified Capabilities
<!-- none. The existing `composite-authoring` capability is deliberately NOT touched here so this refactor does not conflict with the in-flight `add-module-library` change (which modifies `composite-authoring`). Folding `composite-authoring`'s prose into `module-authoring` is a follow-up once both changes land. -->

## Impact

- Engine source: `patch_composite.rs`/`graph_composite.rs` renamed and their `Composite*` identifiers renamed to `Module*`; call sites in `patch.rs`, `graph.rs`, `ffi.rs`, and tests updated.
- FFI: verify no exported/ABI symbol names change (rename internal names only; keep any C-facing symbols stable).
- Tests: `composite_*` test function names renamed to `module_*`; `spec-tests.map` entries referencing them updated.
- Sequencing: recommended to land **before** `add-module-library` so that change is built with the new vocabulary; if it lands after, the module-library code adopts the new names directly.
