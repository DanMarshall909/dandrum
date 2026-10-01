## ADDED Requirements

### Requirement: Invalid sampling declarations fail before rendering

Preparation SHALL report structured diagnostics for missing or unreadable sample files, unsupported decode formats, invalid region and loop boundaries, descending key or velocity ranges, and sample voice limits outside the supported bound. Unknown interpolation and choke modes SHALL fail validation instead of silently choosing another mode.

#### Scenario: Sample file cannot be used

- **WHEN** a declared sample source is absent or has an unsupported audio format
- **THEN** preparation SHALL fail with a diagnostic identifying the source and the file failure

#### Scenario: Invalid drum playback metadata cannot render

- **WHEN** a region or loop is malformed, a zone has a descending range, or a sample map player requests an invalid voice limit
- **THEN** preparation SHALL fail with a diagnostic for the relevant declaration

#### Scenario: Unsupported sampling mode cannot render

- **WHEN** a player declares an unknown interpolation or choke mode
- **THEN** validation SHALL fail with a structured diagnostic before rendering

### Requirement: Prepared sample assets serve the first three sampling families

The engine SHALL support reusable, file-backed sample sources and sample maps as prepared assets for drum-machine hits and layers, explicit break slices, and modest chromatic playback. A sample source SHALL have a stable ID and a resource path; a sample map SHALL have a stable ID and SHALL reference prepared regions by ID. Preparation SHALL resolve each source using the same patch or package resource-root and provenance rules as other sample resources, decode it into engine-owned audio, and retain its sample rate, channel count, and frame count before rendering. These assets SHALL NOT require file access or decoding on the audio thread.

#### Scenario: Drum patch reuses a prepared source

- **WHEN** a drum patch declares a sample source and regions used by multiple hits
- **THEN** preparation SHALL resolve and decode that source before rendering and SHALL make its regions available by stable ID

#### Scenario: Package sample path keeps its provenance

- **WHEN** a defined module from a pinned package declares a relative sample source path
- **THEN** preparation SHALL resolve it under that package version's resource root and SHALL reject a path that escapes the root

### Requirement: Sample regions describe bounded playback windows

A sample source SHALL support named regions with start and end frame positions inside the decoded source. The region declaration SHALL accept optional root note, gain, pan, reverse playback, fade-in, fade-out, and simple loop settings with start/end positions and optional crossfade. Gain SHALL be within -96..=24 dB and pan within -1..=1. Preparation SHALL reject empty or out-of-bounds regions, combined fades longer than the region, and loop crossfades longer than half the loop window. A player SHALL use the prepared region metadata without changing the source asset.

#### Scenario: Chromatic region carries its root note

- **WHEN** a chromatic patch declares a region with a root note and triggers a different note
- **THEN** preparation SHALL retain the root note and playback SHALL derive the note's pitch ratio from it

#### Scenario: Invalid region cannot reach rendering

- **WHEN** a declared region ends before it starts or extends beyond its decoded source
- **THEN** preparation SHALL reject the asset before rendering

### Requirement: Sample maps select prepared regions

A sample map SHALL contain zones that reference prepared regions and declare MIDI key ranges within `0..=127` and velocity ranges within `1..=127`. Zone declarations SHALL accept optional per-zone gain, pan, pitch offset, region overrides, round-robin groups, relative weights, and exclusive/choke groups. For a matching note and velocity, selection SHALL follow the map's declared mode and seed with a stable zone order; it SHALL be repeatable for the same event stream regardless of audio block size, file order, or map storage iteration order. Preparation SHALL reject invalid ranges, unresolved region references, and ambiguous overlapping zones without a declared tie-breaking mode.

#### Scenario: Velocity layers choose different regions

- **WHEN** two zones for the same drum note cover separate velocity ranges
- **THEN** each matching note event SHALL select the region for its velocity range

#### Scenario: Round-robin selection is repeatable

- **WHEN** the same note-event stream is rendered twice with the same sample map and selection seed
- **THEN** round-robin or weighted selection SHALL produce the same region sequence across both renders and across equivalent block splits

#### Scenario: Invalid zone reference is rejected

- **WHEN** a sample-map zone names a region that no prepared source provides
- **THEN** preparation SHALL reject the map before rendering

### Requirement: Explicit slices and optional timing metadata remain prepared assets

A sample source SHALL support an explicit, ordered slice table whose entries name bounded source-frame ranges for break-slicer playback. The source declaration SHALL accept optional cue points and an explicit beat grid with source-frame positions. Preparation SHALL validate slice, cue, and beat positions against the decoded source and SHALL make the prepared values available to playback or metadata consumers. Automatic transient or beat detection SHALL NOT be required to use an explicit slice table.

#### Scenario: Break patch selects a declared slice

- **WHEN** a break patch triggers a numeric index in a valid explicit slice table
- **THEN** playback SHALL use that slice's prepared source-frame range

#### Scenario: Slice beyond source length is rejected

- **WHEN** an explicit slice contains a frame position beyond its decoded source
- **THEN** preparation SHALL reject the slice table before rendering
