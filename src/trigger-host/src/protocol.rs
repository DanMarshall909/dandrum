//! Bounded editor admission and envelope encoding, used only off audio.
//!
//! Version 1 keeps its existing limits. A retained native attachment can opt
//! into version 2 patch/snapshot framing; other methods and events remain v1.
//! Full-patch documents are shared immutable strings with separate decoded,
//! wire and retained-replay byte limits, in addition to the request-count cap.
//!
//! Requests are JSON `{version:1, attachmentId:"1", requestId:"1", method, args}`.
//! Identities are canonical decimal strings carrying full `u64` precision. The
//! host supplies a distinct nonzero instance identity; native `SessionIdentity`
//! and `Attachment` values remain the authority and are never serialized.
//!
//! Supported methods preserve the comparison frontend's positional arguments:
//! - `getSamplerSession`, `getParameterState`, `noteHeartbeat`: `[]`.
//! - `setInstrumentZoneKeyRange`: `[instanceId, attachmentId, sessionRequestId,
//!   revision, mapId, zoneId, low, high]` (the first four are decimal strings).
//! - `undoInstrumentSession`, `redoInstrumentSession`: `[instanceId, attachmentId, sessionRequestId, revision]`.
//! - `getInstrumentSessionJobStatus`: `[sessionRequestId]`.
//! - `beginGesture`, `endGesture`: `[parameterId, generation]`.
//! - `setParameter`: `[parameterId, normalizedValue, generation]`.
//! - `noteOn`: `[note, normalizedVelocity, generation]`; `noteOff`: `[note, generation]`.
//!
//! The ingress limit is 16 KiB, identifiers are at most 256 UTF-8 bytes, and the
//! native session's 16-attachment limit applies. Each attachment retains at most
//! 128 typed transport requests. Accepted transport IDs increase independently
//! of session job IDs. An identical retry returns `Duplicate` and must never be
//! dispatched again; adapters recover structural outcomes through the existing
//! session job identity. Retired transport IDs cannot be reused. Admission does
//! not execute commands or admit structural work into `InstrumentSession`.
//!
//! Replies carry `{version, instanceId, attachmentId, requestId, status}` plus
//! `result` or `error:{code}`. Events carry `{version, instanceId, attachmentId,
//! sequence, event, payload}`. Event sequences are positive decimal strings,
//! local to an attachment. Only `parameterStateChanged` and
//! `instrumentSessionChanged` events are supported. Encoded output is at most
//! 64 KiB; failure returns no partial message and consumes no event sequence.
//! Snapshot/result projection and actual command dispatch belong to adapters.

