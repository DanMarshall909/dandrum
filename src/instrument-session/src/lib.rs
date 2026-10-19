//! Host-independent accepted instrument state and authoring history.
mod native_ffi;
use dandrum_engine::kernel::document::load_kernel_patch_str;
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::{
    HostBuses, PreparationContext, PreparedKernelInstrument,
    prepare_kernel_patch_for_planar_host_with_context, prepare_kernel_patch_with_context,
};
use std::{
    collections::VecDeque,
    fmt,
    sync::{Arc, Weak},
};

pub const CONNECTION_CONTRACT_VERSION: u32 = 1;
pub const ATTACHMENT_LIMIT: usize = 16;
pub const HISTORY_LIMIT: usize = 100;
pub const OUTCOME_LIMIT: usize = 128;
pub const ACTIVE_REQUEST_LIMIT: usize = 1;

/// An opaque lifetime identity, never a serialized pointer or platform handle.
#[derive(Clone, Debug)]
pub struct SessionIdentity(Arc<()>);
impl PartialEq for SessionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SessionIdentity {}

/// An editor connection; retaining a copy cannot reverse explicit retirement.
#[derive(Clone, Debug)]
pub struct Attachment {
    owner: SessionIdentity,
    marker: Arc<()>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub instance: SessionIdentity,
    pub revision: u64,
    pub yaml: String,
    pub undo_len: usize,
    pub redo_len: usize,
    pub undo: Option<JournalEntry>,
    pub redo: Option<JournalEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalEntry {
    pub version: u32,
    pub label: String,
    pub change: ZoneKeyRangeChange,
}

pub struct Publication<'a> {
    pub revision: u64,
    pub yaml: &'a str,
    pub undo_len: usize,
    pub redo_len: usize,
    pub undo: Option<&'a JournalEntry>,
    pub redo: Option<&'a JournalEntry>,
}

/// A host lease keeps audio/snapshot admission closed until publication.
pub trait ActivationLease {
    /// Infallible release: no allocation, notification, error or unwinding.
    fn publish(self, accepted: Publication<'_>);
}

pub trait ActivationHost {
    type Error;
    type Lease<'a>: ActivationLease
    where
        Self: 'a;

