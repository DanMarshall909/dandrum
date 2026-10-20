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
    PreparedRuntime, RuntimeAudio, RuntimeControl, RuntimeError, runtime_handoff,
};
use crate::{TriggerHost, TriggerHostConfiguration};
use dandrum_engine::kernel::document::load_kernel_patch_str;
use std::{collections::VecDeque, num::NonZeroU64, path::Path};

pub const MAX_AUTOMATION_EVENTS: usize = 1_024;

#[derive(Debug, PartialEq, Eq)]
pub enum ParameterError {
    Host(String),
    Session(dandrum_instrument_session::SessionError),
    UnknownJob,
    InvalidJob,
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

pub struct ParameterController {
    session_job: Option<ActiveSessionJob>,
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
}

impl ParameterController {
    /// Save accepted YAML and one coherent vector of applied physical values.
    /// This off-audio read never closes callback admission; only the reader may
    /// retry while audio writes. Queued editor values are not yet applied state.
    pub fn save_state(
        &self,
        output: &mut impl std::io::Write,
    ) -> Result<(), crate::state::StateError> {
        use crate::state::{PRODUCT_ID, RESOURCE_ID, SavedParameter, SavedState};
        let snapshot = self.host.session().owner_snapshot();
        let values = self.runtime.snapshot_state_values();
        crate::state::write(
            &SavedState {
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
            },
            output,
        )
    }
    /// Validate and prepare complete bounded native state before publication.
    /// Held gestures/outbound automation return Busy so no promised End is lost.
    /// Success retires old queued activity and resets structural history; failure
    /// preserves accepted state. No disposable editor attachment is required.
    pub fn load_state(
        &mut self,
        input: &mut impl std::io::Read,
    ) -> Result<(), crate::state::StateError> {
        use crate::state::StateError;
        use dandrum_instrument_session::CommitError;
        let state = crate::state::read(input)?;
        if self.reserved_ends != 0 || !self.automation.is_empty() {
            return Err(StateError::Busy);
        }
        let values = state.values_for(&self.parameters, &self.runtime)?;
        let generation =
            crate::state::restored_generation(self.host.session().owner_snapshot().revision)?;
        crate::PackagedResources::regular_file(
            self.host.resource_root(),
            crate::ADVANCED_DRUM_KIT_SAMPLE,
        )
        .map_err(|_| StateError::Invalid)?;
        let candidate = self
            .host
            .session()
            .prepare_document_replacement_owned(&state.document)
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
        self.end_gestures(Some(owner));
        Ok(())
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
            parameters.push(PublicParameter {
                id: alias.name().into(),
                root_index,
                physical_min: min,
                physical_max: max,
                physical_default: range.default(),
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
                session_job: None,
                host,
                runtime,
                protocol: EditorProtocol::new(instance_id),
                parameters,
                gestures,
                reserved_ends: 0,
                generation: document_generation(0)?,
                admitted_sequence: 0,
                automation: VecDeque::with_capacity(automation_capacity),
                automation_capacity,
            },
            audio,
        ))
    }

    pub fn parameters(&self) -> &[PublicParameter] {
        &self.parameters
    }
    pub fn session(&self) -> &crate::InstrumentSession {
        self.host.session()
    }
    pub fn attach_editor(&mut self) -> Result<EditorConnection, ParameterError> {
        self.protocol
            .attach(self.host.session_mut())
            .map_err(ParameterError::Protocol)
    }
    pub fn admit(&mut self, message: &str) -> Result<Admission, ParameterError> {
        self.protocol
            .admit(self.host.session(), message)
            .map_err(ParameterError::Protocol)
    }

