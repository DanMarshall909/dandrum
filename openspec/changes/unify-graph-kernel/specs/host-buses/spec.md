## ADDED Requirements

### Requirement: Hosts declare named buses

The host boundary SHALL be expressed as named input and output buses, each with an arbitrary channel count. The kernel, primitives, and graph validation SHALL contain no stereo or channel-count assumptions; stereo is simply a two-channel bus.

#### Scenario: Host declares multiple buses

- **WHEN** a host declares output buses `master` (2 channels), `drums` (2), and `cue` (2), and input buses `main` (2) and `sidechain` (1)
- **THEN** preparation SHALL accept the declaration and expose all buses for binding

#### Scenario: Non-stereo channel counts are first-class

- **WHEN** a host declares a 1-channel or 6-channel bus
- **THEN** preparation and rendering SHALL handle it identically to any other channel count

### Requirement: Root ports bind to buses by name

Preparation SHALL bind root graph input/output ports to host buses by name and SHALL validate that each bound pair has matching channel counts. Root output ports with no matching bus SHALL fail preparation; unbound host inputs deliver silence.

#### Scenario: Matching bus binds

- **WHEN** the root graph declares a 2-channel output port `master` and the host declares a 2-channel `master` bus
- **THEN** preparation SHALL bind them and rendering SHALL write the port's samples to that bus

#### Scenario: Channel mismatch fails preparation

- **WHEN** a root port and its same-named bus declare different channel counts
- **THEN** preparation SHALL fail with a structured diagnostic reporting both channel counts

#### Scenario: Unbound root output fails preparation

- **WHEN** the root graph declares an output port for which the host declares no bus
- **THEN** preparation SHALL fail with a structured diagnostic naming the unbound port

#### Scenario: Unbound host input is silent

- **WHEN** the host declares an input bus the root graph does not consume
- **THEN** preparation SHALL succeed and the unused bus SHALL not affect rendering

### Requirement: Bus enumeration over FFI

The FFI SHALL expose enumeration of a prepared instrument's root ports (name, direction, signal type, channel count) and buffer binding per named bus, so JUCE, plugin, CLI, and offline hosts map buses to devices, plugin ports, or files without engine changes.

#### Scenario: Host enumerates ports before binding

- **WHEN** an FFI host queries a prepared instrument
- **THEN** it SHALL receive each root port's name, direction, signal type, and channel count

#### Scenario: Stereo host binds one bus

- **WHEN** a plain stereo host binds a single 2-channel `master` bus to its output buffers
- **THEN** rendering SHALL fill those buffers with the root `master` port's two channels

### Requirement: Render settings are host concerns

Sample rate, block size, and render duration SHALL be supplied by the host or render invocation, not declared inside graph definitions.

#### Scenario: Same patch renders at two sample rates

- **WHEN** the same root definition is prepared at 44100 Hz and 48000 Hz by different hosts
- **THEN** both preparations SHALL succeed without modifying the patch document

#### Scenario: Render settings in patch are rejected

- **WHEN** a patch document declares sample rate, block size, or duration
- **THEN** validation SHALL fail with a diagnostic explaining these are host or render-invocation settings
