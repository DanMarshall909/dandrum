## MODIFIED Requirements

### Requirement: Complete drum kit patch

A `drum-kit` example patch SHALL exist that instantiates multiple `impulse_*` composites routed to different MIDI notes through generic event-routing modules, with per-pad polyphony expressed through `poly` nodes and output through named root ports.

The patch SHALL NOT require a `drum_machine`, `drum_pad`, or drum-specific Rust primitive.

#### Scenario: Drum kit patch loads and renders

- **WHEN** the drum-kit patch is loaded, prepared, and rendered with MIDI events
- **THEN** rendering SHALL complete without error and produce audio on the root `master` output port

#### Scenario: Drum kit uses generic event routing

- **WHEN** the drum-kit patch is inspected
- **THEN** note-to-voice routing SHALL be expressed through generic event-routing modules, `poly` nodes, and explicit connections

#### Scenario: Drum kit polyphony uses poly nodes

- **WHEN** the drum-kit patch is inspected
- **THEN** voice instantiation SHALL be declared through `poly` nodes with explicit `max_voices`, with no `voice_allocation` section

### Requirement: Drum kit supports multiple stereo outputs

The `drum-kit` example patch SHALL declare multiple named 2-channel root output ports so individual drum voices or voice groups can be routed to separate host buses in addition to the main mix.

#### Scenario: Drum kit exposes named output ports

- **WHEN** the drum-kit patch's root ports are inspected
- **THEN** it SHALL declare a 2-channel `master` output port and at least one additional named 2-channel output port fed by a voice group
