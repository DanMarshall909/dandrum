//! Disposable X11 projection of the persistent controller; no borrowed host in callbacks.
use super::{Core, Main};
use clack_extensions::gui::{GuiApiType, GuiConfiguration, GuiSize, PluginGuiImpl, Window};
use clack_plugin::prelude::PluginError;
use dandrum_trigger_editor::{EditorAssets, NativeEditor};
use dandrum_trigger_host::{
    parameters::ParameterError,
    protocol::{Admission, EditorCommand, MESSAGE_BYTES_LIMIT, ProtocolError},
};
use std::{
    ffi::CString, mem::MaybeUninit, os::unix::ffi::OsStrExt, path::Path, rc::Rc, time::Instant,
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
pub(super) struct EditorState {
    created: bool,
    attaching: bool,
    attachment: Option<String>,
    child: Option<Rc<NativeEditor>>,
}

impl Core {
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
            let result = match controller.admit(message) {
                Ok(Admission::New(request)) => match request.command {
                    EditorCommand::GetSamplerSession => {
                        let Ok(reply) = controller.sampler_session_reply(&request) else {
                            return;
                        };
                        drop(controller);
                        self.deliver(&child, &reply);
                        return;
                    }
                    EditorCommand::NoteOn { .. } | EditorCommand::NoteOff { .. } => controller
                        .apply_editor_note(&request, Instant::now())
                        .map(|()| serde_json::Value::Null)
                        .map_err(|error| match error {
                            ParameterError::Protocol(error) => error,
                            _ => ProtocolError::InvalidArguments,
                        }),
                    _ => Err(ProtocolError::CapabilityUnavailable),
                },
                // No reply cache: never execute a duplicate again or invent success.
                Ok(Admission::Duplicate { .. }) => Err(ProtocolError::ConflictingRequest),
                Err(ParameterError::Protocol(error)) => Err(error),
                Err(_) => Err(ProtocolError::InvalidArguments),
            };
            controller.encode_editor_reply(attachment, request_id, result)
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
impl Drop for Core {
    fn drop(&mut self) {
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
        self.core.destroy_editor();
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
        let mut state = self.core.gui.try_borrow_mut().map_err(|_| rejected())?;
        if state.created {
            return Err(rejected());
        }
        state.created = true;
        Ok(())
    }
    fn destroy(&self) {
        self.core.destroy_editor();
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
