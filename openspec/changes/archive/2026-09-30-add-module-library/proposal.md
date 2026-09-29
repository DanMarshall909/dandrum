## Why

YAML-assembled modules like `drum_voice` can only be defined inline inside each patch's `module_definitions`, so they are copied between patches with no way to define one once and reuse it reliably. The existing authoring spec deferred external/shared loading as a future extension; this change builds it so Dandrum can ship a dependable set of reusable modules that others build on top of reproducibly.

The same change also finishes the terminology cleanup: the user-facing noun is **module**, while the engine and some spec prose still call YAML-assembled modules "composites." A module library would make that split worse unless the rename lands with it.

## What Changes

- Introduce a **module library**: reusable, versioned module packages referenced from any patch, loaded and expanded identically to an inline `module_definitions` entry.
- Add `$LIB` for the immutable seeded standard library and `$USER_LIB` for the user's mutable module directory. Unknown macros are hard errors.
- Use a version-first layout with coexisting versions and a `latest` alias: `$LIB/<version>/<module>/<module>.yaml`.
- Treat each reusable module as a folder package whose entry YAML mirrors the folder name and whose resources resolve relative to the package root.
- Reference modules by exact macro-qualified file paths such as `type: $LIB/1.3.9/drum_voice/drum_voice.yaml`. There is no name registry, bare-name lookup, or collision rule.
- Rename the YAML-assembled concept from **composite** to **module** in internal Rust identifiers, source filenames, comments, docs, and tests. Use **defined module** when it must be distinguished from a primitive/built-in module.
- Keep public YAML stable: `module_definitions` and `type` remain unchanged, and inline `module_definitions` continue to work.
- First bundled content: a `drum_voice` module and a reusable multi-stereo-output `drum_machine` module.

## Capabilities

### New Capabilities
- `module-library`: reusable versioned module packages, `$LIB`/`$USER_LIB` path macros, a CRC-refreshed seeded standard library, and reference-by-pinned-file resolution.
- `module-authoring`: YAML-assembled graph building blocks are **defined modules**, distinguished from primitive/built-in modules by category rather than by the term "composite".

### Modified Capabilities
- `yaml-patch-format`: a module `type` MAY be a macro-qualified path reference to an external module package, in addition to a built-in type name or an inline defined-module type.
- `composite-authoring`: legacy capability name only for now; prose is updated to module terminology while preserving existing semantics.

## Impact

- Engine: patch loading/preparation resolves `$LIB`/`$USER_LIB`, loads referenced module packages, and expands them through the same path as inline `module_definitions`.
- Engine rename: `patch_composite.rs`/`graph_composite.rs` become `patch_module.rs`/`graph_module.rs`; `Composite*` identifiers and `*composite*` functions become module equivalents. `BuiltInModule*` names stay unchanged.
- FFI: exported/ABI symbols remain stable; only Rust-internal names change.
- Tests: renamed test functions and `spec-tests.map` ids are updated where needed.
- Bundled content: the seeded library includes `drum_voice` and `drum_machine` module packages.
- Docs: update module packaging, versioning, macro roots, and defined-module terminology.