use crate::{
    ACTIVE_REQUEST_LIMIT, ATTACHMENT_LIMIT, ActivationHost, ActivationLease,
    CONNECTION_CONTRACT_VERSION, HISTORY_LIMIT, InstrumentSession, MappingAdmission, MappingJob,
    MappingRequest, OUTCOME_LIMIT, PreparedMappingJob, Publication, RequestFailure, RequestId,
    SessionError, SessionSnapshot, SetZoneKeyRange,
};
use dandrum_engine::{
    ffi::{
        DandrumKernelBusDeclaration, DandrumKernelInstrument, PlanarBusDeclaration,
        kernel_instrument_from_prepared_yaml, planar_host_buses,
    },
    graph::PortDirection,
    patch::RenderSettings,
    preparation::PreparationContext,
};
use std::{
    ffi::{CStr, c_char, c_void},
    path::PathBuf,
    ptr,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

struct NativeConnection {
    id: u64,
    attachment: crate::Attachment,
}

pub struct DandrumInstrumentSession {
    session: InstrumentSession,
    instance_id: u64,
    next_attachment_id: u64,
    connections: Vec<NativeConnection>,
    sample_rate_hz: u32,
    max_block_size: usize,
    buses: Vec<PlanarBusDeclaration>,
}

enum JobState {
    Admitted(MappingJob),
    Prepared(Box<PreparedMappingJob>),
}

pub struct DandrumInstrumentSessionJob {
    state: Option<JobState>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DandrumInstrumentSessionSnapshot {
    pub contract_version: u32,
    pub instance_id: u64,
    pub attachment_id: u64,
    pub revision: u64,
    pub next_request_id: u64,
    pub undo_count: usize,
    pub redo_count: usize,
    pub attachment_limit: usize,
    pub history_limit: usize,
    pub outcome_limit: usize,
    pub active_request_limit: usize,
    pub yaml_size: usize,
}

pub type ActivateCallback = unsafe extern "C" fn(
    context: *mut c_void,
    candidate: *mut DandrumKernelInstrument,
    yaml: *const u8,
    yaml_size: usize,
    revision: u64,
    undo_count: usize,
    redo_count: usize,
) -> *mut c_void;
pub type PublishCallback = unsafe extern "C" fn(
    context: *mut c_void,
    lease: *mut c_void,
    revision: u64,
    undo_count: usize,
    redo_count: usize,
);
pub type AbortCallback = unsafe extern "C" fn(context: *mut c_void, lease: *mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DandrumInstrumentSessionHostCallbacks {
    pub activate: Option<ActivateCallback>,
    pub publish: Option<PublishCallback>,
    pub abort: Option<AbortCallback>,
}

#[derive(Clone, Copy)]
struct NativeCallbacks {
    activate: ActivateCallback,
    publish: PublishCallback,
    abort: AbortCallback,
}

fn required_callbacks(
    callbacks: &DandrumInstrumentSessionHostCallbacks,
) -> Option<NativeCallbacks> {
    Some(NativeCallbacks {
        activate: callbacks.activate?,
        publish: callbacks.publish?,
        abort: callbacks.abort?,
    })
}

struct NativeHost<'a> {
    callbacks: NativeCallbacks,
    context: *mut c_void,
    sample_rate_hz: u32,
    max_block_size: usize,
    buses: &'a [PlanarBusDeclaration],
}

struct NativeLease {
    callbacks: NativeCallbacks,
    context: *mut c_void,
    lease: Option<*mut c_void>,
}

struct NeverHost;
struct NeverLease;
struct RejectingHost(String);

impl ActivationHost for NeverHost {
    type Error = String;
    type Lease<'a> = NeverLease;

    fn activate<'a>(
        &'a mut self,
        _: &dandrum_engine::preparation::PreparedKernelInstrument,
        _: &SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        unreachable!("explicitly failed work never reaches activation")
    }
}

impl ActivationLease for NeverLease {
    fn publish(self, _: Publication<'_>) {
        unreachable!("explicitly failed work never publishes")
    }
}

impl ActivationHost for RejectingHost {
    type Error = String;
    type Lease<'a> = NeverLease;

    fn activate<'a>(
        &'a mut self,
        _: &dandrum_engine::preparation::PreparedKernelInstrument,
        _: &SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        Err(self.0.clone())
    }
}

