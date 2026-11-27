## ADDED Requirements

### Requirement: Native Trigger playback edits preserve accepted mode and sound

Trigger SHALL compile supported region playback edits into the prepared mapped sampler and publish them only after native acceptance. Undo, redo and fresh-instance restore SHALL reproduce the accepted mode and audible result. Ping-pong and sustain SHALL remain unavailable with an actionable explanation.

#### Scenario: Accepted loop and gated edits change native playback
- **WHEN** the user changes a mapped region from Once to Loop or Gated and the native host accepts the edit
- **THEN** the editor SHALL show the accepted mode and the host SHALL render the matching signed waveform and note-off behavior

#### Scenario: Rejected playback edit preserves the working patch
- **WHEN** a loop boundary, crossfade, or unsupported mode edit fails admission
- **THEN** the current audio, visible mode, and undo history SHALL remain on the last accepted generation and the editor SHALL report the reason

#### Scenario: Playback mode survives history and fresh instance restore
- **WHEN** an accepted playback edit is undone, redone, saved, and restored in a fresh native instance
- **THEN** the projected mode, prepared region settings, and audible behavior SHALL match the selected accepted generation
