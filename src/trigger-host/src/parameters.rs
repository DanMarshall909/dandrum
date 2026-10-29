//! Public parameter policy and its uniquely owned Trigger runtime endpoint.
//!
//! Only immutable descriptions are cached here. Actual scalar values and applied
//! UI command acknowledgements belong to the runtime's atomics. Commands use the
//! controller's protocol/session attachment namespace, never an arbitrary runtime.

use crate::protocol::{
    Admission, AdmittedRequest, EditorCommand, EditorConnection, EditorProtocol,
    MAX_SAFE_GENERATION, ProtocolError,
};
use crate::runtime::{
    PreparedRuntime, QuiescentReport, RuntimeAudio, RuntimeControl, RuntimeError, runtime_handoff,
};
use crate::{TriggerHost, TriggerHostConfiguration};
use dandrum_engine::kernel::document::load_kernel_patch_str;
use std::{collections::VecDeque, num::NonZeroU64, path::Path};

mod selection;
pub use selection::NativeSelectionPermit;

pub const MAX_AUTOMATION_EVENTS: usize = 1_024;

#[derive(Debug, PartialEq, Eq)]
pub enum ParameterError {
    Host(String),
    Session(dandrum_instrument_session::SessionError),
    UnknownJob,
    InvalidJob,
    SessionWorkerUnavailable,
    SessionWorkerAlreadyStarted,
    SessionWorkerBusy,
    Runtime(RuntimeError),
    Protocol(ProtocolError),
    InvalidCapacity,
    InvalidSurface,
    IdentityExhausted,
    StaleGeneration,
    UnknownParameter,
    QueueFull,
    UnsupportedCommand,
    GestureAlreadyActive,
    GestureConflict,
    GestureNotActive,
    EditorNoteConflict,
    ReplayedEditorNote,
    BackwardEditorNoteTime,
}

/// JavaScript-facing counters must be exact integers, including their next value.
pub fn document_generation(revision: u64) -> Result<u64, ParameterError> {
    revision
        .checked_add(1)
        .filter(|value| *value <= MAX_SAFE_GENERATION)
        .ok_or(ParameterError::IdentityExhausted)
}

#[derive(Clone, Debug, PartialEq)]
pub struct PublicParameter {
    pub id: String,
    pub root_index: usize,
    pub physical_min: f64,
    pub physical_max: f64,
    pub physical_default: f64,
    /// Product grouping remains stable when an accepted graph is silent.
    pub control_group: Option<u8>,
}

