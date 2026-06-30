## Why

`process_block`, `process_block_compiled`, and `process_block_polyphonic` in `graph_processor.rs` each contain a near-identical `match module_type` dispatch with 11 module-specific branches. Each branch reads inputs via path-specific helpers (e.g., `sum_audio_input` vs `compiled_sum_audio_input`), but the port-to-processor wiring is identical. This duplication makes every module addition require edits in three or more places, increasing risk of drift.

Compiled rendering now has two-phase voice-then-global processing, and the module dispatch appears four times total across the three paths. Extracting the shared dispatch removes the maintenance burden and makes the rendering contract explicit: "produce outputs from inputs and state."

## What Changes

- Extract per-module processing into a shared function or trait-abstracted dispatch so the `match module_type` block exists once.
- Keep routing/input access behind small helper types so the raw graph path resolves inputs via `Routing` while the compiled path resolves via `CompiledPatch::input_port_map`.
- Keep public `render_offline`, `render_offline_with_sampler_assets`, `render_offline_compiled`, and polyphonic variants unchanged.
- Keep block scheduling, MIDI seeding, output collection, and voice allocation at their current call sites.
- Do not change audio output for any existing test.

## Capabilities

### New Capabilities
- `compiled-render-dispatch-refactor`: Shared module processing dispatch extracted, removal of duplicated module-type match blocks across render paths.

### Modified Capabilities

None — no spec-level behavioral changes.

## Impact

- `src/rust-engine/src/graph_processor.rs`: reduce duplicated match blocks from four to one; add a small trait or type for abstracted input access.
- No C API, JUCE wrapper, YAML schema, dependency, or multithreading changes.
- All existing parity tests must pass with identical output.
