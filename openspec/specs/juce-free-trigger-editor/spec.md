## Purpose

Specify the production Trigger editor transport, host-owned native view and persistent instrument behavior in the JUCE-free host.

## Requirements

### Requirement: Production Trigger uses a versioned JUCE-free transport

The JUCE-free editor SHALL load the locally packaged production Trigger bundle and expose a bounded, versioned native transport. The browser preview SHALL remain explicitly silent, the retained JUCE host SHALL implement the same frontend contract for comparison, and unsupported native operations SHALL return explicit capability errors.

#### Scenario: Copied production bundle connects to the JUCE-free backend

- **WHEN** the exported plugin and its resource tree are copied outside the checkout and its editor is opened in a supported host parent
- **THEN** the production Trigger shell SHALL load only packaged local assets, report the JUCE-free native backend and initialize from the accepted instrument session

#### Scenario: Invalid transport input cannot mutate the instrument

- **WHEN** the WebView sends an unknown method, malformed or oversized payload, stale attachment identity or duplicate conflicting request
- **THEN** the bridge SHALL reject it within documented bounds while preserving accepted session state, public controls and signed audio

#### Scenario: Explicitly unavailable optional services remain honest

- **WHEN** a native backend declares metering, source waveforms, window-title publication or editor resizing unavailable
- **THEN** the editor SHALL initialize and refresh from the real accepted session without requesting the absent services, preserve required notes, heartbeat, public controls and supported mapping/undo, and show unavailable measurements and host-controlled size without simulated native acceptance

#### Scenario: Optional service metadata preserves existing consumers

- **WHEN** a version-1 backend omits optional service metadata or supplies invalid present metadata
- **THEN** omission SHALL retain existing full-contract behavior for every backend identity, while invalid metadata SHALL fail visibly before invoking native methods

### Requirement: Host-parented WebView lifecycle is disposable

The CLAP host SHALL own the native parent and event loop. The editor SHALL create and destroy its child WebView on the plugin main thread, support its declared show/hide and size lifecycle, reject unsupported window APIs or invalid ordering, and release view-owned notes, gestures, callbacks and messages without destroying instrument-owned state.

#### Scenario: Actual X11 child follows the complete host lifecycle

- **WHEN** a host creates the GUI, supplies an actual X11 parent, shows, hides, resizes, destroys and recreates the editor
- **THEN** the child SHALL remain inside the supplied parent, obey supported fixed dimensions, reject unsupported sizes without changing accepted state, and reopen from the same live instrument session

#### Scenario: Failed or premature parenting preserves audio

- **WHEN** a host requests an unsupported API, invalid parent, show before parenting or injected WebView creation failure
- **THEN** the GUI call SHALL fail without leaking a view or callback and the plugin SHALL continue to render its prior known signed audio

#### Scenario: Destruction releases view-owned activity

- **WHEN** the editor is destroyed while a computer note or parameter gesture is held
- **THEN** note-off and gesture-end cleanup SHALL occur exactly once, no later WebView callback SHALL run, and reopening SHALL not inherit a stuck note or gesture

### Requirement: React parameter gestures use host automation

Public scalar controls SHALL remain host-owned automation values. A real Trigger gesture SHALL emit ordered begin, value and end events through the CLAP parameter contract, and host-originated values SHALL return to the live editor without creating structural undo entries or touching the render thread's browser state.

#### Scenario: Real control gesture and host feedback round trip

- **WHEN** the user changes a rendered Trigger control and the host later supplies a different automated value at a nonzero sample offset
- **THEN** the host SHALL receive one balanced gesture with the exact public parameter identity and the editor and signed output SHALL reflect the later host value while native undo availability remains unchanged

### Requirement: Structural work survives complete editor loss

Supported key-range mapping and native undo SHALL use the persistent instrument session. Admitted work and retained outcomes SHALL outlive the WebView and editor attachment, while another plugin instance remains independent.

#### Scenario: Lost editor reconnects and undoes one accepted mapping

- **WHEN** Trigger admits mapping MIDI 36 to 60, its reply is lost and the complete editor is destroyed before a new editor reconnects and invokes native undo
- **THEN** the accepted edit SHALL appear exactly once, note 60 SHALL render the known signed sample before undo, undo SHALL restore note 36 and silence at 60, and a second instance SHALL retain its original mapping, history and signed output