    /// Stage fallible host resources before installing. A failed activation or
    /// unpublished lease must restore the old runtime and metadata.
    fn activate<'a>(
        &'a mut self,
        prepared: &PreparedKernelInstrument,
        next: &SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommitError<E> {
    Session(SessionError),
    Activation(E),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MappingRequest {
    pub instance: SessionIdentity,
    pub request_id: RequestId,
    pub expected_revision: u64,
    pub intent: SetZoneKeyRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestFailure {
    Session(SessionError),
    Activation(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestOutcome {
    pub request_id: RequestId,
    pub result: Result<SessionSnapshot, RequestFailure>,
}

pub enum MappingAdmission {
    Admitted(MappingJob),
    Pending,
    Completed(Arc<RequestOutcome>),
}

/// A completion routed to the wrong session. The owned result remains recoverable.
pub struct CompletionRouteError {
    reason: SessionError,
    completion: Box<PreparedMappingJob>,
}

impl CompletionRouteError {
    pub fn reason(&self) -> &SessionError {
        &self.reason
    }

    pub fn into_completion(self) -> PreparedMappingJob {
        *self.completion
    }
}

impl fmt::Debug for CompletionRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompletionRouteError")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionError {
    InvalidDocument(String),
    PreparationFailed(String),
    InvalidAttachment,
    AttachmentLimit,
    InvalidMapping,
    ForeignCandidate,
    StaleCandidate,
    NothingToUndo,
    NothingToRedo,
    RevisionExhausted,
    InvalidRequestInstance,
    StaleRequest,
    ConflictingRequest,
    RetiredRequest,
    Busy,
    InvalidJob,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A musical intent addressed by native identities; no client inverse is accepted.
pub struct SetZoneKeyRange {
    pub map_id: String,
    pub zone_id: String,
    pub key_range: [u8; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Native before/after data for the future instrument-owned journal.
pub struct ZoneKeyRangeChange {
    pub map_id: String,
    pub zone_id: String,
    pub before: [u8; 2],
    pub after: [u8; 2],
}

/// An uncommitted candidate. Its creation never installs a runtime or edits history.
pub struct PreparedMapping {
    yaml: String,
    prepared: PreparedKernelInstrument,
    change: ZoneKeyRangeChange,
    owner: SessionIdentity,
    base_revision: u64,
}

/// A fully prepared external document that has not changed accepted state.
pub struct PreparedDocumentReplacement {
    yaml: String,
    prepared: PreparedKernelInstrument,
    owner: SessionIdentity,
    base_revision: u64,
}

/// A host render-configuration change that preserves authoring state/history.
pub struct PreparedRenderConfiguration {
    prepared: PreparedKernelInstrument,
    context: PreparationContext,
    settings: RenderSettings,
    host_buses: HostBuses,
    owner: SessionIdentity,
    base_revision: u64,
}

/// Owned preparation work. It can move to a worker without retaining an editor.
pub struct MappingJob {
    request: MappingRequest,
    action: HistoryAction,
    yaml: String,
    context: PreparationContext,
    settings: RenderSettings,
    host_buses: Option<Box<HostBuses>>,
}

pub struct PreparedMappingJob {
    request: MappingRequest,
    action: HistoryAction,
    result: Result<PreparedMapping, SessionError>,
}

impl MappingJob {
    pub fn prepare(self) -> PreparedMappingJob {
        let result = InstrumentSession::prepare_mapping_document(
            &self.yaml,
            &self.context,
            &self.settings,
            self.host_buses.as_deref(),
            self.request.instance.clone(),
            self.request.expected_revision,
            self.request.intent.clone(),
        );
        PreparedMappingJob {
            request: self.request,
            action: self.action,
            result,
        }
    }

    pub fn fail(self, error: SessionError) -> PreparedMappingJob {
        PreparedMappingJob {
            request: self.request,
            action: self.action,
            result: Err(error),
        }
    }
}

impl PreparedMappingJob {
    pub fn fail(mut self, error: SessionError) -> Self {
        self.result = Err(error);
        self
    }
}
impl PreparedMapping {
    pub fn yaml(&self) -> &str {
        &self.yaml
    }
    pub fn prepared(&self) -> &PreparedKernelInstrument {
        &self.prepared
    }
    pub fn change(&self) -> &ZoneKeyRangeChange {
        &self.change
    }
}
/// Off-audio owner. Renderers consume the prepared engine directly.
pub struct InstrumentSession {
    accepted: AcceptedState,
    identity: SessionIdentity,
    attachments: Vec<Weak<()>>,
    context: PreparationContext,
    settings: RenderSettings,
    host_buses: Option<HostBuses>,
    pending: Option<PendingRequest>,
    outcomes: VecDeque<RetainedOutcome>,
    highest_request_id: Option<RequestId>,
}
struct AcceptedState {
    yaml: String,
    prepared: PreparedKernelInstrument,
    revision: u64,
    journal: Vec<JournalEntry>,
    cursor: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HistoryAction {
    Append,
    Undo,
    Redo,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingRequest {
    request: MappingRequest,
    action: HistoryAction,
}
struct RetainedOutcome {
    request: MappingRequest,
    action: HistoryAction,
    outcome: Arc<RequestOutcome>,
}
impl AcceptedState {
    fn publication(&self) -> Publication<'_> {
        Publication {
            revision: self.revision,
            yaml: &self.yaml,
            undo_len: self.cursor,
            redo_len: self.journal.len() - self.cursor,
            undo: self
                .cursor
                .checked_sub(1)
                .and_then(|index| self.journal.get(index)),
            redo: self.journal.get(self.cursor),
        }
    }
}
impl InstrumentSession {
    pub fn admit_mapping(
        &mut self,
        attachment: &Attachment,
        request: MappingRequest,
    ) -> Result<MappingAdmission, SessionError> {
        self.attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        self.admit_mapping_request(request, HistoryAction::Append)
    }

    pub(crate) fn admit_mapping_owned(
        &mut self,
        request: MappingRequest,
    ) -> Result<MappingAdmission, SessionError> {
        self.admit_mapping_request(request, HistoryAction::Append)
    }

    pub(crate) fn admit_undo(
        &mut self,
        attachment: &Attachment,
        request_id: RequestId,
        expected_revision: u64,
    ) -> Result<MappingAdmission, SessionError> {
        self.attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        let entry = self
            .accepted
            .publication()
            .undo
            .ok_or(SessionError::NothingToUndo)?;
        let request = MappingRequest {
            instance: self.identity.clone(),
            request_id,
            expected_revision,
            intent: SetZoneKeyRange {
                map_id: entry.change.map_id.clone(),
                zone_id: entry.change.zone_id.clone(),
                key_range: entry.change.before,
            },
        };
        self.admit_mapping_request(request, HistoryAction::Undo)
    }

    fn admit_mapping_request(
        &mut self,
        request: MappingRequest,
        action: HistoryAction,
    ) -> Result<MappingAdmission, SessionError> {
        if request.instance != self.identity {
            return Err(SessionError::InvalidRequestInstance);
        }
        if let Some(retained) = self
            .outcomes
            .iter()
            .find(|retained| retained.request.request_id == request.request_id)
        {
            return if retained.request == request && retained.action == action {
                Ok(MappingAdmission::Completed(Arc::clone(&retained.outcome)))
            } else {
                Err(SessionError::ConflictingRequest)
            };
        }
        if let Some(pending) = &self.pending {
            return if pending.request.request_id == request.request_id {
                if pending.request == request && pending.action == action {
                    Ok(MappingAdmission::Pending)
                } else {
                    Err(SessionError::ConflictingRequest)
                }
            } else {
                Err(SessionError::Busy)
            };
        }
        if self
            .highest_request_id
            .is_some_and(|highest| request.request_id <= highest)
        {
            return Err(SessionError::RetiredRequest);
        }
        if request.expected_revision != self.accepted.revision {
            return Err(SessionError::StaleRequest);
        }
        let job = MappingJob {
            request: request.clone(),
            action,
            yaml: self.accepted.yaml.clone(),
            context: self.context.clone(),
            settings: self.settings.clone(),
            host_buses: self.host_buses.clone().map(Box::new),
        };
        self.highest_request_id = Some(request.request_id);
        self.pending = Some(PendingRequest { request, action });
        Ok(MappingAdmission::Admitted(job))
    }

    pub fn complete_mapping<H: ActivationHost>(
        &mut self,
        completion: PreparedMappingJob,
        host: &mut H,
    ) -> Result<Arc<RequestOutcome>, CompletionRouteError>
    where
        H::Error: ToString,
    {
        if self.pending.as_ref()
            != Some(&PendingRequest {
                request: completion.request.clone(),
                action: completion.action,
            })
        {
            return Err(CompletionRouteError {
                reason: SessionError::InvalidJob,
                completion: Box::new(completion),
            });
        }
        let pending = self
            .pending
            .take()
            .expect("matching pending request exists");
        let request = pending.request;
        let outcome = match completion.result {
            Err(error) => Arc::new(RequestOutcome {
                request_id: request.request_id,
                result: Err(RequestFailure::Session(error)),
            }),
            Ok(candidate) => match self.stage(candidate, completion.action) {
                Err(error) => Arc::new(RequestOutcome {
                    request_id: request.request_id,
                    result: Err(RequestFailure::Session(error)),
                }),
                Ok(staged) => {
                    let snapshot = self.snapshot_of(&staged);
                    // Both retained and returned references exist before installation.
                    let accepted = Arc::new(RequestOutcome {
                        request_id: request.request_id,
                        result: Ok(snapshot.clone()),
                    });
                    let returned = Arc::clone(&accepted);
                    match self.install_staged(staged, &snapshot, host, None) {
                        Ok(()) => {
                            self.retain_outcome(request, completion.action, accepted);
                            return Ok(returned);
                        }
                        Err(error) => Arc::new(RequestOutcome {
                            request_id: request.request_id,
                            result: Err(RequestFailure::Activation(error.to_string())),
                        }),
                    }
                }
            },
        };
        let returned = Arc::clone(&outcome);
        self.retain_outcome(request, completion.action, outcome);
        Ok(returned)
    }

    pub fn request_outcome(
        &self,
        attachment: &Attachment,
        request_id: RequestId,
    ) -> Result<Option<Arc<RequestOutcome>>, SessionError> {
        self.attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        Ok(self
            .outcomes
            .iter()
            .find(|retained| retained.request.request_id == request_id)
            .map(|retained| Arc::clone(&retained.outcome)))
    }

    pub fn commit_mapping<H: ActivationHost>(
        &mut self,
        candidate: PreparedMapping,
        host: &mut H,
    ) -> Result<SessionSnapshot, CommitError<H::Error>> {
        self.ensure_idle().map_err(CommitError::Session)?;
        self.publish(candidate, HistoryAction::Append, host)
    }

    /// Prepare an externally supplied canonical document without publishing it.
    pub fn prepare_document_replacement(
        &self,
        attachment: &Attachment,
        yaml: &str,
    ) -> Result<PreparedDocumentReplacement, SessionError> {
        self.attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        self.ensure_idle()?;
        let prepared = Self::prepare_document(
            yaml,
            &self.context,
            &self.settings,
            self.host_buses.as_ref(),
        )?;
        Ok(PreparedDocumentReplacement {
            yaml: yaml.into(),
            prepared,
            owner: self.identity.clone(),
            base_revision: self.accepted.revision,
        })
    }

    /// Publish a prepared external document and retire incompatible recovery state.
    pub fn commit_document_replacement<H: ActivationHost>(
        &mut self,
        candidate: PreparedDocumentReplacement,
        host: &mut H,
    ) -> Result<SessionSnapshot, CommitError<H::Error>> {
        self.ensure_idle().map_err(CommitError::Session)?;
        let staged = self
            .stage_document_replacement(candidate)
            .map_err(CommitError::Session)?;
        let snapshot = self.snapshot_of(&staged);
        // Allocate the next bounded recovery window before host installation.
        let outcomes = VecDeque::with_capacity(OUTCOME_LIMIT);
        self.install_staged(staged, &snapshot, host, Some(outcomes))
            .map_err(CommitError::Activation)?;
        Ok(snapshot)
    }

    pub fn prepare_render_configuration(
        &self,
        attachment: &Attachment,
        settings: RenderSettings,
        host_buses: HostBuses,
    ) -> Result<PreparedRenderConfiguration, SessionError> {
        self.attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        self.prepare_render_configuration_owned(settings, host_buses)
    }

    pub(crate) fn prepare_render_configuration_owned(
        &self,
        settings: RenderSettings,
        host_buses: HostBuses,
    ) -> Result<PreparedRenderConfiguration, SessionError> {
        self.ensure_idle()?;
        let context =
            PreparationContext::new(self.context.document_root(), settings.sample_rate_hz)
                .with_macro_roots(self.context.macro_roots().clone());
        let prepared =
            Self::prepare_document(&self.accepted.yaml, &context, &settings, Some(&host_buses))?;
        Ok(PreparedRenderConfiguration {
            prepared,
            context,
            settings,
            host_buses,
            owner: self.identity.clone(),
            base_revision: self.accepted.revision,
        })
    }

    pub fn commit_render_configuration<H: ActivationHost>(
        &mut self,
        candidate: PreparedRenderConfiguration,
        host: &mut H,
    ) -> Result<SessionSnapshot, CommitError<H::Error>> {
        self.ensure_idle().map_err(CommitError::Session)?;
        if candidate.owner != self.identity {
            return Err(CommitError::Session(SessionError::ForeignCandidate));
        }
        if candidate.base_revision != self.accepted.revision {
            return Err(CommitError::Session(SessionError::StaleCandidate));
        }
        let staged = AcceptedState {
            yaml: self.accepted.yaml.clone(),
            prepared: candidate.prepared,
            revision: self.accepted.revision,
            journal: self.accepted.journal.clone(),
            cursor: self.accepted.cursor,
        };
        let snapshot = self.snapshot_of(&staged);
        let lease = host
            .activate(&staged.prepared, &snapshot)
            .map_err(CommitError::Activation)?;
        let previous = std::mem::replace(&mut self.accepted, staged);
        self.context = candidate.context;
        self.settings = candidate.settings;
        self.host_buses = Some(candidate.host_buses);
        lease.publish(self.accepted.publication());
        drop(previous);
        Ok(snapshot)
    }

    pub fn undo<H: ActivationHost>(
        &mut self,
        attachment: &Attachment,
        host: &mut H,
    ) -> Result<SessionSnapshot, CommitError<H::Error>> {
        self.attachment_index(attachment)
            .ok_or(CommitError::Session(SessionError::InvalidAttachment))?;
        self.ensure_idle().map_err(CommitError::Session)?;
        let entry = self
            .accepted
            .publication()
            .undo
            .ok_or(CommitError::Session(SessionError::NothingToUndo))?;
        let intent = SetZoneKeyRange {
            map_id: entry.change.map_id.clone(),
            zone_id: entry.change.zone_id.clone(),
            key_range: entry.change.before,
        };
        let candidate = self
            .prepare_mapping(attachment, intent)
            .map_err(CommitError::Session)?;
        self.publish(candidate, HistoryAction::Undo, host)
    }

    pub fn redo<H: ActivationHost>(
        &mut self,
        attachment: &Attachment,
        host: &mut H,
    ) -> Result<SessionSnapshot, CommitError<H::Error>> {
        self.attachment_index(attachment)
            .ok_or(CommitError::Session(SessionError::InvalidAttachment))?;
        self.ensure_idle().map_err(CommitError::Session)?;
        let entry = self
            .accepted
            .publication()
            .redo
            .ok_or(CommitError::Session(SessionError::NothingToRedo))?;
        let intent = SetZoneKeyRange {
            map_id: entry.change.map_id.clone(),
            zone_id: entry.change.zone_id.clone(),
            key_range: entry.change.after,
        };
        let candidate = self
            .prepare_mapping(attachment, intent)
            .map_err(CommitError::Session)?;
        self.publish(candidate, HistoryAction::Redo, host)
    }

    fn publish<H: ActivationHost>(
        &mut self,
        candidate: PreparedMapping,
        action: HistoryAction,
        host: &mut H,
    ) -> Result<SessionSnapshot, CommitError<H::Error>> {
        let staged = self
            .stage(candidate, action)
            .map_err(CommitError::Session)?;
        // Snapshot/history/string allocations all precede host installation.
        let snapshot = self.snapshot_of(&staged);
        self.install_staged(staged, &snapshot, host, None)
            .map_err(CommitError::Activation)?;
        Ok(snapshot)
    }

    fn install_staged<H: ActivationHost>(
        &mut self,
        staged: AcceptedState,
        snapshot: &SessionSnapshot,
        host: &mut H,
        replacement_outcomes: Option<VecDeque<RetainedOutcome>>,
    ) -> Result<(), H::Error> {
        let lease = host.activate(&staged.prepared, snapshot)?;
        let previous = std::mem::replace(&mut self.accepted, staged);
        let retired_outcomes =
            replacement_outcomes.map(|outcomes| std::mem::replace(&mut self.outcomes, outcomes));
        lease.publish(self.accepted.publication());
        drop(previous); // Retire accepted resources off audio after release.
        drop(retired_outcomes);
        Ok(())
    }

    fn stage(
        &self,
        candidate: PreparedMapping,
        action: HistoryAction,
    ) -> Result<AcceptedState, SessionError> {
        let revision = self.next_candidate_revision(&candidate.owner, candidate.base_revision)?;
        let mut journal = self.accepted.journal.clone();
        let cursor = match action {
            HistoryAction::Append => {
                journal.truncate(self.accepted.cursor);
                journal.push(JournalEntry {
                    version: 1,
                    label: "Change Key Range".into(),
                    change: candidate.change,
                });
                if journal.len() > HISTORY_LIMIT {
                    journal.remove(0);
                }
                journal.len()
            }
            HistoryAction::Undo => self.accepted.cursor - 1,
            HistoryAction::Redo => self.accepted.cursor + 1,
        };
        Ok(AcceptedState {
            yaml: candidate.yaml,
            prepared: candidate.prepared,
            revision,
            journal,
            cursor,
        })
    }

    fn stage_document_replacement(
        &self,
        candidate: PreparedDocumentReplacement,
    ) -> Result<AcceptedState, SessionError> {
        let revision = self.next_candidate_revision(&candidate.owner, candidate.base_revision)?;
        Ok(AcceptedState {
            yaml: candidate.yaml,
            prepared: candidate.prepared,
            revision,
            journal: Vec::with_capacity(HISTORY_LIMIT),
            cursor: 0,
        })
    }

    fn next_candidate_revision(
        &self,
        owner: &SessionIdentity,
        base_revision: u64,
    ) -> Result<u64, SessionError> {
        if owner != &self.identity {
            return Err(SessionError::ForeignCandidate);
        }
        if base_revision != self.accepted.revision {
            return Err(SessionError::StaleCandidate);
        }
        self.accepted
            .revision
            .checked_add(1)
            .ok_or(SessionError::RevisionExhausted)
    }

    fn ensure_idle(&self) -> Result<(), SessionError> {
        if self.pending.is_some() {
            Err(SessionError::Busy)
        } else {
            Ok(())
        }
    }

    fn retain_outcome(
        &mut self,
        request: MappingRequest,
        action: HistoryAction,
        outcome: Arc<RequestOutcome>,
    ) {
        if self.outcomes.len() == OUTCOME_LIMIT {
            self.outcomes.pop_front();
        }
        self.outcomes.push_back(RetainedOutcome {
            request,
            action,
            outcome,
        });
    }

    /// Prepare off audio without publishing or changing accepted state.
    pub fn prepare_mapping(
        &self,
        attachment: &Attachment,
        intent: SetZoneKeyRange,
    ) -> Result<PreparedMapping, SessionError> {
        self.attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        self.ensure_idle()?;
        Self::prepare_mapping_document(
            &self.accepted.yaml,
            &self.context,
            &self.settings,
            self.host_buses.as_ref(),
            self.identity.clone(),
            self.accepted.revision,
            intent,
        )
    }

    fn prepare_mapping_document(
        accepted_yaml: &str,
        context: &PreparationContext,
        settings: &RenderSettings,
        host_buses: Option<&HostBuses>,
        owner: SessionIdentity,
        base_revision: u64,
        intent: SetZoneKeyRange,
    ) -> Result<PreparedMapping, SessionError> {
        if intent.key_range[0] > intent.key_range[1] || intent.key_range[1] > 127 {
            return Err(SessionError::InvalidMapping);
        }
        // Only the native accepted document supplies identities and inverse data.
        let mut document: serde_yaml::Value =
            serde_yaml::from_str(accepted_yaml).expect("accepted YAML remains valid");
        let maps = document["assets"]["sample_maps"]
            .as_sequence_mut()
            .ok_or(SessionError::InvalidMapping)?;
        let map = maps
            .iter_mut()
            .find(|map| map["id"].as_str() == Some(&intent.map_id))
            .ok_or(SessionError::InvalidMapping)?;
        let zones = map["zones"]
            .as_sequence_mut()
            .expect("accepted sample map has zones");
        let mut matching = zones
            .iter_mut()
            .filter(|zone| zone["id"].as_str() == Some(&intent.zone_id));
        let zone = matching.next().ok_or(SessionError::InvalidMapping)?;
        if matching.next().is_some() {
            return Err(SessionError::InvalidMapping);
        }
        let before = serde_yaml::from_value(zone["key_range"].clone())
            .expect("accepted zone has a typed key range");
        zone["key_range"] = serde_yaml::Value::Sequence(
            intent
                .key_range
                .into_iter()
                .map(|note| note.into())
                .collect(),
        );
        // Value serialization preserves authored values; presentation is normalized.
        let yaml = serde_yaml::to_string(&document).expect("YAML values serialize");
        let prepared = Self::prepare_document(&yaml, context, settings, host_buses)?;
        Ok(PreparedMapping {
            yaml,
            prepared,
            owner,
            base_revision,
            change: ZoneKeyRangeChange {
                map_id: intent.map_id,
                zone_id: intent.zone_id,
                before,
                after: intent.key_range,
            },
        })
    }

    pub fn new(
        yaml: &str,
        context: PreparationContext,
        settings: RenderSettings,
    ) -> Result<Self, SessionError> {
        Self::new_with_optional_host_buses(yaml, context, settings, None)
    }

    pub fn new_for_planar_host(
        yaml: &str,
        context: PreparationContext,
        settings: RenderSettings,
        host_buses: HostBuses,
    ) -> Result<Self, SessionError> {
        Self::new_with_optional_host_buses(yaml, context, settings, Some(host_buses))
    }

    fn new_with_optional_host_buses(
        yaml: &str,
        context: PreparationContext,
        settings: RenderSettings,
        host_buses: Option<HostBuses>,
    ) -> Result<Self, SessionError> {
        let prepared = Self::prepare_document(yaml, &context, &settings, host_buses.as_ref())?;
        Ok(Self {
            accepted: AcceptedState {
                yaml: yaml.into(),
                prepared,
                revision: 0,
                journal: Vec::with_capacity(HISTORY_LIMIT),
                cursor: 0,
            },
            identity: SessionIdentity(Arc::new(())),
            attachments: Vec::with_capacity(ATTACHMENT_LIMIT),
            context,
            settings,
            host_buses,
            pending: None,
            outcomes: VecDeque::with_capacity(OUTCOME_LIMIT),
            highest_request_id: None,
        })
    }

    pub fn attach(&mut self) -> Result<(Attachment, SessionSnapshot), SessionError> {
        self.attachments.retain(|marker| marker.strong_count() != 0);
        if self.attachments.len() == ATTACHMENT_LIMIT {
            return Err(SessionError::AttachmentLimit);
        }
        let attachment = Attachment {
            owner: self.identity.clone(),
            marker: Arc::new(()),
        };
        self.attachments.push(Arc::downgrade(&attachment.marker));
        Ok((attachment, self.accepted_snapshot()))
    }

    pub fn detach(&mut self, attachment: &Attachment) -> Result<(), SessionError> {
        let index = self
            .attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        self.attachments.swap_remove(index);
        Ok(())
    }

    pub fn snapshot(&self, attachment: &Attachment) -> Result<SessionSnapshot, SessionError> {
        self.attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        Ok(self.accepted_snapshot())
    }

    pub(crate) fn owner_snapshot(&self) -> SessionSnapshot {
        self.accepted_snapshot()
    }

    pub fn next_request_id(&self) -> Result<RequestId, SessionError> {
        self.highest_request_id.map_or(Ok(RequestId(1)), |highest| {
            highest
                .0
                .checked_add(1)
                .map(RequestId)
                .ok_or(SessionError::RevisionExhausted)
        })
    }

    pub(crate) fn reserve_control_request(
        &mut self,
        attachment: &Attachment,
        request_id: RequestId,
        expected_revision: u64,
    ) -> Result<(), SessionError> {
        self.attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        self.ensure_idle()?;
        if self
            .highest_request_id
            .is_some_and(|highest| request_id <= highest)
        {
            return Err(SessionError::RetiredRequest);
        }
        if expected_revision != self.accepted.revision {
            return Err(SessionError::StaleRequest);
        }
        self.highest_request_id = Some(request_id);
        Ok(())
    }

    pub fn prepared(&self) -> &PreparedKernelInstrument {
        &self.accepted.prepared
    }

    fn prepare_document(
        yaml: &str,
        context: &PreparationContext,
        settings: &RenderSettings,
        host_buses: Option<&HostBuses>,
    ) -> Result<PreparedKernelInstrument, SessionError> {
        let patch = load_kernel_patch_str(yaml)
            .map_err(|error| SessionError::InvalidDocument(error.to_string()))?;
        host_buses
            .map_or_else(
                || prepare_kernel_patch_with_context(&patch, settings, context),
                |buses| {
                    prepare_kernel_patch_for_planar_host_with_context(
                        &patch, settings, buses, context,
                    )
                },
            )
            .map_err(|error| SessionError::PreparationFailed(error.to_string()))
    }

    fn attachment_index(&self, attachment: &Attachment) -> Option<usize> {
        if self.identity != attachment.owner {
            return None;
        }
        let marker = Arc::downgrade(&attachment.marker);
        self.attachments
            .iter()
            .position(|active| active.ptr_eq(&marker))
    }

    fn accepted_snapshot(&self) -> SessionSnapshot {
        self.snapshot_of(&self.accepted)
    }

    fn snapshot_of(&self, accepted: &AcceptedState) -> SessionSnapshot {
        let view = accepted.publication();
        SessionSnapshot {
            instance: self.identity.clone(),
            revision: view.revision,
            yaml: view.yaml.into(),
            undo_len: view.undo_len,
            redo_len: view.redo_len,
            undo: view.undo.cloned(),
            redo: view.redo.cloned(),
        }
    }
}

#[cfg(test)]
#[path = "../tests/internal/revision_limit.rs"]
mod revision_limit;
