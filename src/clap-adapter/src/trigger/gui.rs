//! Disposable X11 projection of the persistent controller; no borrowed host in callbacks.
use super::{Core, Main};
use clack_extensions::gui::{GuiApiType, GuiConfiguration, GuiSize, PluginGuiImpl, Window};
use clack_plugin::prelude::PluginError;
use dandrum_trigger_editor::{EditorAssets, NativeEditor};
use dandrum_trigger_host::{
    parameters::{
        ParameterController, ParameterError, ParameterReply, ParameterValue, SessionJobProgress,
        SessionServiceWake,
    },
    protocol::{Admission, EditorCommand, MESSAGE_BYTES_LIMIT, ProtocolError},
};
use std::{
    cell::Cell,
    ffi::CString,
    mem::MaybeUninit,
    os::unix::ffi::OsStrExt,
    path::Path,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

const SIZE: GuiSize = GuiSize {
    width: 1200,
    height: 800,
};
fn rejected() -> PluginError {
    PluginError::Message("Unsupported Trigger editor operation or lifecycle")
}

// GNU dlfcn.h request value, not exposed by the selected libc crate's constants.
const RTLD_DL_LINKMAP: libc::c_int = 2;

fn ensure_native_code_residency(path: &Path) -> Result<(), PluginError> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| rejected())?;
    let mut info = MaybeUninit::<libc::Dl_info>::uninit();
    let mut executing = std::ptr::null_mut();
    // SAFETY: a private Rust function address belongs to this executing object,
    // not an interposable exported symbol. Both outputs point to writable locals;
    // the opaque link-map identity is compared only, never dereferenced.
    let found = unsafe {
        libc::dladdr1(
            ensure_native_code_residency as *const () as *const libc::c_void,
            info.as_mut_ptr(),
            &mut executing,
            RTLD_DL_LINKMAP,
        )
    };
    if found == 0 || executing.is_null() {
        return Err(rejected());
    }
    let flags = libc::RTLD_NOLOAD | libc::RTLD_NOW | libc::RTLD_LOCAL;
    // SAFETY: the Entry path is NUL-terminated with its exact OS bytes. NOLOAD
    // acquires only an existing object; it does not load a replacement artifact.
    let lookup = unsafe { libc::dlopen(path.as_ptr(), flags) };
    if lookup.is_null() {
        return Err(rejected());
    }
    let mut declared: *mut libc::c_void = std::ptr::null_mut();
    // SAFETY: lookup is a live loader handle; the output is an opaque pointer.
    let matches = unsafe {
        libc::dlinfo(
            lookup,
            libc::RTLD_DI_LINKMAP,
            (&mut declared as *mut *mut libc::c_void).cast(),
        )
    } == 0
        && declared == executing;
    // Keep the verified lookup reference alive throughout promotion. There are
    // no fallible Rust calls or early returns until both acquired handles close.
    let promoted = if matches {
        // SAFETY: only the verified, already-loaded path is promoted, without GLOBAL.
        unsafe { libc::dlopen(path.as_ptr(), flags | libc::RTLD_NODELETE) }
    } else {
        std::ptr::null_mut()
    };
    // SAFETY: close exactly once for each successful open; no handle is retained
    // in an application owner or global. Check close failures before GTK starts.
    let promoted_closed = !promoted.is_null() && unsafe { libc::dlclose(promoted) } == 0;
    let lookup_closed = unsafe { libc::dlclose(lookup) } == 0;
    if promoted_closed && lookup_closed {
        Ok(())
    } else {
        Err(rejected())
    }
}

#[derive(Default)]
struct WorkerWake(AtomicBool);
impl SessionServiceWake for WorkerWake {
    fn request_service(&self) {
        self.0.store(true, Ordering::Release);
    }
}

#[derive(Default)]
pub(super) struct EditorState {
    created: bool,
    attaching: bool,
    attachment: Option<String>,
    child: Option<Rc<NativeEditor>>,
    source: Option<gtk::glib::SourceId>,
    worker_wake: Arc<WorkerWake>,
    job_pending: bool,
}

