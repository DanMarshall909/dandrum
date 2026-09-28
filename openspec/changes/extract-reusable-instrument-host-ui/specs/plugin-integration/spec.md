## ADDED Requirements

### Requirement: Host boundary preserves authored defaults and note velocity

The plugin SHALL keep authored public defaults synchronized between host parameter state and the running Rust instrument, and SHALL forward JUCE MIDI note velocity without rescaling its already encoded 0–127 value.

#### Scenario: Fresh instrument starts with authored defaults

- **GIVEN** a new plugin instance loads an instrument without saved state or parameter automation
- **WHEN** the first audio block is processed
- **THEN** its host parameter values and Rust render SHALL use the instrument's authored public defaults

#### Scenario: Host MIDI velocity reaches the instrument

- **GIVEN** the host sends a JUCE note-on with velocity `V` and a block-local offset
- **WHEN** the plugin processes that block
- **THEN** Rust SHALL receive velocity `V` at the same offset
- **AND** its render SHALL match a direct Rust note event with the same note, velocity, and offset

### Requirement: Plugin editor presents declared public controls

The plugin editor SHALL present playable controls from the loaded instrument's public parameter metadata through the configured editor UI. The UI technology and appearance SHALL not change public parameter identity or host automation slots.

#### Scenario: Loaded instrument declares public parameters

- **GIVEN** a loaded instrument declares public parameters in `preset_surface.parameters`
- **WHEN** the plugin editor opens or the public surface changes after explicit replacement
- **THEN** the editor SHALL display one control for each active public parameter
- **AND** each control SHALL use its declared identity and display name
- **AND** changing a control SHALL notify the host through the corresponding stable parameter slot

#### Scenario: Editor sends playable notes

- **WHEN** a user presses and releases a key in the configured editor
- **THEN** note-on and note-off SHALL enter the bounded editor MIDI queue
- **AND** a full queue SHALL be reported without blocking the audio callback

## REMOVED Requirements

### Requirement: Plugin editor uses native JUCE generic controls

**Reason**: The current TB-303 proof already uses a WebView, and the reusable contract is metadata-driven public controls and stable host parameters rather than a particular widget toolkit.

**Migration**: `Plugin editor presents declared public controls` preserves the public-control and status scenarios while allowing distinct instrument pages.
