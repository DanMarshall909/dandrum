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

#### Scenario: One-shot region plays from a trigger

- **WHEN** a `sample_player` receives a note trigger for a prepared region in one-shot mode
- **THEN** it SHALL start at the region's first frame, emit its signed samples through the region end, and then remain silent until retriggered

#### Scenario: Drum sample voices play independently

- **WHEN** a bounded `poly` region routes simultaneous drum notes to a `sample_player` child
- **THEN** each allocated voice SHALL render its own one-shot instance and the voice outputs SHALL sum

#### Scenario: Drum map voice count uses bounded polyphony

- **WHEN** a drum map player declares two voices and receives overlapping hits for different zones
- **THEN** preparation SHALL create one bounded poly region with two voices, render the sum of both selected hits, and recycle each voice when its one-shot region finishes; omitting the voice count SHALL use the declared default of sixteen

#### Scenario: Overlapping drum voices share alternate selection

- **WHEN** overlapping drum hits select round-robin or seeded weighted zones on separate voices
- **THEN** the selected sequence SHALL advance at kit level and remain the same across equivalent audio block sizes

#### Scenario: Simultaneous drum hits survive a one-frame host block

- **WHEN** multiple drum notes arrive at the same frame in a one-frame host block
- **THEN** prepared event queues SHALL retain the bounded chord, render the signed sum of its selected voices, and perform no heap allocation while routing and rendering it

#### Scenario: Drum voice stealing follows the configured policy

- **WHEN** a drum map has no free voice and receives another hit with `oldest`, `quietest`, or `reject_new` selected
- **THEN** it SHALL respectively replace the oldest voice, replace the voice with the lowest measured audio peak, or preserve both voices; a single-voice map with `reject_new` SHALL accept another hit after its current sample finishes

### Requirement: Choke groups control overlapping drum articulations

Zones MAY declare a named choke group. A newly selected zone SHALL affect active voices in the same group only. A prepared `sample_map_player` SHALL support sample-accurate `cut`, bounded `fade`, and `release` modes when at least two voices are available. `release` SHALL send a gate release while allowing a one-shot sample to finish naturally. Preparation SHALL reject fade durations outside 1..=1000 milliseconds and fade or release on a single-voice map.

#### Scenario: Closed hat cuts its open articulation

- **WHEN** a closed hat in the same choke group starts partway through a block while an unrelated kick and open hat are sounding
- **THEN** the open hat SHALL be silent starting at the closed hat event frame, the kick SHALL continue, and equivalent block splits SHALL render the same signed samples

#### Scenario: Hat choke fades across blocks

- **WHEN** fade choke starts on a sustained open hat with a configured one-millisecond fade
- **THEN** the old voice SHALL ramp from its current level to silence over the prepared frame count and retire at the fade end across audio block boundaries

#### Scenario: Hat choke releases a one-shot tail

- **WHEN** release choke starts a closed hat while an open hat one-shot is playing
- **THEN** the open hat SHALL receive gate release and continue its one-shot tail until the prepared region ends

#### Scenario: Unsupported choke configuration is rejected

- **WHEN** a fade duration is outside 1..=1000 milliseconds or fade or release is selected for one voice
- **THEN** preparation SHALL reject the map player before rendering with a structured unsupported-mode diagnostic

#### Scenario: Gated region stops on release

- **WHEN** a `sample_player` in gated mode receives a note-off event at a frame inside a block
- **THEN** its output SHALL be silent starting at that frame and a later note-on SHALL restart from the region's first frame

#### Scenario: Looped region wraps and crossfades

- **WHEN** a `sample_player` in looped mode reaches its prepared loop end, with or without a declared crossfade
- **THEN** playback SHALL wrap to the prepared loop start, blend the overlap when present, and stop at the gate release frame

#### Scenario: Pitch ratio reads the control signal during playback

- **WHEN** a `sample_player` receives a fractional pitch ratio and a declared nearest, linear, or cubic interpolation mode
- **THEN** its source cursor SHALL advance by that ratio per output frame and its signed output SHALL follow the selected interpolation rule within the prepared region

#### Scenario: Reversed region plays toward its first frame

- **WHEN** a prepared region declares reverse playback and a `sample_player` receives a trigger
- **THEN** playback SHALL begin at the region's last frame, move toward its first frame, and stop or wrap according to its declared playback mode

#### Scenario: Region fades scale boundary samples

- **WHEN** a prepared region declares fade-in and fade-out durations
- **THEN** playback SHALL scale the signed samples at its beginning and end according to those durations, including when the region is reversed

