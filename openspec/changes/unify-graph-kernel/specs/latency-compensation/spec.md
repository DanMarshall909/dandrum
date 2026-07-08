## ADDED Requirements

### Requirement: Nodes report processing latency

Every atomic node kind SHALL report its processing latency in samples (zero for most primitives; nonzero for lookahead, FFT, spectral, and convolution processing) as compile-time metadata available after static-argument resolution.

#### Scenario: Primitive reports zero latency

- **WHEN** a gain primitive's latency metadata is queried after compilation
- **THEN** it SHALL report zero samples

#### Scenario: Latency is available per resolved instance

- **WHEN** a latency-inducing node's latency depends on a static argument such as FFT size
- **THEN** the reported latency SHALL reflect the resolved static arguments

### Requirement: Compiler balances parallel path latency

When parallel paths converge with unequal accumulated latency, the compiler SHALL insert compensation delays so converging signals are time-aligned, and SHALL report the root graph's total latency to the host.

#### Scenario: Unequal parallel paths are aligned

- **WHEN** a dry path (zero latency) and a latency-inducing wet path converge at a mixer
- **THEN** the compiled graph SHALL delay the dry path by the wet path's latency so both arrive aligned

#### Scenario: Host reads total latency

- **WHEN** a host queries a prepared instrument
- **THEN** it SHALL receive the root graph's total latency in samples for plugin latency reporting

### Requirement: Unsupported nonzero latency fails compilation

Until compensation insertion is implemented, compilation SHALL fail with a structured diagnostic when any node reports nonzero latency. The contract SHALL NOT be silently ignored.

#### Scenario: Nonzero latency without balancer is rejected

- **WHEN** a graph contains a node reporting nonzero latency and the compiler build does not implement compensation
- **THEN** compilation SHALL fail with a diagnostic naming the node and its latency rather than rendering misaligned audio
