## MODIFIED Requirements

### Requirement: Echo SHALL expose built-in module ports

The `echo` primitive SHALL expose one channel-aware `audio_in` input and `audio_out` output, each resolving to one or two channels through a static `channels` argument. It SHALL expose control input ports `time_left_ms`, `time_right_ms`, `feedback`, `damping_cutoff`, `wet`, `dry`, `sync_division`, and `ping_pong`. It SHALL NOT require a separate BPM-clock event port.

#### Scenario: Module registration has correct ports

- **WHEN** the kernel built-in registry is queried for `echo`
- **THEN** it SHALL report one channel-aware audio input/output pair and the declared control input ports
- **AND** the same definition SHALL prepare and render with one or two audio channels

## REMOVED Requirements

### Requirement: Echo SHALL process in global (non-voice) scope

**Reason**: The kernel removes graph-wide voice/global execution scope; placement outside or inside an explicit `poly` region determines whether the echo processes a summed signal or an individual voice.
**Migration**: Place echo after a `poly` node to process the summed mix, or inside its wrapped definition for per-voice processing.