impl EditorState {
    pub(super) fn start_worker(
        &self,
        controller: &mut ParameterController,
    ) -> Result<(), PluginError> {
        controller
            .start_session_worker(self.worker_wake.clone())
            .map_err(|_| rejected())
    }
}

impl Core {
    fn ensure_service_source(self: &Rc<Self>) {
        if self.gui.borrow().source.is_some() {
            return;
        }
        let weak = Rc::downgrade(self);
        let source =
            gtk::glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
                let Some(core) = weak.upgrade() else {
                    return gtk::glib::ControlFlow::Break;
                };
                core.service_editor();
                gtk::glib::ControlFlow::Continue
            });
        self.gui.borrow_mut().source = Some(source);
    }
    fn cancel_service_source(&self) {
        let source = self.gui.borrow_mut().source.take();
        if let Some(source) = source {
            source.remove();
        }
    }
    pub(super) fn service_editor(&self) {
        if let Ok(mut controller) = self.controller.try_borrow_mut() {
            let _ = controller.service_editor_notes(Instant::now());
        }
        self.service_structural_jobs();
        self.service_automation();
        for _ in 0..64 {
            let target = {
                let Ok(state) = self.gui.try_borrow() else {
                    return;
                };
                if state.created {
                    state
                        .attachment
                        .as_ref()
                        .zip(state.child.as_ref())
                        .map(|(attachment, child)| (attachment.clone(), Rc::clone(child)))
                } else {
                    None
                }
            };
            let event = {
                let Ok(mut controller) = self.controller.try_borrow_mut() else {
                    return;
                };
                let Some(feedback) = controller.take_feedback() else {
                    return;
                };
                target.as_ref().and_then(|(attachment, _)| {
                    controller.encode_editor_event(attachment, "parameterStateChanged", serde_json::json!({
                        "generation": feedback.generation, "sequence": feedback.sequence,
                        "reset_cancelled": feedback.reset_cancelled, "frame_offset": feedback.frame_offset,
                        "parameters": parameter_values(feedback.parameters),
                    })).ok()
                })
            };
            if let (Some((_, child)), Some(event)) = (target, event) {
                self.deliver(&child, &event);
            }
        }
    }
    fn service_structural_jobs(&self) {
        let Ok(mut controller) = self.controller.try_borrow_mut() else {
            return;
        };
        let Ok(mut state) = self.gui.try_borrow_mut() else {
            return;
        };
        // Only an endpoint returned by CLAP deactivation may be serviced here.
        // Active audio owns its endpoint independently and swaps on real blocks.
        if let Some(mut audio) = self.parked.take() {
            let _ = controller.service_quiescent(&mut audio);
            self.parked.set(Some(audio));
        }
        if state.worker_wake.0.swap(false, Ordering::AcqRel) || state.job_pending {
            // A deferred completion remains owned by the controller. Periodic
            // service retries after audio releases capacity, even without a view.
            state.job_pending = matches!(
                controller.service_session_jobs(),
                Ok(SessionJobProgress::Pending)
            );
        }
    }
    fn cleanup_attachment(&self) -> Result<(), PluginError> {
        let mut controller = self.controller.try_borrow_mut().map_err(|_| rejected())?;
        let mut state = self.gui.try_borrow_mut().map_err(|_| rejected())?;
        if let Some(attachment) = state.attachment.take() {
            controller
                .detach_editor(&attachment)
                .map_err(|_| rejected())?;
        }
        Ok(())
    }
    fn destroy_editor(&self) {
        // Invalidate callback authority before any foreign browser teardown.
        let child = {
            let mut state = self.gui.borrow_mut();
            state.created = false;
            state.attaching = false;
            state.child.take()
        };
        // A reentrant borrow conflict retains the attachment for the next create;
        // the invalidated view cannot dispatch it. Final Core drop owns it directly.
        let _ = self.cleanup_attachment();
        drop(child);
    }
    fn child(&self) -> Result<Rc<NativeEditor>, PluginError> {
        self.gui
            .try_borrow()
            .map_err(|_| rejected())?
            .child
            .clone()
            .ok_or_else(rejected)
    }
    fn set_parent(self: &Rc<Self>, window: Window) -> Result<(), PluginError> {
        let parent = window
            .as_x11_handle()
            .and_then(|p| u32::try_from(p).ok())
            .filter(|p| *p != 0)
            .ok_or_else(rejected)?;
        {
            let state = self.gui.try_borrow().map_err(|_| rejected())?;
            if !state.created || state.attaching || state.child.is_some() {
                return Err(rejected());
            }
        }
        self.cleanup_attachment()?;
        let assets = EditorAssets::open(&self.resource_root).map_err(|_| rejected())?;
        if gtk::is_initialized() && !gtk::is_initialized_main_thread() {
            return Err(rejected());
        }
        ensure_native_code_residency(&self.plugin_path)?;
        gtk::init().map_err(|_| rejected())?;
        self.ensure_service_source();
        let connection = self
            .controller
            .try_borrow_mut()
            .map_err(|_| rejected())?
            .attach_editor()
            .map_err(|_| rejected())?;
        let initializer = format!(
            "window.__DANDRUM_NATIVE__={};",
            serde_json::json!({"protocolVersion":1,"instanceId":connection.instance_id,"attachmentId":connection.attachment_id,"nextRequestId":"1"})
        );
        {
            let mut state = self.gui.borrow_mut();
            state.attaching = true;
            state.attachment = Some(connection.attachment_id.clone());
        }
        let weak = Rc::downgrade(self);
        let attachment = connection.attachment_id;
        let result =
            NativeEditor::create_initialized(parent, assets, &initializer, move |message| {
                if let Some(core) = weak.upgrade() {
                    core.receive(&attachment, &message);
                }
            });
        self.gui.borrow_mut().attaching = false;
        match result {
            Ok(child) => {
                self.gui.borrow_mut().child = Some(Rc::new(child));
                Ok(())
            }
            Err(_) => {
                self.cleanup_attachment()?;
                Err(rejected())
            }
        }
    }
    fn receive(&self, attachment: &str, message: &str) {
        if message.len() > MESSAGE_BYTES_LIMIT {
            return;
        }
        let child = {
            let Ok(state) = self.gui.try_borrow() else {
                return;
            };
            if !state.created || state.attachment.as_deref() != Some(attachment) {
                return;
            }
            let Some(child) = state.child.clone() else {
                return;
            };
            child
        };
        // Only a bounded canonical ID in this current attachment may receive a reply.
        let Ok(frame) = serde_json::from_str::<serde_json::Value>(message) else {
            return;
        };
        if frame["attachmentId"].as_str() != Some(attachment) {
            return;
        }
        let Some(request_id) = frame["requestId"].as_str() else {
            return;
        };
        if request_id.len() > 20
            || !request_id
                .parse::<u64>()
                .is_ok_and(|n| n != 0 && n.to_string() == request_id)
        {
            return;
        }
        let reply = {
            let Ok(mut controller) = self.controller.try_borrow_mut() else {
                return;
            };
            dispatch(&mut controller, attachment, request_id, message)
        };
        if let Ok(reply) = reply {
            self.deliver(&child, &reply);
        }
    }
    fn deliver(&self, child: &NativeEditor, reply: &str) {
        // The broker receives a JSON STRING; successful queueing is not consumption.
        let script = format!(
            "window.__DANDRUM__.receiveNativeMessage({});",
            serde_json::to_string(reply).expect("String JSON serialization")
        );
        let _ = child.evaluate_script(&script);
    }
}
fn dispatch(
    controller: &mut dandrum_trigger_host::parameters::ParameterController,
    attachment: &str,
    request_id: &str,
    message: &str,
) -> Result<String, ParameterError> {
    let result = match controller.admit(message) {
        Ok(Admission::New(request)) => match request.command {
            EditorCommand::GetSamplerSession => return controller.sampler_session_reply(&request),
            EditorCommand::GetInstrumentSessionJobStatus { .. } => {
                match controller.session_job_status(&request) {
                    Ok(reply) => return Ok(reply),
                    Err(error) => Err(command_error(error)),
                }
            }
            EditorCommand::SetZoneKeyRange { .. } | EditorCommand::UndoInstrumentSession { .. } => {
                match controller.admit_session_command(&request) {
                    Ok(reply) => return Ok(reply),
                    Err(error) => Err(command_error(error)),
                }
            }
            EditorCommand::NoteOn { .. }
            | EditorCommand::NoteOff { .. }
            | EditorCommand::NoteHeartbeat => controller
                .apply_editor_note(&request, Instant::now())
                .map(|()| serde_json::Value::Null)
                .map_err(|error| match error {
                    ParameterError::Protocol(error) => error,
                    _ => ProtocolError::InvalidArguments,
                }),
            EditorCommand::GetParameterState
            | EditorCommand::BeginGesture { .. }
            | EditorCommand::SetParameter { .. }
            | EditorCommand::EndGesture { .. } => controller
                .apply(&request)
                .map(parameter_reply)
                .map_err(command_error),
        },
        Ok(Admission::Duplicate { .. }) => Err(ProtocolError::DuplicateRequest),
        Err(ParameterError::Protocol(error)) => Err(error),
        Err(_) => Err(ProtocolError::InvalidArguments),
    };
    controller.encode_editor_reply(attachment, request_id, result)
}

