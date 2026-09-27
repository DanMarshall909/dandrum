## ADDED Requirements

### Requirement: Plugin editor presents declared public controls

The plugin editor SHALL present playable controls from the loaded instrument's public parameter metadata through the configured editor UI. The UI technology and appearance SHALL not change public parameter identity or host automation slots.

#### Scenario: Loaded instrument declares public parameters

- **GIVEN** a loaded instrument declares public parameters in `preset_surface.parameters`
- **WHEN** the plugin editor opens or the public surface changes after explicit replacement
- **THEN** the editor SHALL display one control for each active public parameter
- **AND** each control SHALL use its declared identity and display name
- **AND** changing a control SHALL notify the host through the corresponding stable parameter slot

#### Scenario: Plugin editor displays runtime information

- **GIVEN** a plugin instance has loaded or attempted to load an instrument
- **WHEN** the plugin editor is visible
- **THEN** it SHALL display the instrument identity or name when available
- **AND** it SHALL display load/prepare status or error text when available

#### Scenario: Editor sends playable notes

- **WHEN** a user presses and releases a key in the configured editor
- **THEN** note-on and note-off SHALL enter the bounded editor MIDI queue
- **AND** a full queue SHALL be reported without blocking the audio callback

## REMOVED Requirements

### Requirement: Plugin editor uses native JUCE generic controls

**Reason**: The current TB-303 proof already uses a WebView, and the reusable contract is metadata-driven public controls and stable host parameters rather than a particular widget toolkit.

**Migration**: `Plugin editor presents declared public controls` preserves the public-control and status scenarios while allowing distinct instrument pages.
