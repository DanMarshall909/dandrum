## Purpose

Keep native C++ JUCE and WebView editors viable through shared instrument state, command semantics, visual data and design tokens while preserving existing host and preparation contracts.

## ADDED Requirements

### Requirement: Editors share authoritative instrument state and commands

The plugin SHALL provide renderer-independent descriptions of prepared instrument identity, public parameters, sources, regions, zones, layers, supported routing and capabilities. Both editors SHALL use the same command semantics and stable host parameter identities. Selecting a renderer SHALL NOT require a DSP implementation change or a different persisted instrument state.

#### Scenario: Both editors control the same instrument

- **WHEN** equivalent normalized gestures and note events are submitted through native and WebView editors against the same prepared instrument and processing schedule
- **THEN** they SHALL reach the same host parameter slots and MIDI admission path and produce the same signed audio output
- **AND** both SHALL display the corresponding authoritative parameter and diagnostic state

#### Scenario: Native editor builds without browser support

- **WHEN** the native-only plugin configuration is built and run without browser libraries or a JavaScript toolchain
- **THEN** it SHALL support parameter control, playable notes, meters and prepared-waveform display using the shared services

### Requirement: Commands distinguish acceptance from completion

The UI contract SHALL validate identity, generation, argument ranges and finite numeric values before admitting commands. Expensive work SHALL return a job identity and complete asynchronously. Continuous parameter edits SHALL preserve one begin/update/end host gesture, and editor teardown SHALL close outstanding gestures and release editor-owned notes without waiting on the audio callback.

#### Scenario: Continuous drag preserves a host gesture

- **WHEN** either editor begins a drag, changes its value several times and ends or cancels the drag
- **THEN** the host SHALL receive one matching begin/end gesture and the admitted parameter updates
- **AND** authoritative value notifications SHALL not produce a feedback loop or overwrite a newer local update with a stale echo

#### Scenario: Stale or invalid command is rejected

- **WHEN** a command names an obsolete instrument generation, unknown control, non-finite value or unsupported capability
- **THEN** it SHALL report a bounded error without modifying the current instrument

#### Scenario: Reload is accepted before preparation completes

- **WHEN** an editor requests an instrument reload while preparation is delayed
- **THEN** it SHALL receive job acceptance without waiting for preparation or an audio block
- **AND** job completion or failure SHALL arrive separately, with failure preserving the previous instrument and its displayed generation

#### Scenario: Editor closes during interaction

- **WHEN** an editor closes or its browser page disconnects during a gesture and an editor-triggered gated note
- **THEN** the gesture SHALL end and release intent SHALL remain available to audio processing even when normal event admission is saturated

### Requirement: Revised design components display actual capabilities

The editors SHALL present KeyMap, LayerStack and OutputBusses views from prepared metadata and supported host bindings. Shared and per-pad parameters SHALL retain their declared scope. Overlapping zones SHALL display their declared selection or layering semantics, without inferring that every overlap plays simultaneously. Unsupported data SHALL be explicitly unavailable rather than fabricated from the mockup.

#### Scenario: Loaded kit overrides illustrative mock data

- **WHEN** the maintained advanced drum kit is loaded
- **THEN** its key map SHALL display snare velocities 1-63 and 64-127, actual alternates and choke relationships, and actual shared and per-pad controls
- **AND** it SHALL NOT substitute the export's 1-95/96-127 split or treat every control as patch-wide

#### Scenario: A source type or layer description is unavailable

- **WHEN** the loaded instrument does not expose a synth, nested-patch layer, module chain or other requested display capability
- **THEN** both editors SHALL indicate that capability is unavailable without inventing source metadata or implementing a new engine feature implicitly

#### Scenario: Host provides a restricted bus layout

- **WHEN** the host exposes only a stereo master bus, or exposes named buses with different channel counts
- **THEN** the output view SHALL enumerate those actual bindings and per-channel measurements
- **AND** it SHALL NOT offer the mockup's fixed 1/2 through 15/16 output list as available host routing

### Requirement: Prepared structure remains externally authored

Plugin editors SHALL display source assignments, zone bounds, module order, structural routes, voice limits and choke policies as prepared data. Structural authoring SHALL remain external and take effect through explicit reload. Live gain, send, mute, bypass or other controls SHALL be enabled only when they have a supported public parameter binding; a display label alone SHALL NOT make a value live.

#### Scenario: Prepared component interactions are restricted

- **WHEN** a user selects a zone or opens a layer or output panel
- **THEN** inspection, audition and supported public parameter controls SHALL be available
- **AND** zone dragging, module addition/removal/reordering and structural rerouting SHALL NOT modify or author the running patch

### Requirement: The design system supports both renderers

Both editors SHALL use consistent semantic colours, dimensions, typography, focus and value-formatting rules generated from one maintained token source. The production WebView SHALL serve all executable assets, icons and licensed fonts locally without network dependency. Both renderers SHALL expose accessible keyboard control and usable layouts at the supplied full and compact design sizes.

#### Scenario: Matching token update reaches both renderers

- **WHEN** a shared colour or spacing token changes and assets are regenerated
- **THEN** its CSS and C++ representations SHALL express the same value without independent manual token edits

#### Scenario: Offline plugin opens outside the source checkout

- **WHEN** the packaged WebView plugin opens without internet access and outside its source tree
- **THEN** its interface, fonts, icons and controls SHALL load without CDN requests or runtime source compilation

#### Scenario: Both layouts support keyboard interaction

- **WHEN** either renderer is used at 1200 by 800 or 820 by 560 logical pixels
- **THEN** controls SHALL have visible focus, keyboard-accessible editing and cancellation, readable values and access to the supported instrument functions
