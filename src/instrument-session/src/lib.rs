//! Host-independent accepted instrument state and authoring history.
mod native_ffi;
use dandrum_engine::kernel::document::load_kernel_patch_str;
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::{
    HostBuses, PreparationContext, PreparedKernelInstrument,
    prepare_kernel_patch_realtime_for_planar_host_with_context,
    prepare_kernel_patch_realtime_with_context,
};
use dandrum_engine::render_settings::RealtimeRenderSettings;
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
    pub undo: Option<HistoryEntry>,
    pub redo: Option<HistoryEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalEntry {
    pub version: u32,
    pub label: String,
    pub change: ZoneKeyRangeChange,
}

/// Public history metadata. Document inverses and resource ownership stay native.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryEntry {
    Mapping(JournalEntry),
    Document { version: u32, label: String },
}

pub struct Publication<'a> {
    pub revision: u64,
    pub yaml: &'a str,
    pub undo_len: usize,
    pub redo_len: usize,
    pub undo: Option<&'a HistoryEntry>,
    pub redo: Option<&'a HistoryEntry>,
}

/// A host lease reserves safe publication while keeping unpublished runtime and
/// metadata invisible. Adapters mutating shared active state must close reader
/// admission; an immutable queued replacement may keep the old runtime live.
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
    context: PreparationContext,
    prepared: PreparedKernelInstrument,
    owner: SessionIdentity,
    base_revision: u64,
}

/// A host render-configuration change that preserves authoring state/history.
pub struct PreparedRenderConfiguration {
    prepared: PreparedKernelInstrument,
    context: PreparationContext,
    settings: RealtimeRenderSettings,
    host_buses: HostBuses,
    owner: SessionIdentity,
    base_revision: u64,
}

