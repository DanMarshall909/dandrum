## Purpose

Apply supported structural edits automatically through a muted, callback-safe replacement transaction while preserving the DAW transport, live parameter bindings and the last working configuration.

## ADDED Requirements

### Requirement: Structural edits automatically rebuild

An admitted structural edit SHALL immediately mute this plugin and close audio access to the replaceable engine before validation and preparation begin. The plugin SHALL validate and rebuild off the audio thread, activate the rebuilt instrument and automatically unmute. It SHALL NOT introduce draft mode, an Apply button, confirmation or an unapplied-changes workflow. Each active prepared definition SHALL remain immutable; structural edits SHALL replace it rather than mutate it from audio.

#### Scenario: Edit starts a rebuild without an additional action

- **GIVEN** a working instrument and an idle structural rebuild service
- **WHEN** a supported structural edit is admitted through either renderer
- **THEN** this plugin SHALL become muted before validation or preparation starts and return an asynchronous job identity
- **AND** no additional Apply, confirmation or reload action SHALL be required

#### Scenario: Successful rebuild resumes the edited instrument

- **WHEN** validation and preparation of an admitted edit succeed
- **THEN** the plugin SHALL safely activate the edited configuration, publish its prepared document with a new generation, and unmute automatically
- **AND** subsequent MIDI notes SHALL render the rebuilt instrument
- **AND** voices, held notes and effect tails MAY reset without overlapping engines, state migration or live preview

### Requirement: Rebuilding never blocks the audio callback

Callbacks beginning after structural edit admission and before resume SHALL clear all plugin outputs and return silence without accessing a replaceable engine, waiting, taking rebuild locks, allocating rebuild resources, compiling, loading assets, joining workers or reclaiming engines. The plugin SHALL NOT request the DAW to stop or pause transport. Rebuilding SHALL affect only the edited plugin instance.

#### Scenario: Preparation is stalled while callbacks continue

- **WHEN** a rebuild worker is deliberately stalled before preparation completes and the host continues callbacks
- **THEN** every callback started after admission SHALL overwrite all plugin output samples with exact zero and return independently of that worker
- **AND** no rebuild, engine access, engine destruction, blocking synchronization or resource cleanup SHALL execute on those callbacks

#### Scenario: DAW and another instance continue playing

- **WHEN** one plugin instance rebuilds during DAW playback while another instance is sounding
- **THEN** the DAW transport SHALL continue advancing and the other instance SHALL remain audible
- **AND** only the rebuilding instance SHALL output silence until it resumes

### Requirement: Engine ownership is handed off safely

The plugin SHALL use an explicit callback ownership handoff before replacing, mutating or reclaiming an engine or its resources. An already-entered callback SHALL be allowed to leave its old-engine access before retirement. The worker/coordinator SHALL perform preparation and cleanup off audio; audio SHALL never wait for it. Muting, pointer exchange, suspension or a fixed delay alone SHALL NOT be treated as proof that engine access has ended. No new callback SHALL access the old engine after the gate closes.

#### Scenario: A callback already owns the previous engine

- **WHEN** an edit is admitted while a callback is already accessing the old engine
- **THEN** the old engine SHALL remain valid until that callback releases its reader ownership
- **AND** later callbacks SHALL return silence without acquiring the engine
- **AND** retirement and destruction SHALL happen off audio after acknowledgement, regardless of how long the earlier reader took

#### Scenario: Rebuild occurs without further host callbacks

- **WHEN** an edit is admitted while no callback owns the engine and the host issues no further callbacks
- **THEN** ownership handoff and rebuild SHALL complete without requiring a future callback
- **AND** the first callback after automatic resume SHALL use the selected working engine safely

#### Scenario: Editor or processor closes during rebuild

- **WHEN** an editor closes during a rebuild
- **THEN** processor-owned rebuild and recovery SHALL continue and a reopened editor SHALL obtain current job and configuration state
- **AND** processor destruction SHALL finish worker and engine cleanup off the audio callback without dangling access

### Requirement: Structural rebuilds are serialized

The plugin SHALL execute one structural rebuild at a time. Both renderers SHALL temporarily disable structural controls while rebuilding. The shared service SHALL reject another structural request as busy without storing queued or unapplied structural changes. Ordinary exposed parameter controls SHALL remain on their normal live path.

#### Scenario: Another structural request arrives during rebuild

- **WHEN** another structural edit arrives while a rebuild is active, including from another editor session
- **THEN** it SHALL report busy without starting another worker, changing the working configuration or queuing the edit
- **AND** structural controls SHALL remain disabled until the active rebuild succeeds or recovers

### Requirement: A failed rebuild automatically restores the working configuration

Validation, asset-loading, compilation, worker-startup or activation failure SHALL restore the last working configuration and engine access, automatically unmute and re-enable supported structural controls. The plugin SHALL report a queryable job error to the UI, preserve the working prepared document and generation, reconcile displays to that document and clean failed resources off audio. It SHALL NOT remain muted pending confirmation or retain a hidden unapplied configuration.

#### Scenario: Edited configuration cannot be prepared

- **WHEN** an admitted edit fails validation, sample loading or compilation
- **THEN** new notes SHALL again render the last working configuration after automatic recovery
- **AND** its prepared document and generation SHALL remain authoritative, structural controls SHALL be re-enabled, and the error SHALL be visible
- **AND** a subsequent valid structural edit SHALL be admissible without clearing a draft or pressing Apply

#### Scenario: Worker startup or activation fails

- **WHEN** the rebuild worker cannot start or the candidate cannot safely activate, including obsolete host preparation settings
- **THEN** the last working configuration SHALL resume automatically and the job SHALL report the failure
- **AND** partial resources SHALL be reclaimed off audio and the plugin SHALL not remain muted

### Requirement: Structural rebuilding preserves the host parameter surface

Existing host parameter objects, IDs, count/order, assigned slots and automation bindings SHALL remain stable through successful and failed structural rebuilds. The first implementation SHALL reject edits that add, remove, rename or reorder exposed parameters or change their host ranges. Latest public parameter values SHALL be retained during rebuilding and applied to the engine that resumes.

#### Scenario: Compatible structural edit retains automation identities

- **WHEN** a compatible structural edit succeeds, or a rebuild fails and recovers
- **THEN** existing automation SHALL still address the same public IDs and host parameter objects and slots
- **AND** current public parameter values SHALL be retained without reconstructing or silently remapping the host surface

#### Scenario: Edit would change the exposed host surface

- **WHEN** validation finds that an edit changes the exposed host parameter identity, count/order or range
- **THEN** the edit SHALL fail with an incompatibility error and automatically resume the previous configuration
- **AND** the original host parameter objects, slots and automation bindings SHALL remain unchanged

### Requirement: Ordinary public parameter changes remain live

Exposed parameter changes such as cutoff and volume SHALL use their ordinary live host bindings without structural validation, instrument rebuilding, output muting or generation changes. They SHALL remain admissible during structural rebuilding; the latest admitted host values SHALL be used on both successful activation and failure recovery.

#### Scenario: Ordinary parameter changes while playing

- **WHEN** a host or either editor changes an ordinary exposed cutoff or volume parameter
- **THEN** the running instrument SHALL receive the value through its live binding without a structural job, mute or replacement

#### Scenario: Automation changes while a rebuild is pending

- **WHEN** an ordinary exposed parameter changes after rebuild admission but before success or recovery
- **THEN** the host binding SHALL retain the latest value without starting another rebuild
- **AND** the engine that resumes SHALL use that value rather than an earlier preparation snapshot
