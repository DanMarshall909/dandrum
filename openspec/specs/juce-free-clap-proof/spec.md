## Purpose

Specify a bounded experimental plugin proof that exercises the existing engine through a dynamically loaded JUCE-free CLAP binary before broader host replacement.

## Requirements

### Requirement: Experimental plugin discovery and signed engine rendering

The opt-in experimental plugin SHALL expose separate, deliberate instrument and signed-gain-fixture identities through an exported CLAP binary that builds without JUCE or browser resources. Host audio processing SHALL use the existing prepared engine without opening a device or compiling instrument structure in the callback.

#### Scenario: Discover and render the exported binary

- **WHEN** a host dynamically loads the exported plugin binary and instantiates the signed stereo gain fixture
- **THEN** it SHALL discover the documented descriptors and render literal signed stereo output through the existing child gain module with authored defaults

### Requirement: Sample-offset public controls and instrument notes

The experimental plugin SHALL expose public root parameter identities, ranges and defaults and apply ordered host changes at their block-relative offsets. The instrument SHALL advertise only its supported MIDI 1 note input dialect and preserve its engine note semantics. Unsupported note expression, polyphonic parameter modulation and dialect capabilities SHALL NOT be advertised.

#### Scenario: Automation preserves offsets and same-offset order

- **WHEN** the host sends gain changes at nonzero and shared offsets and then processes another block
- **THEN** the signed stereo waveform SHALL reflect each segment's ordered parameter value and retain the final value without changing parameter identities or rebuilding the graph

#### Scenario: Existing instrument receives ordered timed notes

- **WHEN** the host instantiates TB-303 and supplies MIDI notes at nonzero and shared offsets, including supported overlaps and a public control change
- **THEN** its stereo output SHALL preserve onset/order and public root mapping, include a known signed oracle and agree with the same prepared engine and event sequence

### Requirement: Independent prepared host lifecycle

Each experimental plugin instance SHALL own independent prepared runtime and parameter state. Reset and reactivation SHALL clear transient DSP and pending events while retaining usable public controls; preparation and runtime reclamation SHALL occur off rendering.

#### Scenario: Two instances survive reset and reactivation

- **WHEN** two instances render different control/note histories and one is reset, deactivated and reactivated with supported changed host settings
- **THEN** both SHALL remain independently usable with expected signed output and the unaffected instance SHALL retain its own state

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
