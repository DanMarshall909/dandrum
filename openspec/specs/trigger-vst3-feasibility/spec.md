# Trigger VST3 Feasibility

## Purpose

Define the Linux X11 and REAPER qualification for the direct Rust Trigger VST3
adapter, including installation, audio and automation, portable state, native
editing and the regression obligations for the shared CLAP product.

## Requirements

### Requirement: Installed direct adapter discovers the real Trigger instrument

The Linux VST3 proof SHALL use pinned direct Rust interface bindings and the
existing Trigger owners, with stable distinct class/parameter identities and
bundle-relative resources. It SHALL run independently of JUCE, the checkout,
original selected-sample paths, cwd and network access.

#### Scenario: Installed bundle is discovered outside the checkout
- **WHEN** the installed VST3 bundle is scanned and loaded by the dynamic host and qualified native REAPER configuration with the source checkout hidden and an unrelated working directory
- **THEN** the host discovers the deliberate Trigger instrument identity and stable public controls, loads the bundle's complete instrument/editor resources and renders an independently specified signed waveform
- **AND** missing/invalid required resources fail explicitly without a checkout fallback or a partly usable advertised instrument

### Requirement: One session outlives controller and view projections

Each processor instance SHALL have one authoritative Trigger session/history
with one exclusively owned audio endpoint. A controller/view SHALL be a
revocable projection and SHALL NOT create a competing musical session.

#### Scenario: Processor works without a controller
- **WHEN** a host creates only the processor, restores nondefault state, processes timed notes and flushes stopped controls
- **THEN** signed playback, parameter values and required instrument service work without creating a controller/editor or using their lifetime for ownership

#### Scenario: Controller disconnection preserves one instrument authority
- **WHEN** an initialized controller communicates through a host proxy exposing only standard interfaces, then disconnects or is destroyed and a fresh projection attaches to the same live processor
- **THEN** accepted document/resources/values and native Undo history remain intact, foreign/stale connections are rejected and termination revokes surviving projections without late access or reference cycles
- **AND** inline/reentrant replies or disconnection, failed transport sends and bounded pending-reply exhaustion cannot duplicate an accepted edit or partially mutate the instrument

### Requirement: Signed note processing and public values remain precise

The adapter SHALL preserve the shared stereo float32 Trigger rendering and
pitch/velocity semantics, exact supported host clocks and note offsets. It
SHALL convert every shared public value between normalized and physical units
using stable descriptors and reject unsupported layouts/configurations without
partly replacing the accepted instrument.

#### Scenario: Timed notes and public values render literal signed output
- **WHEN** the actual loaded adapter processes known sample fixtures with nondefault public controls, notes at distinct sample offsets and a supported fractional host clock
- **THEN** onset, silence, polarity and waveform values match an independent numerical oracle, all public endpoints/defaults and representative conversions round-trip within specified precision, and another instance remains unchanged
- **AND** a failed reconfiguration retains accepted playback and state

### Requirement: Automation honors the complete VST3 curve contract

The processor SHALL apply piecewise-linear normalized automation at the correct
sample, including implicit prior endpoints, adjacent-point jumps and cross-block
continuity. Host automation SHALL remain outside native edit history.

#### Scenario: Curves retain exact timing across block partitions
- **WHEN** a known signed source is processed with simultaneous parameter queues containing ramps, omitted frame-zero points, adjacent old/new jump points and timed notes across multiple block partitions
- **THEN** each output sample matches an independent curve/physical-value oracle, controls precede same-frame notes, host note order is retained and both partitionings have the specified waveform
- **AND** a control that shifts the curve by one sample or holds points as steps fails the proving assertion

#### Scenario: Stopped flush changes values without advancing sound
- **WHEN** valid normalized parameter changes arrive in zero-sample/no-audio flush calls with the editor absent and processing stopped
- **THEN** the authoritative physical values and later feedback update without advancing DSP time, playing notes or adding native Undo entries
- **AND** subsequent processing starts from those accepted values

