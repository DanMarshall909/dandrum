## ADDED Requirements

### Requirement: Demo configuration selects a coherent instrument experience

The instrument host SHALL select its starting instrument, display title, UI assets, and optional Sound Lab fixture from one demo configuration. A configured fixture and match-acceptance source SHALL identify the configured instrument.

#### Scenario: TB-303 demo opens fresh

- **WHEN** a new TB-303 demo instance opens without saved plugin state
- **THEN** the active instrument SHALL be the TB-303 patch
- **AND** its UI SHALL display the TB-303 appearance and public parameter controls
- **AND** Sound Lab SHALL use the maintained TB-303 fixture and matching patch source

#### Scenario: Second instrument demo opens

- **WHEN** the second demo configuration opens without saved plugin state
- **THEN** its distinct instrument and UI appearance SHALL load through the same host and bridge
- **AND** its own fixture SHALL render through the shared Sound Lab path
- **AND** its public parameter controls SHALL come from that instrument's metadata

#### Scenario: Demo launches from an unrelated working directory

- **GIVEN** a developer build has the maintained demo patches and fixtures in its source checkout
- **WHEN** the plugin is instantiated with a process working directory outside that checkout and build tree
- **THEN** both demo configurations SHALL resolve their own patch and fixture paths
- **AND** a fresh instance of either demo SHALL load its instrument and public controls

#### Scenario: Host restores a saved instrument

- **GIVEN** saved plugin state contains an instrument definition and current public values
- **WHEN** the host restores the state into a configured demo
- **THEN** the saved instrument and values SHALL be restored through the existing replacement path
- **AND** the fixed host automation slot identities SHALL remain stable

### Requirement: Sound Lab is optional and instrument compatible

The editor SHALL expose Sound Lab actions only when the demo configuration supplies a fixture compatible with the active instrument. Generic parameter and playable-note controls SHALL remain available in either case.

#### Scenario: Configuration has no Sound Lab fixture

- **WHEN** an instrument demo without a fixture opens
- **THEN** its parameter and note controls SHALL work
- **AND** its editor SHALL not expose Sound Lab actions or audio resources

#### Scenario: Active instrument no longer matches the fixture

- **GIVEN** a demo with a Sound Lab fixture is open
- **WHEN** the active instrument is replaced with a different instrument
- **THEN** Sound Lab render, match, and acceptance actions SHALL be unavailable or rejected with a clear status
- **AND** the active instrument SHALL continue processing audio
