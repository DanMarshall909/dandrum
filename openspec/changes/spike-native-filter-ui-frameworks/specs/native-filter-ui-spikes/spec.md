## ADDED Requirements

### Requirement: Comparable real filter effects
Each experiment SHALL process stereo host input through the same Rust high-pass, bell and low-pass graph, expose stable automatable controls, and support bypass and saved-state restoration.

#### Scenario: Signed stereo filter output
- **WHEN** each plugin processes a known stereo impulse at declared settings
- **THEN** it produces the expected signed output on both channels, and changing the cutoff or bell gain changes the response.

#### Scenario: Bypass and state restore
- **WHEN** bypass is enabled or saved parameters are restored into another instance
- **THEN** bypass preserves the input exactly and restored controls produce the same filter output as the saved instance.

### Requirement: Demanding interactive native views
Each experiment SHALL draw an interactive logarithmic response graph with three draggable band nodes, live spectrum, scrolling spectrogram, animated meters and rotary controls using its selected framework.

#### Scenario: Graph and controls share host state
- **WHEN** a user drags a response node or edits its rotary control, or the host changes that parameter
- **THEN** the graph, selected-band controls and audio parameter values agree, and an editor gesture has balanced host begin/end boundaries.

#### Scenario: Actual audio drives visual analysis
- **WHEN** known input is processed with the editor open
- **THEN** the spectrum and spectrogram reflect that input and filtered output, and the meters display observed audio levels.

#### Scenario: Independent editor lifetimes
- **WHEN** two instances are open and one editor is closed and reopened
- **THEN** both processors remain usable, parameter state remains instance-local and recreated rendering works without another event-loop or platform-registration failure.

### Requirement: Maintained discovery and honest comparison
Both experiments SHALL be discoverable and launchable through the demo catalog, and their comparison SHALL identify scope, reproducible commands, implementation costs, resource results and an explicit adopt/revise/defer outcome.

#### Scenario: Optional spike launch
- **WHEN** either spike is selected through the maintained launcher
- **THEN** its required native-only spike build is enabled in an isolated build directory and its actual standalone effect is launched without npm preparation.

#### Scenario: Bounded evaluation evidence
- **WHEN** the experiment is handed off
- **THEN** the report distinguishes measured renderer/backend behaviour from unverified deployment platforms or production-readiness claims.