#### Scenario: Invalid or excessive callback batches fail atomically
- **WHEN** a callback contains malformed/failed parameter retrieval, duplicate queues, invalid values/offsets/order, unsupported buffer configuration or more than the declared event capacity
- **THEN** accessible output is cleared and no subset of its notes/values or pending runtime transitions is consumed
- **AND** accepted state remains usable by a later valid callback with no audio-thread allocation, destruction, blocking lock, file I/O, serialization, browser access or unbounded retry, including the processing-state reset boundary

### Requirement: VST3 state uses the existing portable authority

Processor state SHALL adapt the shared bounded patch/sample/named-parameter
container through VST3 streams. Controller component-state handling SHALL only
derive its projection. A failed load SHALL preserve the current instrument.

#### Scenario: Fresh VST3 instance restores selected samples and all values
- **WHEN** nondefault patch/selected-sample/parameter state is saved, the source instance and original selected files disappear and a fresh isolated instance restores the state
- **THEN** logical resource/content identities and every named physical value restore and independent signed PCM/note assertions pass without source/cache fallback
- **AND** the shared packaged-kit legacy reader remains usable
- **AND** controller component-state restoration only synchronizes its projection without creating a session or sending automation/gesture callbacks to the host

#### Scenario: Invalid streams preserve working playback and history
- **WHEN** malformed, incompatible, excessive, corrupt, truncated or failing VST3 streams are loaded into an instance with nondefault sound and accepted native edits
- **THEN** loading fails before partial publication and document/resources/values/history/feedback and independently specified signed playback remain intact
- **AND** failed or short state writes have an explicit result rather than a false successful save
- **AND** successful partial reads/writes continue to support a complete round trip, with zero-progress operations terminating explicitly

### Requirement: Native editor recreation retains editing and host gestures

The existing production React/Wry editor SHALL attach to the host's real native
parent on the proper main thread. Required worker/reclamation/feedback service
SHALL outlive the view and work while processing is stopped. Native callbacks,
gestures and notes SHALL be revoked once on view loss or partial creation failure.

#### Scenario: Actual native editor destruction retains Undo and service
- **WHEN** native input imports/maps a selected sample and edits public values during real callbacks, then completely destroys the WebView/native child, removes the original source and creates a new editor for the same processor
- **THEN** the new production editor recovers accepted state and uses native Undo/Redo with the independently expected signed waveforms
- **AND** admitted work settles during stopped/editor-absent service, held gestures/notes end once, the host parent survives and no old-view callback mutates the new view

#### Scenario: Failed creation and missing service preserve a usable instrument
- **WHEN** native parenting/creation or required host-service registration fails
- **THEN** partial callbacks/registrations/native children are released and the host receives an explicit failure/capability result while accepted state, history and signed playback remain usable
- **AND** a later valid attach succeeds without duplicate service or late callbacks

#### Scenario: Host content scaling preserves the complete editor
- **WHEN** the Linux host requests 100%, fractional or 200% content scaling before attachment or while the native editor is open
- **THEN** the complete 1200 by 800 logical editor scales consistently with its physical native child and negotiated host view size, without an unused host canvas
- **AND** invalid scale factors and refused host resize requests preserve the previous working geometry and editor state

### Requirement: REAPER acceptance and existing CLAP behavior are proven

The installed current candidate SHALL earn actual qualified Linux/REAPER
acceptance separately from synthetic-host tests. Relevant CLAP/shared-owner
regressions SHALL pass. Evidence SHALL retain exact source/product/host/config
identities, real assertions, limits and independent review/publication results.
Reusable session/runtime/parameter/state and editor-command/orchestration behavior
SHALL use shared implementations; format ABI/lifecycle and platform edges SHALL
remain narrow adapters. Adding VST3 SHALL NOT introduce copied musical authority,
a parallel state codec or a forked editor behavior path.

#### Scenario: REAPER restores independent instances with timed automation
- **WHEN** qualified native REAPER runs two installed VST3 instances with distinct selected samples, native gestures and timed automation, closes/reopens editors and saves/reloads in a fresh process after the original files are removed
- **THEN** both instances preserve their independent patch/resources/values, feedback and actual signed rendered sample/automation timing match independent oracles, and editor recreation retains in-process Undo
- **AND** current callback safety, relevant CLAP/native/shared-owner checks and the unfiltered pinned VST3 validator have retained results with any unsupported capabilities and concrete limitations explicit
