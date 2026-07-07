## ADDED Requirements

### Requirement: Plugin watches the loaded instrument file for external edits

The plugin SHALL watch the currently loaded instrument's YAML file for changes made outside the plugin.

#### Scenario: Author edits the instrument file externally

- **GIVEN** a plugin instance has an instrument loaded from a YAML file
- **AND** file watching is enabled
- **WHEN** the file's contents change on disk
- **THEN** the plugin SHALL detect the change off the audio thread within one polling interval
- **AND** the audio callback SHALL NOT perform the file change detection itself

#### Scenario: Author disables file watching

- **GIVEN** a plugin instance has file watching enabled
- **WHEN** the user disables the file-watch toggle
- **THEN** subsequent external edits to the instrument file SHALL NOT trigger a reload
- **AND** the currently running instrument SHALL remain unchanged until watching is re-enabled or a manual reload is requested

### Requirement: Detected file changes trigger the standard replacement transaction

A detected instrument-file change SHALL be applied through the same explicit, off-audio-thread replacement transaction used by any other instrument reload.

#### Scenario: Detected change validates and compiles successfully

- **GIVEN** the plugin detects a change to the loaded instrument file
- **WHEN** the changed file validates, compiles, and prepares successfully off the audio thread
- **THEN** the plugin SHALL mute audio before retiring the previous DSP
- **AND** the plugin SHALL publish the replacement DSP only after preparation succeeds
- **AND** the plugin SHALL unmute audio once the replacement is active
- **AND** the plugin SHALL refresh the parameter/control surface if the public parameter layout changed

#### Scenario: Detected change fails validation or compilation

- **GIVEN** the plugin detects a change to the loaded instrument file
- **WHEN** the changed file fails validation, compilation, or asset preparation
- **THEN** the plugin SHALL NOT mute or replace the currently running DSP
- **AND** the plugin SHALL report the failure through editor/status reporting
- **AND** the previously loaded instrument SHALL continue rendering normally

#### Scenario: File changes mid-write

- **GIVEN** an external editor is in the middle of saving the instrument file
- **WHEN** the plugin observes a change signal that is not yet stable
- **THEN** the plugin SHALL wait for the change signal to stabilise before validating/compiling
- **AND** the plugin SHALL NOT trigger a replacement transaction from a partially written file

### Requirement: File watching never causes audio callback authoring work

The audio callback SHALL remain free of file-watching and authoring-related work.

#### Scenario: Watcher is checking for changes while audio is rendering

- **GIVEN** the file watcher is due to poll for changes
- **WHEN** the host calls `processBlock` concurrently
- **THEN** `processBlock` SHALL NOT perform file stats, reads, YAML parsing, validation, compilation, or asset loading
