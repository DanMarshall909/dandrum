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

When parallel audio paths converge with unequal accumulated latency, the compiler SHALL insert compensation delays so converging signals are time-aligned. The compiler SHALL also align every root audio output to the maximum accumulated root latency and report that latency to the host. Control and event edges SHALL NOT receive audio compensation delays.

#### Scenario: Unequal parallel paths are aligned

- **WHEN** a dry path (zero latency) and a latency-inducing wet path converge at a mixer
- **THEN** the compiled graph SHALL delay the dry path by the wet path's latency so both arrive aligned

#### Scenario: Host reads total latency

- **WHEN** a host queries a prepared instrument
- **THEN** it SHALL receive the root graph's total latency in samples for plugin latency reporting

#### Scenario: Root audio outputs share reported latency

- **WHEN** separate root audio outputs are fed by paths with unequal accumulated latency
- **THEN** the compiler SHALL delay the earlier outputs so every root audio output matches the single latency reported to the host

#### Scenario: Control and event edges are not audio-compensated

- **WHEN** control or event edges enter a node that also receives a latency-bearing audio path
- **THEN** latency balancing SHALL NOT insert an audio compensation-delay node on those control or event edges

#### Scenario: Composite latency accumulates through flattening

- **WHEN** a composite or `poly` node contains latency-inducing nodes
- **THEN** the flattened paths through it SHALL carry the accumulated internal latency for balancing at downstream convergences

#### Scenario: Inserted compensation is inspectable

- **WHEN** the compiler inserts a compensation delay
- **THEN** compilation diagnostics or discovery output SHALL report where it was inserted and by how many samples

### Requirement: Existing latency-inducing builtins declare true latency

Builtins with inherent processing latency — including the spectral processor (`fft_size - 1` samples) and overlap-add convolution (one partition block) — SHALL declare their actual latency in the module registry, and dry/wet topologies around them SHALL render time-aligned.

#### Scenario: Convolution dry/wet paths align

- **WHEN** an impulse is rendered through a graph mixing a dry path with a unit-impulse-IR convolution path
- **THEN** the two contributions SHALL arrive at the mix on the same sample

#### Scenario: Spectral processor declares fft-dependent latency

- **WHEN** a spectral processor node is compiled with a resolved FFT size
- **THEN** its declared latency SHALL reflect that FFT size and participate in path balancing

### Requirement: Latency inside feedback cycles is rejected

Compensation cannot be inserted inside a feedback loop, so compilation SHALL fail with a structured diagnostic when a feedback cycle contains any node reporting nonzero latency.

#### Scenario: Latency-bearing node in a cycle is rejected

- **WHEN** a feedback cycle through a `feedback_delay` node also contains a nonzero-latency node such as a convolution
- **THEN** compilation SHALL fail with a diagnostic naming the cycle, the node, and its latency rather than rendering a time-smeared loop
