## ADDED Requirements

### Requirement: New sampler patches provide a replaceable default amplitude assignment

A new Trigger sampler patch SHALL create a reusable per-voice envelope assigned to voice amplitude by default and SHALL expose its conventional amplitude-envelope controls. Each new layer SHALL have a default route from the Velocity performance modulator to that layer's gain destination. This velocity response SHALL be composed through the same typed modulation facilities as other routes, with no implicit velocity-to-gain scaling in the sample player or layer selector. The layer's static gain trim and its velocity-controlled gain SHALL remain distinct. Assignment semantics SHALL use stable module and destination identity and SHALL not depend on a modulator's display name or historical ID. Existing silent-mock default-envelope patches SHALL migrate with their stored values, visible controls and explicit Velocity destinations and depths intact; migration SHALL not add a velocity route to a pre-existing layer or duplicate a velocity gain stage. The new native default SHALL match an independently calculated signed reference for those values.

#### Scenario: New patch has a familiar default
- **WHEN** a new sampler patch is prepared and a supported note is rendered
- **THEN** the Voice page SHALL show the default amplitude-envelope controls, each layer SHALL show its Velocity-to-layer-gain route, and signed output SHALL match the declared default envelope, base gain and layer velocity response

#### Scenario: Velocity changes the gain of a selected layer through its default route
- **WHEN** two notes at different velocities select the same layer with the default Velocity route enabled
- **THEN** that layer's output SHALL follow the declared velocity response with known signed values, before voice-amplitude envelope shaping, without changing the layer's static gain trim

#### Scenario: Removing a layer velocity route removes its gain response
- **WHEN** the default Velocity-to-layer-gain route is disabled through a supported modulation edit and two velocities still select the same layer
- **THEN** the accepted graph SHALL contain no hidden velocity gain scaling for that layer, and its known signed output SHALL reflect its static trim and remaining routes

#### Scenario: Rename does not change the assignment
- **WHEN** the assigned default envelope is renamed and the patch is prepared again
- **THEN** the amplitude assignment and signed render SHALL remain unchanged

#### Scenario: Historic default values migrate to a native reference
- **WHEN** a saved patch with the historic fixed `Amp envelope` route is loaded
- **THEN** its envelope route SHALL become the default voice-amplitude assignment with stable identities, its stored and visible envelope values and any explicit Velocity route SHALL match the old mock fixture without redirection or duplication, and its new native render SHALL match the signed reference for those values

### Requirement: Advanced amplitude assignment is reversible and capability checked

The advanced Trigger workflow SHALL provide Replace modulator, Detach assignment and Restore default for the envelope-to-voice-amplitude route without requiring graph editing in the normal view. A supported source SHALL be assignable to voice amplitude and another compatible destination under #8's scope and combination rules. Detach SHALL leave the base voice-amplitude path and any selected layer's independent velocity-to-gain route audible for new notes. Replace, Detach and Restore SHALL preserve those velocity routes where present. Unsupported sources, destinations, scopes, types or cable multiplicities SHALL be rejected before replacing the accepted patch. An adapter without native modulation SHALL report the action unavailable instead of accepting a simulated edit.

#### Scenario: Replace with a reusable source
- **WHEN** an advanced user replaces the default source with a supported per-voice envelope already routed to a compatible filter destination
- **THEN** one source instance SHALL drive both destinations for each voice, and signed amplitude and filter renders SHALL reflect that source

#### Scenario: Per-voice and shared sources retain their scope
- **WHEN** two notes use independent per-voice envelopes and an explicitly shared source under the accepted #8 scope contract
- **THEN** each envelope SHALL follow only its own note while the shared source SHALL retain one shared state across the two voices

#### Scenario: Detach leaves base amplitude and layer velocity response audible
- **WHEN** the amplitude assignment is detached and a new note is played
- **THEN** the new note SHALL render at its bounded base voice amplitude without envelope shaping, retain any selected layer's Velocity-to-gain response, and leave the detached envelope's other routes intact

#### Scenario: Restore the default
- **WHEN** the user restores the default after replacement or detach
- **THEN** the default envelope-to-voice-amplitude assignment and familiar controls SHALL return without deleting unrelated modulators or routes, including the layer Velocity routes

