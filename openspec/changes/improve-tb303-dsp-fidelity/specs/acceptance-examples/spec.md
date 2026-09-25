## ADDED Requirements

### Requirement: TB-303 acid acceptance example sustains an evolving filter spectrum

The TB-303 acid acceptance example SHALL retain meaningful spectral energy through the middle of a held note and SHALL continue evolving toward a darker late-note spectrum instead of collapsing immediately to a static fundamental.

#### Scenario: Held unaccented note retains mid-decay brightness

- **WHEN** the TB-303 acid example renders a held unaccented note at 48 kHz
- **THEN** its Hann-windowed spectral centroid around 500 ms SHALL remain at or above 500 Hz
- **AND** the later centroid around 750 ms SHALL be at least 10 percent lower than the mid-decay centroid
- **AND** the rendered samples SHALL remain finite

### Requirement: TB-303 acid acceptance example drives deliberate resonance

The TB-303 acid acceptance example SHALL drive its resonant low-pass filter through an explicit non-zero resonance control rather than relying on the filter primitive's zero-resonance default.

#### Scenario: Acid patch declares a resonance route

- **WHEN** the TB-303 acid example is loaded and its cables are inspected
- **THEN** a control output SHALL be connected to the filter's `resonance` port
- **AND** that route SHALL provide a non-zero value during a held note

### Requirement: TB-303 acid accent changes level and timbre

The TB-303 acid acceptance example SHALL use note velocity to make accented notes both louder and brighter than unaccented notes while preserving finite output.

#### Scenario: Accented note is louder and brighter

- **WHEN** otherwise identical held notes are rendered at MIDI velocities 80 and 120
- **THEN** the accented note's RMS level SHALL be at least 10 percent greater
- **AND** the accented note's mid-decay spectral centroid SHALL be greater
- **AND** both renders SHALL contain only finite samples