impl ActivationHost for NativeHost<'_> {
    type Error = String;
    type Lease<'a>
        = NativeLease
    where
        Self: 'a;

    fn activate<'a>(
        &'a mut self,
        prepared: &dandrum_engine::preparation::PreparedKernelInstrument,
        next: &SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        let candidate = kernel_instrument_from_prepared_yaml(
            &next.yaml,
            prepared.clone(),
            self.sample_rate_hz,
            self.max_block_size,
            self.buses,
        )
        .ok_or_else(|| "failed to instantiate prepared session engine".to_string())?;
        let candidate = Box::into_raw(candidate);
        let activate = self.callbacks.activate;
        let lease = unsafe {
            activate(
                self.context,
                candidate,
                next.yaml.as_ptr(),
                next.yaml.len(),
                next.revision,
                next.undo_len,
                next.redo_len,
            )
        };
        if lease.is_null() {
            unsafe { drop(Box::from_raw(candidate)) };
            return Err("native activation rejected the prepared session engine".into());
        }
        Ok(NativeLease {
            callbacks: self.callbacks,
            context: self.context,
            lease: Some(lease),
        })
    }
}

impl ActivationLease for NativeLease {
    fn publish(mut self, accepted: Publication<'_>) {
        let lease = self.lease.take().expect("native lease publishes once");
        let publish = self.callbacks.publish;
        unsafe {
            publish(
                self.context,
                lease,
                accepted.revision,
                accepted.undo_len,
                accepted.redo_len,
            )
        };
    }
}

impl Drop for NativeLease {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            unsafe { (self.callbacks.abort)(self.context, lease) };
        }
    }
}

fn bytes<'a>(pointer: *const u8, size: usize) -> Option<&'a str> {
    if size > 0 && pointer.is_null() {
        return None;
    }
    std::str::from_utf8(if size == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(pointer, size) }
    })
    .ok()
}

fn c_text(pointer: *const c_char) -> Option<String> {
    (!pointer.is_null())
        .then(|| {
            unsafe { CStr::from_ptr(pointer) }
                .to_str()
                .ok()
                .map(str::to_owned)
        })
        .flatten()
}

unsafe fn bus_declarations(
    declarations: *const DandrumKernelBusDeclaration,
    count: usize,
) -> Option<Vec<PlanarBusDeclaration>> {
    if count > 0 && declarations.is_null() {
        return None;
    }
    let declarations = if count == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(declarations, count) }
    };
    declarations
        .iter()
        .map(|declaration| {
            Some(PlanarBusDeclaration {
                name: c_text(declaration.name)?,
                direction: match declaration.direction {
                    1 => PortDirection::Input,
                    2 => PortDirection::Output,
                    _ => return None,
                },
                channel_count: declaration.channel_count,
            })
        })
        .collect()
}

fn fill_snapshot(
    session: &DandrumInstrumentSession,
    attachment_id: u64,
    value: &SessionSnapshot,
    output: *mut DandrumInstrumentSessionSnapshot,
) -> bool {
    let Some(output) = (unsafe { output.as_mut() }) else {
        return false;
    };
    *output = DandrumInstrumentSessionSnapshot {
        contract_version: CONNECTION_CONTRACT_VERSION,
        instance_id: session.instance_id,
        attachment_id,
        revision: value.revision,
        next_request_id: match session.session.next_request_id() {
            Ok(id) => id.0,
            Err(_) => return false,
        },
        undo_count: value.undo_len,
        redo_count: value.redo_len,
        attachment_limit: ATTACHMENT_LIMIT,
        history_limit: HISTORY_LIMIT,
        outcome_limit: OUTCOME_LIMIT,
        active_request_limit: ACTIVE_REQUEST_LIMIT,
        yaml_size: value.yaml.len(),
    };
    true
}

fn allocate_session_id() -> Option<u64> {
    NEXT_SESSION_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .ok()
}

fn connection(
    session: &DandrumInstrumentSession,
    instance_id: u64,
    attachment_id: u64,
) -> Option<crate::Attachment> {
    (session.instance_id == instance_id)
        .then(|| {
            session
                .connections
                .iter()
                .find(|connection| connection.id == attachment_id)
                .map(|connection| connection.attachment.clone())
        })
        .flatten()
}