#### Scenario: Invalid replacement preserves the accepted patch
- **WHEN** a source or destination is incompatible by type, domain, scope or multiplicity
- **THEN** preparation SHALL reject the command with a useful diagnostic and SHALL leave the accepted assignment, output and history unchanged

#### Scenario: Native adapter explains missing support
- **WHEN** an adapter cannot compile and render amplitude modulation
- **THEN** the assignment action SHALL be unavailable with an explanation and SHALL not publish a mock-only edit as native state

### Requirement: Voice lifetime is independent of the amplitude assignment

Trigger's prepared poly-region voice SHALL use explicit completion from source and gate lifecycle, independent of the amplitude route, its name or temporary output level. One-shot sources SHALL retire when their prepared region ends; gated and looped sources SHALL release on matching note-off. Choking, stealing and reset SHALL leave no permanently occupied voice. A source whose completion cannot be represented safely SHALL fail preparation instead of relying on temporary silence.

The editor SHALL not imply that envelope Release creates audio after a source has stopped. For playback modes that stop at note-off, it SHALL explain or disable the ineffective release-tail control while preserving the accepted mapped-playback mode semantics.

#### Scenario: Silent modulation does not retire a held loop
- **WHEN** an active looped voice has zero amplitude from an assigned modulator while its note remains held
- **THEN** its bounded voice slot SHALL remain active until matching note-off or an explicit choke, steal or reset event

#### Scenario: One-shot completion ignores assignment and note-off
- **WHEN** a one-shot source receives note-off before its region end under default, replaced and detached assignments
- **THEN** each voice SHALL finish the region, retire its slot, and permit a later note to reuse that slot

#### Scenario: Gated and looped note-off reclaim voices
- **WHEN** a gated or looped source receives matching note-off under default, replaced and detached assignments
- **THEN** its output SHALL stop according to its playback mode and its voice slot SHALL become reusable without affecting another held note

#### Scenario: Release control reflects source lifetime
- **WHEN** a user views the amplitude envelope for a gated or looped source that stops at note-off
- **THEN** the editor SHALL explain or disable the ineffective release-tail control, and a signed render SHALL remain silent after the note-off frame

#### Scenario: Choke and steal stay bounded
- **WHEN** a choke or configured steal replaces an active voice under a replaced or detached amplitude assignment
- **THEN** only the intended voice SHALL be retired or released, later output SHALL contain no stale samples, and the configured voice limit SHALL hold

#### Scenario: Unmatched selection does not occupy a slot
- **WHEN** a note enters the bounded sampled voice definition but selects no playable zone
- **THEN** the attempted voice SHALL produce no audio and SHALL leave its slot available for a later matching note

### Requirement: Assignment commits and saved state preserve voice ownership

Replace, Detach and Restore SHALL be atomic prepared commands in the accepted native session. Preparation failure SHALL leave sounding voices and the accepted patch unchanged. On a successful structural commit, old voices SHALL retire and new notes SHALL use the new assignment; late note-off for the old generation SHALL not occupy a new voice. The editor SHALL disclose the interruption before committing an edit while notes are held. Undo, redo and saved-state restore SHALL preserve the selected assignment and its stable identities.

#### Scenario: Failed edit preserves held sound
- **WHEN** a held note sounds and an amplitude replacement fails preparation
- **THEN** its signed output, voice ownership, accepted assignment and undo history SHALL remain unchanged

#### Scenario: Successful held-note edit retires the old generation
- **WHEN** an amplitude replacement commits while a note is held
- **THEN** the old voice SHALL retire, a late note-off SHALL not claim a new voice, and a later note SHALL render through the new assignment

#### Scenario: Structural edit discloses possible interruption
- **WHEN** a user initiates Replace, Detach or Restore
- **THEN** the editor SHALL explain that a successful commit retires sounding voices before asking the user to commit the change

#### Scenario: Undo and redo restore assignment and sound
- **WHEN** Replace, Detach and Restore are each undone and redone
- **THEN** the accepted assignment, visible controls and signed output SHALL match the corresponding before and after states

#### Scenario: Save and reopen preserve the choice
- **WHEN** default, replaced and detached patches are each saved and opened in a fresh native session
- **THEN** each structural assignment and stable source/destination identity SHALL be restored and SHALL produce its expected signed render
