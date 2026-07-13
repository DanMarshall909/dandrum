## ADDED Requirements

### Requirement: Package entries are graph definitions

Module package entry documents SHALL load through the unified graph-definition parser and SHALL expose the same static parameters and ports as inline definitions. Package loading SHALL NOT require the legacy patch expansion or module-parameter model.

#### Scenario: Packaged definition instantiates like an inline definition

- **WHEN** a graph node references a packaged definition through `$LIB` or `$USER_LIB`
- **THEN** resolution SHALL produce an ordinary graph definition that validates, flattens, and discovers through the same kernel path as an inline definition

### Requirement: Package resources resolve relative to package root

Relative resource static arguments declared by a package SHALL resolve beneath that package's version root and SHALL remain pinned when an explicit package version is referenced.

#### Scenario: Pinned package resource remains pinned

- **WHEN** a graph references a pinned package version whose definition uses a relative sample resource
- **THEN** preparation SHALL load the resource from that pinned version even when a newer package version exists

#### Scenario: Package resource cannot escape its root

- **WHEN** a packaged definition supplies a resource path containing an absolute or parent-directory escape
- **THEN** preparation SHALL fail with a structured path-escape diagnostic
