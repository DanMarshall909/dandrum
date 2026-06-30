## Context

`graph_processor.rs` contains three block-processing functions: `process_block` (raw monophonic), `process_block_compiled` (compiled monophonic), and `process_block_polyphonic` (raw polyphonic). Each contains a full 11-arm `match module_type` dispatch that reads module-specific inputs and calls the corresponding processor function.

The compiled path (`process_block_compiled`) was recently refactored to use a shared closure for its voice and global phases. However, the module dispatch logic is still duplicated across the three paths — four times total when counting the polyphonic voice and global loops separately. The match arms differ only in which input-gathering helper is called (`sum_audio_input(routing, ...)` vs `compiled_sum_audio_input(compiled, ...)`). The port names, processor calls, and output structure are identical.

## Goals / Non-Goals

**Goals:**
- Extract the per-module processing dispatch into a single shared function so the `match module_type` block exists once.
- Abstract the input-gathering behind a trait so the raw graph path uses `Routing`-based resolution and the compiled path uses `CompiledPatch::input_port_map`.
- Keep public render entry points (`render_offline`, `render_offline_with_sampler_assets`, `render_offline_compiled`, `render_offline_polyphonic`, `render_offline_with_sampler_assets_polyphonic`) and their signatures unchanged.
- Keep block scheduling, MIDI event seeding, voice allocation, and output collection at their current call sites.
- Verify parity: all existing compiled/raw parity tests produce identical output.

**Non-Goals:**
- Do not change the `Routing` data structure or `CompiledPatch` data structure.
- Do not change JUCE, FFI, YAML, dependencies, threading, or polyphony voice allocation.
- Do not optimise buffer allocation or introduce new allocation patterns.
- Do not change the public API surface.

## Decisions

### Decision: Trait-based abstraction for input reading

**Decision**: Define a `ModuleInputProvider` trait with methods for each input-reading pattern used by the module dispatch:
- `gather_event_inputs`, `sum_audio_input`, `sum_control_input`, `control_input_or_default`

Implement the trait for `&Routing` (raw graph) and a new `CompiledInputProvider` wrapper around `&CompiledPatch`. A shared `process_module` function accepts `&impl ModuleInputProvider` and contains the single `match module_type` dispatch.

**Rationale**: The trait makes the abstraction explicit and the compiler enforces that all input patterns are covered for each code path. Refactoring is purely mechanical — each existing call site is replaced with the shared function.

Alternatives considered:
- Closure-based injection: more flexible but less structured — each call site must construct the same four closures.
- Macro-based code generation: hides the abstraction and makes debugging harder.
- Enum dispatch (`InputSource::Raw(Routing) | InputSource::Compiled(CompiledPatch)`): creates a coupling between the two input strategies in a single type, and adds branches inside every helper call.

### Decision: `CompiledInputProvider` lives in `graph_processor.rs`

**Decision**: The `CompiledInputProvider` struct and its trait implementation reside in `graph_processor.rs`, not in `compiled_patch.rs`.

**Rationale**: The trait is internal to the render path. Keeping it in `graph_processor.rs` avoids adding a reverse dependency from `compiled_patch.rs` to the processor module and keeps all rendering concerns in one file during this refactor.

### Decision: Extract `collect_audio_output` helper

**Decision**: Extract the identical output-collection block (reading `LEFT`/`RIGHT` from `audio_output` in `all_outputs` and extending `left_out`/`right_out`) into a shared `collect_audio_output` function.

**Rationale**: This block is identical across all three block-processing functions. Extracting it removes a third source of duplication. It is purely mechanical with no behavioral change.

## Risks / Trade-offs

- **[Risk] Trait dispatch adds a small indirection at each module processing call** → Mitigation: the dispatch is monomorphized by the compiler (zero-cost abstraction with `impl trait`).
- **[Risk] Extracting the dispatch changes the module processing order** → Mitigation: the extracted function is called from the same iteration loops, in the same order. The `process_module` function has no side effects beyond mutating `states[module_idx]` and returning `ModuleOutputs`.
- **[Risk] The refactor is large enough to introduce subtle bugs** → Mitigation: rely on existing parity tests (oscillator, MIDI voice, sampler, voice-to-global) as regression guards. Add a monophonic raw-vs-compiled parity test for the full MIDI voice chain if coverage is insufficient.
