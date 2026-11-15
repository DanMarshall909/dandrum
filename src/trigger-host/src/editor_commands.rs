//! Format-neutral editor command routing and wire replies, used only off audio.
//!
//! Native choosers and host callbacks remain capabilities of each adapter.
use crate::{
    parameters::{ParameterController, ParameterError, ParameterReply, ParameterValue},
    protocol::{Admission, EditorCommand, ProtocolError},
};
use std::time::Instant;

/// Route an authenticated frame through the one persistent instrument authority.
/// Native file chooser admission stays with the adapter that owns the dialog.
pub fn dispatch(
    controller: &mut ParameterController,
    attachment: &str,
    request_id: &str,
    message: &str,
) -> Result<String, ParameterError> {
    dispatch_with_publication(controller, attachment, request_id, message).map(|(frame, _)| frame)
}

/// The same command route, with an optional coherent parameter snapshot cutoff
/// for adapters that must order separately delivered feedback. Canonical editor
/// frames and native admission remain unchanged.
pub fn dispatch_with_publication(
    controller: &mut ParameterController,
    attachment: &str,
    request_id: &str,
    message: &str,
) -> Result<(String, Option<u64>), ParameterError> {
    let mut publication = None;
    let result = match controller.admit(message).map_err(input_error) {
        Ok(Admission::New(request)) => match request.command {
            EditorCommand::NegotiateEditorProtocol => controller
                .negotiate_editor_protocol(&request)
                .map(Reply::Encoded)
                .map_err(command_error),
            EditorCommand::GetSamplerSession => controller
                .sampler_session_reply(&request)
                .map(Reply::Encoded)
                .map_err(snapshot_error),
            EditorCommand::GetSamplerSelectionStatus { .. }
            | EditorCommand::ReleaseSamplerSelection { .. } => {
                return controller
                    .sample_selection_reply(&request)
                    .map(|frame| (frame, None));
            }
            EditorCommand::ChooseSamplerFiles => Err(ProtocolError::CapabilityUnavailable),
            EditorCommand::GetInstrumentSessionJobStatus { .. } => controller
                .session_job_status(&request)
                .map(Reply::Encoded)
                .map_err(command_error),
            EditorCommand::SetZoneKeyRange { .. }
            | EditorCommand::UndoInstrumentSession { .. }
            | EditorCommand::RedoInstrumentSession { .. }
            | EditorCommand::PrepareInstrumentSessionPatch { .. } => controller
                .admit_session_command(&request)
                .map(Reply::Encoded)
                .map_err(command_error),
            EditorCommand::NoteOn { .. }
            | EditorCommand::NoteOff { .. }
            | EditorCommand::NoteHeartbeat => controller
                .apply_editor_note(&request, Instant::now())
                .map(|()| Reply::Value(serde_json::Value::Null))
                .map_err(input_error),
            EditorCommand::GetParameterState => controller
                .apply(&request)
                .map(|_| {
                    let (state, stamp) = controller.parameter_state_with_publication();
                    publication = Some(stamp);
                    Reply::Value(parameter_reply(ParameterReply::State(state)))
                })
                .map_err(command_error),
            EditorCommand::BeginGesture { .. }
            | EditorCommand::SetParameter { .. }
            | EditorCommand::EndGesture { .. } => controller
                .apply(&request)
                .map(parameter_reply)
                .map(Reply::Value)
                .map_err(command_error),
        },
        Ok(Admission::Duplicate { .. }) => Err(ProtocolError::DuplicateRequest),
        Err(error) => Err(error),
    };
    let frame = match result {
        Ok(Reply::Encoded(reply)) => Ok(reply),
        Ok(Reply::Value(value)) => {
            controller.encode_editor_reply(attachment, request_id, Ok(value))
        }
        Err(error) => controller.encode_editor_rejection(attachment, request_id, message, error),
    }?;
    Ok((frame, publication))
}

// Preserve already framed snapshots/jobs without decoding and re-encoding them.
enum Reply {
    Encoded(String),
    Value(serde_json::Value),
}

// Admission and note intent use the same existing input-error policy. Keeping
// it in one conversion also avoids separate, unreachable fallback branches
// after exclusive admission has already authenticated a request.
fn input_error(error: ParameterError) -> ProtocolError {
    match error {
        ParameterError::Protocol(error) => error,
        _ => ProtocolError::InvalidArguments,
    }
}
fn snapshot_error(error: ParameterError) -> ProtocolError {
    match error {
        ParameterError::Protocol(error) => error,
        other => command_error(other),
    }
}

pub fn parameter_values(parameters: Vec<ParameterValue>) -> serde_json::Value {
    parameters
        .into_iter()
        .map(|parameter| {
            serde_json::json!({
                "id": parameter.id, "name": parameter.name, "value": parameter.value,
            })
        })
        .collect()
}
fn parameter_reply(reply: ParameterReply) -> serde_json::Value {
    match reply {
        ParameterReply::Accepted {
            generation,
            sequence,
        } => serde_json::json!({
            "status": "accepted", "generation": generation, "sequence": sequence,
        }),
        ParameterReply::State(state) => serde_json::json!({
            "generation": state.generation, "sequence": state.sequence,
            "reset_cancelled": state.reset_cancelled,
            "parameters": parameter_values(state.parameters),
        }),
    }
}
pub fn command_error(error: ParameterError) -> ProtocolError {
    match error {
        ParameterError::QueueFull | ParameterError::SessionWorkerBusy => ProtocolError::Busy,
        ParameterError::Session(crate::SessionError::StaleRequest) => ProtocolError::StaleRequest,
        _ => ProtocolError::CommandRejected,
    }
}
