# Instrument session Specification

## Purpose

Define the persistent, host-independent ownership and transactional lifecycle
for accepted instrument editing state, prepared runtime publication and
disposable editor attachments.

## Requirements

### Requirement: Instrument lifetime owns accepted editing state

Each instrument instance SHALL own its accepted authoring document, revision and bounded native history independently of disposable editor attachments. Session interfaces SHALL remain independent of GUI, operating-system window and plugin-format framework types.

#### Scenario: Reattachment observes the accepted session

- **WHEN** a connection detaches and a fresh connection attaches to the same live instrument
- **THEN** the accepted document, revision and undo/redo availability SHALL remain available without rebuilding or restarting the working engine

#### Scenario: Instrument sessions remain independent

- **WHEN** one of two instrument instances accepts an edit, detaches its editor or undoes an edit
- **THEN** the other instance SHALL retain its own document, revision, history and known signed output

#### Scenario: Session core builds independently

- **WHEN** the session core and its real-engine behavior tests build without any GUI or plugin-format adapter
- **THEN** they SHALL use the same accepted document/history implementation that concrete native adapters consume

### Requirement: Prepared mapping and history publish coherently

A supported sample-zone key-range edit SHALL address native map/zone identities, derive its inverse from accepted state, validate and prepare actual assets off audio, and publish the active engine, document, revision and journal as one accepted transition. The client SHALL NOT supply authoritative inverse values.

#### Scenario: Accepted mapping edit and native undo render real samples

- **WHEN** the first drum mapping is changed from MIDI 36 to 60 and then undone through native history
- **THEN** the accepted mapping and history SHALL follow each successful transition, note 60 SHALL play the known signed sample only after the edit, and undo SHALL restore note 36 with that sample and silence at 60

#### Scenario: Failed preparation or activation preserves all accepted state

- **WHEN** a mapping transaction fails validation, sample preparation or concrete runtime activation
- **THEN** the previous runtime, authoring document, revision and undo/redo state SHALL remain usable with the prior known signed output

### Requirement: Bounded admission survives editor loss

Structural requests SHALL carry instrument, attachment, monotonically increasing request and expected-revision identities. Only a current matching attachment SHALL admit new work. Each session SHALL admit at most one structural preparation at a time, complete admitted work independently of editor polling, retain bounded outcomes and reject conflicting retries, retired request identities or stale revisions.

#### Scenario: Lost acknowledgement does not duplicate an accepted edit

- **WHEN** an admitted mapping edit loses its reply and its editor disappears, then a fresh attachment queries or retries that same request and intent
- **THEN** the job outcome SHALL remain recoverable and the edit SHALL appear exactly once in accepted state and history

#### Scenario: Invalid connections and concurrent edits are rejected

- **WHEN** a request uses another instance, a retired attachment, a stale revision, a conflicting request identity or arrives during an admitted structural job
- **THEN** it SHALL receive an explicit rejection without mutating accepted state or another instance and without silently queueing or replacing work

### Requirement: Native history has bounded deterministic lifecycle

Native history SHALL retain at most 100 entries and request recovery at most 128 terminal outcomes. Successful new edits SHALL retire redo, failed work SHALL preserve it, and external accepted document replacement SHALL reconcile history and revisions. Host automation SHALL remain authoritative parameter feedback without becoming a stream of undo records.

#### Scenario: Bounded history preserves undo and redo meaning

- **WHEN** edits exceed retention, an undo is followed by failed and successful new edits, or the accepted document is externally replaced
- **THEN** retained history/outcomes SHALL obey their bounds, failed work SHALL preserve redo, successful branching SHALL remove redo, and replacement SHALL retire incompatible history and stale requests

### Requirement: Actual editor destruction preserves instrument work

The canonical native React editor SHALL use the persistent session for migrated mapping and undo operations. Complete editor/WebView destruction SHALL dispose attachment-owned view work, notes and gestures while accepted instrument jobs and history survive. Reopening SHALL reconnect to accepted native state.

#### Scenario: Destroyed native WebView reconnects and undoes

- **WHEN** a real native editor changes mapping 36 to 60, its editor and WebView are completely destroyed during or after accepted work, and a new editor reconnects and invokes undo
- **THEN** native state and undo availability SHALL recover, undo SHALL restore mapping 36 and known signed audio, and another live instrument SHALL remain unaffected
