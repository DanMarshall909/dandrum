## ADDED Requirements

### Requirement: Module packages are self-contained versioned folders

A reusable module SHALL be a self-contained folder package containing an entry YAML whose file name mirrors the folder name, plus any co-located resources. Resource paths referenced inside the module SHALL resolve relative to the package root so the package is portable.

#### Scenario: Module package with co-located resources

- **WHEN** a module package at `<root>/drum_909_hat/drum_909_hat.yaml` references a resource such as `samples/hat.wav`
- **THEN** the engine SHALL resolve that resource relative to the module package root (`<root>/drum_909_hat/`)
- **AND** the package SHALL load identically regardless of which library root it lives under

### Requirement: Modules are referenced by a macro-qualified pinned file path

A patch SHALL reference an external module by setting a module `type` to a macro-qualified, version-pinned path to the module's entry YAML file (for example `$LIB/1.3.9/drum_voice/drum_voice.yaml`). The reference SHALL identify an exact file; there SHALL be no name registry, bare-name lookup, or collision-resolution rules.

#### Scenario: External module reference is expanded

- **WHEN** a patch declares a module whose `type` is `$LIB/<version>/drum_voice/drum_voice.yaml`
- **THEN** the engine SHALL load that module package and expand it into its constituent internal modules and connections
- **AND** the expansion SHALL behave identically to an inline `module_definitions` entry

#### Scenario: Unknown macro is a hard error

- **WHEN** a module `type` uses a macro root that is not configured (for example `$NOPE/...`)
- **THEN** patch preparation SHALL fail with a clear error
- **AND** SHALL NOT silently resolve the reference against any other root

#### Scenario: Reference escaping the library or package root is rejected

- **WHEN** a module reference or an internal resource path attempts to escape its macro root or package root (for example via `..` segments)
- **THEN** the engine SHALL reject the reference with a clear error

### Requirement: The standard library `$LIB` is immutable and seeded from a CRC-tracked zip

The `$LIB` macro SHALL resolve to an immutable standard-library directory that is the extraction target of a single shipped seeding zip stored in a separate canonical location. The engine SHALL record the seeding zip's CRC and re-extract it into `$LIB` when the CRC changes, and SHALL skip re-extraction when the CRC is unchanged. Re-seeding SHALL be additive: it SHALL add version directories and repoint `latest` without removing existing versions. `$LIB` SHALL NOT be treated as a writable, hand-edited location.

#### Scenario: First run extracts the seed

- **WHEN** `$LIB` has not yet been seeded, or the seeding zip's recorded CRC differs from the current zip
- **THEN** the engine SHALL extract the seeding zip into `$LIB` before resolving `$LIB` references
- **AND** SHALL record the seeding zip's CRC

#### Scenario: Unchanged seed skips extraction

- **WHEN** the seeding zip's CRC matches the recorded CRC
- **THEN** the engine SHALL use the already-extracted `$LIB` without re-extracting

#### Scenario: Re-seed preserves existing versions

- **WHEN** a newer seeding zip is extracted
- **THEN** previously present version directories SHALL remain resolvable
- **AND** patches that pin an older version SHALL continue to load

### Requirement: The user library `$USER_LIB` is mutable

The `$USER_LIB` macro SHALL resolve to a user-owned, mutable module directory that is not seeded or refreshed by the engine. Modules under `$USER_LIB` SHALL be referenced with the same macro-qualified pinned-path form as `$LIB` modules.

#### Scenario: User module is referenced

- **WHEN** a patch references a module `type` of `$USER_LIB/my_kit/my_kit.yaml`
- **THEN** the engine SHALL load and expand it identically to a `$LIB` module
- **AND** SHALL NOT apply the immutability or seeding behaviour of `$LIB` to it

### Requirement: Library uses a version-first layout with a `latest` alias

Within a library root, module packages SHALL be organised version-first as `<root>/<version>/<module>/<module>.yaml`, with multiple versions able to coexist. A `latest` alias SHALL resolve to the newest version. Pinning a concrete version SHALL be the reproducible form; `latest` SHALL be a floating reference.

#### Scenario: Pinned version is reproducible

- **WHEN** two library versions `1.3.9` and `1.4.0` both contain `drum_voice`
- **AND** a patch references `$LIB/1.3.9/drum_voice/drum_voice.yaml`
- **THEN** the engine SHALL resolve the `1.3.9` package regardless of newer versions being present

#### Scenario: `latest` follows the newest version

- **WHEN** a patch references `$LIB/latest/drum_voice/drum_voice.yaml`
- **THEN** the engine SHALL resolve the newest available version of that module
