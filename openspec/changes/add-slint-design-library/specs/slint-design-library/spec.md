## ADDED Requirements

### Requirement: Complete composable guide library

The library SHALL export every UI component in the design guide manifest plus a themed scrollbar, using reusable subcomponents and the original icons/fonts.

#### Scenario: A consumer imports the guide vocabulary
- **WHEN** the catalog imports the public library surface
- **THEN** every guide UI component and scrollbar compiles and appears in a documented catalog section
- **AND** authored Slint component files remain at or below 200 lines, with generated token/asset files identified separately

### Requirement: Shared native design tokens

The library SHALL consume generated Slint colours, lengths, typography and aliases from the same maintained token source as CSS and C++.

#### Scenario: Semantic tokens change
- **WHEN** a colour or spacing token and its aliases change
- **THEN** all renderer outputs update consistently and the drift check rejects independently modified Slint output without rewriting it

### Requirement: Accessible parameter interaction

Continuous controls SHALL expose actual parameter ranges, typed edits, cancellation, reset, fine adjustments, enabled/read-only states and balanced gesture callbacks. The knob SHALL use a pointer-free cap and a value popup.

#### Scenario: An editable value is committed or cancelled
- **WHEN** a user edits a bounded parameter through pointer, keyboard or typed entry
- **THEN** accepted values stay within the declared range, cancellation preserves the previous value, and callbacks describe accepted gestures
- **AND** disabled or prepared read-only controls emit no edit commands

### Requirement: Predictable layout and navigation

Panels SHALL collapse independently of their header actions; tabs and segmented choices SHALL support keyboard and pointer selection. Feedback SHALL distinguish empty, busy, error and available data.

#### Scenario: A header action is activated
- **WHEN** a user activates a panel header action or toggles the collapse control
- **THEN** the action emits its own identifier without collapsing the panel, while the collapse control updates only the panel's collapsed state

### Requirement: Supplied display data and capabilities

Mapping, layers, routing, waveforms and measurements SHALL render supplied data and explicit unavailable/rebuilding states. Structural commands SHALL require the corresponding capability; measurements SHALL distinguish missing data from valid silence.

#### Scenario: A prepared display has no editing capability
- **WHEN** a supplied zone, source, module or bus is displayed without a supported edit capability
- **THEN** its real labels and ranges remain inspectable and unsupported mutation requests emit no command
- **AND** audition cancellation releases the editor-owned note

### Requirement: Reviewable native catalog

The library SHALL include a runnable catalog that composes its public components with controlled fixture data and documents the host integration boundary.

#### Scenario: The catalog runs at compact and default sizes
- **WHEN** the catalog is compiled and rendered at 820×560 and 1200×800
- **THEN** navigation and component sections remain reachable with readable text, and representative controls respond to real UI input
- **AND** verification records identify the renderer and any unverified native host boundary
