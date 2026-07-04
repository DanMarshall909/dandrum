## Context

Dandrum already supports YAML-defined modules, typed ports, explicit routing, composite module expansion, offline
rendering, and realtime rendering. The missing layer is a single static parameter model that lets Rust declare what can
be tuned while YAML and presets provide values before rendering begins. The same model should become the
machine-readable contract that future tools and LLM-assisted authoring flows use to discover valid controls and repair
invalid patches.

This design separates four related concepts that must not be collapsed:

- Module parameters: static configuration for one concrete built-in module type, declared by Rust and validated per
  module instance.
- Composite parameters: public controls exposed by a reusable YAML composite, declared by the composite author and bound
  to internal module parameters.
- Preset-applied parameter values: tuned values stored in YAML presets or patch instances that target declared module or
  composite parameters.
- CLI overrides: temporary developer experiment-time replacements applied after YAML parsing and before
  validation/resolved graph preparation.

The core pipeline is parameter declaration -> value collection -> validation -> default resolution -> composite
binding -> resolved patch/graph preparation. Capability discovery and repair-friendly diagnostics are first-class
outcomes of this model. CLI overrides participate as one input source, but they are a developer nicety rather than the
center of the architecture.

## Goals / Non-Goals

**Goals:**

- Let Rust define the supported static parameters for built-in module types.
- Let YAML set parameter values for module instances and composite instances without changing Rust DSP code.
- Let composites expose musical public parameters and bind them to internal module parameters using a deliberately small
  binding language.
- Apply defaults deterministically and validate all values before graph construction, composite expansion, compiled
  graph preparation, offline rendering, or realtime rendering.
- Produce structured diagnostics with stable codes, YAML paths, module IDs, parameter names, expected values, actual
  values, messages, and safe suggestions suitable for humans and future LLM repair loops.
- Keep the parameter declaration model suitable for future capability discovery by tools and LLM-assisted authoring as a
  primary architectural target.

**Non-Goals:**

- GUI editing, LLM authoring, automatic tuning, sample analysis, machine-learning optimization, host/plugin automation,
  audio-rate automation, or modulation-matrix routing.
- Arbitrary expression evaluation, arithmetic, functions, conditionals, script execution, references to module runtime
  state, or runtime mutation inside parameter bindings.
- Realtime mutation of static parameters after render preparation.
- Replacing explicit graph ports for signal-rate modulation. Static parameters configure modules before rendering;
  audio/control/event ports remain the mechanism for runtime signals.

## Decisions

### 1. Rust owns declarations; YAML owns values

**Decision:** Built-in module parameter declarations live with the Rust module registry. YAML may set values only for
declared parameters.

**Rationale:** Rust modules know which values are valid and when they must be prepared. Allowing YAML to invent
parameter names would make graph preparation nondeterministic and push module-specific validation into patch files.

**Alternative considered:** Let modules accept arbitrary key/value maps. This is flexible but weakens diagnostics,
capability discovery, and realtime preparation guarantees.

### 2. One scalar value model for static parameters

**Decision:** The first version supports number, string, boolean, and enum parameter values. Enum values are represented
as strings validated against the declaration's allowed set.

**Rationale:** These types cover static module tuning, musical controls, and preset values while remaining deterministic
to parse and easy to expose to future tools.

**Alternative considered:** Add arrays, objects, assets, or typed expressions now. Those are useful later, but they
complicate validation and are not required for the requested tuning loop.

### 3. Defaults resolve before graph preparation

**Decision:** Each declared parameter has either a default or is marked required. The resolver produces a complete
resolved parameter map for every module instance before graph preparation.

**Rationale:** DSP setup should not repeatedly ask whether a value was omitted. A complete resolved map makes module
construction and compiled render paths simpler and deterministic.

**Alternative considered:** Let each DSP module lazily read defaults when constructing state. That scatters default
behavior and makes diagnostics less consistent.

### 4. Composite parameters expose musical intent

**Decision:** Composite definitions may declare public parameters with the same scalar type/constraint vocabulary as
module parameters, but these parameters are authored in YAML and are intended to describe musical controls such as
`tune_hz`, `decay_ms`, `punch`, or `click`.

