## ADDED Requirements

### Requirement: Compiled patches render offline

The engine SHALL provide an offline render path that accepts a `CompiledPatch` and renders without rebuilding topological order or string-keyed routing from the source `Graph`.

#### Scenario: Compiled render uses compiled execution plan
- **WHEN** a caller renders a compiled patch offline
- **THEN** the renderer SHALL iterate `CompiledPatch::execution_order` and read input sources from `CompiledNode::input_port_map`

#### Scenario: Raw graph render remains available
- **WHEN** compiled rendering is added
- **THEN** the existing raw `Graph` offline render functions SHALL remain available and behaviorally unchanged

### Requirement: Compiled rendering preserves audio output

Compiled offline rendering SHALL produce identical audio output to the existing raw `Graph` offline renderer for supported equivalent inputs.

#### Scenario: Oscillator patch matches raw render
- **WHEN** the same oscillator-to-output patch, render settings, and events are rendered through the raw graph path and the compiled patch path
- **THEN** the left and right output buffers SHALL be identical

#### Scenario: MIDI voice patch matches raw render
- **WHEN** a supported MIDI-driven ADSR/gain voice patch is rendered through the raw graph path and the compiled patch path
- **THEN** the left and right output buffers SHALL be identical

#### Scenario: Sampler patch matches raw render
- **WHEN** a supported sampler patch with the same prepared sampler assets is rendered through the raw graph path and the compiled patch path
- **THEN** the left and right output buffers SHALL be identical

### Requirement: Compiled render migration stays scoped

The compiled render migration SHALL NOT change external integration surfaces, patch documents, dependencies, or threading behavior.

#### Scenario: External surfaces are unchanged
- **WHEN** compiled render parity is implemented
- **THEN** JUCE/FFI APIs, YAML patch format, dependency list, and threading model SHALL remain unchanged

#### Scenario: Polyphonic parity is included only when narrow
- **WHEN** polyphonic compiled rendering requires broad redesign beyond consuming the existing compiled execution plan
- **THEN** the change SHALL defer polyphonic parity rather than redesigning polyphony in this step