fn write_error(message: &str, output: *mut c_char, capacity: usize) {
    if output.is_null() || capacity == 0 {
        return;
    }
    let bytes = message.as_bytes();
    let count = bytes.len().min(capacity - 1);
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), output.cast(), count);
        *output.add(count) = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_create(
    yaml: *const u8,
    yaml_size: usize,
    document_root: *const c_char,
    sample_rate_hz: u32,
    max_block_size: usize,
    declarations: *const DandrumKernelBusDeclaration,
    declaration_count: usize,
) -> *mut DandrumInstrumentSession {
    if sample_rate_hz == 0 || max_block_size == 0 {
        return ptr::null_mut();
    }
    let Some(yaml) = bytes(yaml, yaml_size) else {
        return ptr::null_mut();
    };
    let Some(document_root) = c_text(document_root).map(PathBuf::from) else {
        return ptr::null_mut();
    };
    let Some(buses) = (unsafe { bus_declarations(declarations, declaration_count) }) else {
        return ptr::null_mut();
    };
    let Some(host_buses) = planar_host_buses(&buses) else {
        return ptr::null_mut();
    };
    let settings = RenderSettings {
        sample_rate_hz,
        block_size_frames: max_block_size.try_into().unwrap_or(u32::MAX),
        duration_frames: max_block_size as u64,
    };
    if settings.block_size_frames as usize != max_block_size {
        return ptr::null_mut();
    }
    let mut context = PreparationContext::new(document_root, sample_rate_hz);
    if let Ok(roots) = dandrum_engine::module_library::default_host_macro_roots() {
        context = context.with_macro_roots(roots);
    }
    let Ok(session) = InstrumentSession::new_for_planar_host(yaml, context, settings, host_buses)
    else {
        return ptr::null_mut();
    };
    let Some(instance_id) = allocate_session_id() else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(DandrumInstrumentSession {
        session,
        instance_id,
        next_attachment_id: 1,
        connections: Vec::with_capacity(ATTACHMENT_LIMIT),
        sample_rate_hz,
        max_block_size,
        buses,
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_destroy(
    session: *mut DandrumInstrumentSession,
) {
    if !session.is_null() {
        drop(unsafe { Box::from_raw(session) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_snapshot(
    session: *const DandrumInstrumentSession,
    output: *mut DandrumInstrumentSessionSnapshot,
) -> bool {
    let Some(session) = (unsafe { session.as_ref() }) else {
        return false;
    };
    fill_snapshot(session, 0, &session.session.owner_snapshot(), output)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_connect(
    session: *mut DandrumInstrumentSession,
    output: *mut DandrumInstrumentSessionSnapshot,
) -> bool {
    let Some(session) = (unsafe { session.as_mut() }) else {
        return false;
    };
    let Some(next_attachment_id) = session.next_attachment_id.checked_add(1) else {
        return false;
    };
    let Ok((attachment, snapshot)) = session.session.attach() else {
        return false;
    };
    let attachment_id = session.next_attachment_id;
    session.next_attachment_id = next_attachment_id;
    session.connections.push(NativeConnection {
        id: attachment_id,
        attachment,
    });
    if fill_snapshot(session, attachment_id, &snapshot, output) {
        true
    } else {
        let connection = session
            .connections
            .pop()
            .expect("connection was just appended");
        let _ = session.session.detach(&connection.attachment);
        false
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_disconnect(
    session: *mut DandrumInstrumentSession,
    instance_id: u64,
    attachment_id: u64,
) -> bool {
    let Some(session) = (unsafe { session.as_mut() }) else {
        return false;
    };
    if session.instance_id != instance_id {
        return false;
    }
    let Some(index) = session
        .connections
        .iter()
        .position(|connection| connection.id == attachment_id)
    else {
        return false;
    };
    let connection = session.connections.swap_remove(index);
    session.session.detach(&connection.attachment).is_ok()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_connection_snapshot(
    session: *const DandrumInstrumentSession,
    instance_id: u64,
    attachment_id: u64,
    output: *mut DandrumInstrumentSessionSnapshot,
) -> bool {
    let Some(session) = (unsafe { session.as_ref() }) else {
        return false;
    };
    let Some(attachment) = connection(session, instance_id, attachment_id) else {
        return false;
    };
    session
        .session
        .snapshot(&attachment)
        .is_ok_and(|snapshot| fill_snapshot(session, attachment_id, &snapshot, output))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_copy_yaml(
    session: *const DandrumInstrumentSession,
    output: *mut u8,
    capacity: usize,
) -> bool {
    let Some(session) = (unsafe { session.as_ref() }) else {
        return false;
    };
    let snapshot = session.session.owner_snapshot();
    if capacity != snapshot.yaml.len() || (capacity > 0 && output.is_null()) {
        return false;
    }
    if capacity > 0 {
        unsafe { ptr::copy_nonoverlapping(snapshot.yaml.as_ptr(), output, capacity) };
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_admit_mapping(
    session: *mut DandrumInstrumentSession,
    request_id: u64,
    expected_revision: u64,
    map_id: *const u8,
    map_id_size: usize,
    zone_id: *const u8,
    zone_id_size: usize,
    key_low: u8,
    key_high: u8,
    error: *mut c_char,
    error_capacity: usize,
) -> *mut DandrumInstrumentSessionJob {
    let Some(session) = (unsafe { session.as_mut() }) else {
        write_error("invalid instrument session", error, error_capacity);
        return ptr::null_mut();
    };
    let (Some(map_id), Some(zone_id)) = (bytes(map_id, map_id_size), bytes(zone_id, zone_id_size))
    else {
        write_error("invalid UTF-8 mapping identity", error, error_capacity);
        return ptr::null_mut();
    };
    let snapshot = session.session.owner_snapshot();
    let request = MappingRequest {
        instance: snapshot.instance,
        request_id: RequestId(request_id),
        expected_revision,
        intent: SetZoneKeyRange {
            map_id: map_id.into(),
            zone_id: zone_id.into(),
            key_range: [key_low, key_high],
        },
    };
    match session.session.admit_mapping_owned(request) {
        Ok(MappingAdmission::Admitted(job)) => {
            Box::into_raw(Box::new(DandrumInstrumentSessionJob {
                state: Some(JobState::Admitted(job)),
            }))
        }
        Ok(MappingAdmission::Pending) => {
            write_error("mapping request is already pending", error, error_capacity);
            ptr::null_mut()
        }
        Ok(MappingAdmission::Completed(_)) => {
            write_error("mapping request already completed", error, error_capacity);
            ptr::null_mut()
        }
        Err(reason) => {
            write_error(&format!("{reason:?}"), error, error_capacity);
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_admit_mapping_for_connection(
    session: *mut DandrumInstrumentSession,
    instance_id: u64,
    attachment_id: u64,
    request_id: u64,
    expected_revision: u64,
    map_id: *const u8,
    map_id_size: usize,
    zone_id: *const u8,
    zone_id_size: usize,
    key_low: u8,
    key_high: u8,
    error: *mut c_char,
    error_capacity: usize,
) -> *mut DandrumInstrumentSessionJob {
    let Some(session) = (unsafe { session.as_mut() }) else {
        write_error("invalid instrument session", error, error_capacity);
        return ptr::null_mut();
    };
    let Some(attachment) = connection(session, instance_id, attachment_id) else {
        write_error(
            "invalid instrument session connection",
            error,
            error_capacity,
        );
        return ptr::null_mut();
    };
    let (Some(map_id), Some(zone_id)) = (bytes(map_id, map_id_size), bytes(zone_id, zone_id_size))
    else {
        write_error("invalid UTF-8 mapping identity", error, error_capacity);
        return ptr::null_mut();
    };
    let snapshot = match session.session.snapshot(&attachment) {
        Ok(snapshot) => snapshot,
        Err(reason) => {
            write_error(&format!("{reason:?}"), error, error_capacity);
            return ptr::null_mut();
        }
    };
    let request = MappingRequest {
        instance: snapshot.instance,
        request_id: RequestId(request_id),
        expected_revision,
        intent: SetZoneKeyRange {
            map_id: map_id.into(),
            zone_id: zone_id.into(),
            key_range: [key_low, key_high],
        },
    };
    match session.session.admit_mapping(&attachment, request) {
        Ok(MappingAdmission::Admitted(job)) => {
            Box::into_raw(Box::new(DandrumInstrumentSessionJob {
                state: Some(JobState::Admitted(job)),
            }))
        }
        Ok(MappingAdmission::Pending) => {
            write_error("mapping request is already pending", error, error_capacity);
            ptr::null_mut()
        }
        Ok(MappingAdmission::Completed(_)) => {
            write_error("mapping request already completed", error, error_capacity);
            ptr::null_mut()
        }
        Err(reason) => {
            write_error(&format!("{reason:?}"), error, error_capacity);
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_admit_undo_for_connection(
    session: *mut DandrumInstrumentSession,
    instance_id: u64,
    attachment_id: u64,
    request_id: u64,
    expected_revision: u64,
    error: *mut c_char,
    error_capacity: usize,
) -> *mut DandrumInstrumentSessionJob {
    let Some(session) = (unsafe { session.as_mut() }) else {
        write_error("invalid instrument session", error, error_capacity);
        return ptr::null_mut();
    };
    let Some(attachment) = connection(session, instance_id, attachment_id) else {
        write_error(
            "invalid instrument session connection",
            error,
            error_capacity,
        );
        return ptr::null_mut();
    };
    match session
        .session
        .admit_undo(&attachment, RequestId(request_id), expected_revision)
    {
        Ok(MappingAdmission::Admitted(job)) => {
            Box::into_raw(Box::new(DandrumInstrumentSessionJob {
                state: Some(JobState::Admitted(job)),
            }))
        }
        Ok(MappingAdmission::Pending) => {
            write_error("undo request is already pending", error, error_capacity);
            ptr::null_mut()
        }
        Ok(MappingAdmission::Completed(_)) => {
            write_error("undo request already completed", error, error_capacity);
            ptr::null_mut()
        }
        Err(reason) => {
            write_error(&format!("{reason:?}"), error, error_capacity);
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_prepare_job(
    job: *mut DandrumInstrumentSessionJob,
) -> bool {
    let Some(job) = (unsafe { job.as_mut() }) else {
        return false;
    };
    match job.state.take() {
        Some(JobState::Admitted(admitted)) => {
            job.state = Some(JobState::Prepared(Box::new(admitted.prepare())));
            true
        }
        state => {
            job.state = state;
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_fail_job(
    session: *mut DandrumInstrumentSession,
    job: *mut DandrumInstrumentSessionJob,
    message: *const u8,
    message_size: usize,
) -> bool {
    let Some(session) = (unsafe { session.as_mut() }) else {
        return false;
    };
    let Some(message) = bytes(message, message_size) else {
        return false;
    };
    if job.is_null() {
        return false;
    }
    let mut job = unsafe { Box::from_raw(job) };
    let Some(state) = job.state.take() else {
        return false;
    };
    let failure = SessionError::PreparationFailed(message.into());
    let completion = match state {
        JobState::Admitted(job) => job.fail(failure),
        JobState::Prepared(job) => (*job).fail(failure),
    };
    session
        .session
        .complete_mapping(completion, &mut NeverHost)
        .is_ok()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_complete_job(
    session: *mut DandrumInstrumentSession,
    job: *mut DandrumInstrumentSessionJob,
    callbacks: *const DandrumInstrumentSessionHostCallbacks,
    context: *mut c_void,
    revision: *mut u64,
    undo_count: *mut usize,
    redo_count: *mut usize,
    error: *mut c_char,
    error_capacity: usize,
) -> bool {
    let Some(session) = (unsafe { session.as_mut() }) else {
        write_error("invalid instrument session", error, error_capacity);
        return false;
    };
    let Some(callbacks) = (unsafe { callbacks.as_ref() }) else {
        write_error("invalid native activation callbacks", error, error_capacity);
        return false;
    };
    if revision.is_null() || undo_count.is_null() || redo_count.is_null() || job.is_null() {
        write_error("invalid session completion output", error, error_capacity);
        return false;
    }
    let mut job = unsafe { Box::from_raw(job) };
    let Some(JobState::Prepared(completion)) = job.state.take() else {
        write_error("session job was not prepared", error, error_capacity);
        return false;
    };
    let Some(callbacks) = required_callbacks(callbacks) else {
        let message = "native activation callback table is incomplete";
        let _ = session
            .session
            .complete_mapping(*completion, &mut RejectingHost(message.into()));
        write_error(message, error, error_capacity);
        return false;
    };
    let mut host = NativeHost {
        callbacks,
        context,
        sample_rate_hz: session.sample_rate_hz,
        max_block_size: session.max_block_size,
        buses: &session.buses,
    };
    let outcome = match session.session.complete_mapping(*completion, &mut host) {
        Ok(outcome) => outcome,
        Err(route) => {
            write_error(&format!("{:?}", route.reason()), error, error_capacity);
            return false;
        }
    };
    match &outcome.result {
        Ok(snapshot) => {
            unsafe {
                *revision = snapshot.revision;
                *undo_count = snapshot.undo_len;
                *redo_count = snapshot.redo_len;
            }
            true
        }
        Err(failure) => {
            let message = match failure {
                RequestFailure::Session(reason) => format!("{reason:?}"),
                RequestFailure::Activation(reason) => reason.clone(),
            };
            write_error(&message, error, error_capacity);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_undo(
    session: *mut DandrumInstrumentSession,
    instance_id: u64,
    attachment_id: u64,
    request_id: u64,
    expected_revision: u64,
    callbacks: *const DandrumInstrumentSessionHostCallbacks,
    context: *mut c_void,
    revision: *mut u64,
    undo_count: *mut usize,
    redo_count: *mut usize,
    error: *mut c_char,
    error_capacity: usize,
) -> bool {
    let Some(session) = (unsafe { session.as_mut() }) else {
        write_error("invalid instrument session", error, error_capacity);
        return false;
    };
    let Some(attachment) = connection(session, instance_id, attachment_id) else {
        write_error(
            "invalid instrument session connection",
            error,
            error_capacity,
        );
        return false;
    };
    let Some(callbacks) = (unsafe { callbacks.as_ref() }).and_then(required_callbacks) else {
        write_error("invalid native activation callbacks", error, error_capacity);
        return false;
    };
    if revision.is_null() || undo_count.is_null() || redo_count.is_null() {
        write_error("invalid session undo output", error, error_capacity);
        return false;
    }
    if let Err(reason) = session.session.reserve_control_request(
        &attachment,
        RequestId(request_id),
        expected_revision,
    ) {
        write_error(&format!("{reason:?}"), error, error_capacity);
        return false;
    }
    let mut host = NativeHost {
        callbacks,
        context,
        sample_rate_hz: session.sample_rate_hz,
        max_block_size: session.max_block_size,
        buses: &session.buses,
    };
    match session.session.undo(&attachment, &mut host) {
        Ok(snapshot) => {
            unsafe {
                *revision = snapshot.revision;
                *undo_count = snapshot.undo_len;
                *redo_count = snapshot.redo_len;
            }
            true
        }
        Err(reason) => {
            write_error(&format!("{reason:?}"), error, error_capacity);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_reconfigure(
    session: *mut DandrumInstrumentSession,
    sample_rate_hz: u32,
    max_block_size: usize,
    callbacks: *const DandrumInstrumentSessionHostCallbacks,
    context: *mut c_void,
    error: *mut c_char,
    error_capacity: usize,
) -> bool {
    let Some(session) = (unsafe { session.as_mut() }) else {
        write_error("invalid instrument session", error, error_capacity);
        return false;
    };
    let Some(callbacks) = (unsafe { callbacks.as_ref() }) else {
        write_error("invalid native activation callbacks", error, error_capacity);
        return false;
    };
    let Some(callbacks) = required_callbacks(callbacks) else {
        write_error(
            "native activation callback table is incomplete",
            error,
            error_capacity,
        );
        return false;
    };
    if sample_rate_hz == 0 || max_block_size == 0 {
        write_error("invalid native render configuration", error, error_capacity);
        return false;
    }
    let Some(host_buses) = planar_host_buses(&session.buses) else {
        write_error(
            "invalid retained native bus configuration",
            error,
            error_capacity,
        );
        return false;
    };
    let settings = RenderSettings {
        sample_rate_hz,
        block_size_frames: max_block_size.try_into().unwrap_or(u32::MAX),
        duration_frames: max_block_size as u64,
    };
    if settings.block_size_frames as usize != max_block_size {
        write_error(
            "native block size exceeds the engine limit",
            error,
            error_capacity,
        );
        return false;
    }
    let candidate = match session
        .session
        .prepare_render_configuration_owned(settings, host_buses)
    {
        Ok(candidate) => candidate,
        Err(reason) => {
            write_error(&format!("{reason:?}"), error, error_capacity);
            return false;
        }
    };
    let mut host = NativeHost {
        callbacks,
        context,
        sample_rate_hz,
        max_block_size,
        buses: &session.buses,
    };
    match session
        .session
        .commit_render_configuration(candidate, &mut host)
    {
        Ok(_) => {
            session.sample_rate_hz = sample_rate_hz;
            session.max_block_size = max_block_size;
            true
        }
        Err(reason) => {
            write_error(&format!("{reason:?}"), error, error_capacity);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_instrument_session_job_destroy(
    job: *mut DandrumInstrumentSessionJob,
) {
    if !job.is_null() {
        drop(unsafe { Box::from_raw(job) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    struct TestHost {
        reject: bool,
        activations: usize,
        publications: usize,
        revision: u64,
        undo_count: usize,
        redo_count: usize,
    }

    struct TestLease {
        candidate: *mut DandrumKernelInstrument,
    }

    impl Drop for TestLease {
        fn drop(&mut self) {
            unsafe { drop(Box::from_raw(self.candidate)) };
        }
    }

    unsafe extern "C" fn activate(
        context: *mut c_void,
        candidate: *mut DandrumKernelInstrument,
        _: *const u8,
        _: usize,
        _: u64,
        _: usize,
        _: usize,
    ) -> *mut c_void {
        let host = unsafe { &mut *context.cast::<TestHost>() };
        host.activations += 1;
        if host.reject {
            return ptr::null_mut();
        }
        Box::into_raw(Box::new(TestLease { candidate })).cast()
    }

    unsafe extern "C" fn publish(
        context: *mut c_void,
        lease: *mut c_void,
        revision: u64,
        undo_count: usize,
        redo_count: usize,
    ) {
        let host = unsafe { &mut *context.cast::<TestHost>() };
        host.publications += 1;
        host.revision = revision;
        host.undo_count = undo_count;
        host.redo_count = redo_count;
        drop(unsafe { Box::from_raw(lease.cast::<TestLease>()) });
    }

    unsafe extern "C" fn abort(_: *mut c_void, lease: *mut c_void) {
        drop(unsafe { Box::from_raw(lease.cast::<TestLease>()) });
    }

    fn fixture() -> (
        String,
        CString,
        Vec<CString>,
        Vec<DandrumKernelBusDeclaration>,
    ) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/patches/advanced-drum-kit.yaml");
        let yaml = std::fs::read_to_string(&path).unwrap();
        let patch = dandrum_engine::kernel::document::load_kernel_patch_str(&yaml).unwrap();
        let mut names = vec![CString::new("master").unwrap()];
        names.extend(
            patch
                .preset_surface()
                .parameters()
                .iter()
                .map(|parameter| CString::new(parameter.port_name()).unwrap()),
        );
        let declarations = names
            .iter()
            .enumerate()
            .map(|(index, name)| DandrumKernelBusDeclaration {
                name: name.as_ptr(),
                direction: if index == 0 { 2 } else { 1 },
                channel_count: if index == 0 { 2 } else { 1 },
            })
            .collect();
        (
            yaml,
            CString::new(path.parent().unwrap().to_str().unwrap()).unwrap(),
            names,
            declarations,
        )
    }

    #[test]
    fn native_facade_maps_fails_reconfigures_and_rolls_back_activation() {
        let (yaml, root, _names, declarations) = fixture();
        let session = unsafe {
            dandrum_instrument_session_create(
                yaml.as_ptr(),
                yaml.len(),
                root.as_ptr(),
                48_000,
                64,
                declarations.as_ptr(),
                declarations.len(),
            )
        };
        assert!(!session.is_null());
        let mut snapshot = DandrumInstrumentSessionSnapshot {
            contract_version: 0,
            instance_id: 0,
            attachment_id: u64::MAX,
            revision: u64::MAX,
            next_request_id: 0,
            undo_count: usize::MAX,
            redo_count: usize::MAX,
            attachment_limit: 0,
            history_limit: 0,
            outcome_limit: 0,
            active_request_limit: 0,
            yaml_size: 0,
        };
        assert!(unsafe { dandrum_instrument_session_snapshot(session, &mut snapshot) });
        assert_eq!(snapshot.contract_version, 1);
        assert_ne!(snapshot.instance_id, 0);
        assert_eq!(snapshot.attachment_id, 0);
        assert_eq!(snapshot.next_request_id, 1);
        assert_eq!(
            (
                snapshot.attachment_limit,
                snapshot.history_limit,
                snapshot.outcome_limit,
                snapshot.active_request_limit,
            ),
            (16, 100, 128, 1)
        );
        assert_eq!(
            (snapshot.revision, snapshot.undo_count, snapshot.redo_count),
            (0, 0, 0)
        );
        let mut connection = snapshot;
        assert!(unsafe { dandrum_instrument_session_connect(session, &mut connection) });
        assert_eq!(connection.instance_id, snapshot.instance_id);
        assert_ne!(connection.attachment_id, 0);
        let attachment_id = connection.attachment_id;
        assert!(unsafe {
            dandrum_instrument_session_connection_snapshot(
                session,
                connection.instance_id,
                attachment_id,
                &mut connection,
            )
        });
        assert_eq!(connection.attachment_id, attachment_id);
        let mut copied = vec![0; snapshot.yaml_size];
        assert!(unsafe {
            dandrum_instrument_session_copy_yaml(session, copied.as_mut_ptr(), copied.len())
        });
        assert_eq!(copied, yaml.as_bytes());

        let callbacks = DandrumInstrumentSessionHostCallbacks {
            activate: Some(activate),
            publish: Some(publish),
            abort: Some(abort),
        };
        let mut host = TestHost {
            reject: false,
            activations: 0,
            publications: 0,
            revision: 0,
            undo_count: 0,
            redo_count: 0,
        };
        let mut error = [0; 256];
        let job = unsafe {
            dandrum_instrument_session_admit_mapping_for_connection(
                session,
                connection.instance_id,
                attachment_id,
                1,
                0,
                b"kit".as_ptr(),
                3,
                b"kick".as_ptr(),
                4,
                60,
                60,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert!(!job.is_null());
        assert!(unsafe {
            dandrum_instrument_session_disconnect(session, connection.instance_id, attachment_id)
        });
        assert!(!unsafe {
            dandrum_instrument_session_connection_snapshot(
                session,
                connection.instance_id,
                attachment_id,
                &mut connection,
            )
        });
        assert!(unsafe { dandrum_instrument_session_prepare_job(job) });
        assert!(!unsafe { dandrum_instrument_session_prepare_job(job) });
        let mut revision = 0;
        let mut undo_count = 0;
        let mut redo_count = 0;
        assert!(unsafe {
            dandrum_instrument_session_complete_job(
                session,
                job,
                &callbacks,
                (&mut host as *mut TestHost).cast(),
                &mut revision,
                &mut undo_count,
                &mut redo_count,
                error.as_mut_ptr(),
                error.len(),
            )
        });
        assert_eq!((revision, undo_count, redo_count), (1, 1, 0));
        assert_eq!((host.activations, host.publications), (1, 1));
        assert_eq!((host.revision, host.undo_count, host.redo_count), (1, 1, 0));

        let failed = unsafe {
            dandrum_instrument_session_admit_mapping(
                session,
                2,
                1,
                b"kit".as_ptr(),
                3,
                b"kick".as_ptr(),
                4,
                61,
                61,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert!(!failed.is_null());
        assert!(unsafe {
            dandrum_instrument_session_fail_job(
                session,
                failed,
                b"cancelled".as_ptr(),
                b"cancelled".len(),
            )
        });

        let incomplete = DandrumInstrumentSessionHostCallbacks {
            activate: Some(activate),
            publish: Some(publish),
            abort: None,
        };
        let incomplete_job = unsafe {
            dandrum_instrument_session_admit_mapping(
                session,
                3,
                1,
                b"kit".as_ptr(),
                3,
                b"kick".as_ptr(),
                4,
                62,
                62,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert!(!incomplete_job.is_null());
        assert!(unsafe { dandrum_instrument_session_prepare_job(incomplete_job) });
        assert!(!unsafe {
            dandrum_instrument_session_complete_job(
                session,
                incomplete_job,
                &incomplete,
                (&mut host as *mut TestHost).cast(),
                &mut revision,
                &mut undo_count,
                &mut redo_count,
                error.as_mut_ptr(),
                error.len(),
            )
        });
        assert!(unsafe { dandrum_instrument_session_snapshot(session, &mut snapshot) });
        assert_eq!(
            (snapshot.revision, snapshot.undo_count, snapshot.redo_count),
            (1, 1, 0)
        );
        error.fill(0);
        let retained_incomplete = unsafe {
            dandrum_instrument_session_admit_mapping(
                session,
                3,
                1,
                b"kit".as_ptr(),
                3,
                b"kick".as_ptr(),
                4,
                62,
                62,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert!(retained_incomplete.is_null());
        assert_eq!(
            unsafe { CStr::from_ptr(error.as_ptr()) }.to_str().unwrap(),
            "mapping request already completed"
        );
        assert_eq!((host.activations, host.publications), (1, 1));
        assert!(!unsafe {
            dandrum_instrument_session_reconfigure(
                session,
                44_100,
                128,
                &incomplete,
                (&mut host as *mut TestHost).cast(),
                error.as_mut_ptr(),
                error.len(),
            )
        });

        assert!(unsafe {
            dandrum_instrument_session_reconfigure(
                session,
                44_100,
                128,
                &callbacks,
                (&mut host as *mut TestHost).cast(),
                error.as_mut_ptr(),
                error.len(),
            )
        });
        assert_eq!((host.activations, host.publications), (2, 2));
        assert_eq!((host.revision, host.undo_count, host.redo_count), (1, 1, 0));

        host.reject = true;
        let rejected = unsafe {
            dandrum_instrument_session_admit_mapping(
                session,
                4,
                1,
                b"kit".as_ptr(),
                3,
                b"kick".as_ptr(),
                4,
                62,
                62,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert!(!rejected.is_null());
        assert!(unsafe { dandrum_instrument_session_prepare_job(rejected) });
        assert!(!unsafe {
            dandrum_instrument_session_complete_job(
                session,
                rejected,
                &callbacks,
                (&mut host as *mut TestHost).cast(),
                &mut revision,
                &mut undo_count,
                &mut redo_count,
                error.as_mut_ptr(),
                error.len(),
            )
        });
        assert_ne!(error[0], 0);
        assert_eq!((host.activations, host.publications), (3, 2));
        assert!(unsafe { dandrum_instrument_session_snapshot(session, &mut snapshot) });
        assert_eq!(
            (snapshot.revision, snapshot.undo_count, snapshot.redo_count),
            (1, 1, 0)
        );

        host.reject = false;
        assert!(unsafe { dandrum_instrument_session_connect(session, &mut connection) });
        let undo_job = unsafe {
            dandrum_instrument_session_admit_undo_for_connection(
                session,
                connection.instance_id,
                connection.attachment_id,
                5,
                1,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        assert!(!undo_job.is_null());
        assert!(unsafe {
            dandrum_instrument_session_disconnect(
                session,
                connection.instance_id,
                connection.attachment_id,
            )
        });
        assert!(unsafe { dandrum_instrument_session_prepare_job(undo_job) });
        assert!(unsafe {
            dandrum_instrument_session_complete_job(
                session,
                undo_job,
                &callbacks,
                (&mut host as *mut TestHost).cast(),
                &mut revision,
                &mut undo_count,
                &mut redo_count,
                error.as_mut_ptr(),
                error.len(),
            )
        });
        assert_eq!((revision, undo_count, redo_count), (2, 0, 1));
        assert!(unsafe { dandrum_instrument_session_snapshot(session, &mut snapshot) });
        assert_eq!(snapshot.next_request_id, 6);

        unsafe { dandrum_instrument_session_destroy(session) };
    }
}
