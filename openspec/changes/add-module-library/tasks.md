## 1. Macro roots and reference resolution

- [ ] 1.1 Introduce configurable macro roots (`$LIB`, `$USER_LIB`) with engine/host defaults, exposed as constants (no hardcoded literals), and a resolver that expands a `$MACRO/...` path to an absolute path.
- [ ] 1.2 Make an unknown macro a hard error during preparation, with a clear diagnostic; add path-escape (`..`) rejection so references stay within their macro root and package root.
- [ ] 1.3 Detect a `type` beginning with `$` as an external module reference (vs built-in type name / inline composite type) during patch parsing/preparation.

## 2. Module package format and loading

- [ ] 2.1 Define the module package format (folder with entry YAML mirroring the folder name; co-located resources resolved relative to the package root) and a loader that reads a package's entry YAML.
- [ ] 2.2 Resolve module-internal resource (e.g. sample) paths relative to the package root so packages are portable.
- [ ] 2.3 Inject a loaded external package as an inline `module_definitions`-equivalent entry and expand it through the existing composite-expansion path (no second expansion implementation).

## 3. Version-first layout and `$LIB` seeding

- [ ] 3.1 Implement version-first resolution `<root>/<version>/<module>/<module>.yaml` with coexisting versions and a `latest` alias.
- [ ] 3.2 Implement the seeding zip: canonical storage location, CRC recording, and CRC-compare-then-extract into `$LIB/<version>/` off the render path (preparation/load only), with atomic per-version extraction.
- [ ] 3.3 Make re-seeding additive (add version dirs, repoint `latest`, keep old versions resolvable); skip extraction when the CRC is unchanged.

## 4. Bundled content

- [ ] 4.1 Author the `drum_voice` module package (extracted from the existing inline composite) with any resources co-located.
- [ ] 4.2 Author a reusable multi-stereo-output `drum_machine` module package that satisfies the `drum-machine` spec's multiple-stereo-output requirement, carrying its own sample resources.
- [ ] 4.3 Build the seeding zip containing the versioned default module set and wire it into the engine's canonical storage location.

## 5. Tests and docs

- [ ] 5.1 Add tests: an external `$LIB` module reference loads and expands identically to the same definition inline (byte-identical rendered output); `$USER_LIB` reference works; unknown macro and path-escape are rejected.
- [ ] 5.2 Add tests: CRC-unchanged skips extraction, CRC-change re-seeds additively, pinned older versions still resolve, and `latest` follows the newest version.
- [ ] 5.3 Add tests proving the bundled `drum_machine` module exposes a main plus at least one additional stereo output pair and routes voices to distinct outs (closing the drum-machine multi-output acceptance criteria).
- [ ] 5.4 Update example patches to reference the shared `$LIB` modules instead of duplicating them inline (keeping at least one inline example to prove inline still works); document module packaging, versioning, and the macro roots.
