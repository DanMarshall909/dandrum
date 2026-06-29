## ADDED Requirements

### Requirement: Shared module processing dispatch

The three block-processing functions SHALL use a single shared per-module dispatch function. Module-specific input resolution SHALL be abstracted behind a trait so each render path provides its own input provider without duplicating the match arm logic.

#### Scenario: Raw graph rendering uses routing-based input resolution

- **WHEN** `process_block` processes a module via the shared dispatch
- **THEN** inputs are resolved using `Routing` (string-keyed hash maps)

#### Scenario: Compiled rendering uses index-based input resolution

- **WHEN** `process_block_compiled` processes a module via the shared dispatch
- **THEN** inputs are resolved using `CompiledPatch::input_port_map` (index-based `CompiledPortRef` vectors)

#### Scenario: Output is identical after refactor

- **WHEN** any graph is rendered through both raw and compiled paths
- **THEN** the left/right buffers SHALL be identical to before the refactor, as verified by existing parity tests
