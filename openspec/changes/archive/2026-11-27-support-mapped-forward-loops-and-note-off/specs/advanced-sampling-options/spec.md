## ADDED Requirements

### Requirement: Mapped sample regions honor explicit playback modes

A prepared `sample_map_player` SHALL render each selected region as `one_shot`, `gated`, or `looped`. A region without an explicit mode SHALL behave as `looped` when it declares a forward loop and as `one_shot` otherwise. One-shot playback SHALL ignore note-off and finish at the region end. Gated and looped playback SHALL become silent at the matching note-off frame and release their voice. Unsupported or inconsistent modes SHALL fail preparation rather than silently change playback.

#### Scenario: Forward loop repeats the mapped signed waveform
- **WHEN** a native mapped zone selects a region with a forward loop and receives a note-on
- **THEN** playback SHALL cross its exclusive loop end into the declared loop start, repeat without leaving the source or region, and remain deterministic across audio block sizes

#### Scenario: Crossfade and reverse preserve the loop window
- **WHEN** a mapped region loops forward or in reverse with a zero, one-frame, or half-window crossfade at a supported source and host rate
- **THEN** signed output SHALL follow the declared blend and cursor rule without a skipped, duplicated, or out-of-range sample

#### Scenario: Note-off distinguishes one-shot and held playback
- **WHEN** one-shot, gated and looped mapped regions receive note-off within a block
- **THEN** one-shot SHALL finish its region, gated and looped output SHALL be silent starting at the note-off frame, and a later note-on SHALL restart at the selected start offset

#### Scenario: Voice policies retire held mapped samples safely
- **WHEN** retrigger, choke, voice stealing, note-off or reset affects a held mapped sample
- **THEN** the bounded poly region SHALL reuse or retire the intended voice without a stuck voice or an out-of-range sample read

#### Scenario: Invalid mapped playback metadata cannot replace audio
- **WHEN** a mapped region declares an invalid loop window, crossfade or unsupported playback mode
- **THEN** preparation SHALL reject it with a structured diagnostic before replacing the active instrument