    /// Project the accepted native session off audio and encode one bounded v1
    /// reply. This read reuses its admitted editor and never changes history.
    pub fn sampler_session_reply(
        &self,
        request: &AdmittedRequest,
    ) -> Result<String, ParameterError> {
        use dandrum_instrument_session::{
            ACTIVE_REQUEST_LIMIT, ATTACHMENT_LIMIT, CONNECTION_CONTRACT_VERSION, HISTORY_LIMIT,
            OUTCOME_LIMIT,
        };
        use serde_json::{Value, json};
        let context = self
            .protocol
            .admitted_context(self.host.session(), request)
            .map_err(ParameterError::Protocol)?;
        let snapshot = context.snapshot;
        let instance_id = context.instance_id.to_string();
        if request.command != EditorCommand::GetSamplerSession {
            return Err(ParameterError::UnsupportedCommand);
        }
        let source_path = self
            .host
            .resource_root()
            .join(crate::ADVANCED_DRUM_KIT_DOCUMENT);
        let source_path = source_path.to_str().ok_or_else(|| {
            ParameterError::Host("Trigger resource path cannot be represented as UTF-8".into())
        })?;
        let patch =
            load_kernel_patch_str(&snapshot.yaml).expect("accepted session document is valid");
        let state = self.parameter_state();
        let parameters = self.parameters.iter().zip(&state.parameters).map(|(parameter, current)| {
            let alias = patch.preset_surface().parameters().iter().find(|alias| alias.name() == parameter.id)
                .expect("accepted Trigger surface retains public aliases");
            let group = patch.root().ports().iter().find(|port| port.name() == alias.port_name())
                .and_then(|port| match port.internal_targets() { [target] => Some(target), _ => None })
                .and_then(|target| patch.root().nodes().iter().find(|node| node.id() == target.node())
                    .filter(|node| node.definition_ref() == "sample_map_player").map(|_| target))
                .and_then(|target| target.port().strip_prefix("group_"))
                .and_then(|suffix| suffix.split_once('_'))
                .and_then(|(group, _)| group.parse::<u8>().ok()).filter(|group| *group > 0);
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
        let assets = self.host.session().prepared().sample_assets();
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
        let next_request = self
            .host
            .session()
            .next_request_id()
            .map_err(ProtocolError::Session)
            .map_err(ParameterError::Protocol)?;
        let result = json!({"yaml":snapshot.yaml,
            "sourcePath":source_path,
            "document":{"generation":state.generation,"instrumentId":patch.instrument().map(|identity| identity.id.as_str()).unwrap_or(""),
                "parameters":parameters,"sources":sources,"maps":maps},
            "connection":{"version":CONNECTION_CONTRACT_VERSION,"instanceId":instance_id,
                "attachmentId":request.attachment_id.to_string(),"revision":snapshot.revision.to_string(),"nextRequestId":next_request.0.to_string(),
                "limits":{"attachments":ATTACHMENT_LIMIT,"history":HISTORY_LIMIT,"outcomes":OUTCOME_LIMIT,"activeRequests":ACTIVE_REQUEST_LIMIT},
                "history":{"undoCount":snapshot.undo_len,"redoCount":snapshot.redo_len}}});
        self.protocol
            .encode_reply(
                self.host.session(),
                &request.attachment_id.to_string(),
                &request.request_id.to_string(),
                Ok(result),
            )
            .map_err(ParameterError::Protocol)
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
            EditorCommand::UndoInstrumentSession { identity } => (
                identity.request_id,
                self.host.session_mut().admit_undo(
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
        drop(self.runtime.reclaim());
        let Some(active) = self.session_job.as_ref() else {
            return Ok(SessionJobProgress::Idle);
        };
        if !matches!(active.work, SessionWork::Ready(_)) || !self.runtime.activation_available() {
            return Ok(SessionJobProgress::Pending);
        }
        let next_generation = document_generation(self.generation)?;
        let active = self.session_job.take().expect("ready session job exists");
        let SessionWork::Ready(completion) = active.work else {
            unreachable!("ready session job checked above")
        };
        let outcome = match self
            .host
            .session_mut()
            .complete_mapping(*completion, &mut self.runtime)
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