use dandrum_instrument_session::{Attachment, InstrumentSession, SessionError, SessionSnapshot};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MESSAGE_BYTES_LIMIT: usize = 16 * 1024;
pub const REPLAY_LIMIT: usize = 128;
pub const PATCH_PROTOCOL_VERSION: u32 = 2;
pub const PATCH_DOCUMENT_BYTES_LIMIT: usize = 65_536;
pub const PATCH_MESSAGE_BYTES_LIMIT: usize = 524_288;
pub const PATCH_OUTBOUND_BYTES_LIMIT: usize = 1_048_576;
pub const PATCH_REPLAY_DOCUMENT_BYTES_LIMIT: usize = 262_144;

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "code", rename_all = "camelCase")]
pub enum ProtocolError {
    Malformed,
    Oversized,
    UnsupportedVersion,
    StaleAttachment,
    InvalidArguments,
    CapabilityUnavailable,
    UnknownSelection,
    Busy,
    CommandRejected,
    DuplicateRequest,
    ConflictingRequest,
    StaleRequest,
    RetiredRequest,
    IdentityExhausted,
    Session(#[serde(skip)] SessionError),
}

pub struct EditorConnection {
    pub instance_id: String,
    pub attachment_id: String,
    pub snapshot: SessionSnapshot,
}
struct Connection {
    admission_owner: std::sync::Arc<()>,
    id: u64,
    attachment: Attachment,
    highest_request: u64,
    next_event: u64,
    patch_protocol: bool,
    retained: std::collections::VecDeque<(u64, u32, EditorCommand)>,
}
/// Own one protocol per instrument; only the first native session can attach.
pub struct EditorProtocol {
    instance_id: std::num::NonZeroU64,
    owner: Option<dandrum_instrument_session::SessionIdentity>,
    connections: Vec<Connection>,
    next_attachment: u64,
}
impl EditorProtocol {
    pub fn attach(
        &mut self,
        session: &mut InstrumentSession,
    ) -> Result<EditorConnection, ProtocolError> {
        if self.connections.len() == dandrum_instrument_session::ATTACHMENT_LIMIT {
            return Err(ProtocolError::Session(SessionError::AttachmentLimit));
        }
        let next = self
            .next_attachment
            .checked_add(1)
            .ok_or(ProtocolError::IdentityExhausted)?;
        let (attachment, snapshot) = session.attach().map_err(ProtocolError::Session)?;
        if self
            .owner
            .as_ref()
            .is_some_and(|owner| *owner != snapshot.instance)
        {
            session
                .detach(&attachment)
                .expect("newly attached native editor is live");
            return Err(ProtocolError::StaleAttachment);
        }
        self.owner = Some(snapshot.instance.clone());
        let id = self.next_attachment;
        self.connections.push(Connection {
            admission_owner: std::sync::Arc::new(()),
            id,
            attachment,
            highest_request: 0,
            next_event: 1,
            patch_protocol: false,
            retained: std::collections::VecDeque::new(),
        });
        self.next_attachment = next;
        Ok(EditorConnection {
            instance_id: self.instance_id.to_string(),
            attachment_id: id.to_string(),
            snapshot,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EditorCommand {
    NegotiateEditorProtocol,
    GetSamplerSession,
    PrepareInstrumentSessionPatch {
        identity: SessionCommandIdentity,
        document: std::sync::Arc<str>,
    },
    ChooseSamplerFiles,
    GetSamplerSelectionStatus {
        selection_id: u64,
    },
    ReleaseSamplerSelection {
        selection_id: u64,
    },
    GetParameterState,
    SetZoneKeyRange {
        identity: SessionCommandIdentity,
        intent: dandrum_instrument_session::SetZoneKeyRange,
    },
    UndoInstrumentSession {
        identity: SessionCommandIdentity,
    },
    RedoInstrumentSession {
        identity: SessionCommandIdentity,
    },
    GetInstrumentSessionJobStatus {
        request_id: dandrum_instrument_session::RequestId,
    },
    BeginGesture {
        parameter_id: String,
        generation: u64,
    },
    SetParameter {
        parameter_id: String,
        value: f64,
        generation: u64,
    },
    EndGesture {
        parameter_id: String,
        generation: u64,
    },
    NoteOn {
        note: u8,
        velocity: f64,
        generation: u64,
    },
    NoteOff {
        note: u8,
        generation: u64,
    },
    NoteHeartbeat,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionCommandIdentity {
    pub instance_id: u64,
    pub attachment_id: u64,
    pub request_id: dandrum_instrument_session::RequestId,
    pub expected_revision: u64,
}

#[derive(Debug)]
pub struct AdmittedRequest {
    admission_owner: std::sync::Arc<()>,
    wire_version: u32,
    /// Transport correlation, independent of a structural session request.
    pub request_id: u64,
    pub attachment_id: u64,
    pub attachment: Attachment,
    pub command: EditorCommand,
}
#[derive(Debug)]
pub enum Admission {
    New(AdmittedRequest),
    Duplicate { request_id: u64 },
}
impl EditorProtocol {
    pub(crate) fn message_fits_limits(
        &self,
        session: &InstrumentSession,
        attachment: Option<&str>,
        message: &str,
    ) -> bool {
        message.len() <= PATCH_MESSAGE_BYTES_LIMIT
            && (message.len() <= MESSAGE_BYTES_LIMIT
                || self.patch_profile(session, attachment, message).is_some())
    }

    pub fn admit(
        &mut self,
        session: &InstrumentSession,
        message: &str,
    ) -> Result<Admission, ProtocolError> {
        if !self.message_fits_limits(session, None, message) {
            return Err(ProtocolError::Oversized);
        }
        let envelope: Envelope =
            serde_json::from_str(message).map_err(|_| ProtocolError::Malformed)?;
        if !matches!(envelope.version, PROTOCOL_VERSION | PATCH_PROTOCOL_VERSION) {
            return Err(ProtocolError::UnsupportedVersion);
        }
        let attachment_id = decimal(&envelope.attachment_id, false)?;
        let request_id = decimal(&envelope.request_id, false)?;
        let connection = self
            .connections
            .iter_mut()
            .find(|connection| connection.id == attachment_id)
            .ok_or(ProtocolError::StaleAttachment)?;
        let snapshot = session
            .snapshot(&connection.attachment)
            .map_err(|_| ProtocolError::StaleAttachment)?;
        if envelope.version == PATCH_PROTOCOL_VERSION
            && (!connection.patch_protocol || !patch_method(&envelope.method))
            || envelope.version == PROTOCOL_VERSION
                && envelope.method == "prepareInstrumentSessionPatch"
        {
            return Err(ProtocolError::UnsupportedVersion);
        }
        let command = parse_command(&envelope.method, envelope.args)?;
        if let Some((_, version, retained)) = connection
            .retained
            .iter()
            .find(|(id, _, _)| *id == request_id)
        {
            return if *version == envelope.version && *retained == command {
                Ok(Admission::Duplicate { request_id })
            } else {
                Err(ProtocolError::ConflictingRequest)
            };
        }
        if request_id <= connection.highest_request {
            return Err(ProtocolError::RetiredRequest);
        }
        command.validate(self.instance_id.get(), attachment_id, snapshot.revision)?;
        let incoming = command.document_bytes();
        let mut retained_bytes = connection
            .retained
            .iter()
            .map(|(_, _, command)| command.document_bytes())
            .sum::<usize>();
        while retained_bytes + incoming > PATCH_REPLAY_DOCUMENT_BYTES_LIMIT {
            let index = connection.retained.iter().position(|(_, _, command)| command.document_bytes() != 0)
                .expect("byte pressure implies a retained document because one document fits the budget");
            let (_, _, retired) = connection
                .retained
                .remove(index)
                .expect("retained document index");
            retained_bytes -= retired.document_bytes();
        }
        if connection.retained.len() == REPLAY_LIMIT {
            connection.retained.pop_front();
        }
        connection
            .retained
            .push_back((request_id, envelope.version, command.clone()));
        connection.highest_request = request_id;
        Ok(Admission::New(AdmittedRequest {
            admission_owner: connection.admission_owner.clone(),
            wire_version: envelope.version,
            request_id,
            attachment_id,
            attachment: connection.attachment.clone(),
            command,
        }))
    }

    fn patch_profile(
        &self,
        session: &InstrumentSession,
        expected_attachment: Option<&str>,
        message: &str,
    ) -> Option<FrameProfile> {
        // Ignore argument values while inspecting the small framing fields. A
        // larger parsing budget comes from retained native authority, never
        // from the browser's version field alone. Full validation follows.
        let header = serde_json::from_str::<FrameProfile>(message).ok()?;
        if header.version != PATCH_PROTOCOL_VERSION
            || !patch_method(&header.method)
            || expected_attachment.is_some_and(|expected| expected != header.attachment_id)
        {
            return None;
        }
        let id = decimal(&header.attachment_id, false).ok()?;
        let connection = self
            .connections
            .iter()
            .find(|connection| connection.id == id && connection.patch_protocol)?;
        session.snapshot(&connection.attachment).ok()?;
        Some(header)
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrameProfile {
    version: u32,
    attachment_id: String,
    request_id: String,
    method: String,
}

fn patch_method(method: &str) -> bool {
    matches!(
        method,
        "getSamplerSession" | "prepareInstrumentSessionPatch"
    )
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope {
    version: u32,
    attachment_id: String,
    request_id: String,
    method: String,
    args: serde_json::Value,
}
fn decimal(text: &str, zero: bool) -> Result<u64, ProtocolError> {
    if text.is_empty()
        || !text.bytes().all(|byte| byte.is_ascii_digit())
        || (text.starts_with('0') && text != "0")
    {
        return Err(ProtocolError::InvalidArguments);
    }
    let value = text
        .parse::<u64>()
        .map_err(|_| ProtocolError::InvalidArguments)?;
    if !zero && value == 0 {
        return Err(ProtocolError::InvalidArguments);
    }
    Ok(value)
}

fn parse_command(method: &str, args: serde_json::Value) -> Result<EditorCommand, ProtocolError> {
    use EditorCommand::*;
    Ok(match method {
        "prepareInstrumentSessionPatch" => {
            let (instance, attachment, request, revision, document): (
                String,
                String,
                String,
                String,
                String,
            ) = arguments(args)?;
            if document.len() > PATCH_DOCUMENT_BYTES_LIMIT {
                return Err(ProtocolError::Oversized);
            }
            if document.is_empty() {
                return Err(ProtocolError::InvalidArguments);
            }
            PrepareInstrumentSessionPatch {
                identity: command_identity(instance, attachment, request, revision)?,
                document: document.into(),
            }
        }
        "negotiateEditorProtocol" => {
            let (version,): (u32,) = arguments(args)?;
            if version != PATCH_PROTOCOL_VERSION {
                return Err(ProtocolError::UnsupportedVersion);
            }
            NegotiateEditorProtocol
        }
        "getSamplerSession" | "getParameterState" | "noteHeartbeat" | "chooseSamplerFiles" => {
            if args != serde_json::json!([]) {
                return Err(ProtocolError::InvalidArguments);
            }
            match method {
                "getSamplerSession" => GetSamplerSession,
                "getParameterState" => GetParameterState,
                "chooseSamplerFiles" => ChooseSamplerFiles,
                _ => NoteHeartbeat,
            }
        }
        "getSamplerSelectionStatus" | "releaseSamplerSelection" => {
            let (selection,): (String,) = arguments(args)?;
            let selection_id = decimal(&selection, false)?;
            if method == "getSamplerSelectionStatus" {
                GetSamplerSelectionStatus { selection_id }
            } else {
                ReleaseSamplerSelection { selection_id }
            }
        }
        "setInstrumentZoneKeyRange" => {
            let (instance, attachment, request, revision, map_id, zone_id, low, high): (
                String,
                String,
                String,
                String,
                String,
                String,
                u8,
                u8,
            ) = arguments(args)?;
            SetZoneKeyRange {
                identity: command_identity(instance, attachment, request, revision)?,
                intent: dandrum_instrument_session::SetZoneKeyRange {
                    map_id,
                    zone_id,
                    key_range: [low, high],
                },
            }
        }
        "undoInstrumentSession" | "redoInstrumentSession" => {
            let (instance, attachment, request, revision): (String, String, String, String) =
                arguments(args)?;
            let identity = command_identity(instance, attachment, request, revision)?;
            if method == "undoInstrumentSession" {
                UndoInstrumentSession { identity }
            } else {
                RedoInstrumentSession { identity }
            }
        }
        "getInstrumentSessionJobStatus" => {
            let (request,): (String,) = arguments(args)?;
            GetInstrumentSessionJobStatus {
                request_id: dandrum_instrument_session::RequestId(decimal(&request, false)?),
            }
        }
        "beginGesture" | "endGesture" => {
            let (parameter_id, generation): (String, u64) = arguments(args)?;
            if method == "beginGesture" {
                BeginGesture {
                    parameter_id,
                    generation,
                }
            } else {
                EndGesture {
                    parameter_id,
                    generation,
                }
            }
        }
        "setParameter" => {
            let (parameter_id, value, generation): (String, f64, u64) = arguments(args)?;
            SetParameter {
                parameter_id,
                value,
                generation,
            }
        }
        "noteOn" => {
            let (note, velocity, generation): (u8, f64, u64) = arguments(args)?;
            NoteOn {
                note,
                velocity,
                generation,
            }
        }
        "noteOff" => {
            let (note, generation): (u8, u64) = arguments(args)?;
            NoteOff { note, generation }
        }
        _ => return Err(ProtocolError::CapabilityUnavailable),
    })
}
fn arguments<T: serde::de::DeserializeOwned>(args: serde_json::Value) -> Result<T, ProtocolError> {
    serde_json::from_value(args).map_err(|_| ProtocolError::InvalidArguments)
}
fn command_identity(
    instance: String,
    attachment: String,
    request: String,
    revision: String,
) -> Result<SessionCommandIdentity, ProtocolError> {
    Ok(SessionCommandIdentity {
        instance_id: decimal(&instance, false)?,
        attachment_id: decimal(&attachment, false)?,
        request_id: dandrum_instrument_session::RequestId(decimal(&request, false)?),
        expected_revision: decimal(&revision, true)?,
    })
}

pub const IDENTIFIER_BYTES_LIMIT: usize = 256;
pub const MAX_SAFE_GENERATION: u64 = 9_007_199_254_740_991;
impl EditorCommand {
    fn document_bytes(&self) -> usize {
        match self {
            Self::PrepareInstrumentSessionPatch { document, .. } => document.len(),
            _ => 0,
        }
    }

    fn validate(
        &self,
        instance_id: u64,
        attachment_id: u64,
        revision: u64,
    ) -> Result<(), ProtocolError> {
        use EditorCommand::*;
        let valid = match self {
            PrepareInstrumentSessionPatch { identity, .. } => {
                validate_identity(identity, instance_id, attachment_id, revision)?;
                true
            }
            SetZoneKeyRange { identity, intent } => {
                validate_identity(identity, instance_id, attachment_id, revision)?;
                identifier(&intent.map_id)
                    && identifier(&intent.zone_id)
                    && intent.key_range[0] <= intent.key_range[1]
                    && intent.key_range[1] <= 127
            }
            UndoInstrumentSession { identity } | RedoInstrumentSession { identity } => {
                validate_identity(identity, instance_id, attachment_id, revision)?;
                true
            }
            BeginGesture {
                parameter_id,
                generation,
            }
            | EndGesture {
                parameter_id,
                generation,
            } => identifier(parameter_id) && *generation <= MAX_SAFE_GENERATION,
            SetParameter {
                parameter_id,
                value,
                generation,
            } => {
                identifier(parameter_id)
                    && (0.0..=1.0).contains(value)
                    && *generation <= MAX_SAFE_GENERATION
            }
            NoteOn {
                note,
                velocity,
                generation,
            } => {
                *note <= 127
                    && *velocity > 0.0
                    && *velocity <= 1.0
                    && *generation <= MAX_SAFE_GENERATION
            }
            NoteOff { note, generation } => *note <= 127 && *generation <= MAX_SAFE_GENERATION,
            _ => true,
        };
        if valid {
            Ok(())
        } else {
            Err(ProtocolError::InvalidArguments)
        }
    }
}
fn identifier(text: &str) -> bool {
    !text.is_empty() && text.len() <= IDENTIFIER_BYTES_LIMIT && !text.chars().any(char::is_control)
}
fn validate_identity(
    identity: &SessionCommandIdentity,
    instance: u64,
    attachment: u64,
    revision: u64,
) -> Result<(), ProtocolError> {
    if identity.instance_id != instance || identity.attachment_id != attachment {
        return Err(ProtocolError::StaleAttachment);
    }
    // A past revision may name an exact retained native action. The session
    // checks that identity before rejecting genuinely new stale work.
    if identity.expected_revision > revision {
        return Err(ProtocolError::StaleRequest);
    }
    Ok(())
}

impl EditorProtocol {
    /// Negotiate only this retained native attachment, after a bounded v1 reply
    /// has been constructed. No session work or authoring state is changed.
    pub fn negotiate(
        &mut self,
        session: &InstrumentSession,
        request: &AdmittedRequest,
    ) -> Result<String, ProtocolError> {
        let context = self.admitted_context(session, request)?;
        if request.command != EditorCommand::NegotiateEditorProtocol {
            return Err(ProtocolError::InvalidArguments);
        }
        let reply = self.encode_reply(session, &context.attachment_id.to_string(), &request.request_id.to_string(), Ok(serde_json::json!({
            "wireVersion":PATCH_PROTOCOL_VERSION,
            "methods":["prepareInstrumentSessionPatch","getSamplerSession"],
            "limits":{"documentBytes":PATCH_DOCUMENT_BYTES_LIMIT,"requestBytes":PATCH_MESSAGE_BYTES_LIMIT,
                "replyBytes":PATCH_OUTBOUND_BYTES_LIMIT,"replayDocumentBytes":PATCH_REPLAY_DOCUMENT_BYTES_LIMIT}
        })))?;
        self.connections
            .iter_mut()
            .find(|connection| connection.id == context.attachment_id)
            .expect("admitted native connection remains live")
            .patch_protocol = true;
        Ok(reply)
    }
    pub fn new(instance_id: std::num::NonZeroU64) -> Self {
        Self {
            instance_id,
            owner: None,
            connections: Vec::new(),
            next_attachment: 1,
        }
    }
    pub fn detach(
        &mut self,
        session: &mut InstrumentSession,
        attachment_id: &str,
    ) -> Result<(), ProtocolError> {
        let id = decimal(attachment_id, false)?;
        let index = self
            .connections
            .iter()
            .position(|connection| connection.id == id)
            .ok_or(ProtocolError::StaleAttachment)?;
        session
            .detach(&self.connections[index].attachment)
            .map_err(|_| ProtocolError::StaleAttachment)?;
        self.connections.swap_remove(index);
        Ok(())
    }
}

pub(crate) struct AdmittedContext {
    pub instance_id: u64,
    pub attachment_id: u64,
    pub attachment: Attachment,
    pub snapshot: SessionSnapshot,
}

pub const OUTBOUND_BYTES_LIMIT: usize = 64 * 1024;
impl EditorProtocol {
    /// Recover native authority from this protocol's retained admission. Public
    /// request fields carry correlation, not permission to read another owner.
    pub(crate) fn admitted_context(
        &self,
        session: &InstrumentSession,
        request: &AdmittedRequest,
    ) -> Result<AdmittedContext, ProtocolError> {
        let connection = self
            .connections
            .iter()
            .find(|connection| {
                connection.id == request.attachment_id
                    && std::sync::Arc::ptr_eq(&connection.admission_owner, &request.admission_owner)
            })
            .ok_or(ProtocolError::StaleAttachment)?;
        if !connection.retained.iter().any(|(id, version, command)| {
            *id == request.request_id
                && *version == request.wire_version
                && *command == request.command
        }) {
            return Err(ProtocolError::StaleRequest);
        }
        let snapshot = session
            .snapshot(&connection.attachment)
            .map_err(|_| ProtocolError::StaleAttachment)?;
        Ok(AdmittedContext {
            instance_id: self.instance_id.get(),
            attachment_id: connection.id,
            attachment: connection.attachment.clone(),
            snapshot,
        })
    }
    pub fn encode_reply(
        &self,
        session: &InstrumentSession,
        attachment_id: &str,
        request_id: &str,
        result: Result<serde_json::Value, ProtocolError>,
    ) -> Result<String, ProtocolError> {
        let (index, _) = self.connection_snapshot(session, attachment_id)?;
        let request = decimal(request_id, false)?;
        let version = self.connections[index]
            .retained
            .iter()
            .find(|(id, _, _)| *id == request)
            .map_or(PROTOCOL_VERSION, |(_, version, _)| *version);
        self.encode_reply_versioned(index, request, version, result)
    }

    /// A rejected request is not retained admission, but a negotiated caller
    /// still needs a correlated reply in its permitted request version.
    pub fn encode_rejection(
        &self,
        session: &InstrumentSession,
        attachment_id: &str,
        request_id: &str,
        message: &str,
        error: ProtocolError,
    ) -> Result<String, ProtocolError> {
        let (index, _) = self.connection_snapshot(session, attachment_id)?;
        let request = decimal(request_id, false)?;
        let version = self
            .patch_profile(session, Some(attachment_id), message)
            .filter(|header| header.request_id == request_id)
            .map_or(PROTOCOL_VERSION, |_| PATCH_PROTOCOL_VERSION);
        self.encode_reply_versioned(index, request, version, Err(error))
    }

    fn encode_reply_versioned(
        &self,
        index: usize,
        request: u64,
        version: u32,
        result: Result<serde_json::Value, ProtocolError>,
    ) -> Result<String, ProtocolError> {
        let mut envelope = serde_json::json!({"version":version,"instanceId":self.instance_id.to_string(),"attachmentId":self.connections[index].id.to_string(),"requestId":request.to_string()});
        match result {
            Ok(value) => {
                envelope["status"] = "accepted".into();
                envelope["result"] = value;
            }
            Err(error) => {
                envelope["status"] = "rejected".into();
                envelope["error"] =
                    serde_json::to_value(error).map_err(|_| ProtocolError::Malformed)?;
            }
        }
        encode_bounded(
            &envelope,
            if version == PATCH_PROTOCOL_VERSION {
                PATCH_OUTBOUND_BYTES_LIMIT
            } else {
                OUTBOUND_BYTES_LIMIT
            },
        )
    }
    pub fn encode_event(
        &mut self,
        session: &InstrumentSession,
        attachment_id: &str,
        event: &str,
        payload: serde_json::Value,
    ) -> Result<String, ProtocolError> {
        let (index, _) = self.connection_snapshot(session, attachment_id)?;
        if !matches!(event, "parameterStateChanged" | "instrumentSessionChanged") {
            return Err(ProtocolError::CapabilityUnavailable);
        }
        let connection = &mut self.connections[index];
        let next = connection
            .next_event
            .checked_add(1)
            .ok_or(ProtocolError::IdentityExhausted)?;
        let envelope = serde_json::json!({"version":PROTOCOL_VERSION,"instanceId":self.instance_id.to_string(),"attachmentId":connection.id.to_string(),"sequence":connection.next_event.to_string(),"event":event,"payload":payload});
        let encoded = encode_bounded(&envelope, OUTBOUND_BYTES_LIMIT)?;
        connection.next_event = next;
        Ok(encoded)
    }
    fn connection_snapshot(
        &self,
        session: &InstrumentSession,
        attachment_id: &str,
    ) -> Result<(usize, SessionSnapshot), ProtocolError> {
        let id = decimal(attachment_id, false)?;
        let index = self
            .connections
            .iter()
            .position(|connection| connection.id == id)
            .ok_or(ProtocolError::StaleAttachment)?;
        let snapshot = session
            .snapshot(&self.connections[index].attachment)
            .map_err(|_| ProtocolError::StaleAttachment)?;
        Ok((index, snapshot))
    }
}

fn encode_bounded(value: &serde_json::Value, limit: usize) -> Result<String, ProtocolError> {
    let mut buffer = vec![0; limit];
    let length = {
        let mut cursor = std::io::Cursor::new(buffer.as_mut_slice());
        serde_json::to_writer(&mut cursor, value).map_err(|_| ProtocolError::Oversized)?;
        cursor.position() as usize
    };
    buffer.truncate(length);
    Ok(String::from_utf8(buffer).expect("JSON writer emits UTF-8"))
}
