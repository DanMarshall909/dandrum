## ADDED Requirements

### Requirement: Trigger products share the persistent instrument

The JUCE-free Trigger beta SHALL provide deliberate VST3 and standalone products using the production React editor, existing Rust engine and the same format-neutral session behavior. Product adapters SHALL NOT introduce independent instrument histories or dependencies on JUCE.

#### Scenario: Installed VST3 renders the real Trigger instrument

- **WHEN** a host loads the installed VST3, instantiates Trigger and sends a mapped note and nondefault public value at documented offsets
- **THEN** it SHALL discover the intended identity and render the independently expected signed sample waveform through the persistent instrument, with usable parameter and state interfaces

#### Scenario: Standalone owns real audio and MIDI connections

- **WHEN** the installed standalone opens supported native audio/MIDI connections and receives notes while its production editor is available
- **THEN** it SHALL render the expected signed sample through its owned audio callback, expose the actual device configuration and release only its own handles on shutdown

### Requirement: Supported host clocks retain their precision

Trigger SHALL retain supported requested floating-point host sample rates through preparation, reconfiguration and playback without integer rounding or truncation. Invalid or unavailable configurations SHALL fail explicitly without partially changing an existing instance.

#### Scenario: Fractional host clocks render at the requested rate

- **WHEN** Trigger activates and reconfigures at supported fractional rates including 1234.5678 Hz and 48000.5 Hz
- **THEN** its accepted configuration and sample timing SHALL correspond to the requested clocks, preserve independent instance state and pass the previously failing official fractional-rate case

#### Scenario: Failed configuration preserves accepted playback

- **WHEN** a running or inactive instrument receives an invalid clock, incompatible configuration or failed candidate preparation
- **THEN** it SHALL retain its prior accepted patch, public values and usable signed output without disturbing another instance

### Requirement: Native editor lifetime is independent of playback

Each product SHALL own its native parent/main-loop responsibilities and dispose of its editor without destroying the persistent instrument. Admitted work, accepted edits and undo/redo SHALL survive editor loss, while view-owned notes, gestures and callbacks SHALL be released exactly once.

#### Scenario: Native editing survives complete editor replacement

- **WHEN** native input loads/maps a sample and edits controls, then the editor is destroyed during continuing audio and recreated
- **THEN** audio SHALL remain usable with the expected signed waveform, the accepted edits and undo/redo SHALL remain available, and no stuck note, gesture or stale application callback SHALL survive

#### Scenario: Failed native creation leaves the instrument usable

- **WHEN** a product receives invalid GUI ordering/parenting or actual native creation fails
- **THEN** it SHALL report failure, release partial view resources and preserve accepted state and signed audio, with a later valid creation remaining possible

### Requirement: Complete render callbacks obey realtime ownership

The complete product render callback SHALL use bounded event/parameter handoffs and prepared runtimes without locks, allocation/deallocation, file I/O, serialization, browser access or resource destruction. Runtime construction and reclamation SHALL occur under off-audio ownership.

#### Scenario: Native callbacks retain bounded ownership during replacement

- **WHEN** actual product callbacks process notes/automation while accepted runtime replacement, reset and editor lifecycle changes occur
- **THEN** allocation/deallocation sentinels and ownership checks SHALL remain clean for the complete callback, signed output SHALL follow accepted transitions, and an independent instance SHALL retain its own state and sound

### Requirement: Named DAW acceptance proves automation and independence

The beta SHALL be exercised in a named DAW with recorded version/configuration. Real host automation, editor-absent servicing, independent instances and fresh host-state restoration SHALL be proven through that DAW rather than inferred from an instrumented synthetic host.

#### Scenario: Two DAW instances retain automation and selected samples

- **WHEN** two differently edited Trigger VST3 instances receive DAW automation, one editor is closed/reopened and the DAW project is saved and reopened into fresh instances
- **THEN** each SHALL retain its own selected sample, patch and public values with expected signed playback, balanced editor gestures and later host feedback, without cross-instance mutation or stalled admitted work

### Requirement: Installed beta artifacts work offline without JUCE

The release artifact set SHALL install outside the source checkout and contain the required products, local editor/sample resources and notices. Its build and runtime dependency/linkage evidence SHALL show no JUCE requirement, and execution SHALL require neither checkout/caches nor a runtime asset download.

#### Scenario: Source-hidden offline installation remains usable

- **WHEN** the installed artifact set runs from an unrelated working directory with the checkout/build caches unavailable and network fallback disabled
- **THEN** VST3 and standalone SHALL load their intended resources and real editor, play/restore selected samples and expose audited dependencies/notices without loading or building JUCE