**Rationale:** Composite authors can hide internal graph details and provide a stable surface for patch authors and
presets. This aligns with the existing instrument preset direction, where public surfaces should not leak internal paths
unnecessarily.

**Alternative considered:** Expose every internal module parameter automatically. That is convenient initially but makes
composite internals part of the user-facing API and breaks presets when internals change.

### 5. Bindings are intentionally boring

**Decision:** Composite bindings support only literal number/string/boolean values and direct parameter references of
the form `${name}`. A binding resolves to exactly one scalar value before expansion.

**Rationale:** Direct references solve the immediate tuning problem while preserving determinism and realtime safety.
Expressions can be designed later with explicit syntax, diagnostics, and evaluation rules.

**Alternative considered:** Support arithmetic expressions such as `${decay_ms * 0.5}` now. This was rejected because
expression semantics, units, type coercion, and diagnostics deserve a separate change.

### 6. Capability metadata is a primary output

**Decision:** Parameter declarations are stored in a structured form that can later answer tool and LLM questions about
module types, supported parameters, types, defaults, ranges, enums, units, static timing, and examples without scraping
Rust implementation code.

**Rationale:** LLM-assisted patch authoring needs a compact, authoritative contract describing what can be written
safely. The same metadata improves documentation and validation tests.

**Alternative considered:** Treat capability discovery as a separate future model. That would risk the implementation
choosing parameter structures that are good for validation but poor for authoring tools.

### 7. CLI overrides are a late developer-nicety input layer, not a special validation path

**Decision:** CLI overrides are parsed as temporary replacements addressed by module ID plus parameter name. They apply
after YAML parsing and before validation/resolved graph preparation, then flow through the same type and constraint
validator as YAML values.

**Rationale:** This keeps developer experiments fast without creating a second parameter system. Overrides never mutate
source YAML and do not define the primary user or LLM authoring workflow.

**Alternative considered:** Validate CLI overrides separately in the CLI frontend. That would duplicate engine rules and
produce inconsistent diagnostics.

### 8. Structured diagnostics are part of the model

**Decision:** Parameter validation returns structured diagnostics with stable codes, severity, YAML path, module ID
where applicable, parameter name where applicable, expected type/range/value, actual type/value, message, and suggested
fix where safe.

**Rationale:** Good diagnostics are necessary for human patch authors now and for future LLM repair loops. Stable codes
also make tests less brittle than matching prose only.

**Alternative considered:** Return free-form strings. That is faster to implement but poor for tests, tools, and
LLM-assisted repair.

### 9. Realtime render sees prepared values only

**Decision:** The audio callback must not parse YAML, resolve bindings, validate values, allocate due to parameter
lookup, format diagnostics, or evaluate scripts/expressions for static parameters.

**Rationale:** Static parameter resolution belongs to load/preparation time. Realtime rendering should consume prepared
module state and graph structures.

**Alternative considered:** Resolve parameters on first use during processing. This risks allocations and
nondeterministic callback behavior.

## Risks / Trade-offs

- **[Risk] The scalar model may feel too limited for future complex modules** -> Keep arrays, objects, assets, and
  expressions out of scope until real use cases require a separate spec.
- **[Risk] Composite authors may duplicate constraints already declared by internal modules** -> Validate bindings
  against destination module declarations so mismatches are caught early.
- **[Risk] Direct bindings cannot derive related values such as half-decay or scaled punch** -> Require explicit
  internal module parameters or separate composite controls for now; add expressions later by a dedicated change.
- **[Risk] Existing patches may rely on ad hoc hardcoded defaults** -> Implement this additively and migrate built-ins
  incrementally with tests documenting current defaults.
- **[Risk] Capability metadata may be too sparse for useful LLM authoring** -> Include descriptions, units, defaults,
  constraints, timing metadata, and examples where possible; defer richer documentation schemas to later changes.
- **[Risk] CLI override paths may conflict with future preset target names** -> Keep CLI overrides secondary and define
  this version as `module_id.parameter` only; public preset target addressing remains governed by the preset capability.

## Open Questions

- Whether CLI overrides should eventually target public preset target names in addition to `module_id.parameter` paths
  is left for a later change.
- Whether units should be free-form strings or a closed enum is left to implementation; the spec only requires units to
  be preserved and reported consistently.
