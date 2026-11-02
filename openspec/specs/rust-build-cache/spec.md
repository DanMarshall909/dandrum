# Rust Build Cache

## Purpose

Keep Rust development warm while bounding managed build storage and preserving current verification evidence.

## Requirements

### Requirement: Compatible builds retain warm intermediates

Maintained Rust build and test entry points SHALL reuse compatible compiled dependencies across output owners while preserving incremental development and correct final artifacts.

#### Scenario: Unchanged and small-edit turnaround
- **WHEN** a real build is repeated unchanged and then one application source value is edited
- **THEN** the unchanged build reuses its compiled artifacts, the edited build produces the new behavior, and unchanged dependencies remain reusable

#### Scenario: Product variants remain independently correct
- **WHEN** different product configurations or checkouts use the shared cache in succession
- **THEN** each published artifact matches its requested source and configuration, including when an earlier output has a newer timestamp

### Requirement: Coverage remains a fresh independent measurement

Coverage commands SHALL use separate instrumented storage and measure the current source without combining previous-run profiles with the new result.

#### Scenario: Coverage does not contaminate normal reuse
- **WHEN** a coverage run follows an ordinary build and is repeated after a source change
- **THEN** ordinary compiled artifacts remain reusable and the new coverage result belongs to the changed source

### Requirement: Managed cache retention is bounded and preserves active work

Managed producers SHALL measure retained storage and free space before execution, retire only obsolete owned inactive generations, and refuse growth that cannot fit the configured allowance. Failed or interrupted production SHALL preserve active cache state and report its outcome. Actual completion growth SHALL be measured; an over-budget result SHALL prevent further growth until resolved.

#### Scenario: Obsolete generations can be retired
- **WHEN** a new compiler or dependency generation needs capacity and an old owned generation is inactive
- **THEN** the old generation can be removed while current warm state and retained evidence remain intact

#### Scenario: Current warm state cannot fit the budget
- **WHEN** the current generation plus its growth allowance exceeds the retention or free-space budget
- **THEN** the producer does not start, the current cache remains intact, and the diagnostic identifies the capacity conflict

#### Scenario: Active or uncertain paths are preserved
- **WHEN** cleanup encounters an active lease, interrupted producer, unknown ownership or linked/replaced cache path
- **THEN** it preserves that state and refuses destructive cleanup

### Requirement: Repeated verification does not accumulate source and fixture generations

Source-free verification SHALL reconstruct a clean source snapshot at a stable exclusively owned path. Native fixture publication SHALL preserve the current binding and required evidence while retiring superseded inactive owned output.

#### Scenario: Repeated source-free verification uses clean stable inputs
- **WHEN** source-free verification runs again after a source file is removed
- **THEN** the removed file and previous CMake state are absent, the source path remains stable, and actual source-free build and runtime checks still execute

#### Scenario: Native fixture replacement retains the current proof
- **WHEN** a new native fixture is published successfully
- **THEN** its plugin, resources and test executable match the recorded hashes, superseded owned output does not accumulate, and failed publication preserves the previous usable fixture and its evidence
