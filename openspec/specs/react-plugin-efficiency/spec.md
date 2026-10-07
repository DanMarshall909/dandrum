# react-plugin-efficiency Specification

## Purpose
Keep embedded React editors responsive while bounding redundant parameter, rendering and prepared-analysis work.

## Requirements

### Requirement: Responsive TB-303 layout
The TB-303 editor SHALL fit native viewport widths from 760px to 1500px using layout without whole-panel scaling, retain access to every prepared control and audition key, and render labels at least 11px.

#### Scenario: Native widths retain readable controls
- **WHEN** the embedded TB-303 editor is resized to 760, 820, 1180 or 1500px
- **THEN** its panel has no scale transform, controls and audition keys fit horizontally, and labels remain at least 11px

### Requirement: Bounded asynchronous parameter refresh
The React editors SHALL admit gesture commands without waiting for full state reads, coalesce reads to one in flight and one pending refresh, and preserve authoritative generation and admitted-sequence ordering.

#### Scenario: Writes progress while reads stall
- **WHEN** a full parameter read stalls during a gesture
- **THEN** begin, accepted writes and end proceed in order without waiting for that read, with at most one read in flight and one replacement refresh

#### Scenario: Accepted drag stays visible while snapshots catch up
- **WHEN** a drag is released after its write is accepted while a parameter read is stalled
- **THEN** its accepted value remains visible, current host automation or rejection reconciles it, and a later interaction owns its display

#### Scenario: Rejection and reload remain authoritative
- **WHEN** a write is rejected or a read completes across a newer admitted write or generation
- **THEN** rejection is reported, a refresh recovers current state, and stale replies cannot restore old state

### Requirement: Suppress unchanged parameter work
Native publication and React reconciliation SHALL suppress unchanged parameter snapshots while retaining updates for automation, admission sequence changes, visibility restoration and reload.

#### Scenario: Idle state does not publish repeatedly
- **WHEN** an editor acknowledges a snapshot and values remain unchanged over timer ticks
- **THEN** no further snapshot is queued until parameter state changes or the editor is shown or reloaded

#### Scenario: Equal sequence automation updates the display
- **WHEN** a snapshot repeats the current generation and sequence
- **THEN** identical parameter fields retain display identity and changed host automation values or metadata update the display

### Requirement: Suppress unchanged meter reconciliation
React meters SHALL retain current display identity for equal normalized visible state while consuming and acknowledging every transport packet.

#### Scenario: Equal meters still acknowledge packets
- **WHEN** consecutive packets have equal display fields but different transport sequences
- **THEN** display identity is retained and each packet is acknowledged with its own exact sequence and generation

#### Scenario: Changed measurement and clip state update
- **WHEN** generation, validity, completeness, peak, RMS, clip latch or clip ticket changes
- **THEN** the meter display updates without losing numeric fidelity or a locally acknowledged clip change

### Requirement: Retire completed prepared timers
Prepared waveform and spectrum polling SHALL stop after terminal completion or failure and restart when a new selection, retry or visibility restoration requires a job.

#### Scenario: Prepared completion retires polling
- **WHEN** a prepared job completes or fails admission, or its view is closed
- **THEN** its interval is cleared, late results cannot restart polling, and partial spectrum pages keep polling until the assembled result is complete
