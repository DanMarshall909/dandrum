## Purpose

Specify the drum-machine capability that packages a drum kit into a reusable instrument with routable outputs.

## Requirements

### Requirement: Drum machine supports multiple stereo outputs

The drum machine SHALL support multiple stereo output pairs so individual drum voices or voice groups can be routed to separate stereo outs in addition to the main mix.

#### Scenario: Drum machine exposes stereo output pairs

- **WHEN** the drum machine is inspected
- **THEN** it SHALL expose a main stereo output pair plus at least one additional named stereo output pair

#### Scenario: Voices can route to separate stereo outs

- **WHEN** the drum machine is wired for multi-output routing
- **THEN** selected voice instances or groups SHALL be connectable to distinct stereo output pairs without requiring a drum-specific primitive
