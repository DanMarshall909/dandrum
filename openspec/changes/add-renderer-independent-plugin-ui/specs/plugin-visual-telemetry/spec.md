## Purpose

Provide asynchronous level, activity and cursor measurements to either editor with bounded resource use and no dependence of audio progress on visualization consumers.

## ADDED Requirements

### Requirement: Audio never waits for visual consumers

Telemetry capture in the audio callback SHALL perform only preallocated, bounded measurement and publication work. It SHALL NOT allocate, acquire locks, wait, perform I/O, post operating-system messages, execute analysis transforms, serialize UI messages or invoke renderer code. Capture capacity and tap counts SHALL be fixed during preparation, and visual overload SHALL not change the rendered audio or delay it waiting for a consumer.

#### Scenario: Consumers stall and capture capacity fills

- **WHEN** visualization workers and UI consumers are stopped while the same deterministic audio fixture continues rendering
- **THEN** memory usage and per-callback capture work SHALL remain bounded and visual losses SHALL be counted
- **AND** signed PCM output SHALL match capture-disabled rendering without callback allocation, locking or waiting

#### Scenario: Published buffers outlive editor activity safely

- **WHEN** an editor closes, reopens or is destroyed while audio continues
- **THEN** audio SHALL retain valid capture storage and SHALL NOT join workers or reclaim UI-owned allocations
- **AND** reopened subscribers SHALL receive a current state without accessing the previous editor

### Requirement: Measurement semantics remain meaningful under load

Meter data SHALL identify the instrument generation, bus or prepared tap, channel and measured sample interval. Peak SHALL be the maximum absolute sample and RMS SHALL use sample-weighted energy over the measured interval. Missing history SHALL be reported as incomplete rather than presented as a complete measurement. Clipping SHALL remain latched independently of lossy visual history until acknowledged for the appropriate generation.

#### Scenario: Known signed stereo signal has independent measurements

- **WHEN** left-channel samples are 0.5 and -0.5 and right-channel samples are 0.25 and -0.25
- **THEN** left peak and RMS SHALL be 0.5 and right peak and RMS SHALL be 0.25 within documented numeric tolerance

#### Scenario: Unequal block lengths are aggregated correctly

- **WHEN** a measurement includes two samples at magnitude 1 and six samples at 0
- **THEN** peak SHALL be 1 and RMS SHALL be 0.5, irrespective of how those samples were partitioned into blocks

#### Scenario: Clipping occurs during visual overload

- **WHEN** a finite sample reaches or exceeds magnitude 1 while visual history cannot be published
- **THEN** clipping SHALL remain visible after publication resumes until a valid acknowledgement
- **AND** an acknowledgement for an earlier generation SHALL NOT clear a newer instrument's clip state

### Requirement: Presentation transport applies bounded backpressure

Both editor paths SHALL consume coherent generation-tagged snapshots at a configured bounded rate. Browser publication SHALL cap outstanding payloads and coalesce newer visual state while acknowledgement is delayed. Lossy presentation SHALL NOT silently discard note-release intent, command failures or job terminal state. Hidden editors SHALL unsubscribe from unnecessary visual capture and analysis.

#### Scenario: Browser stops acknowledging visual frames

- **WHEN** JavaScript stops consuming telemetry while native processing continues
- **THEN** browser-bound messages and retained payload memory SHALL remain within the configured limits
- **AND** consumption resumption SHALL receive current state without replaying an unbounded backlog

#### Scenario: Editor becomes hidden

- **WHEN** the last subscriber to a visual stream becomes hidden or disconnects
- **THEN** unnecessary analysis and browser publication for that stream SHALL stop
- **AND** audio, automation and instrument state SHALL continue normally

### Requirement: Activity and cursor state describe observed playback

Pad activity, alternate selection and region cursors SHALL identify the actual prepared source or voice and audio-frame timing when that capability is available. Both host MIDI and editor MIDI SHALL contribute to observed playback feedback. Visual interpolation SHALL never become an input to audio scheduling.

#### Scenario: Host MIDI triggers an alternate

- **WHEN** a host note selects a prepared alternate without an editor pointer event
- **THEN** subscribed editors SHALL identify the observed pad/alternate and its cursor according to the published capability
- **AND** they SHALL not synthesize a successful playback indication merely from a local button animation

### Requirement: Audio-priority claims are verified under defined workloads

Verification SHALL compare callback duration distributions, deadline misses where observable, memory limits and telemetry losses with visualization disabled and enabled under the same documented workloads. Functional PCM parity alone SHALL NOT be reported as evidence of timing safety. Worker count, capture rate and resource limits SHALL be recorded before stress verification.

#### Scenario: Both renderers are stress tested

- **WHEN** native and WebView editors are separately stressed by analysis, resizing, gestures, hide/show, reload and multiple plugin instances
- **THEN** verification SHALL report the host, platform, sample rates, block sizes, fixture, resource limits and observed callback/deadline results
- **AND** any observed audio regression attributable to visualization SHALL block acceptance of the tested configuration