#### Scenario: Oversized host block preserves sample playback

- **WHEN** a host requests a root-bus render longer than the prepared maximum block size while sample triggers and releases occur inside it
- **THEN** the renderer SHALL split the work into prepared-size segments and emit the same samples as equivalent smaller host calls

### Requirement: Sample maps select prepared regions

A sample map SHALL contain at least one zone that references a prepared region and declares MIDI key ranges within `0..=127` and velocity ranges within `1..=127`. Zone declarations SHALL accept optional per-zone gain, pan, pitch offset, region overrides, round-robin groups, positive relative weights, and exclusive/choke groups. Zone gain SHALL be within -96..=24 dB, pan within -1..=1, and pitch offset within -48..=48 semitones. Preparation SHALL resolve zones to stable source and region indices in authored order and build a keyed map lookup before rendering. For a matching note and velocity, selection SHALL follow the map's declared mode and seed with a stable zone order; it SHALL be repeatable for the same event stream regardless of audio block size, file order, or map storage iteration order. Schema validation SHALL reject zero weights. Preparation SHALL reject invalid ranges or numeric modifiers, unresolved region references, duplicate IDs, and ambiguous overlapping zones without a declared tie-breaking mode.

#### Scenario: Drum map resolves its zones before rendering

- **WHEN** a drum map references declared sample regions by ID
- **THEN** preparation SHALL retain authored zone order and resolve every zone to stable source and region indices

#### Scenario: Drum key range selects a hit

- **WHEN** a note-on falls inside a zone's inclusive key range
- **THEN** the map player SHALL trigger that zone's prepared region, while a note outside all key ranges SHALL leave the current playback unchanged

#### Scenario: Velocity layers choose different regions

- **WHEN** two zones for the same drum note cover separate velocity ranges
- **THEN** each matching note event SHALL select the region for its velocity range

#### Scenario: Zone modifiers shape the selected hit

- **WHEN** a zone selects a prepared region with per-zone gain, pan, and pitch offset
- **THEN** the rendered hit SHALL use that region, combine region and zone gain and pan, and advance its source cursor by the zone pitch offset

#### Scenario: Invalid zone modifiers are rejected

- **WHEN** a zone declares gain, pan, or pitch offset outside the supported bounds
- **THEN** preparation SHALL reject the zone before rendering

#### Scenario: Round-robin selection is repeatable

- **WHEN** the same note-event stream is rendered twice with the same sample map and selection seed
- **THEN** round-robin or weighted selection SHALL produce the same region sequence across both renders and across equivalent block splits

#### Scenario: Live variation chooses a compatible alternate

- **WHEN** a drum-map hit has multiple matching alternates in the same round-robin and choke groups and the public variation control rises above zero
- **THEN** playback SHALL select a later compatible alternate without changing the kit-level turn order or choke ownership

#### Scenario: Source declaration order does not change weighted hits

- **WHEN** the same weighted drum map is prepared with its sample sources declared in a different order
- **THEN** equivalent note events SHALL select the same signed hit sequence at different audio block sizes

#### Scenario: Zero-weight zone is rejected

- **WHEN** a sample-map zone declares zero relative weight
- **THEN** schema validation SHALL reject the zone before rendering

#### Scenario: Invalid zone reference is rejected

- **WHEN** a sample-map zone names a region that no prepared source provides
- **THEN** preparation SHALL reject the map before rendering

### Requirement: Explicit slices and optional timing metadata remain prepared assets

A sample source SHALL support an explicit, ordered slice table whose entries name bounded source-frame ranges for break-slicer playback. A numeric slice index SHALL address that authored table directly, starting at zero. The source declaration SHALL accept optional cue points and an explicit beat grid with source-frame positions. Preparation SHALL validate slice, cue, and beat positions against the decoded source and SHALL make the prepared values available to playback or metadata consumers. Downbeats SHALL identify beats in the same grid. Automatic transient or beat detection SHALL NOT be required to use an explicit slice table.

#### Scenario: Explicit slice indices remain stable

- **WHEN** a source declares multiple named slices in order
- **THEN** preparation SHALL retain that order as a stable zero-based slice table

#### Scenario: Break patch selects a declared slice

- **WHEN** a break patch triggers a numeric index in a valid explicit slice table
- **THEN** playback SHALL use that slice's prepared source-frame range

#### Scenario: Slice beyond source length is rejected

- **WHEN** an explicit slice contains a frame position beyond its decoded source
- **THEN** preparation SHALL reject the slice table before rendering