impl PublicParameter {
    fn project(&self, physical: f64) -> ParameterValue {
        ParameterValue {
            id: self.id.clone(),
            name: self.id.clone(),
            value: (physical - self.physical_min) / (self.physical_max - self.physical_min),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterValue {
    pub id: String,
    pub name: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterState {
    pub generation: u64,
    pub sequence: u64,
    /// Whether the latest settled UI command was cancelled by reset rather
    /// than applied. A later host value retains this cause; a new UI write
    /// clears it when applied. It is not a history of all earlier cancellations.
    pub reset_cancelled: bool,
    pub parameters: Vec<ParameterValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterFeedback {
    pub generation: u64,
    pub sequence: u64,
    pub reset_cancelled: bool,
    pub frame_offset: u32,
    pub parameters: Vec<ParameterValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParameterReply {
    Accepted { generation: u64, sequence: u64 },
    State(ParameterState),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AutomationEvent {
    Begin { parameter_index: usize },
    Value { parameter_index: usize, value: f64 },
    End { parameter_index: usize },
}

/// A native job can leave the controller for preparation without an editor.
/// Its private identity must return with the completion to the same active slot.
pub struct SessionPreparation {
    authority: std::sync::Arc<()>,
    request_id: dandrum_instrument_session::RequestId,
    job: dandrum_instrument_session::MappingJob,
}
/// An owned completion can be submitted only once. Rejection returns the same
/// owned value so its actual controller can still accept it.
///
/// ```compile_fail
/// use dandrum_trigger_host::parameters::{ParameterController, SessionCompletion};
/// fn submit_twice(host: &mut ParameterController, completion: SessionCompletion) {
///     let _ = host.submit_session_completion(completion);
///     let _ = host.submit_session_completion(completion);
/// }
/// ```
pub struct SessionCompletion {
    authority: std::sync::Arc<()>,
    request_id: dandrum_instrument_session::RequestId,
    job: Box<dandrum_instrument_session::PreparedMappingJob>,
}
impl SessionPreparation {
    pub fn prepare(self) -> SessionCompletion {
        SessionCompletion {
            authority: self.authority,
            request_id: self.request_id,
            job: Box::new(self.job.prepare()),
        }
    }
}
pub struct RejectedSessionCompletion {
    completion: SessionCompletion,
}
impl RejectedSessionCompletion {
    pub fn into_completion(self) -> SessionCompletion {
        self.completion
    }
}
impl std::fmt::Debug for RejectedSessionCompletion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RejectedSessionCompletion")
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionJobProgress {
    Idle,
    Pending,
    Completed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Whether the accepted render settings replaced the current DSP.
pub enum ReconfigurationReport {
    Unchanged,
    Changed,
}
struct ActiveSessionJob {
    authority: std::sync::Arc<()>,
    request_id: dandrum_instrument_session::RequestId,
    work: SessionWork,
}
enum SessionWork {
    Queued(dandrum_instrument_session::MappingJob),
    Preparing,
    Ready(Box<dandrum_instrument_session::PreparedMappingJob>),
}

/// Request a future instrument-level main/control service opportunity.
/// Implementations must be nonblocking and nonpanicking, must outlive editors,
/// and must never call controller service inline. Native adapters also request
/// service after a real audio `ProcessReport::swapped` restores capacity.
pub trait SessionServiceWake: Send + Sync {
    fn request_service(&self);
}

enum WorkerRequest {
    Mapping(SessionPreparation),
    Selection(selection::Preparation),
}
enum WorkerCompletion {
    Mapping(SessionCompletion),
    Selection(selection::Completion),
}
impl WorkerRequest {
    fn prepare(self) -> WorkerCompletion {
        match self {
            Self::Mapping(work) => WorkerCompletion::Mapping(work.prepare()),
            Self::Selection(work) => WorkerCompletion::Selection(work.prepare()),
        }
    }
}
struct SessionWorker {
    requests: Option<std::sync::mpsc::SyncSender<WorkerRequest>>,
    completions: std::sync::mpsc::Receiver<WorkerCompletion>,
    thread: Option<std::thread::JoinHandle<Option<WorkerCompletion>>>,
}
impl SessionWorker {
    fn available(&self) -> bool {
        self.thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
    }
    fn shutdown(&mut self) -> Result<(), ParameterError> {
        // Disconnect before joining: the idle worker waits only on this sender.
        self.requests.take();
        let result = self.thread.take().map(std::thread::JoinHandle::join);
        // Join owns undeliverable completions; the channel owns delivered ones.
        // Both are destroyed on the constructing control thread, after join.
        drop(self.completions.try_recv());
        match result {
            None | Some(Ok(None)) => Ok(()),
            Some(Ok(Some(_))) | Some(Err(_)) => Err(ParameterError::SessionWorkerUnavailable),
        }
    }
}
impl Drop for SessionWorker {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

enum EditorNoteCommand {
    On {
        note: u8,
        velocity: f64,
        generation: u64,
    },
    Off {
        note: u8,
        generation: u64,
    },
    Heartbeat,
}

#[derive(Clone, Copy)]
struct EditorNoteAttachment {
    owner: NonZeroU64,
    watermark: u64,
    activity: Option<std::time::Instant>,
}

/// The constructing control thread owns session publication and teardown.
/// A format facade must construct this owner on its actual host main thread.
///
/// ```compile_fail
/// use dandrum_trigger_host::parameters::ParameterController;
/// fn move_control_to_worker(controller: ParameterController) {
///     std::thread::spawn(move || drop(controller));
/// }
/// ```
pub struct ParameterController {
    control_thread: std::marker::PhantomData<std::rc::Rc<()>>,
    worker: Option<SessionWorker>,
    session_job: Option<ActiveSessionJob>,
    selections: selection::Selections,
    host: TriggerHost,
    runtime: RuntimeControl,
    protocol: EditorProtocol,
    parameters: Vec<PublicParameter>,
    generation: u64,
    admitted_sequence: u64,
    automation: VecDeque<AutomationEvent>,
    automation_capacity: usize,
    gestures: Vec<Option<u64>>,
    reserved_ends: usize,
    editor_presses: [Option<(NonZeroU64, u64)>; 128],
    editor_note_service_time: Option<std::time::Instant>,
    editor_note_attachments:
        [Option<EditorNoteAttachment>; dandrum_instrument_session::ATTACHMENT_LIMIT],
}

impl ParameterController {
    /// Install one persistent preparation worker before admitting structural work.
    /// Without installation the existing explicit take/submit API remains manual.
    pub fn start_session_worker(
        &mut self,
        wake: std::sync::Arc<dyn SessionServiceWake>,
    ) -> Result<(), ParameterError> {
        if self.worker.is_some() {
            return Err(ParameterError::SessionWorkerAlreadyStarted);
        }
        if self.session_job.is_some() {
            return Err(ParameterError::SessionWorkerBusy);
        }
        let (requests, jobs) = std::sync::mpsc::sync_channel::<WorkerRequest>(1);
        let (results, completions) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("trigger-preparation".into())
            .spawn(move || {
                struct WakeOnPanic {
                    results: Option<std::sync::mpsc::SyncSender<WorkerCompletion>>,
                    wake: std::sync::Arc<dyn SessionServiceWake>,
                }
                impl Drop for WakeOnPanic {
                    fn drop(&mut self) {
                        // The wake may be serviced before JoinHandle reports
                        // finished. Disconnect first so that service sees failure.
                        self.results.take();
                        if std::thread::panicking() {
                            self.wake.request_service();
                        }
                    }
                }
                let exit = WakeOnPanic {
                    results: Some(results),
                    wake,
                };
                while let Ok(job) = jobs.recv() {
                    let result = exit
                        .results
                        .as_ref()
                        .expect("worker owns sender")
                        .try_send(job.prepare());
                    exit.wake.request_service();
                    if let Err(
                        std::sync::mpsc::TrySendError::Full(completion)
                        | std::sync::mpsc::TrySendError::Disconnected(completion),
                    ) = result
                    {
                        // A full/disconnected result slot is fatal, never an
                        // excuse to overwrite or discard the only completion.
                        return Some(completion);
                    }
                }
                None
            })
            .map_err(|_| ParameterError::SessionWorkerUnavailable)?;
        self.worker = Some(SessionWorker {
            requests: Some(requests),
            completions,
            thread: Some(thread),
        });
        Ok(())
    }

    /// Stop the installed worker permanently and join it off audio. This is
    /// idempotent and independent of editor detach; a manual controller has no
    /// worker to stop. Current preparation may finish, but shutdown publishes
    /// nothing and does not promise cancellation of blocking resource I/O.
    pub fn shutdown_session_worker(&mut self) -> Result<(), ParameterError> {
        let result = match &mut self.worker {
            Some(worker) => worker.shutdown(),
            None => Ok(()),
        };
        self.selections.stop_worker();
        result
    }

    fn check_session_worker(&self) -> Result<(), ParameterError> {
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| !worker.available())
        {
            Err(ParameterError::SessionWorkerUnavailable)
        } else {
            Ok(())
        }
    }

    /// Save accepted YAML and one coherent vector of applied physical values.
    /// This off-audio read never closes callback admission; only the reader may
    /// retry while audio writes. Queued editor values are not yet applied state.
    /// Packaged payload acquisition uses the catalog under this main-owner borrow.
    pub fn save_state(
        &mut self,
        output: &mut impl std::io::Write,
    ) -> Result<(), crate::state::StateError> {
        use crate::state::{PRODUCT_ID, RESOURCE_ID, SavedParameter, SavedState};
        let snapshot = self.host.session().owner_snapshot();
        let values = self.runtime.snapshot_state_values();
        let mut state = SavedState {
            version: 2,
            product: PRODUCT_ID.into(),
            resource: RESOURCE_ID.into(),
            document: snapshot.yaml,
            parameters: self
                .parameters
                .iter()
                .map(|parameter| SavedParameter {
                    id: parameter.id.clone(),
                    value: values[parameter.root_index],
                })
                .collect(),
        };
        // Preserve existing packaged v2 bytes; selected-only and resource-free
        // accepted documents use the bounded content-owning representation.
        let mut samples = Vec::new();
        if state.values_for(&self.parameters, &self.runtime).is_err() {
            samples = Self::patch_samples(
                &state.document,
                &self.parameters,
                &self.runtime,
                &self.host.sample_catalog,
            )
            .map_err(|_| crate::state::StateError::Invalid)?;
            // This boundary adds mixed state while preserving the existing
            // packaged-only v2 path and interim refusal for its changed defaults.
            if samples.is_empty()
                && !self
                    .host
                    .session()
                    .prepared()
                    .sample_assets()
                    .sources()
                    .is_empty()
            {
                return Err(crate::state::StateError::Invalid);
            }
            state.version = 3;
            state.values_for(&self.parameters, &self.runtime)?;
        }
        let mut resources = samples
            .into_iter()
            .map(|sample| (sample.resource_id().to_owned(), sample))
            .collect::<std::collections::BTreeMap<_, _>>();
        if state.version == 3 {
            let factory_revision = self
                .host
                .session()
                .prepared()
                .sample_assets()
                .sources()
                .iter()
                .find(|source| {
                    source.declaration().resource.path()
                        == std::path::Path::new("assets/advanced-drums.wav")
                })
                .map(|source| *source.sample().content_revision());
            if let Some(revision) = factory_revision {
                resources.insert(
                    "assets/advanced-drums.wav".into(),
                    self.packaged_state_sample(&revision)?,
                );
            }
        }
        crate::state::write(&state, &resources.into_iter().collect::<Vec<_>>(), output)
    }
    fn packaged_state_sample(
        &mut self,
        expected: &[u8; 32],
    ) -> Result<
        std::sync::Arc<dandrum_engine::sample_snapshot::SampleSnapshot>,
        crate::state::StateError,
    > {
        use crate::state::StateError;
        let hex = expected
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let id = format!("selected/{hex}.wav");
        if let Ok(sample) = self.host.sample_catalog.acquire(&id) {
            return Ok(sample);
        }
        let path = crate::PackagedResources::regular_file(
            self.host.resource_root(),
            crate::ADVANCED_DRUM_KIT_SAMPLE,
        )
        .map_err(|_| StateError::Invalid)?;
        let mut file = std::fs::File::open(path).map_err(|_| StateError::Io)?;
        let length = file.metadata().map_err(|_| StateError::Io)?.len();
        let length = usize::try_from(length).map_err(|_| StateError::TooLarge)?;
        let mut samples = crate::state::restore_samples(
            &mut file,
            &mut self.host.sample_catalog,
            &[(id, length)],
        )?;
        Ok(samples
            .pop()
            .expect("one factory payload produces one snapshot"))
    }
    /// Validate and prepare complete bounded native state before publication.
    /// Held gestures/outbound automation return Busy so no promised End is lost.
    /// Success retires old queued activity and resets structural history; failure
    /// preserves accepted state. No disposable editor attachment is required.
    pub fn load_state(
        &mut self,
        input: &mut impl std::io::Read,
    ) -> Result<(), crate::state::StateError> {
        let saved = crate::state::read_header(input)?;
        self.restore_state(saved, input)
    }

    /// Check current native admission before foreign payload I/O. Revalidate
    /// after I/O: reentrant host callbacks may change the instrument meanwhile.
    pub fn preflight_state(
        &self,
        saved: &crate::state::StateHeader,
    ) -> Result<(), crate::state::StateError> {
        self.state_restore_values(saved)?;
        if !saved.resources.is_empty() && self.host.sample_catalog.acquisition_pending() {
            return Err(crate::state::StateError::Busy);
        }
        Ok(())
    }

    /// Foreign reads have ended; the same native checks/catalog/publication own
    /// this replacement, with no host callback under the main-owner borrow.
    pub fn load_buffered_state(
        &mut self,
        buffered: crate::state::BufferedState,
    ) -> Result<(), crate::state::StateError> {
        self.restore_state(buffered.header, &mut buffered.payload.as_slice())
    }

    fn state_restore_values(
        &self,
        saved: &crate::state::StateHeader,
    ) -> Result<(Vec<f64>, u64), crate::state::StateError> {
        use crate::state::StateError;
        let state = &saved.state;
        if self.reserved_ends != 0
            || !self.automation.is_empty()
            || (state.version == 3 && self.session_job.is_some())
        {
            return Err(StateError::Busy);
        }
        if state.version == 3 {
            super::patch_budget::check(&state.document).map_err(|_| StateError::Invalid)?;
            let patch = load_kernel_patch_str(&state.document).map_err(|_| StateError::Invalid)?;
            let required = patch
                .sample_assets()
                .sample_sources
                .iter()
                .map(|source| source.resource.path())
                .collect::<std::collections::BTreeSet<_>>();
            let declared = saved
                .resources
                .iter()
                .map(|resource| std::path::Path::new(&resource.path))
                .collect::<std::collections::BTreeSet<_>>();
            if required != declared {
                return Err(StateError::Invalid);
            }
        }
        let values = state.values_for(&self.parameters, &self.runtime)?;
        let generation =
            crate::state::restored_generation(self.host.session().owner_snapshot().revision)?;
        if state.version == 2 {
            crate::PackagedResources::regular_file(
                self.host.resource_root(),
                crate::ADVANCED_DRUM_KIT_SAMPLE,
            )
            .map_err(|_| StateError::Invalid)?;
        }
        Ok((values, generation))
    }

    fn restore_state(
        &mut self,
        saved: crate::state::StateHeader,
        input: &mut impl std::io::Read,
    ) -> Result<(), crate::state::StateError> {
        use crate::state::StateError;
        use dandrum_instrument_session::CommitError;
        let (values, generation) = self.state_restore_values(&saved)?;
        let state = &saved.state;
        let candidate = if state.version == 3 {
            let samples = saved.read_samples(input, &mut self.host.sample_catalog)?;
            Self::patch_samples(
                &state.document,
                &self.parameters,
                &self.runtime,
                &self.host.sample_catalog,
            )
            .map_err(|_| StateError::Invalid)?;
            self.host
                .session()
                .prepare_document_replacement_with_resources_owned(
                    &state.document,
                    saved
                        .resources
                        .iter()
                        .map(|r| std::path::PathBuf::from(&r.path))
                        .zip(samples),
                )
        } else {
            self.host
                .session()
                .prepare_document_replacement_owned(&state.document)
        }
        .map_err(StateError::Session)?;
        // A previous swap's retired runtime is destroyed only here, off audio.
        // An unconsumed candidate still has no retirement and remains Busy.
        drop(self.runtime.reclaim());
        self.host
            .session_mut()
            .commit_document_replacement(candidate, &mut self.runtime.restoring(values))
            .map_err(|error| match error {
                CommitError::Session(error) => StateError::Session(error),
                CommitError::Activation(error) => StateError::Runtime(error),
            })?;
        // Runtime restore has atomically retired old intent. Keep each still
        // valid attachment's dispatch watermark, but no old press or lease.
        self.editor_presses.fill(None);
        for record in self.editor_note_attachments.iter_mut().flatten() {
            record.activity = None;
        }
        self.generation = generation;
        self.admitted_sequence = 0;
        Ok(())
    }
    pub fn detach_editor(&mut self, attachment_id: &str) -> Result<(), ParameterError> {
        self.protocol
            .detach(self.host.session_mut(), attachment_id)
            .map_err(ParameterError::Protocol)?;
        let owner = attachment_id
            .parse::<u64>()
            .expect("protocol validated the retired attachment identity");
        self.selections.detach(owner);
        self.end_gestures(Some(owner));
        self.clear_editor_notes(NonZeroU64::new(owner).expect("attachment is nonzero"));
        for record in &mut self.editor_note_attachments {
            if record.is_some_and(|record| record.owner.get() == owner) {
                *record = None;
            }
        }
        Ok(())
    }
    fn clear_editor_notes(&mut self, owner: NonZeroU64) {
        self.runtime.detach_editor_notes(owner);
        for press in &mut self.editor_presses {
            if press.is_some_and(|(held, _)| held == owner) {
                *press = None;
            }
        }
    }

    /// Dispatch authenticated native editor intent off audio. Releases use the
    /// generation recorded at onset, rather than the current graph generation.
    /// Dispatch admitted notes in transport order with a native monotonic time.
    /// Semantic rejection consumes that request ID; retry requires a fresh ID.
    /// An unknown release is an idempotent no-op. The wire has no physical press
    /// nonce, so repeated presses within one generation cannot be distinguished.
    pub fn apply_editor_note(
        &mut self,
        request: &AdmittedRequest,
        now: std::time::Instant,
    ) -> Result<(), ParameterError> {
        let context = self
            .protocol
            .admitted_context(self.host.session(), request)
            .map_err(ParameterError::Protocol)?;
        let owner = NonZeroU64::new(context.attachment_id).expect("live attachment is nonzero");
        let command = match request.command {
            EditorCommand::NoteOn {
                note,
                velocity,
                generation,
            } => EditorNoteCommand::On {
                note,
                velocity,
                generation,
            },
            EditorCommand::NoteOff { note, generation } => {
                EditorNoteCommand::Off { note, generation }
            }
            EditorCommand::NoteHeartbeat => EditorNoteCommand::Heartbeat,
            _ => return Err(ParameterError::UnsupportedCommand),
        };
        let record = self
            .editor_note_attachments
            .iter_mut()
            .flatten()
            .find(|record| record.owner == owner)
            .expect("live attachment owns note dispatch record");
        if request.request_id <= record.watermark {
            return Err(ParameterError::ReplayedEditorNote);
        }
        // Terminal even when semantic dispatch fails. Authentication failures
        // above cannot consume a genuine request's dispatch authority.
        record.watermark = request.request_id;
        match command {
            EditorNoteCommand::On { generation, .. } if generation != self.generation => {
                return Err(ParameterError::StaleGeneration);
            }
            EditorNoteCommand::On { note, .. }
                if self.editor_presses[usize::from(note)]
                    .is_some_and(|(held, _)| held != owner) =>
            {
                return Err(ParameterError::Runtime(RuntimeError::EditorOwnerConflict));
            }
            EditorNoteCommand::Off { note, generation }
                if self.editor_presses[usize::from(note)]
                    .is_some_and(|press| press != (owner, generation)) =>
            {
                return Err(ParameterError::EditorNoteConflict);
            }
            _ => {}
        }
        self.service_editor_notes(now)?;
        let mut renew = false;
        match command {
            EditorNoteCommand::On {
                note,
                velocity,
                generation,
            } => {
                let velocity = (velocity.clamp(1.0 / 127.0, 1.0) * 127.0).round() as u8;
                self.runtime
                    .editor_note_on(owner, note, velocity)
                    .map_err(ParameterError::Runtime)?;
                self.editor_presses[usize::from(note)] = Some((owner, generation));
                renew = true;
            }
            EditorNoteCommand::Off { note, .. } => {
                if self.editor_presses[usize::from(note)].is_some() {
                    // The sealed range and matching press owner were checked
                    // above; exclusive control ownership prevents a competing
                    // publisher between that validation and this release.
                    self.runtime
                        .editor_note_off(owner, note)
                        .expect("validated pitch release belongs to its recorded owner");
                    self.editor_presses[usize::from(note)] = None;
                    renew = true;
                }
            }
            EditorNoteCommand::Heartbeat => {}
        }
        let record = self
            .editor_note_attachments
            .iter_mut()
            .flatten()
            .find(|record| record.owner == owner)
            .expect("live dispatch record");
        if renew
            || matches!(request.command, EditorCommand::NoteHeartbeat) && record.activity.is_some()
        {
            record.activity = Some(now);
        }
        Ok(())
    }

    /// Expire native note leases independently of browser or audio progress.
    /// One pass visits at most the native attachment limit and never queues work.
    /// Returns the number of expired leases (including unheard released taps).
    /// Backward time rejects without changing policy. The format owner must
    /// service this independently of browser visibility and audio callbacks.
    pub fn service_editor_notes(
        &mut self,
        now: std::time::Instant,
    ) -> Result<usize, ParameterError> {
        if self
            .editor_note_service_time
            .is_some_and(|previous| now.checked_duration_since(previous).is_none())
        {
            return Err(ParameterError::BackwardEditorNoteTime);
        }
        self.editor_note_service_time = Some(now);
        let mut expired = 0;
        for index in 0..self.editor_note_attachments.len() {
            let Some(record) = self.editor_note_attachments[index] else {
                continue;
            };
            if record.activity.is_some_and(|activity| {
                now.checked_duration_since(activity)
                    .is_some_and(|age| age >= std::time::Duration::from_secs(2))
            }) {
                self.clear_editor_notes(record.owner);
                self.editor_note_attachments[index]
                    .as_mut()
                    .expect("existing record")
                    .activity = None;
                expired += 1;
            }
        }
        Ok(expired)
    }

    fn end_gestures(&mut self, owner: Option<u64>) {
        for (index, gesture) in self.gestures.iter_mut().enumerate() {
            if gesture.is_some() && owner.is_none_or(|owner| *gesture == Some(owner)) {
                self.automation.push_back(AutomationEvent::End {
                    parameter_index: index,
                });
                *gesture = None;
                self.reserved_ends -= 1;
            }
        }
    }
    pub fn take_feedback(&mut self) -> Option<ParameterFeedback> {
        let (feedback, sequence, reset_cancelled) = self.runtime.take_sequenced_feedback()?;
        let parameter = self
            .parameters
            .iter()
            .find(|parameter| parameter.root_index == feedback.index)
            .expect("public aliases cover every prepared root control exactly once");
        Some(ParameterFeedback {
            generation: self.generation,
            sequence,
            reset_cancelled,
            frame_offset: feedback.frame_offset,
            parameters: vec![parameter.project(feedback.value)],
        })
    }
    /// Service accepted runtime ownership after the audio endpoint is returned.
    pub fn service_quiescent(
        &mut self,
        audio: &mut RuntimeAudio,
    ) -> Result<QuiescentReport, ParameterError> {
        self.runtime
            .service_quiescent(audio)
            .map_err(ParameterError::Runtime)
    }

    /// Change host render settings with returned audio ownership. Outstanding
    /// work remains owned and returns Busy; callers service it separately.
    /// Equal settings preserve live DSP without touching packaged resources.
    pub fn reconfigure_quiescent(
        &mut self,
        audio: &mut RuntimeAudio,
        configuration: TriggerHostConfiguration,
    ) -> Result<ReconfigurationReport, ParameterError> {
        self.runtime
            .validate_endpoint(audio)
            .map_err(ParameterError::Runtime)?;
        if !(1..=crate::runtime::MAX_BLOCK_FRAMES).contains(&configuration.max_block_frames) {
            return Err(ParameterError::Runtime(RuntimeError::InvalidConfiguration));
        }
        if self.session_job.is_some() || !self.runtime.activation_available() {
            return Err(ParameterError::Runtime(RuntimeError::Busy));
        }
        if configuration == self.runtime.configuration() {
            return Ok(ReconfigurationReport::Unchanged);
        }
        use dandrum_engine::graph::PortDirection;
        use dandrum_engine::preparation::HostBuses;
        use dandrum_engine::render_settings::RealtimeRenderSettings;
        use dandrum_instrument_session::CommitError;
        // Accepted root metadata names the same planar buses as its document.
        // The unchanged accepted YAML retains the already validated aliases;
        // activation also checks the complete retained root control layout.
        let buses = self
            .host
            .session()
            .prepared()
            .root_port_metadata()
            .iter()
            .fold(HostBuses::new(), |buses, port| match port.direction() {
                PortDirection::Input => buses.with_input(port.name(), 1),
                PortDirection::Output => buses.with_output(port.name(), 2),
            });
        let settings = RealtimeRenderSettings {
            sample_rate_hz: configuration.sample_rate_hz,
            block_size_frames: configuration.max_block_frames,
            duration_frames: u64::from(configuration.max_block_frames),
        };
        let candidate = self
            .host
            .session()
            .prepare_realtime_render_configuration_owned(settings, buses)
            .map_err(ParameterError::Session)?;
        self.host
            .session_mut()
            .commit_render_configuration(
                candidate,
                &mut self.runtime.reconfiguring(audio, configuration),
            )
            .map_err(|error| match error {
                CommitError::Session(error) => ParameterError::Session(error),
                CommitError::Activation(error) => ParameterError::Runtime(error),
            })?;
        Ok(ReconfigurationReport::Changed)
    }
    pub fn open(
        root: impl AsRef<Path>,
        configuration: TriggerHostConfiguration,
        instance_id: NonZeroU64,
        event_capacity: usize,
        automation_capacity: usize,
    ) -> Result<(Self, RuntimeAudio), ParameterError> {
        if !(3..=MAX_AUTOMATION_EVENTS).contains(&automation_capacity) {
            return Err(ParameterError::InvalidCapacity);
        }
        let host = TriggerHost::open(root, configuration)
            .map_err(|error| ParameterError::Host(error.to_string()))?;
        let prepared = PreparedRuntime::new(host.session().prepared(), configuration)
            .map_err(ParameterError::Runtime)?;
        let (runtime, audio) =
            runtime_handoff(prepared, event_capacity).map_err(ParameterError::Runtime)?;
        // The freshly opened host has prepared this immutable cached document.
        let patch = load_kernel_patch_str(host.resources.document())
            .expect("fresh Trigger document was prepared");
        let mut parameters = Vec::new();
        for alias in patch.preset_surface().parameters() {
            let range = alias.control_default();
            let (Some(min), Some(max)) = (range.min(), range.max()) else {
                return Err(ParameterError::InvalidSurface);
            };
            if min >= max || !(min as f32).is_finite() || !(max as f32).is_finite() {
                return Err(ParameterError::InvalidSurface);
            }
            let root_index = runtime
                .parameter_index(alias.port_name())
                .expect("prepared alias names a mono root control");
            if parameters
                .iter()
                .any(|parameter: &PublicParameter| parameter.root_index == root_index)
            {
                return Err(ParameterError::InvalidSurface);
            }
            let control_group = patch
                .root()
                .ports()
                .iter()
                .find(|port| port.name() == alias.port_name())
                .and_then(|port| match port.internal_targets() {
                    [target] => Some(target),
                    _ => None,
                })
                .and_then(|target| {
                    patch
                        .root()
                        .nodes()
                        .iter()
                        .find(|node| node.id() == target.node())
                        .filter(|node| node.definition_ref() == "sample_map_player")
                        .map(|_| target)
                })
                .and_then(|target| target.port().strip_prefix("group_"))
                .and_then(|suffix| suffix.split_once('_'))
                .and_then(|(group, _)| group.parse::<u8>().ok())
                .filter(|group| *group > 0);
            parameters.push(PublicParameter {
                id: alias.name().into(),
                root_index,
                physical_min: min,
                physical_max: max,
                physical_default: range.default(),
                control_group,
            });
        }
        if parameters.len()
            != host
                .session()
                .prepared()
                .root_port_metadata()
                .iter()
                .filter(|port| port.direction() == dandrum_engine::graph::PortDirection::Input)
                .count()
        {
            return Err(ParameterError::InvalidSurface);
        }
        let gestures = vec![None; parameters.len()];
        Ok((
            Self {
                control_thread: std::marker::PhantomData,
                worker: None,
                session_job: None,
                selections: selection::Selections::default(),
                host,
                runtime,
                protocol: EditorProtocol::new(instance_id),
                parameters,
                gestures,
                reserved_ends: 0,
                editor_presses: [None; 128],
                editor_note_service_time: None,
                editor_note_attachments: [None; dandrum_instrument_session::ATTACHMENT_LIMIT],
                generation: document_generation(0)?,
                admitted_sequence: 0,
                automation: VecDeque::with_capacity(automation_capacity),
                automation_capacity,
            },
            audio,
        ))
    }

    /// Read an applied physical value by public alias index, without normalization.
    pub fn physical_value(&self, public_index: usize) -> Option<f64> {
        self.runtime
            .parameter_value(self.parameters.get(public_index)?.root_index)
    }

    pub fn parameters(&self) -> &[PublicParameter] {
        &self.parameters
    }
    pub fn session(&self) -> &crate::InstrumentSession {
        self.host.session()
    }
    pub fn attach_editor(&mut self) -> Result<EditorConnection, ParameterError> {
        let connection = self
            .protocol
            .attach(self.host.session_mut())
            .map_err(ParameterError::Protocol)?;
        let owner = NonZeroU64::new(
            connection
                .attachment_id
                .parse()
                .expect("native attachment identity"),
        )
        .expect("nonzero attachment");
        let slot = self
            .editor_note_attachments
            .iter_mut()
            .find(|slot| slot.is_none())
            .expect("same native attachment limit");
        *slot = Some(EditorNoteAttachment {
            owner,
            watermark: 0,
            activity: None,
        });
        Ok(connection)
    }
    /// Framing gate for a native view. Small frames still require full admission;
    /// only a live negotiated attachment may use the larger patch profile.
    pub fn editor_message_fits_limits(&self, attachment: &str, message: &str) -> bool {
        self.protocol
            .message_fits_limits(self.host.session(), Some(attachment), message)
    }

    pub fn admit(&mut self, message: &str) -> Result<Admission, ParameterError> {
        self.protocol
            .admit(self.host.session(), message)
            .map_err(ParameterError::Protocol)
    }

    pub fn negotiate_editor_protocol(
        &mut self,
        request: &AdmittedRequest,
    ) -> Result<String, ParameterError> {
        self.protocol
            .negotiate(self.host.session(), request)
            .map_err(ParameterError::Protocol)
    }

    /// Encode a bounded reply off audio using this controller's live editor authority.
    pub fn encode_editor_reply(
        &self,
        attachment_id: &str,
        request_id: &str,
        result: Result<serde_json::Value, ProtocolError>,
    ) -> Result<String, ParameterError> {
        self.protocol
            .encode_reply(self.host.session(), attachment_id, request_id, result)
            .map_err(ParameterError::Protocol)
    }

    pub fn encode_editor_rejection(
        &self,
        attachment_id: &str,
        request_id: &str,
        message: &str,
        error: ProtocolError,
    ) -> Result<String, ParameterError> {
        self.protocol
            .encode_rejection(
                self.host.session(),
                attachment_id,
                request_id,
                message,
                error,
            )
            .map_err(ParameterError::Protocol)
    }

    /// Encode a bounded event off audio with the live editor's retained sequence.
    pub fn encode_editor_event(
        &mut self,
        attachment_id: &str,
        event: &str,
        payload: serde_json::Value,
    ) -> Result<String, ParameterError> {
        self.protocol
            .encode_event(self.host.session(), attachment_id, event, payload)
            .map_err(ParameterError::Protocol)
    }

    /// Project the accepted native session off audio in its admitted wire
    /// version. This read reuses its editor and never changes history.
    pub fn sampler_session_reply(
        &self,
        request: &AdmittedRequest,
    ) -> Result<String, ParameterError> {
        let context = self
            .protocol
            .admitted_context(self.host.session(), request)
            .map_err(ParameterError::Protocol)?;
        if request.command != EditorCommand::GetSamplerSession {
            return Err(ParameterError::UnsupportedCommand);
        }
        let projection = SamplerProjection {
            source_path: self
                .host
                .resource_root()
                .join(crate::ADVANCED_DRUM_KIT_DOCUMENT),
            parameters: &self.parameters,
            state: self.parameter_state(),
            instance_id: context.instance_id.to_string(),
            selections: self.selections.summaries(),
            active_request_id: self.session_job.as_ref().map(|job| job.request_id.0),
            next_request_id: self
                .host
                .session()
                .next_request_id()
                .map_err(ProtocolError::Session)
                .map_err(ParameterError::Protocol)?
                .0
                .to_string(),
        };
        let result = projection.value(
            &context.snapshot,
            self.host.session().prepared(),
            &request.attachment_id.to_string(),
        )?;
        self.protocol
            .encode_reply(
                self.host.session(),
                &request.attachment_id.to_string(),
                &request.request_id.to_string(),
                Ok(result),
            )
            .map_err(ParameterError::Protocol)
    }

    fn patch_samples(
        document: &str,
        parameters: &[PublicParameter],
        runtime: &RuntimeControl,
        catalog: &crate::samples::SampleCatalog,
    ) -> Result<Vec<std::sync::Arc<dandrum_engine::sample_snapshot::SampleSnapshot>>, ParameterError>
    {
        use dandrum_engine::kernel::{StaticArg, StaticValue};
        // Sealed protocol admission and the counted parser own document bounds.
        // Resource kind/origin are fixed by the YAML sample-source parser;
        // package/catalog identity below supplies the remaining authority.
        super::patch_budget::check(document).map_err(|_| ParameterError::InvalidSurface)?;
        let patch = load_kernel_patch_str(document).map_err(|_| ParameterError::InvalidSurface)?;
        let root = patch.root();
        let assets = patch.sample_assets();
        let short = |name: &str| name.len() <= 256;
        if patch.registry().definitions().any(|definition| {
            definition.metadata().category()
                != dandrum_engine::kernel::DefinitionCategory::Primitive
        }) || !root.static_params().is_empty()
            || root.nodes().len() > 64
            || root.connections().len() > 128
            || root.ports().len() > 64
            || assets.sample_sources.len() > 256
            || assets.sample_maps.len() > 64
            || assets
                .sample_sources
                .iter()
                .map(|s| s.regions.len())
                .sum::<usize>()
                > 1024
            || assets
                .sample_maps
                .iter()
                .map(|m| m.zones.len())
                .sum::<usize>()
                > 1024
            || patch.metadata().name().is_some_and(|name| !short(name))
            || patch.instrument().is_none_or(|id| !short(&id.id))
        {
            return Err(ParameterError::InvalidSurface);
        }
        for kind in ["midi_input", "sample_map_player"] {
            if root
                .nodes()
                .iter()
                .filter(|n| n.definition_ref() == kind)
                .count()
                > 1
            {
                return Err(ParameterError::InvalidSurface);
            }
        }
        for node in root.nodes() {
            if !short(node.id().as_str())
                || !matches!(
                    node.definition_ref(),
                    "midi_input" | "sample_map_player" | "audio_mixer" | "control_mixer"
                )
            {
                return Err(ParameterError::InvalidSurface);
            }
            for (key, value) in node.static_args() {
                if !short(key)
                    || matches!(
                        value,
                        StaticArg::ParamRef(_) | StaticArg::Literal(StaticValue::Resource(_))
                    )
                {
                    return Err(ParameterError::InvalidSurface);
                }
                if key == "max_voices"
                    && !matches!(value, StaticArg::Literal(StaticValue::Int(1..=128)))
                {
                    return Err(ParameterError::InvalidSurface);
                }
                if key == "channels" {
                    let allowed = match value {
                        StaticArg::Literal(StaticValue::Int(channels)) => {
                            if node.definition_ref() == "control_mixer" {
                                *channels == 1
                            } else {
                                (1..=2).contains(channels)
                            }
                        }
                        _ => false,
                    };
                    if !allowed {
                        return Err(ParameterError::InvalidSurface);
                    }
                }
            }
        }
        let aliases = patch.preset_surface().parameters();
        if aliases.len() != parameters.len() {
            return Err(ParameterError::InvalidSurface);
        }
        for (alias, parameter) in aliases.iter().zip(parameters) {
            if alias.name() != parameter.id
                || runtime.parameter_index(alias.port_name()) != Some(parameter.root_index)
                || alias.control_default().min() != Some(parameter.physical_min)
                || alias.control_default().max() != Some(parameter.physical_max)
            {
                return Err(ParameterError::InvalidSurface);
            }
        }
        let mut samples = std::collections::BTreeMap::new();
        for source in &assets.sample_sources {
            if !short(&source.id)
                || source.regions.iter().any(|r| !short(&r.id))
                || source.slices.iter().any(|slice| !short(&slice.id))
                || source.cues.iter().any(|cue| !short(&cue.id))
            {
                return Err(ParameterError::InvalidSurface);
            }
            if source.resource.path() != std::path::Path::new("assets/advanced-drums.wav") {
                let id = source
                    .resource
                    .path()
                    .to_str()
                    .expect("sample paths originate in UTF-8 YAML");
                let sample = catalog
                    .acquire(id)
                    .map_err(|_| ParameterError::InvalidSurface)?;
                samples.insert(id.to_string(), sample);
            }
        }
        for map in &assets.sample_maps {
            if !short(&map.id)
                || map.zones.iter().any(|zone| {
                    [
                        zone.id.as_deref(),
                        zone.round_robin_group.as_deref(),
                        zone.choke_group.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|id| !short(id))
                })
            {
                return Err(ParameterError::InvalidSurface);
            }
        }
        if root.ports().iter().any(|p| !short(p.name())) {
            return Err(ParameterError::InvalidSurface);
        }
        Ok(samples.into_values().collect())
    }

    /// Admit structural work once. Transport correlation is separate from the
    /// native request ID retained by the persistent session.
    pub fn admit_session_command(
        &mut self,
        request: &AdmittedRequest,
    ) -> Result<String, ParameterError> {
        use dandrum_instrument_session::{MappingAdmission, MappingRequest};
        let context = self
            .protocol
            .admitted_context(self.host.session(), request)
            .map_err(ParameterError::Protocol)?;
        self.check_session_worker()?;
        if self.selections.busy() {
            let retained = match &request.command {
                EditorCommand::SetZoneKeyRange { identity, .. }
                | EditorCommand::PrepareInstrumentSessionPatch { identity, .. }
                | EditorCommand::UndoInstrumentSession { identity }
                | EditorCommand::RedoInstrumentSession { identity } => self
                    .host
                    .session()
                    .request_outcome(&context.attachment, identity.request_id)
                    .map_err(ParameterError::Session)?
                    .is_some(),
                _ => false,
            };
            if !retained {
                return Err(ParameterError::SessionWorkerBusy);
            }
            // Retention only bypasses chooser backpressure. The native admission
            // below still compares the complete request and rejects conflicts.
        }
        let (id, admission) = match &request.command {
            EditorCommand::SetZoneKeyRange { identity, intent } => (
                identity.request_id,
                self.host.session_mut().admit_mapping(
                    &context.attachment,
                    MappingRequest {
                        instance: context.snapshot.instance,
                        request_id: identity.request_id,
                        expected_revision: identity.expected_revision,
                        intent: intent.clone(),
                    },
                ),
            ),
            EditorCommand::PrepareInstrumentSessionPatch { identity, document } => {
                let catalog = &self.host.sample_catalog;
                let parameters = &self.parameters;
                let runtime = &self.runtime;
                (
                    identity.request_id,
                    self.host.session.admit_document_with(
                        &context.attachment,
                        identity.request_id,
                        identity.expected_revision,
                        document.clone(),
                        || {
                            Self::patch_samples(document, parameters, runtime, catalog).map_err(
                                |error| {
                                    dandrum_instrument_session::SessionError::InvalidDocument(
                                        format!("{error:?}"),
                                    )
                                },
                            )
                        },
                    ),
                )
            }
            EditorCommand::UndoInstrumentSession { identity } => (
                identity.request_id,
                self.host.session_mut().admit_undo(
                    &context.attachment,
                    identity.request_id,
                    identity.expected_revision,
                ),
            ),
            EditorCommand::RedoInstrumentSession { identity } => (
                identity.request_id,
                self.host.session_mut().admit_redo(
                    &context.attachment,
                    identity.request_id,
                    identity.expected_revision,
                ),
            ),
            _ => return Err(ParameterError::UnsupportedCommand),
        };
        match admission.map_err(ParameterError::Session)? {
            MappingAdmission::Admitted(job) => {
                self.session_job = Some(ActiveSessionJob {
                    authority: std::sync::Arc::new(()),
                    request_id: id,
                    work: SessionWork::Queued(job),
                });
            }
            MappingAdmission::Pending | MappingAdmission::Completed(_) => {}
        }
        if self.worker.is_some()
            && let Some(work) = self.take_queued_preparation()
        {
            let worker = self.worker.as_ref().expect("installed worker");
            if let Err(
                std::sync::mpsc::TrySendError::Full(work)
                | std::sync::mpsc::TrySendError::Disconnected(work),
            ) = worker
                .requests
                .as_ref()
                .expect("live sender")
                .try_send(WorkerRequest::Mapping(work))
            {
                let WorkerRequest::Mapping(work) = work else {
                    unreachable!("try_send returns the submitted mapping request")
                };
                self.session_job.as_mut().expect("queued job").work = SessionWork::Queued(work.job);
                return Err(ParameterError::SessionWorkerUnavailable);
            }
        }
        self.protocol
            .encode_reply(
                self.host.session(),
                &context.attachment_id.to_string(),
                &request.request_id.to_string(),
                Ok(serde_json::json!({"status":"accepted","job_id":id.0.to_string()})),
            )
            .map_err(ParameterError::Protocol)
    }
    pub fn take_session_preparation(&mut self) -> Option<SessionPreparation> {
        if self.worker.is_some() {
            return None;
        }
        self.take_queued_preparation()
    }
    fn take_queued_preparation(&mut self) -> Option<SessionPreparation> {
        let active = self.session_job.as_mut()?;
        match std::mem::replace(&mut active.work, SessionWork::Preparing) {
            SessionWork::Queued(job) => Some(SessionPreparation {
                authority: active.authority.clone(),
                request_id: active.request_id,
                job,
            }),
            work => {
                active.work = work;
                None
            }
        }
    }
    pub fn submit_session_completion(
        &mut self,
        completion: SessionCompletion,
    ) -> Result<(), RejectedSessionCompletion> {
        let Some(active) = self.session_job.as_mut().filter(|active| {
            active.request_id == completion.request_id
                && std::sync::Arc::ptr_eq(&active.authority, &completion.authority)
                && matches!(active.work, SessionWork::Preparing)
        }) else {
            return Err(RejectedSessionCompletion { completion });
        };
        active.work = SessionWork::Ready(completion.job);
        Ok(())
    }
    /// A bounded main/control service step. Preparation is performed elsewhere;
    /// returned retired processors are destroyed only here, away from audio.
    pub fn service_session_jobs(&mut self) -> Result<SessionJobProgress, ParameterError> {
        self.check_session_worker()?;
        let mut selection_completed = false;
        if let Some(worker) = &self.worker {
            match worker.completions.try_recv() {
                Ok(WorkerCompletion::Mapping(completion)) => self
                    .submit_session_completion(completion)
                    .map_err(|_| ParameterError::InvalidJob)?,
                Ok(WorkerCompletion::Selection(completion)) => {
                    self.complete_selected_job(completion)?;
                    selection_completed = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return Err(ParameterError::SessionWorkerUnavailable);
                }
            }
        }
        drop(self.runtime.reclaim());
        let Some(active) = self.session_job.as_ref() else {
            return Ok(if self.selections.preparing() {
                SessionJobProgress::Pending
            } else if selection_completed {
                SessionJobProgress::Completed
            } else {
                SessionJobProgress::Idle
            });
        };
        if !matches!(active.work, SessionWork::Ready(_)) || !self.runtime.activation_available() {
            return Ok(SessionJobProgress::Pending);
        }
        let next_generation = document_generation(self.generation)?;
        let active = self.session_job.take().expect("ready session job exists");
        let SessionWork::Ready(completion) = active.work else {
            unreachable!("ready session job checked above")
        };
        let mut state = self.parameter_state();
        state.generation = next_generation;
        let projection = SamplerProjection {
            source_path: self.host.resource_root().join(crate::ADVANCED_DRUM_KIT_DOCUMENT),
            parameters: &self.parameters, state,
            instance_id: "18446744073709551615".into(),
            selections: (0..33).map(|_| serde_json::json!({"selectionId":"18446744073709551615","phase":"preparing"})).collect(),
            next_request_id: "18446744073709551615".into(),
            active_request_id: Some(u64::MAX - 1),
        };
        let mut activation = ProjectedActivation {
            runtime: &mut self.runtime,
            projection,
        };
        let outcome = match self
            .host
            .session_mut()
            .complete_mapping(*completion, &mut activation)
        {
            Ok(outcome) => outcome,
            Err(route) => {
                self.session_job = Some(ActiveSessionJob {
                    authority: active.authority,
                    request_id: active.request_id,
                    work: SessionWork::Ready(Box::new(route.into_completion())),
                });
                return Err(ParameterError::InvalidJob);
            }
        };
        if outcome.result.is_ok() {
            // Every active gesture already reserved its End slot. Publication
            // retires this generation for all editors without dropping that End.
            self.end_gestures(None);
            self.generation = next_generation;
        }
        Ok(SessionJobProgress::Completed)
    }
    pub fn session_job_status(&self, request: &AdmittedRequest) -> Result<String, ParameterError> {
        let context = self
            .protocol
            .admitted_context(self.host.session(), request)
            .map_err(ParameterError::Protocol)?;
        let EditorCommand::GetInstrumentSessionJobStatus { request_id } = request.command else {
            return Err(ParameterError::UnsupportedCommand);
        };
        let result = if self
            .session_job
            .as_ref()
            .is_some_and(|active| active.request_id == request_id)
        {
            self.check_session_worker()?;
            serde_json::json!({"request_id":request_id.0.to_string(),"state":"running","revision":context.snapshot.revision.to_string(),"undoCount":context.snapshot.undo_len,"redoCount":context.snapshot.redo_len,"error":""})
        } else {
            let outcome = self
                .host
                .session()
                .request_outcome(&context.attachment, request_id)
                .map_err(ParameterError::Session)?
                .ok_or(ParameterError::UnknownJob)?;
            match &outcome.result {
                Ok(snapshot) => {
                    serde_json::json!({"request_id":request_id.0.to_string(),"state":"completed","revision":snapshot.revision.to_string(),"undoCount":snapshot.undo_len,"redoCount":snapshot.redo_len,"error":""})
                }
                Err(error) => {
                    serde_json::json!({"request_id":request_id.0.to_string(),"state":"failed","error":format!("{error:?}")})
                }
            }
        };
        self.protocol
            .encode_reply(
                self.host.session(),
                &context.attachment_id.to_string(),
                &request.request_id.to_string(),
                Ok(result),
            )
            .map_err(ParameterError::Protocol)
    }

    pub fn apply(&mut self, request: &AdmittedRequest) -> Result<ParameterReply, ParameterError> {
        let context = self
            .protocol
            .admitted_context(self.host.session(), request)
            .map_err(ParameterError::Protocol)?;
        let generation = document_generation(context.snapshot.revision)?;
        match &request.command {
            EditorCommand::GetParameterState => {
                return Ok(ParameterReply::State(self.parameter_state()));
            }
            EditorCommand::BeginGesture {
                parameter_id,
                generation: expected,
            } => {
                let index = self.resolve(parameter_id, *expected, generation)?;
                if self.gesture_owned(index, context.attachment_id)? {
                    return Err(ParameterError::GestureAlreadyActive);
                }
                // One slot publishes Begin; the other remains reserved until
                // this owner ends/detaches, even while other values fill the queue.
                if self.automation_slots() < 2 {
                    return Err(ParameterError::QueueFull);
                }
                self.automation.push_back(AutomationEvent::Begin {
                    parameter_index: index,
                });
                self.gestures[index] = Some(context.attachment_id);
                self.reserved_ends += 1;
            }
            EditorCommand::EndGesture {
                parameter_id,
                generation: expected,
            } => {
                let index = self.resolve(parameter_id, *expected, generation)?;
                if !self.gesture_owned(index, context.attachment_id)? {
                    return Err(ParameterError::GestureNotActive);
                }
                self.automation.push_back(AutomationEvent::End {
                    parameter_index: index,
                });
                self.gestures[index] = None;
                self.reserved_ends -= 1;
            }
            EditorCommand::SetParameter {
                parameter_id,
                value,
                generation: expected,
            } => {
                let index = self.resolve(parameter_id, *expected, generation)?;
                // The sealed context recovered the exact retained command;
                // normalization bounds were checked before its admission.
                let active = self.gesture_owned(index, context.attachment_id)?;
                let sequence = document_generation(self.admitted_sequence)?;
                if self.automation_slots() < if active { 1 } else { 3 }
                    || self.runtime.event_slots() == 0
                {
                    return Err(ParameterError::QueueFull);
                }
                let parameter = &self.parameters[index];
                // Rounding can otherwise put normalized endpoints just outside
                // an accepted nonbinary physical interval (for example -0.1..0.3).
                let physical = (parameter.physical_min
                    + value * (parameter.physical_max - parameter.physical_min))
                    .clamp(parameter.physical_min, parameter.physical_max);
                // Both producers are exclusively owned here. The audio consumer
                // can only free runtime slots, and no caller can fill our main
                // queue between this check and publication of the complete value.
                self.runtime
                    .send_parameter(parameter.root_index, physical, sequence)
                    .map_err(ParameterError::Runtime)?;
                if !active {
                    self.automation.push_back(AutomationEvent::Begin {
                        parameter_index: index,
                    });
                }
                self.automation.push_back(AutomationEvent::Value {
                    parameter_index: index,
                    value: physical,
                });
                if !active {
                    self.automation.push_back(AutomationEvent::End {
                        parameter_index: index,
                    });
                }
                self.admitted_sequence = sequence;
            }
            _ => return Err(ParameterError::UnsupportedCommand),
        }
        Ok(ParameterReply::Accepted {
            generation,
            sequence: self.admitted_sequence,
        })
    }

    fn resolve(&self, id: &str, expected: u64, generation: u64) -> Result<usize, ParameterError> {
        if expected != generation {
            return Err(ParameterError::StaleGeneration);
        }
        self.parameters
            .iter()
            .position(|parameter| parameter.id == id)
            .ok_or(ParameterError::UnknownParameter)
    }

    fn gesture_owned(&self, index: usize, owner: u64) -> Result<bool, ParameterError> {
        match self.gestures[index] {
            None => Ok(false),
            Some(current) if current == owner => Ok(true),
            _ => Err(ParameterError::GestureConflict),
        }
    }

    fn automation_slots(&self) -> usize {
        self.automation_capacity - self.automation.len() - self.reserved_ends
    }

    pub fn parameter_state(&self) -> ParameterState {
        let (sequence, reset_cancelled, physical) = self.runtime.snapshot_parameters();
        ParameterState {
            generation: self.generation,
            sequence,
            reset_cancelled,
            parameters: self
                .parameters
                .iter()
                .map(|parameter| parameter.project(physical[parameter.root_index]))
                .collect(),
        }
    }

    /// Read pending events in delivery order without acknowledging them.
    pub fn pending_automation(&self) -> impl Iterator<Item = AutomationEvent> + '_ {
        self.automation.iter().copied()
    }

    /// A format adapter acknowledges only after its host accepts this event.
    pub fn peek_automation(&self) -> Option<AutomationEvent> {
        self.automation.front().copied()
    }
    pub fn acknowledge_automation(&mut self) -> bool {
        self.automation.pop_front().is_some()
    }
}

#[cfg(test)]
#[path = "../tests/internal/state.rs"]
mod state_tests;

/// One projection serves accepted reads and pre-publication representability.
struct SamplerProjection<'a> {
    source_path: std::path::PathBuf,
    parameters: &'a [PublicParameter],
    state: ParameterState,
    instance_id: String,
    selections: serde_json::Value,
    next_request_id: String,
    active_request_id: Option<u64>,
}
impl SamplerProjection<'_> {
    fn value(
        &self,
        snapshot: &dandrum_instrument_session::SessionSnapshot,
        prepared: &dandrum_engine::preparation::PreparedKernelInstrument,
        attachment_id: &str,
    ) -> Result<serde_json::Value, ParameterError> {
        use dandrum_instrument_session::{
            ACTIVE_REQUEST_LIMIT, ATTACHMENT_LIMIT, CONNECTION_CONTRACT_VERSION, HISTORY_LIMIT,
            OUTCOME_LIMIT,
        };
        use serde_json::{Value, json};
        let source_path = self.source_path.to_str().ok_or_else(|| {
            ParameterError::Host("Trigger resource path cannot be represented as UTF-8".into())
        })?;
        let patch =
            load_kernel_patch_str(&snapshot.yaml).expect("accepted session document is valid");
        let state = &self.state;
        let parameters = self.parameters.iter().zip(&state.parameters).map(|(parameter, current)| {
            let group = parameter.control_group;
            let mut value = json!({"id":parameter.id,"name":parameter.id,"normalisedValue":current.value,
                "normalisedDefaultValue":parameter.project(parameter.physical_default).value,
                "minValue":parameter.physical_min,"maxValue":parameter.physical_max,
                "scope":if group.is_some() {"sampleGroup"} else {"instrument"}});
            if let Some(group) = group { value["controlGroup"] = group.into(); }
            value
        }).collect::<Vec<_>>();
        // Omit absent authored optionals, retaining the comparison DTO's default
        // semantics without manufacturing values or exposing PCM.
        let present = |mut value: Value| {
            value
                .as_object_mut()
                .expect("DTO is an object")
                .retain(|_, value| !value.is_null());
            value
        };
        let assets = prepared.sample_assets();
        let sources = assets.sources().iter().map(|source| {
            let regions = source.declaration().regions.iter().map(|region| {
                let loop_settings = region.loop_settings.as_ref().map(|settings| json!({
                    "mode":settings.mode,"startFrame":settings.start_frame.to_string(),
                    "endFrame":settings.end_frame.to_string(),"crossfadeMs":settings.crossfade_ms.unwrap_or(0.0)}));
                present(json!({"id":region.id,"startFrame":region.start_frame.to_string(),"endFrame":region.end_frame.to_string(),
                    "rootNote":region.root_note,"gainDb":region.gain_db,"pan":region.pan,"reverse":region.reverse,
                    "fadeInMs":region.fade_in_ms.unwrap_or(0.0),"fadeOutMs":region.fade_out_ms.unwrap_or(0.0),"loop":loop_settings}))
            }).collect::<Vec<_>>();
            json!({"id":source.id(),"sampleRateHz":source.sample().sample_rate_hz(),
                "channelCount":source.sample().source_channel_count(),"frameCount":source.sample().frame_count().to_string(),"regions":regions})
        }).collect::<Vec<_>>();
        let maps = assets.maps().iter().map(|map| {
            let prepared = assets.map_by_id(&map.id).expect("prepared map is indexed");
            let zones = prepared.zones().iter().map(|zone| {
                let declaration = zone.declaration();
                present(json!({"id":declaration.id.as_deref().unwrap_or(""),
                    "sourceIndex":zone.source_index(),"regionIndex":zone.region_index(),
                    "startFrame":zone.effective_region().start_frame.to_string(),"endFrame":zone.effective_region().end_frame.to_string(),
                    "keyLow":declaration.key_range[0],"keyHigh":declaration.key_range[1],
                    "velocityLow":declaration.velocity_range[0],"velocityHigh":declaration.velocity_range[1],
                    "roundRobinGroup":declaration.round_robin_group,"chokeGroup":declaration.choke_group,
                    "controlGroup":declaration.control_group,"weight":declaration.weight.unwrap_or(1),
                    "gainDb":declaration.gain_db,"pan":declaration.pan,"pitchSemitones":declaration.pitch_semitones}))
            }).collect::<Vec<_>>();
            json!({"id":map.id,"selectionMode":prepared.selection_mode().unwrap_or("first_match"),"selectionSeed":prepared.selection_seed().to_string(),"zones":zones})
        }).collect::<Vec<_>>();
        let mut result = json!({"yaml":snapshot.yaml,
            "sourcePath":source_path,
            "sampleSelections":self.selections,
            "document":{"generation":state.generation,"instrumentId":patch.instrument().map(|identity| identity.id.as_str()).unwrap_or(""),
                "parameters":parameters,"sources":sources,"maps":maps},
            "connection":{"version":CONNECTION_CONTRACT_VERSION,"instanceId":self.instance_id,
                "attachmentId":attachment_id,"revision":snapshot.revision.to_string(),"nextRequestId":self.next_request_id,
                "limits":{"attachments":ATTACHMENT_LIMIT,"history":HISTORY_LIMIT,"outcomes":OUTCOME_LIMIT,"activeRequests":ACTIVE_REQUEST_LIMIT},
                "history":{"undoCount":snapshot.undo_len,"redoCount":snapshot.redo_len}}});
        if let Some(id) = self.active_request_id {
            result["connection"]["activeRequestId"] = id.to_string().into();
        }
        Ok(result)
    }
}
struct ProjectedActivation<'a> {
    runtime: &'a mut RuntimeControl,
    projection: SamplerProjection<'a>,
}
impl dandrum_instrument_session::ActivationHost for ProjectedActivation<'_> {
    type Error = RuntimeError;
    type Lease<'a>
        = <RuntimeControl as dandrum_instrument_session::ActivationHost>::Lease<'a>
    where
        Self: 'a;
    fn activate<'a>(
        &'a mut self,
        prepared: &dandrum_engine::preparation::PreparedKernelInstrument,
        next: &dandrum_instrument_session::SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        // Reserve worst-width connection/correlation fields and the full bounded
        // selection-summary window. No accepted document can strand a new view.
        let mut result = self
            .projection
            .value(next, prepared, "18446744073709551615")
            .map_err(|_| RuntimeError::IncompatibleRuntime)?;
        result["document"]["generation"] = MAX_SAFE_GENERATION.into();
        result["connection"]["revision"] = u64::MAX.to_string().into();
        result["connection"]["history"] = serde_json::json!({
            "undoCount":dandrum_instrument_session::HISTORY_LIMIT,
            "redoCount":dandrum_instrument_session::HISTORY_LIMIT,
        });
        // The locked serde_json formatter uses zmij's24-byte buffer. A quoted
        // 25-character size-only sentinel reserves27 bytes for each numeric
        // field. This conservative projection is never sent to an editor.
        for parameter in result["document"]["parameters"]
            .as_array_mut()
            .expect("projection parameters are an array")
        {
            for field in [
                "normalisedValue",
                "normalisedDefaultValue",
                "minValue",
                "maxValue",
            ] {
                parameter[field] = "0".repeat(25).into();
            }
        }
        let frame = serde_json::json!({"version":2,"instanceId":self.projection.instance_id,
            "attachmentId":"18446744073709551615","requestId":"18446744073709551615",
            "status":"accepted","result":result});
        if serde_json::to_vec(&frame)
            .expect("finite snapshot values serialize")
            .len()
            > crate::protocol::PATCH_OUTBOUND_BYTES_LIMIT
        {
            return Err(RuntimeError::IncompatibleRuntime);
        }
        dandrum_instrument_session::ActivationHost::activate(self.runtime, prepared, next)
    }
}
