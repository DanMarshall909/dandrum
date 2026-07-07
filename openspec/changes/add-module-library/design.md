## Context

Dandrum already expands YAML-defined modules ("composites") declared inline in a patch's `module_definitions` into flat primitive graphs (`patch_composite.rs` / `graph_composite.rs`), with public ports and `preset_surface` parameters. The `composite-authoring` spec fixed inline definitions as the canonical model and deferred external/shared loading as a future extension that must "behave identically to an inline `module_definitions` entry after loading."

This change realizes that extension as a **module library**: reusable module packages referenced by path from any patch, resolved during preparation and fed into the existing expansion path. It also gives Dandrum a dependable, versioned standard library (`$LIB`) that authors can build on reproducibly.

Terminology: user-facing, these are **modules**. The internal `composite` naming is left untouched here; a `composite → module` rename is a recommended separate change.

## Goals / Non-Goals

**Goals:**
- Define a reusable module package format (folder + entry YAML + co-located resources).
- Reference a module package from a patch by a macro-qualified, version-pinned file path, expanded identically to an inline `module_definitions` entry.
- Ship an immutable standard library (`$LIB`) seeded from a single CRC-tracked zip, plus a mutable `$USER_LIB`.
- Support coexisting versions with a `latest` alias for reproducible builds.
- Bundle a `drum_voice` module and a reusable multi-stereo-output `drum_machine` module as first content.

**Non-Goals:**
- No internal rename of `composite` → `module` (separate change).
- No name registry, fuzzy resolution, or dependency solver — references are exact pinned files.
- No user-facing package publishing/registry beyond local `$LIB`/`$USER_LIB` directories.
- No runtime mutation of `$LIB` (it is regenerated from the seed).
- No changes to how expanded graphs render — only how their definitions are sourced.

## Decisions

### Reference form: macro-qualified, version-pinned file path

A module `type` MAY be a path of the form `$MACRO/<version>/<module>/<module>.yaml` (e.g. `type: $LIB/1.3.9/drum_voice/drum_voice.yaml`). Chosen over a name registry or bare `type: drum_voice` because it is unambiguous by construction (no collision rules), greppable, and reproducible. A built-in type name or an inline composite type continues to work unchanged; the leading `$` is the discriminator that triggers library resolution.

**Alternative considered:** register library modules by name and reference bare `type: drum_voice`. Rejected — reintroduces collisions/ambiguity and hidden resolution order.

### Two macro roots: `$LIB` (immutable, seeded) and `$USER_LIB` (mutable)

Macros resolve to directories. `$LIB` is the extraction target of the shipped seeding zip and is treated as immutable (regenerated from the seed, never hand-edited). `$USER_LIB` is the user's mutable directory. An unknown macro is a hard error — no silent fallback, so a typo fails loudly rather than resolving somewhere surprising. Macro roots are engine/host-provided configuration with built-in defaults.

**Alternative considered:** a single auto-discovered composites directory. Rejected — the immutable/mutable split is what enables "build reliably on top of the defaults," and explicit macros beat magic discovery.

### Seeding zip + CRC refresh

The full default module set ships as one versioned zip stored in a canonical location separate from `$LIB` (embedded or install path). On startup/preparation the engine compares the zip's CRC against the extracted state; on mismatch it re-extracts into `$LIB/<version>/`, adding new version directories and repointing `latest` without removing existing versions. Identical CRC = skip. This keeps `$LIB` a pure, verifiable derivative of the immutable seed and makes upgrades additive so old pinned references keep resolving.

### Version-first layout with `latest` alias

`$LIB/<version>/<module>/<module>.yaml`. Versions live above modules so a release is one coherent set, mirroring the seed. `latest` is an alias to the newest version for floating references; pinning a concrete version (`1.3.9`) is the reproducible path.

### Module = self-contained folder package

Each module is a folder whose entry YAML mirrors the folder name, plus co-located resources (e.g. sample WAVs). Resource paths referenced inside the module resolve relative to the package root, so a package drops in anywhere. The bundled 909 metallic voices carry their sample resources inside their own packages rather than in the shared `examples/assets/` location.

### Integration point: resolve during preparation, reuse existing expansion

Macro/version resolution and package loading happen during patch preparation, before graph compilation. A resolved package's entry YAML is loaded and injected as if it were an inline `module_definitions` entry, then flows through the existing composite-expansion code unchanged. This guarantees the spec's "behaves identically to inline" requirement and avoids a second expansion implementation.

## Risks / Trade-offs

- **Seed extraction on the realtime/prepare path** → Do CRC check + extraction only during preparation/load (off the audio callback); never touch `$LIB` from the render path.
- **Path traversal / escaping the package or library root** (e.g. `../` in references or internal resource paths) → Resolve and validate all paths stay within the intended macro root / package root; reject escapes with a clear error.
- **Stale or partially-written `$LIB`** (interrupted extraction) → Treat extraction as atomic per version dir (extract to temp, then rename); re-seed on CRC mismatch.
- **Version-directory sprawl** as releases accumulate → Acceptable for reproducibility; pruning old versions is a future concern, out of scope.
- **`latest` reproducibility footgun** → Document that `latest` is floating; pinned versions are the reproducible form.

## Migration Plan

- Inline `module_definitions` remains fully supported — this is purely additive, no migration required.
- Existing example patches MAY be updated to reference the shared `$LIB` modules; the duplicated inline `drum_voice` definitions can be removed in a follow-up once the library path is proven.

## Open Questions

- Exact canonical storage location/mechanism for the seeding zip (embedded resource vs install path) and the on-disk `$LIB` extraction root — to be finalized in implementation against the host/plugin packaging.
- Whether `$USER_LIB` references require version directories or allow a flat `<module>/<module>.yaml` (leaning: versions optional for `$USER_LIB`, required for `$LIB`).
