## Context

Patch YAML currently defines an instrument graph, including modules, ports, assets, render settings, and explicit connections. The preset problem starts after that graph exists: users need multiple named sounds for the same instrument without copying the patch or allowing preset files to secretly alter routing.

This design treats presets as validated overlays on a patch-defined public preset surface. The patch remains the source of truth for the instrument structure; presets provide values for declared public targets.

## Goals / Non-Goals

**Goals:**
- Let users create many presets for one patch-defined instrument.
- Keep presets compatible with headless loading, offline rendering, and future realtime/plugin frontends.
- Validate preset compatibility before graph construction or composite expansion applies values.
- Make preset files portable, inspectable YAML documents.
- Prevent presets from changing modules, connections, scheduling, or feedback behavior.

**Non-Goals:**
- Preset morphing, automation lanes, sequencing, bank management, or DAW-specific preset formats.
- Runtime preset switching guarantees for realtime audio callbacks.
- Inferring a preset surface automatically from every internal module parameter.

## Decisions

1. Presets are separate YAML documents that reference an instrument identity.

   A separate document keeps the base patch readable and makes preset libraries possible without duplicating instrument graphs. Embedding presets in the patch was considered, but it couples sound-library churn to graph editing and makes sharing one preset awkward.

2. The patch declares a public preset surface.

   Presets can only set named public targets declared by the patch, such as `tone.decay`, `osc.pitch`, or `snare.sample`. This mirrors composite modules exposing public parameters and asset bindings explicitly. Allowing arbitrary `module.parameter` paths was considered, but that leaks implementation details and makes internal refactors break user presets.

3. Preset application happens before graph construction or expansion.

   The loader parses and validates the patch, parses and validates the preset against the patch's preset surface, applies preset values to the patch instance model, and then constructs the graph. This keeps offline rendering deterministic and avoids a second execution path for preset values.

4. Preset compatibility uses stable instrument identity plus preset schema version.

   A patch declares an instrument ID and preset schema version. Presets must match both unless an explicit migration mechanism is added in a later change. Hashing the full graph was considered, but small non-sound-affecting patch edits would create unnecessary incompatibility.

5. Presets may override values, not structure.

   Preset documents may set declared scalar parameters, asset binding selections, and human-facing metadata. They must not declare modules, connections, render settings, events, scripts, or graph topology. This preserves the architectural boundary that patches define instruments and presets define named sounds for those instruments.

## Risks / Trade-offs

- Public preset-surface declarations add authoring work -> Keep the schema compact and allow defaults/constraints to live beside each exposed target.
- Instrument authors may need to rename targets -> Stable target IDs should be treated as user-facing API; future migration support can handle intentional renames.
- Separate preset files require load ordering -> Engine APIs should accept patch plus optional preset in one load request and report combined diagnostics.
- Schema version matching is strict -> This avoids accidental wrong-sound loads now; migrations can be proposed once real preset libraries exist.
