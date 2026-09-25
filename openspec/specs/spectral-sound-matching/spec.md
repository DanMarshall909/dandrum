## Purpose

Define deterministic offline spectral matching of modular instrument patches against aligned reference audio, including bounded search, reproducible artifacts, and realtime isolation.

## Requirements

### Requirement: Sound fixtures declare a bounded matching problem

A sound fixture used for matching SHALL declare a finite analysis region, deterministic search seed, maximum evaluation count, multi-resolution spectral windows, objective weights, and the public numeric patch parameters that may change. Dandrum SHALL reject undeclared, non-numeric, unbounded, duplicate, or otherwise invalid search parameters before rendering candidates.

#### Scenario: Valid matching declaration resolves public parameters

- **WHEN** the maintained TB-303 sound fixture is loaded for matching
- **THEN** every declared search parameter SHALL resolve to one public numeric patch parameter with finite ordered bounds
- **AND** the matching region SHALL fall within the fixture render
- **AND** the spectral windows and objective weights SHALL be finite and usable

#### Scenario: Invalid matching declaration is rejected before search

- **WHEN** a matching declaration names an unknown parameter or contains invalid bounds, region, seed, evaluation count, spectral window, or weight
- **THEN** Dandrum SHALL reject the declaration with an actionable diagnostic
- **AND** it SHALL NOT begin candidate rendering

### Requirement: Matching uses aligned multi-resolution spectral and trajectory evidence

Dandrum SHALL compare the declared regions of candidate and aligned reference audio after one fixed least-squares gain alignment per candidate. The objective SHALL report separate multi-resolution log-spectral, RMS-envelope, and spectral-centroid losses and a weighted total; it SHALL NOT normalize individual frames independently.

#### Scenario: Matching identical audio produces the minimum loss

- **WHEN** the same finite non-silent signal is supplied as candidate and reference
- **THEN** every objective component and the weighted total SHALL be zero within numerical tolerance

#### Scenario: Spectral and temporal differences increase their owning terms

- **WHEN** otherwise comparable candidate audio differs in harmonic balance, level trajectory, or spectral-centroid trajectory
- **THEN** the corresponding objective term SHALL increase above the identical-audio result
- **AND** every reported component SHALL remain finite

#### Scenario: Reference format does not satisfy the fixture contract

- **WHEN** a reference is not supported PCM WAV, uses another sample rate, is shorter than the matching region, or contains unusable silence
- **THEN** matching SHALL fail with an actionable diagnostic before optimization

### Requirement: Parameter search is deterministic, bounded, and cancellable

Dandrum SHALL run a seeded offline bounded search over only the declared public parameters. It SHALL report monotonic best-so-far progress, respect the evaluation budget and parameter bounds, and observe cooperative cancellation between candidate renders.

#### Scenario: Same inputs reproduce the same search

- **WHEN** matching is run twice with the same fixture, reference bytes, seed, and configuration
- **THEN** both runs SHALL evaluate the same candidate history
- **AND** they SHALL return the same best values and score breakdown

#### Scenario: Search improves a recoverable self-reference

- **WHEN** the reference was rendered by the same patch with declared parameter values different from the defaults
- **THEN** the final best total loss SHALL be lower than the default candidate loss
- **AND** all best values SHALL remain within their declared bounds

#### Scenario: User cancels a running search

- **WHEN** cancellation is requested after at least one candidate has been evaluated
- **THEN** matching SHALL stop before exhausting the remaining budget
- **AND** it SHALL return the best completed candidate marked as cancelled

### Requirement: Match results are coherent reproducible artifacts

A completed or cancelled match SHALL preserve one coherent result containing reference identity, fixture identity, seed and configuration, evaluation history, best public values, fixed gain alignment, score breakdown, candidate/reference trajectories, and playable candidate/reference WAV data.

#### Scenario: Match artifact records provenance and audition data

- **WHEN** matching returns a best candidate
- **THEN** its manifest SHALL include a content fingerprint for the reference and the matching configuration needed to reproduce the search
- **AND** its best values, metrics, loss breakdown, and candidate WAV SHALL all describe that same evaluated candidate
- **AND** the retained reference WAV and metrics SHALL describe the input reference

### Requirement: Matching remains outside realtime processing

Reference loading, hashing, offline rendering, FFT analysis, optimization, manifest construction, and WAV encoding SHALL NOT execute from Dandrum's realtime audio processing path.

#### Scenario: Realtime source remains free of matching work

- **WHEN** the realtime source boundary is inspected by the build guard
- **THEN** it SHALL contain no spectral-matching, reference-file, optimizer, hashing, or AI-provider operations
