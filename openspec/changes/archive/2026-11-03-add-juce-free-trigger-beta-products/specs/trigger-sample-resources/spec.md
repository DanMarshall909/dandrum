## ADDED Requirements

### Requirement: Native sample selection uses real supported data

Trigger SHALL let the production native editor select supported WAV files and obtain truthful decoded metadata. Selection, decoding and retention SHALL remain outside the render callback and within documented bounds; cancellation or invalid input SHALL preserve the accepted instrument.

#### Scenario: A distinct selected sample is mapped and played natively

- **WHEN** native input selects a distinct supported WAV and accepts a valid mapping through the production editor
- **THEN** the accepted patch SHALL refer to that sample's real metadata/content identity and render its independently known signed waveform only at the accepted mapping, without substituting the packaged kit or mock data

#### Scenario: Cancelled or invalid selection preserves the instrument

- **WHEN** selection is cancelled or a selected file is malformed, unreadable, unsupported or beyond the documented limits
- **THEN** no partial patch/resource publication SHALL occur and the prior patch, public values, history and signed audio SHALL remain usable

### Requirement: Accepted samples have immutable instance ownership

Each instance SHALL retain immutable owned sample content needed by its accepted, pending and historical patch states. Resource identity SHALL be based on verified owned content rather than a browser assertion, ambient path or shared mutable file.

#### Scenario: Original-file changes do not alter admitted playback

- **WHEN** a selected file is admitted and its original path is subsequently replaced or removed
- **THEN** accepted playback and native undo/redo SHALL continue from the admitted content, and another instance's resources and output SHALL remain independent

### Requirement: Interactive patch publication preserves native authority

Supported full-patch edits SHALL use bounded native admission, validation/preparation, atomic runtime publication and native history. They SHALL preserve stable supported public identities and current host-owned physical values; malformed, stale, conflicting or busy work SHALL not partially mutate accepted state.

#### Scenario: Import preserves nondefault controls and native history

- **WHEN** a sample import or related patch edit is accepted while public controls have nondefault host values, then the user invokes undo and redo
- **THEN** each accepted patch/resource transition SHALL occur once with the corresponding signed output and native history, while public identities and host-owned values remain intact

#### Scenario: Empty patch and ordinary import form a usable transition

- **WHEN** the production editor creates an empty patch and then imports one supported sample with its normal mapping flow
- **THEN** the empty patch SHALL be accepted and silent, the valid import SHALL publish without an accidental overlap/layout failure, and the new mapping SHALL play the selected signed waveform with the supported public surface intact

#### Scenario: Failed patch candidates retain the accepted instrument

- **WHEN** a full-patch request has invalid resource authority, incompatible ports/ranges, stale ownership, conflicting identity, backpressure or failed preparation
- **THEN** it SHALL return an explicit bounded result and preserve the complete accepted patch, resources, public feedback/history and signed audio, permitting a later valid request

### Requirement: Sample work survives disposable editor loss

Admitted sample/patch work and its retained outcomes SHALL belong to the instrument rather than its WebView. Unadmitted cancelled work and retired callbacks SHALL release their own resources without changing accepted state or another instance.

#### Scenario: Lost editor recovers one admitted sample edit

- **WHEN** an admitted selected-sample edit loses its reply and the editor is destroyed before a new editor reconnects
- **THEN** the result SHALL be recoverable exactly once with the selected sample, accepted history and signed playback, and native undo SHALL remain usable without a late callback to the destroyed view

### Requirement: Portable state owns patch samples and physical values

CLAP host state SHALL use a bounded versioned representation of the accepted patch, verified sample content and named physical parameter values. Fresh restoration SHALL not depend on the original selected paths, the original instance or a shared checkout/cache resource directory.

#### Scenario: Fresh instance restores selected content after source removal

- **WHEN** an edited selected-sample patch and nondefault parameters are saved, the original instance and selected files are removed, and a fresh instance loads that state
- **THEN** it SHALL restore the same logical patch/resource identities and public values and render the independently expected signed waveform from the restored mapping, with no original-path or packaged-sample fallback

#### Scenario: Corrupt portable state preserves the live instrument

- **WHEN** state has corrupt or mismatched sample content, unsupported version, incompatible parameters, invalid lengths/counts, excessive size or short-reading data
- **THEN** loading SHALL fail before partial publication and the prior accepted patch, resources, feedback/history and signed audio SHALL remain usable

#### Scenario: Packaged-kit state remains readable

- **WHEN** a valid existing version 2 packaged-kit state is loaded into the beta
- **THEN** its accepted mapping, packaged sample and named physical values SHALL restore with the same known signed playback and explicit rejection of invalid legacy state