fn parameter_values(parameters: Vec<ParameterValue>) -> serde_json::Value {
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
fn command_error(error: ParameterError) -> ProtocolError {
    match error {
        ParameterError::QueueFull => ProtocolError::Busy,
        _ => ProtocolError::CommandRejected,
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        self.cancel_service_source();
        // Close admission and join preparation on the owning main thread before
        // controller/session/runtime fields can be destroyed.
        let _ = self.controller.get_mut().shutdown_session_worker();
        let state = self.gui.get_mut();
        state.created = false;
        if let Some(attachment) = state.attachment.take() {
            let _ = self.controller.get_mut().detach_editor(&attachment);
        }
        drop(state.child.take());
    }
}
impl Drop for Main<'_> {
    fn drop(&mut self) {
        self.creating.set(false);
        self.timer.disable();
        self.core.cancel_service_source();
        self.core.destroy_editor();
        let _ = self.timer.remove(|id| self.remove_host_timer(id));
    }
}
impl PluginGuiImpl for Main<'_> {
    fn is_api_supported(&self, configuration: GuiConfiguration) -> bool {
        configuration.api_type == GuiApiType::X11 && !configuration.is_floating
    }
    fn get_preferred_api(&self) -> Option<GuiConfiguration<'_>> {
        Some(GuiConfiguration {
            api_type: GuiApiType::X11,
            is_floating: false,
        })
    }
    fn create(&self, configuration: GuiConfiguration) -> Result<(), PluginError> {
        if !self.is_api_supported(configuration) {
            return Err(rejected());
        }
        {
            let state = self.core.gui.try_borrow().map_err(|_| rejected())?;
            if state.created || self.creating.get() {
                return Err(rejected());
            }
            self.creating.set(true);
        }
        let newly_registered = match self.timer.prepare(
            || {
                self.host
                    .get_extension::<clack_extensions::timer::HostTimer>()
                    .ok_or_else(rejected)?
                    .register_timer(&self.host, 16)
                    .map_err(|_| rejected())
            },
            |id| self.remove_host_timer(id),
        ) {
            Ok(newly_registered) => newly_registered,
            Err(error) => {
                self.creating.set(false);
                return Err(error);
            }
        };
        let committed = if let Ok(mut state) = self.core.gui.try_borrow_mut() {
            if self.creating.replace(false) && !state.created {
                state.created = true;
                true
            } else {
                false
            }
        } else {
            false
        };
        if !committed {
            self.creating.set(false);
            if newly_registered {
                let _ = self.timer.remove(|id| self.remove_host_timer(id));
            }
            return Err(rejected());
        }
        self.timer.enable()
    }

    fn destroy(&self) {
        self.creating.set(false);
        self.core.destroy_editor();
        self.service_parameter_delivery();
    }
    fn set_scale(&self, _: f64) -> Result<(), PluginError> {
        Err(rejected())
    }
    fn get_size(&self) -> Option<GuiSize> {
        self.core.gui.try_borrow().ok()?.created.then_some(SIZE)
    }
    fn set_size(&self, size: GuiSize) -> Result<(), PluginError> {
        if self.core.gui.try_borrow().map_err(|_| rejected())?.created && size == SIZE {
            Ok(())
        } else {
            Err(rejected())
        }
    }
    fn set_parent(&self, window: Window) -> Result<(), PluginError> {
        self.core.set_parent(window)
    }
    fn set_transient(&self, _: Window) -> Result<(), PluginError> {
        Err(rejected())
    }
    fn show(&self) -> Result<(), PluginError> {
        self.core.child()?.set_visible(true).map_err(|_| rejected())
    }
    fn hide(&self) -> Result<(), PluginError> {
        self.core
            .child()?
            .set_visible(false)
            .map_err(|_| rejected())
    }
}

