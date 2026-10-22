//! Chooser authority is view-owned; admitted decoding and results are not.
use super::*;
use crate::samples::{
    NativeSampleFile, SampleError, SampleSelectionCompletion, SampleSelectionJob, SelectedFile,
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

pub struct NativeSelectionPermit {
    id: u64,
    authority: Arc<()>,
}
enum Phase {
    Choosing,
    Preparing,
    Ready(Vec<SelectedFile>),
    Cancelled,
    Failed(SampleError),
    Released,
}
impl Phase {
    fn name(&self) -> &'static str {
        match self {
            Self::Choosing => "choosing",
            Self::Preparing => "preparing",
            Self::Ready(_) => "ready",
            Self::Cancelled => "cancelled",
            Self::Failed(_) => "failed",
            Self::Released => "released",
        }
    }
    fn active(&self) -> bool {
        matches!(self, Self::Choosing | Self::Preparing)
    }
}
struct Record {
    id: u64,
    attachment_id: u64,
    attachment: dandrum_instrument_session::Attachment,
    authority: Arc<()>,
    phase: Phase,
}
pub(super) struct Selections {
    next_id: u64,
    records: VecDeque<Record>,
    dispatched: BTreeMap<u64, u64>,
}
impl Default for Selections {
    fn default() -> Self {
        Self {
            next_id: 1,
            records: VecDeque::with_capacity(33),
            dispatched: BTreeMap::new(),
        }
    }
}
impl Selections {
    pub(super) fn busy(&self) -> bool {
        self.records.iter().any(|record| record.phase.active())
    }
    pub(super) fn preparing(&self) -> bool {
        self.records
            .iter()
            .any(|record| matches!(record.phase, Phase::Preparing))
    }
    pub(super) fn detach(&mut self, attachment: u64) {
        self.dispatched.remove(&attachment);
        for record in &mut self.records {
            if record.attachment_id == attachment && matches!(record.phase, Phase::Choosing) {
                record.phase = Phase::Cancelled;
            }
        }
        self.trim();
    }
    pub(super) fn stop_worker(&mut self) {
        for record in &mut self.records {
            if record.phase.active() {
                record.phase = Phase::Failed(SampleError::WorkerUnavailable);
            }
        }
        self.trim();
    }
    fn trim(&mut self) {
        while self
            .records
            .iter()
            .filter(|record| !record.phase.active())
            .count()
            > 32
        {
            let index = self
                .records
                .iter()
                .position(|record| !record.phase.active())
                .expect("excess terminal record exists");
            self.records.remove(index);
        }
    }
    pub(super) fn summaries(&self) -> serde_json::Value {
        self.records
            .iter()
            .map(|record| {
                serde_json::json!({
                    "selectionId":record.id.to_string(), "phase":record.phase.name()
                })
            })
            .collect()
    }
}
pub(super) struct Preparation {
    id: u64,
    work: SampleSelectionJob,
}
pub(super) struct Completion {
    id: u64,
    work: SampleSelectionCompletion,
}
impl Preparation {
    pub(super) fn prepare(self) -> Completion {
        Completion {
            id: self.id,
            work: self.work.prepare(),
        }
    }
}

