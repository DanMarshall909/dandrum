## ADDED Requirements

### Requirement: Advanced sampling primitives declare connectable registry surfaces

The built-in registry SHALL expose `sample_player`, `sample_zone_selector`, `sample_slicer`, and a thin `sample_map_player` that composes selection, playback, and bounded voice/choke behavior. Source, region, slice-table, and sample-map identities SHALL be typed static arguments. Performance values SHALL be control input ports and triggers, gates, and selected-zone messages SHALL be event ports. Audio outputs SHALL declare their channel count. An existing `sampler` patch SHALL keep its current registry contract while migration to the shared playback implementation proceeds.

#### Scenario: Region player surface is discoverable

- **WHEN** an authoring tool inspects `sample_player`
- **THEN** it SHALL find source, region, mode, interpolation, and channel static arguments; trigger and gate event inputs; pitch ratio, start offset, level, and pan control inputs; and a typed audio output

#### Scenario: Map selection surface is discoverable

- **WHEN** an authoring tool inspects `sample_zone_selector`
- **THEN** it SHALL find a sample-map static argument, note event input, variation control input, and selected-zone event output

#### Scenario: Slice player surface is discoverable

- **WHEN** an authoring tool inspects `sample_slicer`
- **THEN** it SHALL find source and slice-table static arguments, a trigger event input, slice index, pitch ratio, and level control inputs, and a typed audio output

#### Scenario: Map player declares bounded voice and choke options

- **WHEN** an authoring tool inspects `sample_map_player`
- **THEN** it SHALL find a sample-map static argument, bounded voice-count and voice-stealing arguments, a declared choke mode, a note event input, live playback control inputs, and a typed audio output
