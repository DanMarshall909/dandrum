## Why

YAML-assembled modules (e.g. `drum_voice`) can only be defined inline inside each patch's `module_definitions`, so they are copy-pasted between patches (`drum_voice` is duplicated in `composite-drum-voice.yaml` and `event-routing-drum-machine.yaml`) with no way to define one once and reuse it reliably. The `composite-authoring` spec explicitly deferred this as "External composite libraries are optional future extension"; this change builds it, so Dandrum can ship a dependable set of reusable modules that others build on top of reproducibly.

## What Changes

- Introduce a **module library**: reusable, versioned module packages referenced from any patch, loaded and expanded identically to an inline `module_definitions` entry.
- Add two path-macro roots resolvable in a patch's module `type` reference:
  - `$LIB` — immutable standard library; the directory a single shipped **seeding zip** (stored elsewhere) is extracted into. The engine tracks the seeding zip's CRC and re-seeds `$LIB` when it changes.
  - `$USER_LIB` — the user's mutable module directory (no seed, no refresh).
  - An unknown macro is a hard error (no silent fallback).
- **Version-first layout** with coexisting versions and a `latest` alias: `$LIB/<version>/<module>/<module>.yaml` (e.g. `$LIB/1.3.9/drum_voice/drum_voice.yaml`, `$LIB/latest/drum_voice/drum_voice.yaml`).
- **Module = folder package**: a folder whose entry YAML name mirrors the folder, plus co-located resources; internal asset paths resolve relative to the package root so packages are portable.
- **Reference always pins the exact versioned file** via `type: $LIB/<version>/<module>/<module>.yaml` — no name registry, no bare names, no collision rules; reproducible by construction.
- Terminology: these are user-facing **modules**. (An internal rename of "composite" → "module" is recommended as a separate change and is **out of scope** here.)
- First bundled content: a `drum_voice` module and a reusable multi-stereo-output `drum_machine` module (which also satisfies the existing `drum-machine` multi-output requirement).

## Capabilities

### New Capabilities
- `module-library`: reusable versioned module packages, `$LIB`/`$USER_LIB` path macros, a CRC-refreshed seeding zip extracted into `$LIB`, and reference-by-pinned-file resolution that expands identically to inline `module_definitions`.

### Modified Capabilities
- `composite-authoring`: realizes the previously-deferred "External composite libraries are optional future extension" requirement — external module packages now load and behave identically to inline definitions.
- `yaml-patch-format`: a module `type` MAY be a macro-qualified path reference to an external module package (`$LIB`/`$USER_LIB`), in addition to a built-in type name or an inline composite type.

## Impact

- Engine: patch loading/preparation resolves `$LIB`/`$USER_LIB` macros, extracts/refreshes the seeding zip by CRC, loads the referenced module package, and expands it through the existing composite-expansion path.
- New bundled artifact: the seeding zip and its canonical storage/extraction location; the `drum_voice` and `drum_machine` module packages (the latter closing the `drum-machine` spec's unbuilt multi-output requirement) with co-located sample resources.
- Existing example patches MAY be updated to reference the shared modules instead of duplicating them inline (backward compatible — inline `module_definitions` continues to work).
- Docs: authoring guide for module packages, versioning, and the macro roots.
