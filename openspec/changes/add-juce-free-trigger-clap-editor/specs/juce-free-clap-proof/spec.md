## ADDED Requirements

### Requirement: Session-backed Trigger identity and packaged sample resources

The exported JUCE-free CLAP binary SHALL expose a deliberate Trigger sampler identity backed by the persistent instrument session and existing Rust engine. Its accepted default document, production editor assets and supported sample resources SHALL resolve from the copied product resource tree without JUCE, source-checkout lookup or plugin-owned audio devices.

#### Scenario: Discover and render copied Trigger product

- **WHEN** a host dynamically loads the copied release binary, instantiates Trigger and sends the default mapped note at a nonzero block offset
- **THEN** it SHALL discover stable replacement descriptors and public parameters and render the packaged sample's known signed stereo waveform at the correct offset

### Requirement: Trigger state restores a fresh instance atomically

Trigger CLAP state SHALL use a bounded versioned replacement format containing the accepted instrument document, packaged resource identity and public parameter values. Loading SHALL validate and prepare the complete state off audio before publishing it; invalid, incompatible, oversized or unreadable state SHALL preserve the current instrument.

#### Scenario: Fresh instance restores mapping sample and public values

- **WHEN** a changed mapping and nondefault automated value are saved, the original instance is destroyed and a fresh Trigger instance loads that state
- **THEN** the fresh instance SHALL expose the same accepted revision-independent document and public value and render the same known signed sample only from the restored mapping

#### Scenario: Invalid state preserves the running Trigger instance

- **WHEN** a running instance receives malformed, foreign-version, unknown-resource, duplicate-parameter, out-of-range, oversized or short-reading state
- **THEN** loading SHALL fail without partial publication and its prior session snapshot, editor feedback, public values and signed output SHALL remain usable

### Requirement: Trigger instances remain independent across GUI and audio lifecycle

Each Trigger instance SHALL own its session, runtime handoff, parameter queues and disposable GUI attachment. Audio activation, reset, GUI creation and state changes in one instance SHALL NOT alter another instance.

#### Scenario: Two Trigger instances retain independent session and runtime state

- **WHEN** two instances use different mapping, automation, note and editor histories while one is reset, reactivated and has its GUI destroyed
- **THEN** both SHALL retain their own expected signed output and the unaffected instance SHALL preserve its accepted document, parameter feedback and editor lifecycle
