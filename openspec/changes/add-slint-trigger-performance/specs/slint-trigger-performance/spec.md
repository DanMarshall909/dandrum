## ADDED Requirements

### Requirement: Faithful macro performance preview
The preview SHALL display Trigger's eight macro controls in the same order, using the React palette, font families, 48px rotary geometry, 64px cells, 6px gaps and labels of at least 11px. The preview SHALL remain usable at 900px and 720px widths and SHALL be identified as silent in its launch documentation.

#### Scenario: Loaded performance strip
- **WHEN** the preview starts
- **THEN** it displays Tone 56%, Body 40%, Space 32%, Drive 18%, Snap 64%, Width 50%, and disabled Macro 7 and Macro 8 at 0%
- **AND** all eight controls are visible without overlapping the Macros heading at both supported widths

### Requirement: Editable assigned macros
Assigned macros SHALL support vertical drag, Shift precision, wheel and keyboard adjustments, reset to their initial value, and typed percentage editing. Values SHALL remain between 0% and 100%; disabled slots SHALL reject editing. Hovering or focusing a control SHALL expose its current value.

#### Scenario: Change and reset a macro
- **WHEN** the user changes Tone using drag, precision drag, wheel, keyboard or a typed value
- **THEN** its visible value and arc agree with the new clamped percentage
- **AND** middle-click or Alt-click resets it to 56%

#### Scenario: Disabled and invalid editing
- **WHEN** the user attempts to edit Macro 7 or submits an invalid typed value
- **THEN** the existing value remains unchanged
- **AND** a valid typed value outside the range is clamped to the nearest endpoint

### Requirement: Maintained launch entrypoint
The maintained demo launcher SHALL list and launch the silent Slint performance preview from a registered checkout, including when the current checkout lacks its optional UI source.

#### Scenario: Launch native GUI preview
- **WHEN** the user invokes `./demo trigger-slint`
- **THEN** the launcher finds a checkout declaring the preview's native GUI target, builds its isolated configuration and launches the correct platform artifact with forwarded arguments
