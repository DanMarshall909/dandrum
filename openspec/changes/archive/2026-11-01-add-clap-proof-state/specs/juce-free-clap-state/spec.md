## ADDED Requirements

### Requirement: Replacement-owned fixture state

The experimental CLAP fixtures SHALL save versioned bounded state identifying the built-in instrument and every public musical parameter by name. A fresh instance SHALL restore supported state before activation, preserve public parameter identities and produce the corresponding signed engine output. Serialization SHALL run off the audio callback.

#### Scenario: Fresh instance restores saved public controls

- **WHEN** the host saves nondefault controls and restores the state into a fresh matching fixture before activation
- **THEN** every public control SHALL match its named saved value and rendering SHALL produce the expected signed audio independently of another instance

#### Scenario: Named controls restore independently of entry order

- **WHEN** a matching instrument receives independently authored supported state with its complete controls in a different order
- **THEN** it SHALL restore by musical identity, notify an available host of changed parameter values and use the restored controls when processing resumes

### Requirement: Bounded rejected-state recovery

State load SHALL validate the complete input before changing controls. Unsupported schema or instrument, malformed or oversized input, duplicate/missing/unknown controls, invalid values and stream read failure SHALL report failure and preserve the working parameters and signed output. State save SHALL handle partial writes and report stream failure.

#### Scenario: Invalid state preserves the working instrument

- **WHEN** an otherwise usable fixture receives invalid, incompatible, oversized or unreadable state
- **THEN** load SHALL fail with unchanged public controls and the fixture SHALL continue to render its known signed output and accept subsequent valid state

#### Scenario: Partial streams and save failures are reported correctly

- **WHEN** the host limits successful reads or writes to short chunks or fails the save stream
- **THEN** short operations SHALL still support a complete round trip and failed writes SHALL report failure without changing the instrument

### Requirement: Stopped parameter synchronization

The fixture SHALL accept supported global parameter changes through CLAP flush while inactive or active with processing stopped. The final accepted value SHALL be reflected in get-value, saved state and subsequent signed rendering. Invalid or oversized flush batches SHALL preserve all prior values.

#### Scenario: Flush updates survive save and processing restart

- **WHEN** the host flushes ordered changes while inactive and while active with processing stopped, including rejected batches
- **THEN** only valid complete batches SHALL update public values, saved state SHALL retain the final value and resumed rendering SHALL produce the corresponding signed output