#[cfg(test)]
#[path = "../../tests/internal/trigger_gui_dispatch.rs"]
mod dispatch_tests;

#[cfg(test)]
#[path = "../../tests/internal/trigger_timer.rs"]
mod timer_tests;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum TimerState {
    #[default]
    Empty,
    Registering,
    Disabled(clack_extensions::timer::TimerId),
    Live(clack_extensions::timer::TimerId),
    Removing(clack_extensions::timer::TimerId),
}
#[derive(Default)]
pub(super) struct TimerRegistration(Cell<TimerState>);
impl TimerRegistration {
    fn prepare(
        &self,
        register: impl FnOnce() -> Result<clack_extensions::timer::TimerId, PluginError>,
        unregister: impl FnOnce(clack_extensions::timer::TimerId) -> Result<(), PluginError>,
    ) -> Result<bool, PluginError> {
        match self.0.get() {
            TimerState::Live(_) => return Ok(false),
            TimerState::Disabled(_) => self.remove(unregister)?,
            TimerState::Empty => {}
            TimerState::Registering | TimerState::Removing(_) => return Err(rejected()),
        }
        self.0.set(TimerState::Registering);
        match register() {
            Ok(id) => {
                self.0.set(TimerState::Disabled(id));
                Ok(true)
            }
            Err(error) => {
                self.0.set(TimerState::Empty);
                Err(error)
            }
        }
    }
    fn enable(&self) -> Result<(), PluginError> {
        match self.0.get() {
            TimerState::Disabled(id) | TimerState::Live(id) => {
                self.0.set(TimerState::Live(id));
                Ok(())
            }
            _ => Err(rejected()),
        }
    }
    fn disable(&self) {
        if let TimerState::Live(id) = self.0.get() {
            self.0.set(TimerState::Disabled(id));
        }
    }
    fn is_live(&self, id: clack_extensions::timer::TimerId) -> bool {
        self.0.get() == TimerState::Live(id)
    }
    fn remove(
        &self,
        unregister: impl FnOnce(clack_extensions::timer::TimerId) -> Result<(), PluginError>,
    ) -> Result<(), PluginError> {
        let id = match self.0.get() {
            TimerState::Empty => return Ok(()),
            TimerState::Disabled(id) | TimerState::Live(id) => id,
            TimerState::Registering | TimerState::Removing(_) => return Err(rejected()),
        };
        self.0.set(TimerState::Removing(id));
        match unregister(id) {
            Ok(()) => {
                self.0.set(TimerState::Empty);
                Ok(())
            }
            Err(error) => {
                self.0.set(TimerState::Disabled(id));
                Err(error)
            }
        }
    }
}
impl Main<'_> {
    fn remove_host_timer(&self, id: clack_extensions::timer::TimerId) -> Result<(), PluginError> {
        self.host
            .get_extension::<clack_extensions::timer::HostTimer>()
            .ok_or_else(rejected)?
            .unregister_timer(&self.host, id)
            .map_err(|_| rejected())
    }
}
impl clack_extensions::timer::PluginTimerImpl for Main<'_> {
    fn on_timer(&self, id: clack_extensions::timer::TimerId) {
        if self.timer.is_live(id) {
            self.service_parameter_delivery();
        }
    }
}
