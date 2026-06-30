## Why

Once a patch YAML document defines an instrument, users need a stable way to save and recall named sounds for that exact instrument without duplicating or editing the graph. Presets should capture performable variations of the instrument contract while preserving deterministic loading and validation.

## What Changes

- Add instrument preset documents that reference a specific patch-defined instrument and apply named parameter, asset, and metadata values.
- Allow a patch YAML document to declare the public preset surface for an instrument: which parameters and asset bindings may be set by presets, including defaults and validation constraints.
- Add validation that rejects presets for the wrong instrument identity, unknown preset targets, incompatible values, and attempts to change the module graph or routing.
- Preserve deterministic rendering by making preset application an explicit, validated load step before graph construction or expansion.

## Capabilities

### New Capabilities
- `instrument-presets`: Defines how named preset documents target a patch-defined instrument, apply allowed values, and are validated and loaded.

### Modified Capabilities
- `yaml-patch-format`: Adds the patch-level declaration of an instrument's public preset surface and stable identity needed for preset compatibility.

## Impact

- Rust engine patch loading and validation will gain preset schema parsing and preset application before graph construction.
- YAML patch schema will gain instrument identity and preset-surface declarations.
- Offline rendering and future realtime/plugin frontends can load a patch plus a preset name or preset file while sharing the same validated engine path.
