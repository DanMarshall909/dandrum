## Purpose

Specify a bounded experimental plugin proof that exercises the existing engine through a dynamically loaded JUCE-free CLAP binary before broader host replacement.

## ADDED Requirements

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
