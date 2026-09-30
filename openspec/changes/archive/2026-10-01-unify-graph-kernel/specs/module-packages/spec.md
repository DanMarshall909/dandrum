## ADDED Requirements

### Requirement: Package entries are graph definitions

Module package entry documents SHALL load through the unified graph-definition parser as provenance-bearing kernel graph definitions and SHALL expose the same static parameters and ports as inline definitions. Nested package references SHALL resolve through the same preparation context. Package loading SHALL NOT require the legacy patch expansion, module-parameter model, or `asset_bindings` mechanism.

#### Scenario: Packaged definition instantiates like an inline definition

- **WHEN** a graph node references a packaged definition through `$LIB` or `$USER_LIB`
- **THEN** resolution SHALL produce an ordinary graph definition that validates, flattens, and discovers through the same kernel path as an inline definition

#### Scenario: Nested packaged definition retains provenance

- **WHEN** a packaged graph definition references another packaged definition whose resource default is relative
- **THEN** recursive resolution SHALL retain each literal's concrete package-version root through flattening

### Requirement: Package resources resolve relative to package root

Relative resource static arguments authored by a package SHALL resolve beneath that package's version root and SHALL remain pinned when an explicit package version is referenced. Preparation SHALL compare canonical target and root paths so lexical traversal and symlink escapes are both rejected before loading.

#### Scenario: Pinned package resource remains pinned

- **WHEN** a graph references a pinned package version whose definition uses a relative sample resource
- **THEN** preparation SHALL load the resource from that pinned version even when a newer package version exists

#### Scenario: Package resource cannot escape its root

- **WHEN** a packaged definition supplies a resource path containing an absolute or parent-directory escape
- **THEN** preparation SHALL fail with a structured path-escape diagnostic

#### Scenario: Package resource symlink cannot escape its root

- **WHEN** a resource path inside a package resolves through a symbolic link to a target outside the canonical package-version root
- **THEN** preparation SHALL fail with the same structured path-escape diagnostic before reading the target
