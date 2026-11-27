## Purpose

Connect the full React sampler shell to actual prepared sample playback through the native JUCE host boundary.

## Requirements

### Requirement: Native shell resource and adapter selection

The integrated sampler SHALL load the full locally bundled React shell and select its native adapter. The browser preview SHALL retain an explicitly silent backend.

#### Scenario: Embedded shell uses the native bridge
- **WHEN** the integrated standalone opens its editor
- **THEN** it loads the local full sampler shell, initializes from the prepared native instrument and identifies the real audio backend

#### Scenario: Native connection status preserves editor geometry
- **WHEN** the connected native editor opens or its WebView is resized
- **THEN** backend status is displayed within the sampler header and the editor remains inside the available viewport without horizontal page overflow

#### Scenario: Native size choices resize the host window
- **WHEN** the user selects Min, Default, Expanded or the compact performance view
- **THEN** the native editor window adopts the selected preset dimensions and reopening the editor restores its full layout dimensions

#### Scenario: Standalone title bar supports window movement
- **WHEN** the integrated standalone opens its editor
- **THEN** it provides a system title bar for window-manager movement, keeps host audio/settings and state actions available through Options, and preserves the editor dimensions of size choices

#### Scenario: Standalone title follows the accepted patch name
- **WHEN** a patch is opened, renamed, undone or replaced in the integrated standalone
- **THEN** the title is `trigger - {name of patch}` using the accepted patch name, and rejected edits or malformed title requests preserve the previous title

### Requirement: Transactional sampler edits

Supported sample edits and undo SHALL prepare immutable replacement data off the audio thread and publish the edited model only after native acceptance. Rejected or stale edits SHALL retain the working instrument and editor patch.

#### Scenario: Failed preparation retains the working patch
- **WHEN** an edit fails preparation or uses a retired generation
- **THEN** the native engine and accepted editor model remain unchanged and the editor reports the error

#### Scenario: Sample mapping edit and undo affect playback
- **WHEN** a supported region or mapping edit and its inverse are accepted
- **THEN** both replacements update real sample playback with their respective mapping and region values

### Requirement: Real runtime feedback and capabilities

Notes, host controls, sample waveforms and output meters SHALL use native engine services. Unsupported DSP operations SHALL report their capability limitation rather than publish simulated success.

#### Scenario: Notes and controls reach the host
- **WHEN** the user plays a pad or changes an exposed parameter
- **THEN** the adapter sends generation-tagged native note or host parameter commands

#### Scenario: Feedback comes from prepared audio
- **WHEN** waveform or output meter data is displayed
- **THEN** the adapter uses native prepared samples and measured output packets without synthetic feedback

#### Scenario: Unsupported edit is rejected
- **WHEN** the editor requests a DSP operation absent from the native sampler adapter
- **THEN** it reports the limitation and retains the last accepted patch

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
