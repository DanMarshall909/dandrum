## Context

The engine calls a YAML-assembled graph building block a "composite" (`patch_composite.rs`, `graph_composite.rs`, `Composite*` types), while the public YAML already uses `module_definitions` and users/specs increasingly say "module." The `add-module-library` change introduces a first-class module library, widening the gap. This change realigns internal naming with "module." It is a behavior-preserving refactor: no rendered output, public YAML, or requirements change.

## Goals / Non-Goals

**Goals:**
- Rename the internal "composite" concept to "module" (specifically "defined module") across engine source, tests, comments, and docs.
- Preserve the primitive-vs-composed distinction via an adjective (`BuiltInModule*` primitives vs "defined module" for YAML-assembled), not the noun "composite."
- Keep the change mechanical, reviewable, and free of behavior changes.

**Non-Goals:**
- No functional/behavioral changes; rendered output stays byte-identical.
- No public YAML changes (`module_definitions`, `type` unchanged).
- No spec requirement changes; the `composite-authoring` capability wording (and any capability-folder rename to `module-authoring`) is deferred to avoid conflicting with the in-flight `add-module-library` change.
- No FFI/ABI symbol changes.

## Decisions

### Naming scheme: `Composite*` → `Module*`, primitives stay `BuiltInModule*`

The YAML-assembled kind becomes a "defined module." Internal identifiers map `Composite → Module` (e.g. `CompositeInputDeclaration → ModuleInputDeclaration`, `CompositeMappingDirection → ModuleMappingDirection`, `validate_composite_* → validate_module_*`). Built-in primitives already use `BuiltInModule*` and are left as-is, so the two kinds remain distinguishable without the word "composite."

**Alternative considered:** `DefinedModule*`/`ComposedModule*` prefixes. Rejected as verbose; `Module*` + the existing `BuiltInModule*` already reads clearly given the `module_definitions` context.

**Collision check (implementation):** before renaming, confirm target names (e.g. `ModuleInputDeclaration`, `ModuleDefinition`) don't clash with existing types; disambiguate any collision at implementation time.

### File renames

`patch_composite.rs → patch_module.rs`, `graph_composite.rs → graph_module.rs`, with `mod` declarations and imports updated. Use `git mv` so history follows.

### Mechanical, compiler-driven

Rename via find/replace guided by the compiler and test suite: each rename must leave the build green and all tests passing with identical behavior. No logic edits.

### Test-map upkeep

Renamed `composite_*` test functions become `module_*`; any `spec-tests.map` `rust:<fn>` id that references a renamed test is updated in the same change so the spec-coverage gate stays green.

## Risks / Trade-offs

- **Accidental behavior change during a large rename** → Keep edits purely mechanical; rely on the full test suite (byte-identical render tests already exist) to catch any drift; no logic changes in this change.
- **FFI/ABI symbol drift** (`ffi.rs` references composite) → Only rename Rust-internal names; leave any `#[no_mangle]`/`extern "C"` symbols and their string contracts unchanged; verify with a symbol diff.
- **Coupling with `add-module-library`** (which modifies `composite-authoring`) → This change makes no spec deltas, so it cannot conflict at the spec level; if it lands first, `add-module-library` simply adopts the new identifiers. Spec-prose terminology is a deliberate follow-up.
- **Merge churn against in-flight work** → Land promptly and rebase; the rename is broad but shallow.

## Migration Plan

- Single mechanical refactor commit (or a few), each green.
- No consumer migration: public YAML and FFI contracts are unchanged.

## Open Questions

- Whether to also rename the `composite-authoring` spec capability to `module-authoring` (and reword its requirements) — deferred here; revisit after `add-module-library` lands to avoid a spec conflict.