impl ParameterController {
    pub fn begin_sample_selection(
        &mut self,
        request: &AdmittedRequest,
    ) -> Result<(NativeSelectionPermit, String), ParameterError> {
        let context = self
            .protocol
            .admitted_context(self.host.session(), request)
            .map_err(ParameterError::Protocol)?;
        if request.command != EditorCommand::ChooseSamplerFiles {
            return Err(ParameterError::UnsupportedCommand);
        }
        let watermark = self
            .selections
            .dispatched
            .entry(context.attachment_id)
            .or_default();
        if request.request_id <= *watermark {
            return Err(ParameterError::Protocol(ProtocolError::DuplicateRequest));
        }
        *watermark = request.request_id;
        self.check_session_worker()?;
        if !self.worker.as_ref().is_some_and(SessionWorker::available) {
            return Err(ParameterError::SessionWorkerUnavailable);
        }
        if self.session_job.is_some() || self.selections.busy() {
            return Err(ParameterError::SessionWorkerBusy);
        }
        let id = self.selections.next_id;
        let next = id
            .checked_add(1)
            .ok_or(ParameterError::Protocol(ProtocolError::IdentityExhausted))?;
        let authority = Arc::new(());
        let reply = self
            .protocol
            .encode_reply(
                self.host.session(),
                &context.attachment_id.to_string(),
                &request.request_id.to_string(),
                Ok(serde_json::json!({"selectionId":id.to_string(),"phase":"choosing"})),
            )
            .map_err(ParameterError::Protocol)?;
        self.selections.records.push_back(Record {
            id,
            attachment_id: context.attachment_id,
            attachment: context.attachment,
            authority: Arc::clone(&authority),
            phase: Phase::Choosing,
        });
        self.selections.next_id = next;
        Ok((NativeSelectionPermit { id, authority }, reply))
    }
    fn selection_permit_index(
        &self,
        permit: &NativeSelectionPermit,
    ) -> Result<usize, ParameterError> {
        self.selections
            .records
            .iter()
            .position(|record| {
                record.id == permit.id
                    && Arc::ptr_eq(&record.authority, &permit.authority)
                    && matches!(record.phase, Phase::Choosing)
                    && self.host.session().snapshot(&record.attachment).is_ok()
            })
            .ok_or(ParameterError::Protocol(ProtocolError::StaleAttachment))
    }
    pub fn cancel_sample_selection(
        &mut self,
        permit: &NativeSelectionPermit,
    ) -> Result<(), ParameterError> {
        let index = self.selection_permit_index(permit)?;
        self.selections.records[index].phase = Phase::Cancelled;
        self.selections.trim();
        Ok(())
    }
    pub fn fail_sample_selection(
        &mut self,
        permit: &NativeSelectionPermit,
        error: SampleError,
    ) -> Result<(), ParameterError> {
        let index = self.selection_permit_index(permit)?;
        self.selections.records[index].phase = Phase::Failed(error);
        self.selections.trim();
        Ok(())
    }
    pub fn admit_selected_files(
        &mut self,
        permit: &NativeSelectionPermit,
        files: Vec<NativeSampleFile>,
    ) -> Result<(), ParameterError> {
        let index = self.selection_permit_index(permit)?;
        let Some(worker) = self.worker.as_ref().filter(|worker| worker.available()) else {
            self.selections.records[index].phase = Phase::Failed(SampleError::WorkerUnavailable);
            self.selections.trim();
            return Err(ParameterError::SessionWorkerUnavailable);
        };
        let work = match self.host.sample_catalog.begin(files) {
            Ok(work) => work,
            Err(error) => {
                self.selections.records[index].phase = Phase::Failed(error);
                self.selections.trim();
                return Err(ParameterError::Protocol(ProtocolError::CommandRejected));
            }
        };
        match worker
            .requests
            .as_ref()
            .expect("available worker has a sender")
            .try_send(WorkerRequest::Selection(Preparation {
                id: permit.id,
                work,
            })) {
            Ok(()) => self.selections.records[index].phase = Phase::Preparing,
            Err(_) => {
                self.selections.records[index].phase =
                    Phase::Failed(SampleError::WorkerUnavailable);
                self.selections.trim();
                return Err(ParameterError::SessionWorkerUnavailable);
            }
        }
        Ok(())
    }
    pub(super) fn complete_selected_job(
        &mut self,
        completion: Completion,
    ) -> Result<(), ParameterError> {
        let record = self
            .selections
            .records
            .iter_mut()
            .find(|record| record.id == completion.id && matches!(record.phase, Phase::Preparing))
            .ok_or(ParameterError::InvalidJob)?;
        record.phase = match self
            .host
            .sample_catalog
            .accept(completion.work)
            .map_err(|_| ParameterError::InvalidJob)?
        {
            Ok(files) => Phase::Ready(files),
            Err(error) => Phase::Failed(error),
        };
        self.selections.trim();
        Ok(())
    }
    pub fn sample_selection_reply(
        &mut self,
        request: &AdmittedRequest,
    ) -> Result<String, ParameterError> {
        let context = self
            .protocol
            .admitted_context(self.host.session(), request)
            .map_err(ParameterError::Protocol)?;
        let (id, release) = match request.command {
            EditorCommand::GetSamplerSelectionStatus { selection_id } => (selection_id, false),
            EditorCommand::ReleaseSamplerSelection { selection_id } => (selection_id, true),
            _ => return Err(ParameterError::UnsupportedCommand),
        };
        let result = match self
            .selections
            .records
            .iter_mut()
            .find(|record| record.id == id)
        {
            None => Err(ProtocolError::UnknownSelection),
            Some(record) if release && record.phase.active() => Err(ProtocolError::Busy),
            Some(record) => {
                if release {
                    record.phase = Phase::Released;
                }
                let mut value =
                    serde_json::json!({"selectionId":id.to_string(),"phase":record.phase.name()});
                match &record.phase {
                    Phase::Ready(files) => value["files"] = files.iter().map(|file| serde_json::json!({
                        "resourceId":file.sample.resource_id(),"name":file.name,
                        "contentSha256":file.sample.content_sha256(),"encodedBytes":file.sample.encoded_bytes().len(),
                        "sampleRateHz":file.sample.sample().sample_rate_hz(),"channelCount":file.sample.sample().source_channel_count(),
                        "frameCount":file.sample.sample().frame_count(),
                    })).collect(),
                    Phase::Failed(error) => value["error"] = serde_json::to_value(error).expect("fieldless error enum serializes"),
                    _ => {},
                }
                Ok(value)
            }
        };
        self.protocol
            .encode_reply(
                self.host.session(),
                &context.attachment_id.to_string(),
                &request.request_id.to_string(),
                result,
            )
            .map_err(ParameterError::Protocol)
    }
}