/// Owned preparation work shared by mapping and whole-document edits.
/// A job owns its authority and can move to a worker without an editor.
pub struct MappingJob {
    request: SessionRequest,
    action: HistoryAction,
    target: PreparationTarget,
    context: Box<PreparationContext>,
    settings: RealtimeRenderSettings,
    host_buses: Option<Box<HostBuses>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionRequest {
    instance: SessionIdentity,
    request_id: RequestId,
    expected_revision: u64,
    intent: SessionIntent,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum SessionIntent {
    Mapping(SetZoneKeyRange),
    Document(Arc<str>),
    Undo,
    Redo,
}
impl From<MappingRequest> for SessionRequest {
    fn from(request: MappingRequest) -> Self {
        Self {
            instance: request.instance,
            request_id: request.request_id,
            expected_revision: request.expected_revision,
            intent: SessionIntent::Mapping(request.intent),
        }
    }
}
/// Context clones share selected PCM owners. Historical clock values are never
/// used for preparation: admission installs the session's current precise clock.
#[derive(Clone)]
struct DocumentState {
    yaml: Arc<str>,
    context: PreparationContext,
}
#[derive(Clone)]
struct HistoryRecord {
    entry: HistoryEntry,
    documents: Option<(DocumentState, DocumentState)>,
}
enum PreparationTarget {
    Mapping {
        yaml: Arc<str>,
        intent: SetZoneKeyRange,
    },
    Document(Arc<str>),
}
struct PreparedEdit {
    yaml: Arc<str>,
    prepared: PreparedKernelInstrument,
    context: PreparationContext,
    entry: HistoryEntry,
    owner: SessionIdentity,
    base_revision: u64,
}
impl PreparedEdit {
    fn mapping(candidate: PreparedMapping, context: PreparationContext) -> Self {
        Self {
            yaml: candidate.yaml.into(),
            prepared: candidate.prepared,
            context,
            entry: HistoryEntry::Mapping(JournalEntry {
                version: 1,
                label: "Change Key Range".into(),
                change: candidate.change,
            }),
            owner: candidate.owner,
            base_revision: candidate.base_revision,
        }
    }
}
pub struct PreparedMappingJob {
    request: SessionRequest,
    action: HistoryAction,
    result: Result<PreparedEdit, SessionError>,
}
impl MappingJob {
    pub fn prepare(self) -> PreparedMappingJob {
        let result = match self.target {
            PreparationTarget::Mapping { yaml, intent } => {
                InstrumentSession::prepare_mapping_document(
                    &yaml,
                    &self.context,
                    &self.settings,
                    self.host_buses.as_deref(),
                    self.request.instance.clone(),
                    self.request.expected_revision,
                    intent,
                )
                .map(|candidate| PreparedEdit::mapping(candidate, *self.context))
            }
            PreparationTarget::Document(yaml) => InstrumentSession::prepare_document(
                &yaml,
                &self.context,
                &self.settings,
                self.host_buses.as_deref(),
            )
            .map(|prepared| PreparedEdit {
                yaml,
                prepared,
                context: *self.context,
                entry: HistoryEntry::Document {
                    version: 1,
                    label: "Edit Instrument".into(),
                },
                owner: self.request.instance.clone(),
                base_revision: self.request.expected_revision,
            }),
        };
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
    settings: RealtimeRenderSettings,
    host_buses: Option<HostBuses>,
    pending: Option<PendingRequest>,
    outcomes: VecDeque<RetainedOutcome>,
    highest_request_id: Option<RequestId>,
}
struct AcceptedState {
    yaml: Arc<str>,
    context: PreparationContext,
    prepared: PreparedKernelInstrument,
    revision: u64,
    journal: Vec<HistoryRecord>,
    cursor: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HistoryAction {
    Append,
    Undo,
    Redo,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HistoryDirection {
    Undo,
    Redo,
}
impl HistoryDirection {
    fn action(self) -> HistoryAction {
        match self {
            Self::Undo => HistoryAction::Undo,
            Self::Redo => HistoryAction::Redo,
        }
    }
    fn intent(self) -> SessionIntent {
        match self {
            Self::Undo => SessionIntent::Undo,
            Self::Redo => SessionIntent::Redo,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingRequest {
    request: SessionRequest,
    action: HistoryAction,
}
struct RetainedOutcome {
    request: SessionRequest,
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
                .and_then(|index| self.journal.get(index))
                .map(|record| &record.entry),
            redo: self.journal.get(self.cursor).map(|record| &record.entry),
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
        self.admit_mapping_request(request)
    }

    pub(crate) fn admit_mapping_owned(
        &mut self,
        request: MappingRequest,
    ) -> Result<MappingAdmission, SessionError> {
        self.admit_mapping_request(request)
    }

    /// Native-authorized document admission. This generic session imposes no
    /// Trigger/browser resource profile.
    pub fn admit_document(
        &mut self,
        attachment: &Attachment,
        request_id: RequestId,
        expected_revision: u64,
        yaml: Arc<str>,
        samples: impl IntoIterator<Item = Arc<dandrum_engine::sample_snapshot::SampleSnapshot>>,
    ) -> Result<MappingAdmission, SessionError> {
        self.admit_document_with(attachment, request_id, expected_revision, yaml, || {
            Ok(samples.into_iter().collect())
        })
    }
    /// The trusted native factory validates/acquires resources only for new
    /// work. Pending and terminal retries never need released source authority.
    pub fn admit_document_with(
        &mut self,
        attachment: &Attachment,
        request_id: RequestId,
        expected_revision: u64,
        yaml: Arc<str>,
        resources: impl FnOnce() -> Result<
            Vec<Arc<dandrum_engine::sample_snapshot::SampleSnapshot>>,
            SessionError,
        >,
    ) -> Result<MappingAdmission, SessionError> {
        self.attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        self.admit_request_with_target(
            SessionRequest {
                instance: self.identity.clone(),
                request_id,
                expected_revision,
                intent: SessionIntent::Document(yaml.clone()),
            },
            |session| {
                Ok((
                    HistoryAction::Append,
                    PreparationTarget::Document(yaml),
                    session
                        .accepted
                        .context
                        .clone()
                        .with_selected_samples(resources()?),
                ))
            },
        )
    }
    pub fn admit_undo(
        &mut self,
        attachment: &Attachment,
        request_id: RequestId,
        expected_revision: u64,
    ) -> Result<MappingAdmission, SessionError> {
        self.admit_history(
            attachment,
            request_id,
            expected_revision,
            HistoryDirection::Undo,
        )
    }
    pub fn admit_redo(
        &mut self,
        attachment: &Attachment,
        request_id: RequestId,
        expected_revision: u64,
    ) -> Result<MappingAdmission, SessionError> {
        self.admit_history(
            attachment,
            request_id,
            expected_revision,
            HistoryDirection::Redo,
        )
    }
    fn admit_history(
        &mut self,
        attachment: &Attachment,
        request_id: RequestId,
        expected_revision: u64,
        direction: HistoryDirection,
    ) -> Result<MappingAdmission, SessionError> {
        self.attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        self.admit_request_with_target(
            SessionRequest {
                instance: self.identity.clone(),
                request_id,
                expected_revision,
                intent: direction.intent(),
            },
            |session| session.history_target(direction),
        )
    }
    fn admit_mapping_request(
        &mut self,
        request: MappingRequest,
    ) -> Result<MappingAdmission, SessionError> {
        self.admit_request_with_target(request.clone().into(), |session| {
            Ok((
                HistoryAction::Append,
                PreparationTarget::Mapping {
                    yaml: session.accepted.yaml.clone(),
                    intent: request.intent,
                },
                session.accepted.context.clone(),
            ))
        })
    }
    /// All commands share the same request window before resolving owned work.
    fn request_admission(
        &self,
        request: &SessionRequest,
    ) -> Result<Option<MappingAdmission>, SessionError> {
        if request.instance != self.identity {
            return Err(SessionError::InvalidRequestInstance);
        }
        if let Some(retained) = self
            .outcomes
            .iter()
            .find(|retained| retained.request.request_id == request.request_id)
        {
            return if retained.request == *request {
                Ok(Some(MappingAdmission::Completed(Arc::clone(
                    &retained.outcome,
                ))))
            } else {
                Err(SessionError::ConflictingRequest)
            };
        }
        if let Some(pending) = &self.pending {
            return if pending.request.request_id == request.request_id {
                if pending.request == *request {
                    Ok(Some(MappingAdmission::Pending))
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
        Ok(None)
    }
    fn admit_request_with_target(
        &mut self,
        request: SessionRequest,
        prepare: impl FnOnce(
            &Self,
        ) -> Result<
            (HistoryAction, PreparationTarget, PreparationContext),
            SessionError,
        >,
    ) -> Result<MappingAdmission, SessionError> {
        if let Some(admission) = self.request_admission(&request)? {
            return Ok(admission);
        }
        let (action, target, context) = prepare(self)?;
        let job = MappingJob {
            request: request.clone(),
            action,
            target,
            context: Box::new(context.with_sample_rate(self.settings.sample_rate_hz)),
            settings: self.settings.clone(),
            host_buses: self.host_buses.clone().map(Box::new),
        };
        self.highest_request_id = Some(request.request_id);
        self.pending = Some(PendingRequest { request, action });
        Ok(MappingAdmission::Admitted(job))
    }
    fn history_target(
        &self,
        direction: HistoryDirection,
    ) -> Result<(HistoryAction, PreparationTarget, PreparationContext), SessionError> {
        let action = direction.action();
        let record = match direction {
            HistoryDirection::Undo => self
                .accepted
                .cursor
                .checked_sub(1)
                .and_then(|index| self.accepted.journal.get(index))
                .ok_or(SessionError::NothingToUndo)?,
            HistoryDirection::Redo => self
                .accepted
                .journal
                .get(self.accepted.cursor)
                .ok_or(SessionError::NothingToRedo)?,
        };
        let (target, context) = match &record.entry {
            HistoryEntry::Mapping(entry) => (
                PreparationTarget::Mapping {
                    yaml: self.accepted.yaml.clone(),
                    intent: SetZoneKeyRange {
                        map_id: entry.change.map_id.clone(),
                        zone_id: entry.change.zone_id.clone(),
                        key_range: if action == HistoryAction::Undo {
                            entry.change.before
                        } else {
                            entry.change.after
                        },
                    },
                },
                self.accepted.context.clone(),
            ),
            HistoryEntry::Document { .. } => {
                let (before, after) = record
                    .documents
                    .as_ref()
                    .expect("document history owns both sides");
                let target = if action == HistoryAction::Undo {
                    before
                } else {
                    after
                };
                (
                    PreparationTarget::Document(target.yaml.clone()),
                    target.context.clone(),
                )
            }
        };
        Ok((action, target, context))
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
                            self.retain_outcome(request, accepted);
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
        self.retain_outcome(request, outcome);
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
        self.prepare_document_replacement_owned(yaml)
    }

    /// Prepare native host state without borrowing a disposable editor identity.
    pub fn prepare_document_replacement_owned(
        &self,
        yaml: &str,
    ) -> Result<PreparedDocumentReplacement, SessionError> {
        self.ensure_idle()?;
        let context = self.replacement_context(yaml)?;
        self.prepare_replacement_in_context(yaml, context)
    }

    /// Prepare externally owned sample state without consulting prior selected
    /// authority or a disposable editor. Caller supplies native-validated owners.
    pub fn prepare_document_replacement_with_samples_owned(
        &self,
        yaml: &str,
        samples: impl IntoIterator<Item = Arc<dandrum_engine::sample_snapshot::SampleSnapshot>>,
    ) -> Result<PreparedDocumentReplacement, SessionError> {
        self.ensure_idle()?;
        let context = self.accepted.context.clone().with_selected_samples(samples);
        self.prepare_replacement_in_context(yaml, context)
    }

    fn prepare_replacement_in_context(
        &self,
        yaml: &str,
        context: PreparationContext,
    ) -> Result<PreparedDocumentReplacement, SessionError> {
        let prepared =
            Self::prepare_document(yaml, &context, &self.settings, self.host_buses.as_ref())?;
        Ok(PreparedDocumentReplacement {
            yaml: yaml.into(),
            context,
            prepared,
            owner: self.identity.clone(),
            base_revision: self.accepted.revision,
        })
    }

    fn replacement_context(&self, yaml: &str) -> Result<PreparationContext, SessionError> {
        use dandrum_engine::kernel::{ResourceKind, ResourceOrigin, StaticArg, StaticValue};
        let patch = load_kernel_patch_str(yaml)
            .map_err(|error| SessionError::InvalidDocument(error.to_string()))?;
        let declared = std::iter::once(patch.root())
            .chain(patch.registry().definitions())
            .flat_map(|definition| {
                definition
                    .static_params()
                    .iter()
                    .filter_map(|parameter| match parameter.default() {
                        Some(StaticValue::Resource(resource)) => Some(resource),
                        _ => None,
                    })
                    .chain(
                        definition
                            .nodes()
                            .iter()
                            .flat_map(|node| node.static_args().values())
                            .filter_map(|value| match value {
                                StaticArg::Literal(StaticValue::Resource(resource)) => {
                                    Some(resource)
                                }
                                _ => None,
                            }),
                    )
            });
        let required = declared
            .chain(
                patch
                    .sample_assets()
                    .sample_sources
                    .iter()
                    .map(|source| &source.resource),
            )
            .filter(|resource| {
                resource.kind() == ResourceKind::Sample
                    && resource.origin() == &ResourceOrigin::Document
            })
            .map(|resource| resource.path())
            .collect::<std::collections::BTreeSet<_>>();
        let samples = self
            .accepted
            .context
            .selected_samples()
            .filter(|sample| required.contains(std::path::Path::new(sample.resource_id())))
            .cloned()
            .collect::<Vec<_>>();
        Ok(self.accepted.context.clone().with_selected_samples(samples))
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

    /// Prepare host render settings without borrowing a disposable editor.
    /// The candidate retains this Session's private owner and accepted revision.
    pub fn prepare_render_configuration_owned(
        &self,
        settings: RenderSettings,
        host_buses: HostBuses,
    ) -> Result<PreparedRenderConfiguration, SessionError> {
        self.ensure_idle()?;
        let settings = RealtimeRenderSettings::try_from(&settings)
            .map_err(|error| SessionError::PreparationFailed(error.to_string()))?;
        self.prepare_realtime_render_configuration_owned(settings, host_buses)
    }

    /// Prepare a precise host clock without borrowing a disposable editor.
    pub fn prepare_realtime_render_configuration_owned(
        &self,
        settings: RealtimeRenderSettings,
        host_buses: HostBuses,
    ) -> Result<PreparedRenderConfiguration, SessionError> {
        self.ensure_idle()?;
        let context = self
            .accepted
            .context
            .clone()
            .with_sample_rate(settings.sample_rate_hz);
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
            context: candidate.context,
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
        self.publish_history(attachment, HistoryDirection::Undo, host)
    }
    pub fn redo<H: ActivationHost>(
        &mut self,
        attachment: &Attachment,
        host: &mut H,
    ) -> Result<SessionSnapshot, CommitError<H::Error>> {
        self.publish_history(attachment, HistoryDirection::Redo, host)
    }
    fn publish_history<H: ActivationHost>(
        &mut self,
        attachment: &Attachment,
        direction: HistoryDirection,
        host: &mut H,
    ) -> Result<SessionSnapshot, CommitError<H::Error>> {
        self.attachment_index(attachment)
            .ok_or(CommitError::Session(SessionError::InvalidAttachment))?;
        self.ensure_idle().map_err(CommitError::Session)?;
        let (action, target, context) = self
            .history_target(direction)
            .map_err(CommitError::Session)?;
        let job = MappingJob {
            request: SessionRequest {
                instance: self.identity.clone(),
                request_id: RequestId(0),
                expected_revision: self.accepted.revision,
                intent: direction.intent(),
            },
            action,
            target,
            context: Box::new(context.with_sample_rate(self.settings.sample_rate_hz)),
            settings: self.settings.clone(),
            host_buses: self.host_buses.clone().map(Box::new),
        };
        let candidate = job.prepare().result.map_err(CommitError::Session)?;
        self.publish_edit(candidate, action, host)
    }

    fn publish<H: ActivationHost>(
        &mut self,
        candidate: PreparedMapping,
        action: HistoryAction,
        host: &mut H,
    ) -> Result<SessionSnapshot, CommitError<H::Error>> {
        self.publish_edit(
            PreparedEdit::mapping(candidate, self.accepted.context.clone()),
            action,
            host,
        )
    }
    fn publish_edit<H: ActivationHost>(
        &mut self,
        candidate: PreparedEdit,
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
        candidate: PreparedEdit,
        action: HistoryAction,
    ) -> Result<AcceptedState, SessionError> {
        let revision = self.next_candidate_revision(&candidate.owner, candidate.base_revision)?;
        let mut journal = self.accepted.journal.clone();
        let cursor = match action {
            HistoryAction::Append => {
                journal.truncate(self.accepted.cursor);
                let documents =
                    matches!(candidate.entry, HistoryEntry::Document { .. }).then(|| {
                        (
                            DocumentState {
                                yaml: self.accepted.yaml.clone(),
                                context: self.accepted.context.clone(),
                            },
                            DocumentState {
                                yaml: candidate.yaml.clone(),
                                context: candidate.context.clone(),
                            },
                        )
                    });
                journal.push(HistoryRecord {
                    entry: candidate.entry,
                    documents,
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
            context: candidate.context,
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
            yaml: candidate.yaml.into(),
            context: candidate.context,
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

    fn retain_outcome(&mut self, request: SessionRequest, outcome: Arc<RequestOutcome>) {
        if self.outcomes.len() == OUTCOME_LIMIT {
            self.outcomes.pop_front();
        }
        self.outcomes
            .push_back(RetainedOutcome { request, outcome });
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
            &self.accepted.context,
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
        settings: &RealtimeRenderSettings,
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

    /// The precise configuration also owns subsequent mapping/history preparation.
    pub fn new_realtime_for_planar_host(
        yaml: &str,
        context: PreparationContext,
        settings: RealtimeRenderSettings,
        host_buses: HostBuses,
    ) -> Result<Self, SessionError> {
        Self::new_realtime_with_optional_host_buses(yaml, context, settings, Some(host_buses))
    }

    fn new_with_optional_host_buses(
        yaml: &str,
        context: PreparationContext,
        settings: RenderSettings,
        host_buses: Option<HostBuses>,
    ) -> Result<Self, SessionError> {
        let settings = RealtimeRenderSettings::try_from(&settings)
            .map_err(|error| SessionError::PreparationFailed(error.to_string()))?;
        Self::new_realtime_with_optional_host_buses(yaml, context, settings, host_buses)
    }

    fn new_realtime_with_optional_host_buses(
        yaml: &str,
        context: PreparationContext,
        settings: RealtimeRenderSettings,
        host_buses: Option<HostBuses>,
    ) -> Result<Self, SessionError> {
        let prepared = Self::prepare_document(yaml, &context, &settings, host_buses.as_ref())?;
        Ok(Self {
            accepted: AcceptedState {
                yaml: yaml.into(),
                context,
                prepared,
                revision: 0,
                journal: Vec::with_capacity(HISTORY_LIMIT),
                cursor: 0,
            },
            identity: SessionIdentity(Arc::new(())),
            attachments: Vec::with_capacity(ATTACHMENT_LIMIT),
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

    /// Native accepted state; does not create or consume an editor attachment.
    pub fn owner_snapshot(&self) -> SessionSnapshot {
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

    pub fn prepared(&self) -> &PreparedKernelInstrument {
        &self.accepted.prepared
    }

    fn prepare_document(
        yaml: &str,
        context: &PreparationContext,
        settings: &RealtimeRenderSettings,
        host_buses: Option<&HostBuses>,
    ) -> Result<PreparedKernelInstrument, SessionError> {
        let patch = load_kernel_patch_str(yaml)
            .map_err(|error| SessionError::InvalidDocument(error.to_string()))?;
        host_buses
            .map_or_else(
                || prepare_kernel_patch_realtime_with_context(&patch, settings, context),
                |buses| {
                    prepare_kernel_patch_realtime_for_planar_host_with_context(
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
