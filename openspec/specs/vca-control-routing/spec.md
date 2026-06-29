## Purpose

Specify VCA/control signal compatibility, modulatable control ports, and explicit control summing.

## Requirements

### Requirement: VCA/control signal compatibility
The engine SHALL treat VCA/control signals as first-class routable signal types with explicit compatibility rules.

#### Scenario: Compatible VCA output connects to compatible VCA input
- **WHEN** a patch connects a VCA/control output port to a compatible VCA/control input port
- **THEN** validation SHALL succeed for that connection

#### Scenario: Any compatible VCA source can modulate any compatible destination
- **WHEN** a patch connects an envelope output, LFO output, or script control output to a compatible VCA/control input
- **THEN** the engine SHALL accept the route without requiring destination-specific hardcoded modulation fields

### Requirement: Modulation destinations are ports
Modulation destinations such as gain, pitch, cutoff, pan, and envelope parameters SHALL be represented as routable ports when they are externally modulatable.

#### Scenario: Filter cutoff is routed through a control port
- **WHEN** a patch modulates filter cutoff from an LFO
- **THEN** the patch SHALL represent that modulation as a connection to a cutoff control port rather than as a hidden `cutoff_lfo` field

### Requirement: Explicit control summing
Multiple VCA/control signals SHALL be combined through explicit mixer, attenuator, or summing modules unless a destination declares multi-source behavior.

#### Scenario: Two modulators feed a mixer before a destination
- **WHEN** an envelope and LFO are both intended to modulate a gain input
- **THEN** the patch SHALL be valid when both sources feed an explicit control mixer whose output connects to the gain input
