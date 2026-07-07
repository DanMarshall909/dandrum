## Context

Dandrum already expands YAML-defined modules declared inline in a patch's `module_definitions` into flat primitive graphs, with public ports and `preset_surface` parameters. The existing authoring spec fixed inline definitions as the canonical model and deferred external/shared loading as a future extension that must behave identically to an inline `module_definitions` entry after loading.

This change realizes that extension as a **module library**: reusable module packages referenced by path from any patch, resolved during preparation and fed into the existing expansion path. It also gives Dandrum a dependable, versioned standard library (`$LIB`) that authors can build on reproducibly.

This is also the right boundary to finish the vocabulary cleanup. User-facing YAML already says `module_definitions`, patch authors already write modules, and the module library makes "composite" actively misleading. The YAML-assembled kind becomes a **defined module** when it must be distinguished from a primitive/built-in module.

## Goals / Non-Goals

**Goals:**
- Define a reusable module package format (folder + entry YAML + co-located resources).
- Reference a module package from a patch by a macro-qualified, version-pinned file path, expanded identically to an inline `module_definitions` entry.
- Ship an immutable standard library (`$LIB`) seeded from a single CRC-tracked archive, plus a mutable `$USER_LIB`.
- Support coexisting versions with a `latest` alias for reproducible builds.
- Bundle a `drum_voice` module and a reusable multi-stereo-output `drum_machine` module as first content.
- Rename internal `composite` terminology to module/defined-module terminology across engine source, tests, comments, docs, and spec prose.

**Non-Goals:**
- No public YAML field rename (`module_definitions` and `type` remain unchanged).
- No name registry, fuzzy resolution, or dependency solver — references are exact pinned files.
- No user-facing package publishing/registry beyond local `$LIB`/`$USER_LIB` directories.
- No runtime mutation of `$LIB` (it is regenerated from the seed).
- No changes to how expanded graphs render — only how their definitions are sourced and named.
- No FFI/ABI symbol changes.

## Decisions

### Reference form: macro-qualified, version-pinned file path

A module `type` MAY be a path of the form `$MACRO/<version>/<module>/<module>.yaml` (e.g. `type: $LIB/1.3.9/drum_voice/drum_voice.yaml`). Chosen over a name registry or bare `type: drum_voice` because it is unambiguous by construction, greppable, and reproducible. A built-in type name or an inline defined-module type continues to work unchanged; the leading `$` is the discriminator that triggers library resolution.

**Alternative considered:** register library modules by name and reference bare `type: drum_voice`. Rejected because it reintroduces collisions, ambiguity, and hidden resolution order.

### Two macro roots: `$LIB` and `$USER_LIB`

Macros resolve to directories. `$LIB` is the extraction target of the shipped seed and is treated as immutable. `$USER_LIB` is the user's mutable directory. An unknown macro is a hard error — no silent fallback, so a typo fails loudly rather than resolving somewhere surprising. Macro roots are engine/host-provided configuration with built-in defaults.

**Alternative considered:** a single auto-discovered modules directory. Rejected because the immutable/mutable split is what enables reliable defaults, and explicit macros beat magic discovery.

### Seed archive + CRC refresh

The full default module set ships as one versioned seed archive stored in a canonical location separate from `$LIB` (embedded or install path). On startup/preparation the engine compares the seed's CRC against the extracted state; on mismatch it re-extracts into `$LIB/<version>/`, adding new version directories and repointing `latest` without removing existing versions. Identical CRC means skip. This keeps `$LIB` a pure, verifiable derivative of the immutable seed and makes upgrades additive so old pinned references keep resolving.

### Version-first layout with `latest` alias

`$LIB/<version>/<module>/<module>.yaml`. Versions live above modules so a release is one coherent set, mirroring the seed. `latest` is an alias to the newest version for floating references; pinning a concrete version (`1.3.9`) is the reproducible path.

### Module = self-contained folder package

Each module is a folder whose entry YAML mirrors the folder name, plus co-located resources. Resource paths referenced inside the module resolve relative to the package root, so a package drops in anywhere.

### Naming scheme: defined modules vs primitive/built-in modules

The YAML-assembled kind becomes a **defined module**. Internal identifiers map `Composite` to `Module` where clear, with disambiguation only where required by existing names. Primitive implementations keep `BuiltInModule*`, so the primitive-vs-defined distinction remains explicit without using the word "composite."

Source files follow the same rule: `patch_composite.rs` becomes `patch_module.rs`, and `graph_composite.rs` becomes `graph_module.rs`. The rename is mechanical and behavior-preserving.

### Integration point: resolve during preparation, reuse existing expansion

Macro/version resolution and package loading happen during patch preparation, before graph compilation. A resolved package's entry YAML is loaded and injected as if it were an inline `module_definitions` entry, then flows through the same expansion code. This guarantees the spec's "behaves identically to inline" requirement and avoids a second expansion implementation.

## Risks / Trade-offs

- **Seed extraction on the realtime/prepare path** → Do CRC check + extraction only during preparation/load; never touch `$LIB` from the render path.
- **Path traversal / escaping the package or library root** → Resolve and validate all paths stay within the intended macro root / package root; reject escapes with a clear error.
- **Stale or partially-written `$LIB`** → Treat extraction as atomic per version dir; re-seed on CRC mismatch.
- **Version-directory sprawl** → Acceptable for reproducibility; pruning old versions is a future concern, out of scope.
- **`latest` reproducibility footgun** → Document that `latest` is floating; pinned versions are the reproducible form.
- **Accidental behavior change during rename** → Keep rename edits mechanical and rely on render equivalence tests plus the full suite.
- **FFI/ABI symbol drift** → Rename Rust-internal names only; leave exported symbols and contracts unchanged.

## Migration Plan

- Inline `module_definitions` remains fully supported — this is additive and does not require user YAML migration.
- Existing example patches MAY be updated to reference the shared `$LIB` modules; keep at least one inline example to prove inline defined modules still work.
- Rename internal files, identifiers, tests, and docs in the same implementation sequence so the module-library work is built on the final vocabulary.

## Open Questions

- Exact canonical storage location/mechanism for the seed archive and the on-disk `$LIB` extraction root — to be finalized in implementation against the host/plugin packaging.
- Whether `$USER_LIB` references require version directories or allow a flat `<module>/<module>.yaml` form (leaning: versions optional for `$USER_LIB`, required for `$LIB`).